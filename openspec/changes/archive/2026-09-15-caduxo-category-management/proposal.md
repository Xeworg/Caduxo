# Proposal: caduxo-category-management

> **Naming note.** The change directory is `caduxo-category-management` and the
> slice now goes well beyond a UX picker swap: it lands a database migration
> (`V4: product_categories`), a many-to-many product-category model, a new
> category search API, two updated filter surfaces, a new picker primitive, and
> the service-layer case-fold guard. The session preflight pins the rails
> (`executionMode: interactive`, `artifactStore: both`, `deliveryStrategy:
> ask-on-risk`) and the user typed a session **3,000-line** review-budget
> override for this change (project default 800; canonical threshold 400).
> The estimate below lands comfortably under 3,000 lines, so a single
> implementation slice / single PR stays the right shape and a chained-PR
> split is not required. No `size:exception` needed.

## Research note: picker UX substrate

The picker UX decisions cite a parent UX-web brief already on file and the
following named references from that brief:

- **USWDS combo box** — searchable combobox pattern with chips, Clear all,
  and keyboard / focus accessibility.
- **WAI-ARIA combobox pattern** — `role="combobox"`, `aria-expanded`,
  `aria-controls`, `aria-activedescendant`, focus management on open and
  selection.
- **GOV.UK long-list guidance** — preferred behaviour for long lists
  (search-first, paginated results, deliberate creation).
- **NN/g dropdown-list article** — searchable results plus removable chips,
  keyboard / focus ergonomics for multi-select.
- **Adobe Commerce category admin docs** — controlled creation flow,
  archive over hard-delete, no implicit category from arbitrary text.

This proposal treats the brief's named sources as the only authoritative
references for picker UX decisions. No additional sources are invented
beyond the brief, and no exact citations are added for sources not present
in the explore document.

## Intent

Two user-visible outcomes in one slice, sharing one category data model:

1. **Real multi-category support.** Products may belong to one, many, or
   zero categories. A product with no categories is valid and shows as
   "Uncategorized" wherever categories are listed or filtered. Archive
   (`is_active = 0`) hides a category from pickers while preserving history
   and junction rows.
2. **Filter-by-multiple-categories with `ANY-of` semantics.** Both the
   catalog filter (`ProductCatalogPage`) and the reports filter
   (`ReportsPage`) accept a multi-selection of categories. Lots whose
   product belongs to **any** of the selected categories appear exactly
   once (no duplicate counts). Selection of "Uncategorized" alone returns
   products with zero active category relations.

The two outcomes share one schema (`product_categories` is the new source
of truth), one category search API, and one picker primitive
(`CategoryPicker.svelte`), so neither outcome carries a duplicate
category-read path or a parallel multi-select implementation.

## Scope

### In scope (this slice)

#### A. Schema — `V4: product_categories` migration

A new migration in `src-tauri/src/db/migrations.rs` introduces a junction
table and demotes `products.category_id` from "source of truth" to
"ignored legacy data" if SQLite makes dropping the column costly.

- **New table** `product_categories`:
  - `product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE`
  - `category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE RESTRICT`
  - `created_at TEXT NOT NULL`
  - **Primary key** `(product_id, category_id)` — composite uniqueness.
  - **Index** `idx_product_categories_category ON product_categories
    (category_id)` for "products in category X" report / catalog lookups.
  - **Index** `idx_product_categories_product ON product_categories
    (product_id)` for "categories for product X" detail-bundle reads
    (the PK already covers product-leading lookups; this is a redundant
    safety index for symmetry with the unit-catalog pattern in V3).
- **Back-fill** — idempotent insert from `products.category_id`:
  `INSERT INTO product_categories (product_id, category_id, created_at)
  SELECT p.id, p.category_id, p.created_at FROM products p WHERE
  p.category_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM
  product_categories pc WHERE pc.product_id = p.id AND pc.category_id =
  p.category_id);`. Re-running V4 on a partially-migrated DB is a no-op.
- **Legacy `products.category_id`:** drop it if the project's SQLite
  build supports `ALTER TABLE ... DROP COLUMN` (SQLite ≥ 3.35); otherwise
  leave the column in place but **stop writing to it** and stop reading
  from it for runtime decisions. Whichever path is taken, the column is
  not the source of truth anymore — `product_categories` is.
- **Existing-data case-fold duplicates** — applied as part of V4 in the
  same migration: when two `categories` rows differ only in case (e.g.
  `Dairy` and `dairy`), the **oldest** row wins and remains the canonical
  category id. Younger duplicates are flipped to `is_active = 0` (archived)
  and their `product_categories` rows (and, if legacy still exists,
  `products.category_id` rows) are reassigned to the canonical id. No
  hard delete. The migration logs how many duplicates were found and
  remapped.

#### B. DTOs

- `ProductCreate.category_ids: Option<Vec<String>>` — empty `Vec` or `None`
  means "unassigned".
- `ProductUpdate.category_ids: Option<Vec<String>>` — full set on each
  update (delete + insert inside a transaction).
- `ProductResponse.category_ids: Vec<String>` — always present, possibly
  empty. The legacy `category_id` field is **renamed** (no deprecation
  cycle; the app is pre-1.0 and there is no external consumer).
- `ProductDetailResponse.categories: Vec<CategoryResponse>` — replaces the
  single `category: Option<CategoryResponse>`.
- `ProductSearchResult.category_ids: Vec<String>` — replaces `category_id`.
- `DashboardFilters.category_ids: Option<Vec<String>>` — new field; `None`
  or empty list means "no filter".
- `ReportFilters.category_ids: Option<Vec<String>>` — replaces `category_id`.

#### C. Repositories

- `insert_product` / `update_product` — write through the junction
  (transactional delete-then-insert on update; replace the single-FK
  write).
- `get_product` — returns `category_ids: Vec<String>` from a single junction
  read.
- `search_products` and `find_by_*_exact` — return `category_ids: Vec<String>`
  from the junction.
- `list_all_products_for_export` — joins category names through the
  junction and emits a **comma-joined names column** for CSV export so
  consumers of the existing single-category CSV continue to parse the
  first name (back-compat).
- `list_dashboard_lots` — accepts `category_ids: Option<Vec<String>>` and
  applies it as an **`EXISTS` subquery** (not a `JOIN`), so no product /
  lot row is double-counted.
- **New helper** `list_product_category_ids_by_product_ids(ids:
  &[String]) -> HashMap<String, Vec<String>>` — batched read for the
  reports service and any consumer that needs all category ids across a
  set of products in one round trip. Replaces the per-row `get_product`
  N+1 in `services::reports::filter_by_category`.

#### D. Services

- `services::products::create_product` / `update_product` — validate
  `category_ids`: empty `Vec` or `None` allowed (unassigned); every id
  must exist; archived categories may be referenced only if the caller
  passes an explicit `include_archived: true` flag (default `false`).
  Writes go through the junction.
- `services::products::get_product` — resolves all category ids to
  `Vec<CategoryResponse>` via the batched helper (no per-row N+1).
- `services::reports::filter_by_category` — **removed**. Filter is
  SQL-level now; `ReportFilters.category_ids` flows down to
  `DashboardFilters.category_ids` and lands in the SQL via `EXISTS`.
- `services::csv_io::resolve_category_name` — unchanged. Single-category
  CSV import continues to work as today. Multi-category CSV import is
  out of scope for this slice (see Non-goals).
- **Service-layer case-fold guard** — `create_category` and
  `update_category` (rename path) check `LOWER(name)` for collisions
  before insert / update; SQLite `UNIQUE` violation on `categories.name`
  is translated to `DuplicateField { field: "name" }` on the conflict
  path. Documented in the design phase. Pre-existing case-only duplicates
  are resolved in V4 (see §A); the guard prevents new duplicates.

#### E. New category search API

A new Tauri command exposes category search so the picker can scale to
200+ active categories without rendering the full list.

- **Command:** `list_categories_search({ query: String, limit: usize })
  -> CategorySearchPage`.
- **Response shape:** `CategorySearchPage { items: Vec<CategoryResponse>,
  total: usize, has_more: bool }`.
- **Semantics:** case-insensitive, **prefix match first**, substring
  fallback when no prefix results, ordered by name asc. `limit` defaults
  to 50. `has_more = total > items.len()` so the picker knows when more
  results exist.
- **Filter:** active categories only (`is_active = 1`). Archived
  categories are excluded by default; the picker does not surface an
  explicit "show archived" toggle in this slice.
- **Uncategorized pseudo-row:** the picker treats `UNCATEGORIZED_SENTINEL`
  as a synthetic category id representing "products with zero active
  category relations". The pseudo-row is rendered as a special chip /
  picker row and **does not** appear in `list_categories_search` results
  — it is added by the picker itself.

#### F. Frontend — picker primitive `CategoryPicker.svelte`

A new component used by `ProductForm.svelte`, `ProductCatalogPage.svelte`,
and `ReportsPage.svelte`. Composed of:

- A search input (`<input type="text" role="combobox"
  aria-expanded="true|false" aria-controls="…" aria-activedescendant="…">`).
- A floating popover with the result list (`position: absolute` relative
  to the trigger via `bind:this` + `getBoundingClientRect`, same family
  as the existing date picker popover from `caduxo-custom-date-picker`).
- A chip row of selected categories above the input. Each chip is
  removable via an inline `×` button (`aria-label="Remove <name>"`).
- A "Clear all" link below the chips when at least one chip is present.
- An inline-create row at the bottom of the popover: when the typed
  query has **no match** in the result list, a `Create "<query>"` action
  appears. Clicking it issues a `create_category` Tauri command and adds
  the new category to the selection. **No silent arbitrary text** — the
  user must click `Create "X"` explicitly.

WAI-ARIA combobox pattern: `role="combobox"`, `aria-expanded`,
`aria-controls`, `aria-activedescendant`, full keyboard ergonomics
(`Tab` enters / leaves, `Esc` closes, `Arrow Up` / `Arrow Down` move the
active descendant, `Enter` selects, `Backspace` in an empty input
removes the last chip).

#### G. Frontend — host adoptions

| # | File | Change |
|---|---|---|
| 1 | `src/components/ProductForm.svelte` | Replace `let categoryId: string = ""` and the empty `<select>` (~line 326) and the `<ul class="category-pills">` (~line 330) with `let categoryIds: string[] = []` and `<CategoryPicker bind:value={categoryIds} includeUncategorized={false} />`. Submit payload uses `category_ids: categoryIds`. |
| 2 | `src/components/ProductDetailPage.svelte` (~line 207) | Render `detail.categories.map(c => c.name).join(', ')` or one badge per category instead of the single `.category-badge`. |
| 3 | `src/components/DashboardPage.svelte` (~line 606) | Render `detailProduct.categories.map(c => c.name).join(', ')` in the detail modal. |
| 4 | `src/components/ProductCatalogPage.svelte` | Add a multi-select category filter (uses `CategoryPicker`) bound to `categoryIds: string[]`. Passes through `category_ids` to the catalog query. Default empty selection = no filter (not an explicit All chip). Selection may include the `Uncategorized` pseudo-row. |
| 5 | `src/components/ReportsPage.svelte` (~line 353, ~line 143) | Replace `<select bind:value={categoryId}>` with `<CategoryPicker bind:value={categoryIds} includeUncategorized={true} />`. `buildFilters()` uses `category_ids: categoryIds`. Filter summary (~line 265) renders `category=<n>` (joined ids) or `n categories`. |
| 6 | `src/lib/products.ts`, `src/lib/dashboard.ts`, `src/lib/reports.ts` | TS DTOs mirror the new Rust DTOs. |

#### H. Restore-from-backup adapter

Restore / import of pre-V4 backups (Slice 12) translates any
`products.category_id` rows from the backup file into junction rows at
restore time. The adapter runs before the junction insert so the restored
DB ends up in the V4-onwards shape. No on-disk re-migration is required
by the user.

#### I. Spec delta on `openspec/specs/caduxo-expiry-tracker/spec.md`

- **`## Capability: Categories > editable category list`** — **MODIFIED
  in place** to clarify: products may belong to one, many, or zero
  categories; unassigned is valid; archived categories are hidden from
  pickers but preserved in history.
- **`## Capability: Categories`** — **ADDED** requirement `multi-category
  product model` covering the junction table, composite uniqueness, and
  the `EXISTS` filter semantics.
- **`## Capability: Product catalog`** — **MODIFIED** `mandatory unique
  SKU` / product registration pointers to clarify `category_ids` is a
  list (zero or more).
- **`## Capability: Reports > report filters`** — **MODIFIED** to say
  `category_ids` (any-of), default no filter, includes an
  `Uncategorized` pseudo-row.
- **`## Capability: Dashboard`** — **MODIFIED** to add a `category_ids`
  filter (any-of) to the operational main screen.
- **`## Capability: Data persistence and safety > migrations`** —
  pointer edit to note V4 introduces `product_categories` and the
  case-fold deduplication step.

### Out of scope (Non-goals)

- **Category admin page.** No virtualized list, no archive / restore UI,
  no rename UI, no product counts per category in this slice. Archive is
  possible via the existing `update_category({ is_active: false })` call
  from any future code path, but no UI is built in this slice.
- **Merge-duplicates UI.** Case-only deduplication is a one-time
  V4 migration step, not a UI feature.
- **Hierarchy / subcategories.** The schema does not support a tree of
  categories in this slice and the picker does not display hierarchy.
- **Analytics by category.** No category-level KPIs, charts, or trend
  views in this slice.
- **POS, billing, payments, accounting, full inventory.** Already in the
  project config's `product.nonGoals`; restated here for the record.
- **Multi-category CSV import.** Existing single-category CSV files
  continue to work; semicolon-separated or multiple-columns CSV is
  deferred. The import path treats a row with `category` only as a single
  category (status quo).
- **Fuzzy category search.** Case-insensitive prefix first, substring
  fallback; no fuzzy / Levenshtein / typo-tolerance in this slice.
- **Maximum categories per product.** No artificial cap.
- **Vitest / Playwright / Cypress harness.** The project has no frontend
  test harness; `strictTdd: false`; manual smoke is the canonical gate.
- **Locale / i18n** for picker labels.

## Affected areas

| Area | Change | Approx. LOC delta |
|---|---|---|
| `src-tauri/src/db/migrations.rs` | New `V4: product_categories` migration; indexes; idempotent back-fill; case-fold dedup step; legacy-column drop attempt (or leave-and-ignore fallback). | 120–180 |
| `src-tauri/src/db/repositories/products.rs` | `insert_product` / `update_product` write through junction; `get_product` / `search_products` / `find_by_*_exact` return `category_ids`; `list_all_products_for_export` joins through junction with comma-joined names; new helper `list_product_category_ids_by_product_ids`. | 180–280 |
| `src-tauri/src/db/repositories/dashboard.rs` | `list_dashboard_lots` accepts `category_ids: Option<Vec<String>>` and applies it as `EXISTS` (no double count). | 40–80 |
| `src-tauri/src/dto/products.rs` | Replace `category_id` with `category_ids: Vec<String>` across `ProductCreate` / `ProductUpdate` / `ProductResponse` / `ProductDetailResponse` / `ProductSearchResult`. | 30–60 |
| `src-tauri/src/dto/dashboard.rs` | Add `category_ids: Option<Vec<String>>` to `DashboardFilters`. | 10–20 |
| `src-tauri/src/dto/reports.rs` | Replace `category_id` with `category_ids: Option<Vec<String>>` on `ReportFilters`. | 10–20 |
| `src-tauri/src/dto/categories.rs` (new or extended) | New `CategorySearchPage { items, total, has_more }`. | 20–40 |
| `src-tauri/src/services/products.rs` | `create_product` / `update_product` validate `category_ids`; `get_product` resolves via batched helper; case-fold guard on `create_category` and `update_category` (rename path). | 80–140 |
| `src-tauri/src/services/reports.rs` | Remove `filter_by_category` post-filter; `to_dashboard_filters` passes `category_ids` through. | 20–40 |
| `src-tauri/src/services/csv_io.rs` | Unchanged. | 0 |
| `src-tauri/src/services/restore.rs` (or wherever restore lives) | Pre-V4 backup adapter: translate `products.category_id` rows into junction rows at restore time. | 40–80 |
| `src-tauri/src/commands/products.rs`, `dashboard.rs`, `reports.rs`, `categories.rs` | Wire new DTO fields and the new `list_categories_search` command. Register the command in `src-tauri/src/lib.rs`. | 30–60 |
| `src-tauri/src/lib.rs` | Register the new `list_categories_search` command. | 5–10 |
| Backend tests | New unit tests for: V4 back-fill correctness, V4 idempotency, case-fold guard (lowercase / uppercase / mixed duplicates), multi-category round-trip, update replaces full set, report filter any-of, report filter no double count, archive flow stays RESTRICT-enforced. | 200–320 |
| `src/components/inputs/CategoryPicker.svelte` (new) | Searchable multi-select combobox with chips, `Clear all`, inline `Create "X"`, keyboard ergonomics, `Uncategorized` pseudo-row. | 300–450 |
| `src/components/ProductForm.svelte` | Replace `categoryId` / dead `<select>` / pill `<ul>` with `CategoryPicker`; submit payload uses `category_ids`. | 40–80 |
| `src/components/ProductDetailPage.svelte` | Multiple category badges or comma-joined label. | 10–25 |
| `src/components/DashboardPage.svelte` | Detail modal renders comma-joined names or list. | 10–25 |
| `src/components/ProductCatalogPage.svelte` | New `CategoryPicker` filter wired to `categoryIds: string[]`; default empty = no filter. | 30–60 |
| `src/components/ReportsPage.svelte` | Replace single `<select>` with `CategoryPicker`; `buildFilters()` uses `category_ids`; filter summary uses joined ids or `n categories`. | 40–80 |
| `src/lib/products.ts`, `dashboard.ts`, `reports.ts` | TS DTOs mirror new Rust DTOs. | 30–60 |
| `src/lib/categories.ts` (new or extended) | `listCategoriesSearch(query, limit)` wrapper around the new Tauri command; `UNCATEGORIZED_SENTINEL` constant. | 20–40 |
| CSS (component-scoped) | Visual styling for `CategoryPicker` + minor adjustments in host pages. | 80–150 |
| `openspec/specs/caduxo-expiry-tracker/spec.md` | One MODIFIED + one ADDED under `Categories`; one MODIFIED under `Product catalog`; one MODIFIED under `Reports > report filters`; one MODIFIED under `Dashboard`; one pointer edit under `Data persistence and safety > migrations`. | 50–90 |
| `docs/prd.md` | Aligned references to multi-category support, picker, and filter semantics. | 10–30 |
| **Total estimate** | | **~1,400–2,400 LOC** |

The 1,400–2,400 LOC envelope deliberately spans a wide range because the
realistic size depends on design decisions locked in `design.md`
(popover positioning, picker chip layout, inline-create confirmation
copy, no-match rendering, `Uncategorized` pseudo-row treatment, restore
adapter path). The lower bound assumes a tight implementation reusing
existing CSS variables; the upper bound assumes a polished picker with
arrow + flip-up + outside-click handler + full a11y attributes + restore
adapter. The estimate stays comfortably under the **3,000-line session
budget** the user typed.

## Reusable primitives — single source of truth for category reads

To avoid parallel category-read paths in the codebase, the proposal pins
the responsibility split as follows:

| Concern | Owned by | Notes |
|---|---|---|
| Junction table + indexes + back-fill | `db/migrations.rs` (`V4`) | Single migration; idempotent. |
| Junction reads per product | `list_product_category_ids_by_product_ids` in `db/repositories/products.rs` | Batched helper; replaces per-row `get_product` reads. |
| Junction reads for the report / catalog filter | `db/repositories/dashboard.rs::list_dashboard_lots` (`EXISTS`) | SQL-level filter; no double count. |
| Category search | `list_categories_search` Tauri command | Prefix then substring; active only. |
| Picker | `src/components/inputs/CategoryPicker.svelte` | Used by `ProductForm`, `ProductCatalogPage`, `ReportsPage`. |
| `Uncategorized` pseudo-row rendering | `CategoryPicker.svelte` + `src/lib/categories.ts` | One sentinel id; not persisted. |
| Restore-from-backup adapter | `services/restore.rs` (or wherever restore lives) | Translates pre-V4 backups into V4-onwards shape. |

The picker popover does not duplicate a category-read primitive. The
filters do not duplicate a junction-read primitive. Both compose the
batch helper or the SQL-level filter. A future change to category naming,
archive policy, or picker UX lands in one place.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Total LOC exceeds the 400/800 project review budgets | Reviewer fatigue; shallow review of a large cross-cutting slice | User explicitly raised this change's review budget to **3,000 lines**. Mitigate with a strict shared-primitive design and focused verify evidence. |
| SQLite `ALTER TABLE ... DROP COLUMN` fails on the user's build | Legacy `products.category_id` remains, creates a "where is the source of truth?" ambiguity | Fallback: keep the column, stop writing to it, stop reading from it. The column becomes ignored legacy data; `product_categories` is canonical. |
| Existing case-only duplicates (`Dairy` vs `dairy`) appear in real data | Junction back-fill could create duplicate `(product_id, category_id)` pairs (blocked by composite PK) | V4 picks the **oldest** row as canonical; younger duplicates are archived (`is_active = 0`); all junction rows reference the canonical id. Documented in design. |
| ANY-of filter semantics misalign with user intuition | Users expect intersection when they meant union, or vice versa | Confirmed in this slice: **ANY-of** is the default and only semantics in this slice. UI labels the picker clearly; SQL uses `EXISTS`. ALL-of is a future enhancement if asked. |
| Duplicate-count risk in report aggregations | A product in two selected categories is counted twice | `EXISTS` subquery at SQL level, not a `JOIN`. Row shape stays distinct by `lot_id`. |
| N+1 regression in detail-bundle reads | `ProductDetailPage` and `DashboardPage` modal fetch categories per product | `list_product_category_ids_by_product_ids` batched helper; one read per list. |
| New `list_categories_search` command adds a command surface | Command registry grows; review surface expands | Mirrors the existing `list_categories` command shape; one new entry in `lib.rs`. |
| Restore-from-backup misses pre-V4 backups | Restore silently drops categories | Restore adapter translates `products.category_id` rows from the backup file into junction rows before the junction insert. |
| `Uncategorized` pseudo-row mistaken for a real category | Users can edit or delete it | Pseudo-row is rendered by the picker; no `id` exists in the `categories` table; picker filters it out of `list_categories_search` results. |
| Inline-create adds an arbitrary category name | The "controlled creation" product rule is violated | Inline-create is **explicit**: the user must click `Create "X"`. There is no silent arbitrary-text path. Service-layer case-fold guard rejects duplicates. |
| Archived categories appear in the picker and confuse the user | Picker pollutes with non-selectable rows | Picker filters to active only. Restore / archive is admin work, out of scope. |
| Popover positioning clips on small windows or near the bottom edge | Picker becomes unusable | Reuse the date picker popover's flip-up math (`getBoundingClientRect` + overflow flip). |
| Esc inside the search input discards a typed query unexpectedly | User loses confidence in search behavior | Esc closes the popover without committing the typed query (the input keeps the typed text and re-opens with the same query on next open). |
| `category_id` rename breaks an unsynced frontend type | Compile errors in dev / build | All consumers migrate in the same slice; TS types mirror Rust DTOs in lockstep. No deprecation cycle because the app is pre-1.0 with no external consumer. |
| Spec delta drifts from implementation | Capability statement vs. code mismatch | New capability `multi-category product model` is the canonical place; pointer edits under existing capabilities are minimal. |
| Frontend manual verification only | Regression risk between slices | Recorded in the slice notes; covered by the canonical `Engineering safety > tests accompany implementation` requirement's manual-verify clause. |

## Rollback

Rollback is straightforward at every layer:

1. **Git revert.** `git revert <merge-sha>` of the slice restores the
   previous single-FK state. No data is destroyed by the V4 back-fill
   because junction rows reference `categories(id)` and `products(id)`
   with `ON DELETE CASCADE` / `RESTRICT` — the V4 insert is additive,
   not destructive. Reverting the slice removes the code that reads
   from the junction, restores `products.category_id` reads, and
   returns the system to the single-FK semantics.
2. **Schema rollback.** If `ALTER TABLE ... DROP COLUMN` was applied
   successfully, the next migration step (`V5: rollback_v4`) re-creates
   `products.category_id` and back-fills it from the oldest junction row
   per product (deterministic `MIN(created_at)`). If the column was left
   in place by V4 (the fallback path), no schema rollback is required
   — `products.category_id` is already present and ignored.
3. **Junction rollback.** A `DROP TABLE product_categories` would lose
   the canonical data; rollback **does not** drop the table. Reverting
   the code is sufficient.
4. **Picker rollback.** Swap `<CategoryPicker bind:value={categoryIds}
   />` back to the legacy `<select bind:value={categoryId}>` (or to the
   pre-this-slice `<ul class="category-pills">` shape) in
   `ProductForm.svelte`, `ProductCatalogPage.svelte`, and
   `ReportsPage.svelte`. The dead `<select>` removal is a user-visible
   change; reverting restores the (already-broken) picker.
5. **Spec delta rollback.** Remove the new `## Capability: Categories >
   multi-category product model` requirement; revert the in-place edits
   under `Categories > editable category list`, `Product catalog`,
   `Reports > report filters`, `Dashboard`, and `Data persistence and
   safety > migrations`.
6. **Restore-from-backup adapter rollback.** The adapter is a code path,
   not a migration; reverting the slice disables it. Pre-V4 backups
   would then need a manual re-migration step (out of scope for this
   slice's rollback).
7. **Data integrity.** `products.category_id` rows from pre-V4 backups
   remain valid SQL even after V4; the junction rows are the canonical
   reads, but the legacy column continues to exist as ignored data.

The picker defect (if any) returns after rollback; revert should only be
considered if the new picker introduces a worse regression.

## Success criteria

- A user can open `ProductForm.svelte`, search for a category by prefix
  (e.g. `Dai` matches `Dairy`), select multiple categories from the
  result list, see them as removable chips, and submit the form with the
  expected junction rows in `product_categories`.
- A user can type a category name that has **no match** in the result
  list and see a `Create "X"` action at the bottom of the popover.
  Clicking it creates the category and selects it. There is no
  silent-create path.
- A user can remove a chip via the inline `×` button; the picker drops
  the chip from the selection without re-querying the backend.
- A user can clear all chips via the `Clear all` link below the chip
  row; the picker emits `[]` (empty array).
- A user can open `ProductCatalogPage.svelte` and select one or more
  categories from the filter. The catalog shows products whose category
  set intersects the selection (`ANY-of`). Selecting the
  `Uncategorized` pseudo-row returns products with zero active category
  relations.
- A user can open `ReportsPage.svelte` and select one or more
  categories. The report shows lots whose product belongs to **any** of
  the selected categories, **exactly once per lot**. The filter summary
  renders `n categories` rather than leaking raw ids.
- A user can search the picker with a substring (e.g. `iry` when no
  prefix match exists) and see substring results.
- A user with no selection in either filter sees the full catalog /
  report (no `All categories` chip is rendered; empty selection means
  no filter).
- A user who archives a category (via the existing `update_category({
  is_active: false })` call path) sees that category disappear from
  pickers. Existing junction rows are preserved; existing lots stay
  matched in the catalog only if the archived category was the **only**
  one (otherwise they shift to "Uncategorized" representation because
  no active category remains). The FK `ON DELETE RESTRICT` blocks any
  future hard-delete path that would orphan junction rows.
- `npx svelte-check --workspace . --threshold error` passes.
- `npm run build` passes.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` keeps the same
  baseline as the most recent archived slice (2 pre-existing failures
  unchanged; this slice touches Rust extensively and must not introduce
  new failures).
- New unit tests cover: V4 back-fill correctness, V4 idempotency,
  case-fold guard (lowercase / uppercase / mixed duplicates),
  multi-category round-trip, update replaces full set, report filter
  any-of, report filter no-double-count, archive flow stays
  RESTRICT-enforced.
- Manual smoke on Linux/WebKitGTK and Windows/WebView2 covers: picker
  open / search-prefix / search-substring / select / chip-remove /
  Clear all / Create "X" / archived-category behaviour / Uncategorized
  pseudo-row / catalog filter / reports filter / restore-from-backup
  with a pre-V4 fixture.

## Confirmed product decisions (from question round and parent context)

These are the user-confirmed decisions captured in the user's framing for
this slice. They are restated here so the design phase has them in one
place:

**Data model:**

1. Implement real multi-category support now, including the database
   migration. Defer is not an option.
2. App is only used by the owner in test phase; prefer the clean
   long-term model over preserving legacy runtime compatibility.
3. Migration shape: create `product_categories` junction as the source
   of truth. Backfill from existing `products.category_id`. If SQLite
   makes dropping `products.category_id` costly, it may remain as
   ignored legacy data — but it is **not** the runtime source of truth
   anymore.
4. Product-category relation: many-to-many with composite uniqueness
   on `(product_id, category_id)`. Product delete cascades relation
   rows. Category delete is restricted; archive is preferred.
5. Existing case-only duplicates: oldest category wins; reassign
   product / category relationships to canonical oldest; archive
   duplicates where safe; no hard-delete.

**Filters:**
6. Category filters in `ProductCatalogPage` and `ReportsPage` support
   multiple selected categories with **ANY-of** semantics.
7. Filters use SQL semantics such as `EXISTS` to avoid duplicated
   product / lot / report rows.
8. `Uncategorized` means **no active category relation**. Products with
   no categories, or only archived categories, appear as Uncategorized
   in normal filters.
9. Both filters include Uncategorized. In the picker, use a
   pseudo-row / chip for Uncategorized. Zero selection means
   **no filter / all categories**, not an explicit All chip.

**Picker UX:**
10. `ProductForm` uses a searchable multi-select combobox with visible
    removable chips / tokens.
11. Inline category creation is explicit via `Create "X"`; no silent
    arbitrary text.
12. Search semantics: case-insensitive prefix first; substring fallback
    when no prefix results; no fuzzy search in this slice.
13. No artificial maximum categories per product.

**Backend search API:**
14. New category search API / command with query, limit, and
    `total` / `has_more` shape.

**Compatibility:**
15. CSV / backup / restore: compatibility-simple. Existing CSV
    single-category behavior continues; restore / import adapts old
    `category_id` into the junction. No multi-category CSV syntax in
    the first slice unless design later proves it cheap and safe.
16. Archive / deactivate, not hard delete. Full category admin page is
    out of scope.

**Non-goals:** category admin page, merge-duplicates UI, hierarchy /
subcategories, analytics by category, POS / billing / payments /
accounting / full inventory.

The deliverable is a single SDD change covering all 16 decisions plus the
non-goals list.

## Next steps (after this proposal is approved)

1. Move to `design.md` — concrete shapes for:
   - V4 migration DDL and case-fold deduplication SQL,
   - `list_product_category_ids_by_product_ids` helper signature,
   - `list_categories_search` command response shape and search SQL,
   - `CategoryPicker.svelte` props / events / a11y attributes,
   - `Uncategorized` sentinel id and pseudo-row rendering,
   - inline-create confirmation copy and Disabled state,
   - restore-from-backup adapter path (where it lives, how the
     translation works),
   - `Uncategorized` representation in the catalog / reports filter
     summary.
2. Move to `tasks.md` — ordered work units sized to the chosen single
   PR / single slice delivery shape. Manual-frontend verify gap recorded
   explicitly per the canonical `Engineering safety > tests accompany
   implementation` requirement.
3. Spec delta on `openspec/specs/caduxo-expiry-tracker/spec.md`:
   - MODIFY `## Capability: Categories > editable category list` in
     place to absorb multi-category semantics;
   - ADD `## Capability: Categories > multi-category product model`;
   - MODIFY `## Capability: Product catalog > mandatory unique SKU`
     (or the most-fitting product registration requirement) to point
     at the new model;
   - MODIFY `## Capability: Reports > report filters` for `category_ids`
     any-of;
   - MODIFY `## Capability: Dashboard > operational main screen` (or
     the most-fitting dashboard requirement) to add `category_ids`;
   - MODIFY `## Capability: Data persistence and safety > migrations`
     to note V4 introduces `product_categories` and the case-fold
     deduplication step.
4. PRD alignment on `docs/prd.md` (small, in design).
5. Apply-phase order: backend (V4 migration + repos + services +
   command + tests) → frontend DTOs → frontend picker primitive →
   host adoptions in the order `ProductForm`, `ProductDetailPage`,
   `DashboardPage`, `ProductCatalogPage`, `ReportsPage` → manual
   smoke on Linux/WebKitGTK and Windows/WebView2 → restore-from-backup
   smoke with a pre-V4 fixture.
