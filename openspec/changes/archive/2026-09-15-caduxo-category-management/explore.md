# Explore — caduxo-category-management

## Scope of this exploration

The user has decided to land real **multi-category support for products** in
this SDD change — including the underlying database migration — rather than
deferring it to a follow-up. This revise re-anchors the exploration around that
decision. The prior "solo UX picker" framing is folded into the new scope as a
sub-task (the picker must grow into a multi-select anyway), but the explore no
longer treats picker-only work as a viable outcome.

The session preflight pins the SDD rails: `executionMode: interactive`,
`artifactStore: both` (project config; OpenSpec artifacts live in repo; Engram
mirror is written via `mem_save`), `deliveryStrategy: ask-on-risk`, and the user
typed a session review-budget override of **3 000 changed lines** for this
change (the project config defaults to 800 and the canonical threshold is 400;
the typed 3 000 is explicit and takes precedence for this change only).

A parent UX-web brief is already on file. This explore cites the brief's
direction as substrate but does not invent new sources beyond the brief's
named references (USWDS combo box, WAI-ARIA combobox pattern, GOV.UK
long-list guidance, NN/g dropdown-list article, Adobe Commerce category admin
docs). No new research is conducted here.

This document is **exploration substrate for the proposal phase**. It does
not propose the design — junction vs. JSON column, hybrid (keep FK +
junction) vs. drop-FK, "any-of" vs. "all-of" filter semantics, picker UX
shape (combobox with chips vs. multi-select listbox), inline-create guard,
and archive-rule UX are deferred to `proposal.md` and require user
confirmation before tasks land.

---

## 1. Repository state

Working tree clean at session start. No in-flight category change. The
`openspec/changes/caduxo-category-management/` folder contains only this
`explore.md`; no collisions with other active changes.

### Project config (`openspec/config.yaml`)

| Knob                            | Value                          |
|---------------------------------|--------------------------------|
| `project`                       | `Caduxo`                       |
| `language`                      | `en`                           |
| `artifactStore` (project)       | `both`                         |
| `sdd.executionMode`             | `interactive`                  |
| `sdd.reviewBudgetChangedLines`  | `800` (project default)        |
| `sdd.chainedPrStrategy`         | `auto-forecast`                |
| `sdd.strictTdd`                 | `false`                        |
| `product.priorityPlatforms`     | `windows`, `linux`             |
| `product.nonGoals`              | POS, billing, payments, accounting, full inventory |

This session's preflight overrides `artifactStore: both` (project default
applies — OpenSpec artifacts go to
`openspec/changes/caduxo-category-management/`, and the Engram mirror is
persisted via `mem_save`), keeps `executionMode: interactive`, and pins
`reviewBudgetChangedLines: 3000` from the user's typed session override.

### Frontend / backend split

- Frontend: Svelte 5 + Vite + TypeScript, mounted via Tauri 2. Scripts in
  `package.json`: `dev`, `build`, `preview`, `tauri`. Type-check:
  `npx svelte-check --workspace . --threshold error`.
- Backend: Rust + Tauri in `src-tauri/` (SQLite, command/handler layer,
  services + repositories). Test harness:
  `cargo test --manifest-path src-tauri/Cargo.toml --lib`.
- **No frontend test harness.** Canonical `Engineering safety >
  tests accompany implementation` accepts manual smoke for frontend changes
  when the slice records the gate. Do not introduce vitest here.

---

## 2. Current schema (V2) — single-FK model

The relevant V2 fragment from `src-tauri/src/db/migrations.rs`:

```sql
CREATE TABLE categories (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,    -- case-sensitive in SQLite by default
    is_active  INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE products (
    id                     TEXT PRIMARY KEY,
    sku                    TEXT NOT NULL UNIQUE,
    description            TEXT NOT NULL,
    category_id            TEXT REFERENCES categories(id),   -- single FK
    default_unit           TEXT,
    default_alert_days_before INTEGER NOT NULL DEFAULT 30,
    notes                  TEXT,
    is_active              INTEGER NOT NULL DEFAULT 1,
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL
);
```

Key observations:

- `products.category_id` is a **single nullable FK**. There is no
  `product_categories` junction table. Multi-category support is structurally
  impossible without a schema change.
- `categories.name` carries `UNIQUE`, which in SQLite is **case-sensitive by
  default**. Two rows with `Dairy` and `dairy` are both permitted at the DB
  layer today; the form accepts both, the CSV importer folds case and would
  resolve to the first one (semantic gap; see §3.5).
- `categories.is_active` supports archive, and `update_category` already
  writes it. There is no UI that toggles it today.
- Migration `V2` is a single CREATE for everything; V3 added unit catalog.
  The new schema work becomes **V4**.

### Existing V4 prior-art pattern (unit catalog)

V3 demonstrates the project's preferred migration shape (see
`db/migrations.rs`):

1. Add a new table with `UNIQUE` constraints where appropriate.
2. Add nullable FK columns to `products` via `ALTER TABLE ... ADD COLUMN`.
3. Back-fill existing rows in the same migration with deterministic SQL.
4. Seed presets via `INSERT OR IGNORE` against a stable list.

A junction-table migration should follow the same shape, plus an explicit
back-fill from `products.category_id` and a decision about whether to keep or
drop the legacy FK.

---

## 3. Surfaces that depend on single `products.category_id`

This is the consumer map the proposal must touch. Every entry here is a
shape change when the model becomes many-to-many.

### 3.1 Database (`src-tauri/src/db/`)

| File | Surface | Current behavior | Multi-category impact |
|---|---|---|---|
| `migrations.rs` | `categories` table, `products.category_id` FK | Single FK; no junction. | **Add V4** with `product_categories` table; back-fill; decide drop-FK vs. keep-FK. |
| `repositories/products.rs::insert_product` (line ~179) | INSERT includes `category_id` | Writes one FK. | Replace with INSERT into junction (or insert + junction write). |
| `repositories/products.rs::update_product` (line ~244) | UPDATE SET includes `category_id` | Replaces one FK. | Replace full set; transactional delete + insert into junction. |
| `repositories/products.rs::get_product` (line ~215) | `SELECT category_id` | Returns one id. | Return `category_ids: Vec<String>` (or keep `category_id` plus a new field — see decision §6). |
| `repositories/products.rs::search_products` (line ~292) | Selects `category_id` per row | One id per row. | Multi-select; new `ProductSearchResult.category_ids`. |
| `repositories/products.rs::find_by_barcode_exact` (line ~415) / `find_by_sku_exact` (line ~436) | Returns `ProductSearchResult` | One id per row. | Same `category_ids` change. |
| `repositories/products.rs::list_all_products_for_export` (line ~477) | CSV export | One id per row. | Join through junction; consider comma-joined column for back-compat. |
| `repositories/products.rs::list_active_categories` (line ~63) / `list_all_categories` (line ~89) | Category list | Unchanged. | Unchanged. |
| `repositories/dashboard.rs::list_dashboard_lots` (line ~18) | Joins `products` | Filters by `store_id` / `location_id` only — **no category filter at this level today**. | Add optional `category_ids` join filter (EXISTS subquery) so `DashboardFilters` can express multi-category. |

### 3.2 DTOs (`src-tauri/src/dto/`)

| File | Field | Multi-category impact |
|---|---|---|
| `dto/products.rs::ProductCreate` | `category_id: Option<String>` (line ~48) | Replace with `category_ids: Option<Vec<String>>` (empty list ≠ one None). |
| `dto/products.rs::ProductUpdate` | `category_id: Option<String>` (line ~63) | Same — full set on each update. |
| `dto/products.rs::ProductResponse` | `category_id: Option<String>` (line ~79) | Add `category_ids: Vec<String>` (always present, possibly empty). Optionally deprecate `category_id` (kept for one release). |
| `dto/products.rs::ProductDetailResponse` | `category: Option<CategoryResponse>` (line ~125) | Replace with `categories: Vec<CategoryResponse>` (empty list = unassigned). |
| `dto/products.rs::ProductSearchResult` | `category_id: Option<String>` (line ~149) | Add `category_ids: Vec<String>`. |
| `dto/dashboard.rs::DashboardFilters` | `store_id`, `location_id`, `preset`, `urgency` — **no category field** | Add `category_ids: Option<Vec<String>>` to push the filter down to SQL (replaces the per-row N+1 post-filter). |
| `dto/reports.rs::ReportFilters` | `category_id: Option<String>` (line ~81) | Replace with `category_ids: Option<Vec<String>>`; update semantics (any-of) in spec; backend translates to `EXISTS` join. |

### 3.3 Services (`src-tauri/src/services/`)

| File | Surface | Multi-category impact |
|---|---|---|
| `services/products.rs::create_product` / `update_product` | Validates input and writes one FK | Validate `category_ids` (non-empty list of valid FKs) and replace the single-FK write with a junction write. |
| `services/products.rs::get_product` (line ~262) | Resolves single FK to `CategoryResponse` | Resolve all FKs to `Vec<CategoryResponse>` (one batched query, not N — see §4 N+1). |
| `services/products.rs::category_unique_error` | Translates SQLite UNIQUE violation on `categories.name` | Unchanged; the `categories` table itself is unchanged. |
| `services/reports.rs::filter_by_category` (line ~145) | Per-row `get_product` N+1 to compare `product.category_id` to target | Replace with **batched** `list_product_category_ids_by_product_ids(&[product_ids])` → HashMap. Plus filter semantics becomes "any-of" (`category_ids.contains(target)`) when `category_ids: Vec<String>`. |
| `services/reports.rs::to_dashboard_filters` (line ~89) | Translates `ReportFilters` → `DashboardFilters` | Pass `category_ids` through (do not consume at translation time). |
| `services/csv_io.rs::resolve_category_name` (line ~1000) | Resolves a CSV category name to id (case-insensitive) | Unchanged for single category; if CSV import gains multi-category support, allow semicolon-separated names (see §6). |
| `services/csv_io.rs::import_row` (line ~831) | Calls `products_repo::insert_product` / `update_product` | Pass through category info unchanged; one `category_id` per row today. |
| `services/dashboard.rs::get_dashboard` (line ~103) | Calls repo with `DashboardFilters` | If `category_ids` is added to `DashboardFilters`, it passes through to SQL. |
| `services/notifications.rs`, `services/expiry_lots.rs`, `services/unit_definitions.rs` | Test fixtures reference `category_id: None` | No semantic impact; if fixtures ever need to seed categories, they use the junction. |

### 3.4 Commands (`src-tauri/src/commands/`)

| File | Surface | Multi-category impact |
|---|---|---|
| `commands/products.rs::list_categories` (line ~26) / `create_category` / `update_category` | Category CRUD | Unchanged. |
| `commands/products.rs::create_product` / `update_product` (line ~75) | Wire DTO → service | Accept the new `category_ids: Vec<String>` field. |
| `commands/products.rs::get_product` (line ~99) | Returns detail bundle | Returns `categories: Vec<CategoryResponse>`. |
| `commands/products.rs::search_products` (line ~106) | Wire DTO → service | Return new `ProductSearchResult` shape. |
| `commands/dashboard.rs::list_dashboard_lots` (line ~21) | Wire `DashboardFilters` | Accept `category_ids`. |
| `commands/reports.rs::preview_report` / `export_report_pdf` | Wire `ReportRequest` → `ReportFilters` | Accept `category_ids`; pass through. |

All these commands are registered in `src-tauri/src/lib.rs` lines 48–60 (no
registration change needed, only signature edits).

### 3.5 Frontend — product surfaces

| File | Surface | Current shape | Multi-category impact |
|---|---|---|---|
| `src/lib/products.ts::ProductCreate` (line ~55) / `ProductUpdate` (line ~67) | TS DTO | `category_id?: string \| null` | Replace with `category_ids?: string[]`. |
| `src/lib/products.ts::ProductResponse` (line ~39) | TS DTO | `category_id: string \| null` | Add `category_ids: string[]`; optionally deprecate `category_id`. |
| `src/lib/products.ts::ProductDetailResponse` (line ~109) | TS DTO | `category: CategoryResponse \| null` | Replace with `categories: CategoryResponse[]`. |
| `src/lib/products.ts::ProductSearchResult` (line ~114) | TS DTO | `category_id: string \| null` | Add `category_ids: string[]`. |
| `src/lib/dashboard.ts::DashboardFilters` (line ~21) | TS DTO | `store_id`, `location_id`, no category | Add `category_ids?: string[]`. |
| `src/lib/reports.ts::ReportFilters` (line ~28) | TS DTO | `category_id?: string \| null` | Replace with `category_ids?: string[]`. |
| `src/components/ProductForm.svelte` (line ~43, ~326) | `let categoryId: string = ""`; `<select bind:value={categoryId}>` (only "— None —" option) + `<ul class="category-pills">` with one button per category | Single-select; empty dropdown + pill storm | **Replace selector** with a multi-select combobox bound to `categoryIds: string[]`; chips for selected; remove dead `<select>` (D1); remove pill `<ul>` (D2). See UX brief. |
| `src/components/ProductForm.svelte` (line ~247) | Submit payload `category_id: categoryId \|\| null` | One FK | `category_ids: categoryIds`. Empty array means "no category", not `null`. |
| `src/components/ProductForm.svelte` (lines ~133–149) | Inline category create, no case-insensitive guard | Surfaces raw SQL error | Unchanged in scope; case-insensitive guard lands alongside as a service-layer fix (D4, D5). |
| `src/components/ProductDetailPage.svelte` (line ~207) | Renders `{#if detail.category}{detail.category.name}{/if}` as a single `.category-badge` | One badge | Render multiple badges (one per category) or a comma-joined label. |
| `src/components/DashboardPage.svelte` (line ~606) | Detail modal `<dd>{detailProduct.category?.name ?? "—"}</dd>` | One name | Render `detailProduct.categories.map(c => c.name).join(', ')` or list. |
| `src/components/ReportsPage.svelte` (line ~353) | `<select bind:value={categoryId}>` with all categories | Single-select filter | Multi-select combobox bound to `categoryIds: string[]`; default "All categories". |

### 3.6 Frontend — non-product surfaces (worth noting, not in scope for shape change)

| File | Surface | Status |
|---|---|---|
| `src/components/ProductCatalogPage.svelte` | Loads `listCategories()` for the form; passes through unchanged | No direct binding; gets the new `category_ids` for free when `ProductResponse`/`ProductSearchResult` change. |
| `src/components/ColumnMapper.svelte` (line ~167) | `<select bind:value={selectedCategory}>` where `selectedCategory` is a **CSV column index**, not a category id | Out of scope — column-mapper semantics are "which CSV column has the category name". If the CSV import grows multi-category support, this becomes "which column has the category string (semicolon-separated or first-wins)". Decision deferred. |
| `src/components/CalendarPage.svelte` | Uses `DashboardFilters` with only `store_id` / `location_id` | Inherits `category_ids` automatically; no direct edit needed. |

### 3.7 Backend — known semantic gap (carried into the slice)

- `csv_io::resolve_category_name` (line ~1000) does case-insensitive name
  match against `list_all_categories`. The form's inline create does not.
  This produces a hidden `Dairy` vs. `dairy` situation depending on entry
  path. Lands alongside the service-layer case-fold guard (D4 in §4).

---

## 4. Migration considerations

### 4.1 Schema shape options (proposal must pick)

**Option A — Pure junction (drop legacy FK).**

- V4 creates `product_categories(product_id TEXT NOT NULL REFERENCES
  products(id) ON DELETE CASCADE, category_id TEXT NOT NULL REFERENCES
  categories(id) ON DELETE RESTRICT, created_at TEXT NOT NULL,
  PRIMARY KEY (product_id, category_id))`.
- V4 back-fills: `INSERT INTO product_categories (product_id, category_id,
  created_at) SELECT id, category_id, created_at FROM products WHERE
  category_id IS NOT NULL`.
- V4 then `ALTER TABLE products DROP COLUMN category_id` (SQLite supports
  `DROP COLUMN` since 3.35).
- Pros: clean, no redundant column, every consumer reads the same shape.
- Cons: requires every consumer of `products.category_id` to be migrated in
  the same change (otherwise we have readers that crash on `no such
  column`). High blast radius.

**Option B — Hybrid (keep legacy FK + junction; junction is source of
truth).**

- V4 creates `product_categories` with the same shape as A.
- V4 back-fills from `products.category_id`.
- V4 does **not** drop the FK; instead, V4 (or the service layer) keeps
  `products.category_id` in sync with the **first** assigned category
  ("primary category") for one release.
- Pros: back-compat for any external reader (none today — closed-source
  app — but safer in theory); incremental migration possible.
- Cons: redundant storage; ambiguous semantics ("primary" vs. unordered
  list); needs a sync rule. Likely overkill for a single-tenant desktop
  app where the same change migrates every consumer.

**Option C — JSON-encoded array column.**

- V4 adds `products.category_ids TEXT NOT NULL DEFAULT '[]'` (JSON
  array of strings).
- Pros: no junction; simple.
- Cons: no referential integrity; `EXISTS` joins become JSON path
  queries; SQLite JSON support requires `JSON1` extension (off by default
  on some platforms); indexing is poor. Caduxo's V3 unit-catalog migration
  didn't pick this path either; consistency argues against it.

**Proposal must pick A or B before tasks land.** Default recommendation: **A**
(SQLite `DROP COLUMN` works on the in-use build; the consumer list is
closed; one migration per consumer is acceptable). Hybrid is acceptable
only if proposal round finds a reason.

### 4.2 Back-fill correctness

The back-fill must be **idempotent** so re-running V4 on a partially-migrated
DB is a no-op. The shape used in V3 (case-insensitive `UPDATE ... WHERE ...`)
suggests using `INSERT ... SELECT ... WHERE NOT EXISTS` for the back-fill:

```sql
INSERT INTO product_categories (product_id, category_id, created_at)
SELECT p.id, p.category_id, p.created_at
FROM products p
WHERE p.category_id IS NOT NULL
  AND NOT EXISTS (
    SELECT 1 FROM product_categories pc
    WHERE pc.product_id = p.id AND pc.category_id = p.category_id
  );
```

If Option A is chosen, the back-fill and the `DROP COLUMN` should be in the
**same** migration (and same transaction if SQLite supports it for DDL).
SQLite DDL inside transactions is allowed in `sqlx`'s `MigrationType::Simple`
mode the project already uses.

### 4.3 Index strategy

- `CREATE INDEX idx_product_categories_category ON product_categories
  (category_id)` — drives "products in category X" lookups for the report
  filter.
- `CREATE INDEX idx_product_categories_product ON product_categories
  (product_id)` — drives "categories for product X" when assembling the
  detail bundle (and the existing PK already gives the product_id leading
  column, but a redundant index speeds up reverse lookups if products grow
  large).
- `category_id REFERENCES categories(id) ON DELETE RESTRICT` is the
  canonical FK behavior — matches the existing
  `products.category_id` semantics (no `ON DELETE CASCADE`, child may
  outlive parent) and prevents accidental archive-by-deleting-a-category.

### 4.4 N+1 risk in reports (carry-forward + amplify)

`services::reports::filter_by_category` already does a per-row `get_product`
lookup. With multi-category, the per-row check is still O(1) (look up
`category_ids.contains(target)`) — but the underlying per-row read must
also return all category ids per row. The current shape does one read per
row × one FK per row = N reads. After migration:

- Option 1: keep the per-row `get_product` and read all FKs per row
  (existing pattern, works but is N reads).
- Option 2: add `list_product_category_ids_by_product_ids(ids:
  &[String]) -> HashMap<String, Vec<String>>` to the products repository
  and call it once per `filter_by_category` invocation. This is the
  **batched** path and should land in the same slice.

The existing 2-pre-existing-failure baseline in
`services::reports::tests::preview_report_in_alert_window_returns_alert_lots`
and `..._next_30_days_returns_30d_lots` is unrelated to the category filter;
this slice must preserve the baseline and not add new failures.

### 4.5 Down-migration / rollback

The proposal should specify the down-migration shape (or "no down-migration,
data preserved as-is"). Caduxo's prior slices do not ship down migrations,
so the default is "no down". The change-folder's `tasks.md` must call this
out explicitly for the user's confirmation.

---

## 5. Report / filter semantics — duplicate-count risk

This is the highest-impact product decision in the change. With single
category, "products in category X" is an equality predicate and is trivially
distinct. With multi-category, **two distinct risks appear**:

### 5.1 ANY-of vs ALL-of (filter semantics)

When the user selects `{Dairy, Bakery}` in the Reports filter, the product
intuition is:

- **ANY-of (default expectation):** show me lots whose product belongs to
  Dairy **or** Bakery. Union semantics.
- **ALL-of (intersection):** show me lots whose product belongs to **both**
  Dairy **and** Bakery. Intersection semantics. With single category per
  product, this is empty; with multi-category, it's "products that are
  classified into every selected category."

Default expectation in enterprise tooling is ANY-of (OR / union), matching
how filter multi-selects work in Linear, Asana, JIRA, and most BI tools. The
proposal must confirm this with the user before design locks.

The SQL translates as:

- ANY-of: `EXISTS (SELECT 1 FROM product_categories pc WHERE
  pc.product_id = el.product_id AND pc.category_id IN (?))`.
- ALL-of: `category_ids` must contain every selected id. Implemented as
  `(SELECT COUNT(*) FROM product_categories pc WHERE pc.product_id =
  el.product_id AND pc.category_id IN (?)) = N` (where N is the number of
  selected categories).

### 5.2 Duplicate-count risk in summary aggregations

If the report SQL joins `expiry_lots` to `product_categories` without
`DISTINCT`, a product that belongs to two selected categories will be
counted twice. Today's `filter_by_category` does per-row filtering and
avoids this by construction. The new SQL needs to use either `EXISTS` (no
join, no double count) or `JOIN ... GROUP BY product_id`.

Concrete plan:

- Use `EXISTS` subquery in the SQL filter, not a join. Keeps the existing
  row-per-lot shape (`DashboardLotRow`).
- Apply `category_ids` as a **SQL-level filter** inside
  `db/repositories/dashboard.rs::list_dashboard_lots`, not as a
  post-filter. This eliminates the N+1 read and the duplicate-count risk
  at the same time.
- For the urgency counts (which are computed client-side in
  `services::dashboard.rs::count_by_urgency` after the SQL filter), keep
  the row shape unchanged — no double-count possible because rows are
  distinct by `lot_id`.

### 5.3 Metadata in `ReportMetadata.filters_used`

`ReportFilters.category_id` becomes `ReportFilters.category_ids`. The
`filterSummary` UI (`ReportsPage.svelte` line ~265) renders
`category=<id.slice(0,8)>…`; with a list, render the same for each id joined
by `,` (or `n categories` summary to avoid leaking ids in screenshots).

### 5.4 Empty selection semantics

- `category_ids: None` or `category_ids: Some(vec![])` → no filter applied
  (matches today's "All categories").
- One-element list → behaves like today's single-category filter
  (preserves back-compat for stored filter presets if any exist).

---

## 6. Recommended proposal scope

A single PR that does all of the following, given the 3 000-line session
budget and the closed consumer surface:

### 6.1 Backend

1. **V4 migration** (`src-tauri/src/db/migrations.rs`): add
   `product_categories` table + indexes; back-fill from
   `products.category_id`; decide drop-FK vs. keep-FK (recommend drop).
   Add migration tests for idempotency and back-fill correctness.
2. **DTO updates** (`src-tauri/src/dto/products.rs`,
   `dto/dashboard.rs`, `dto/reports.rs`):
   - `ProductCreate.category_ids: Option<Vec<String>>`
   - `ProductUpdate.category_ids: Option<Vec<String>>`
   - `ProductResponse.category_ids: Vec<String>`
   - `ProductDetailResponse.categories: Vec<CategoryResponse>`
   - `ProductSearchResult.category_ids: Vec<String>`
   - `DashboardFilters.category_ids: Option<Vec<String>>`
   - `ReportFilters.category_ids: Option<Vec<String>>`
3. **Repository updates** (`src-tauri/src/db/repositories/products.rs`,
   `dashboard.rs`):
   - `insert_product` / `update_product` write through to junction
     (transactional insert/delete).
   - `get_product` returns `category_ids: Vec<String>` (single read,
     join through junction).
   - `search_products` and `find_by_*_exact` add `category_ids` from
     junction.
   - `list_all_products_for_export` joins category names (comma-joined
     text column for CSV export — back-compat with single-name CSV
     consumers).
   - `list_dashboard_lots` accepts `category_ids: Option<Vec<String>>`
     and applies it as an `EXISTS` subquery (no double count).
   - New helper: `list_product_category_ids_by_product_ids(ids:
     &[String]) -> HashMap<String, Vec<String>>` for batched reads.
4. **Service updates** (`src-tauri/src/services/`):
   - `services/products.rs::create_product` / `update_product` validate
     `category_ids` (empty list allowed; each id must exist and be active
     unless explicitly allowing archived). Writes go through junction.
   - `services/products.rs::get_product` resolves all category ids to
     `Vec<CategoryResponse>` via the batched helper.
   - `services/reports.rs::filter_by_category` becomes obsolete (filter
     is SQL-level now) — remove the post-filter; the `category_ids`
     field on `ReportFilters` flows down to `DashboardFilters` and lands
     in the SQL.
   - `services/csv_io.rs::resolve_category_name` stays case-insensitive;
     a separate guard for the multi-category CSV import (semicolon-
     separated names) is **out of scope** for this slice unless the
     proposal round decides otherwise.
5. **Service-layer case-fold guard** (lands in this slice because the
   schema change already touches category writes):
   - `services/products.rs::create_category` and `update_category` (rename
     path) check for `LOWER(name)` collisions before insert/update;
     translate SQLite UNIQUE violation to `DuplicateField { field: "name" }`
     on the conflict path; document the existing-data case (see §7).
6. **Tests** (unit + migration):
   - V4 back-fill test (seed a V3-only DB with rows in
     `products.category_id`, run V4, assert junction rows).
   - V4 idempotency test.
   - Service-layer case-fold guard tests (lowercase / uppercase / mixed
     duplicates).
   - New product with multiple categories (`create_product({ category_ids:
     vec!["a","b"] })`) and read back.
   - Update replaces the full set (delete all + insert new).
   - Report filter ANY-of: products matching any selected category show up
     exactly once.
   - Report filter no-double-count: a product in both Dairy and Bakery,
     with both selected, shows once.
   - Archive flow: archiving a category still in junction stays RESTRICT-
     enforced (FK prevents accidental loss).

### 6.2 Frontend

1. **TS types** (`src/lib/products.ts`, `lib/dashboard.ts`,
   `lib/reports.ts`): mirror the new Rust DTOs. Deprecate
   `category_id`-shaped fields behind a single major version (this app is
   pre-1.0; no deprecation cycle — just rename).
2. **New picker** (`src/components/inputs/CategoryPicker.svelte`,
   consumed by `ProductForm.svelte`): multi-select combobox (search
   input + popover with matches) with chip list for selected categories.
   - WAI-ARIA combobox pattern: `role="combobox"`, `aria-expanded`,
     `aria-controls`, `aria-activedescendant`, focus management on open
     and selection.
   - Backed by `list_categories` paginated/searched (or extended with a
     `query` arg) — proposal must pick shape.
   - Inline-create guarded by the service-layer case-fold check (no
     extra client-side check needed if backend is the source of truth).
3. **`ProductForm.svelte` updates**:
   - Replace `categoryId: string` with `categoryIds: string[]` (line ~43).
   - Drop the empty `<select>` (lines ~326–328) and the `<ul
     class="category-pills">` (lines ~330–344).
   - Submit payload uses `category_ids: categoryIds` (line ~247).
   - Remove the dead "all categories as pills" UX; chips render above the
     input for selected categories only.
4. **`ProductDetailPage.svelte`** (line ~207): render multiple badges
   instead of one.
5. **`DashboardPage.svelte`** (line ~606): render comma-joined names or a
   list.
6. **`ReportsPage.svelte`** (line ~353): replace the single `<select
   bind:value={categoryId}>` with the multi-select combobox; remove
   `categoryId` state and `buildFilters()` (line ~143) uses
   `category_ids`. Filter summary (line ~265) shows `category=<n>` or
   joined ids.

### 6.3 Out of scope for this slice (deferred to follow-up changes)

- **Category admin page** (virtualized list, archive/restore, rename,
  product counts). The existing `update_category({ is_active: false })`
  is sufficient to archive from code paths or future admin UI; no admin
  page is required to land multi-category.
- **CSV import with multiple categories per row.** Today's `import_row`
  takes one `category_id`. Proposal must decide: (a) keep single-category
  CSV import (semicolon-separated rejected as Invalid), (b) accept
  semicolon-separated names and split, (c) add a separate
  `category_2`/`category_3` column mapping. (a) is the lowest-risk
  default and is recommended.
- **Batch N+1 fix for `filter_by_category` as a standalone concern.** With
  the SQL-level filter, the N+1 is gone — there's nothing left to fix.
- **Existing-data duplicate dedup** (`Dairy` vs `dairy` if any exists
  from prior testing). Decision in §7.
- **Search backend shape.** Whether to add a `query`/`limit` parameter to
  `list_categories` or to introduce `list_categories_search`. Picker UX
  needs server-side search at 200+ categories; this slice must include
  it.

### 6.4 Line-count envelope

Rough budget for the recommended scope (rough; refine at design lock):

- Backend: ~700–1 100 LOC (V4 migration + new indexes + repo rewrite +
  service rewrite + ~150 LOC of new tests + case-fold guard).
- Frontend: ~500–800 LOC (new `CategoryPicker` + `ProductForm` swap +
  `ProductDetailPage` badges + `DashboardPage` join + `ReportsPage`
  multi-select + TS type updates).
- Spec delta: 1 `MODIFIED` requirement under `## Capability: Categories`
  (`editable category list` grows multi-category semantics) + 1–2 new
  `ADDED` requirements (e.g., `multi-category product model`,
  `category filter any-of semantics`).
- **Total: ~1 400–2 300 LOC**, comfortably under the **3 000-line session
  budget**. No `size:exception` needed; no chained PR needed.

This is one slice — no `chainedPrStrategy` choice required unless the
proposal round decides otherwise.

---

## 7. Existing-data risks (must be answered before proposal locks)

1. **Case-only duplicates in `categories`.** If any existing rows are
   `Dairy` and `dairy` from prior testing, the new case-fold guard will
   reject the **next** create/rename attempt on those names. Proposal must
   decide: (a) accept the guard and surface a clear message, (b) dedupe
   existing rows as a one-time step in V4, (c) back-port `COLLATE NOCASE`
   on `categories.name` (risky if data exists).
2. **CSV import back-compat.** Existing CSV files with single category
   names continue to work via `resolve_category_name`. If V4 enables
   multi-category CSV import later, old files still parse (one category
   per row). No data migration needed.
3. **Restore from backup (Slice 12).** Backups produced by V2/V3 contain
   `products.category_id` rows. After V4 (Option A), restore would fail or
   silently drop categories. Proposal must decide: (a) require
   re-migration on restore, (b) translate `products.category_id` →
   `product_categories` rows during restore. (b) is the safer default.
4. **Pre-existing test fixtures** in
   `services/products.rs`, `services/expiry_lots.rs`,
   `services/notifications.rs`, `services/unit_definitions.rs`,
   `services/csv_io.rs`, `services/reports.rs`, `pdf/report_pdf.rs` use
   `category_id: None`. They keep compiling if the field is renamed to
   `category_ids: Vec::new()` — but tests that seed a category via the
   old single-FK shape need to switch to junction seeding or use the
   service-layer API. The slice must update these fixtures.

---

## 8. Canonical spec surface

`openspec/specs/caduxo-expiry-tracker/spec.md` is the only domain in scope.
The slice touches three capabilities:

| Capability | Requirement | Multi-category impact |
|---|---|---|
| `Categories` | `editable category list` (line 247) | **MODIFIED in place** to clarify: "products may belong to one or more categories from this list; an unassigned product is valid; archive (`is_active=false`) hides from pickers but preserves history." |
| `Categories` | `category filter` scenario (line 251) | **ADDED scenario** for any-of semantics: "given multiple categories selected, lots whose product belongs to any of the selected categories are returned, exactly once per lot." |
| `Products` | `product registration` (via `ProductCreate.category_id`) | **MODIFIED**: field becomes `category_ids` (list); one or many. |
| `Reports` | `report filters` (line 588) | **MODIFIED** filter vocabulary: `category_id` → `category_ids` with any-of semantics. |
| `Engineering safety` | `tests accompany implementation` | Backend adds V4 + service-level tests; frontend stays on manual smoke (no vitest). |

`openspec/specs/caduxo-expiry-tracker/spec.md` currently has only one
requirement for categories (`editable category list`). The slice **MODIFIES
that requirement in place** to absorb the multi-category model rather than
adding a sibling — keeping the historical scenario intact.

---

## 9. Risks the parent should know before proposal

1. **Schema change is the dominant risk.** Every `category_id` consumer
   (DTO, repository, service, command, frontend type, frontend page) is a
   touch point. Doing this in one slice keeps the migration atomic; the
   trade-off is a larger PR. The 3 000-line budget accommodates it.
2. **ANY-of vs. ALL-of semantics must be confirmed.** Users expect ANY-of
   (OR / union) by default but the slice must ask before locking. ALL-of
   is also valid but rare; if chosen, the SQL and filter UI both change.
3. **Existing-data case-fold duplicates.** If any pre-existing
   `Dairy`/`dairy` rows exist, the new service-layer guard will reject
   further operations on them. Proposal must specify a remediation path.
4. **N+1 in reports disappears with the SQL-level filter, but a new
   per-row read in `get_product` must be batched** so the detail bundle
   (used by `ProductDetailPage` and dashboard detail modal) doesn't
   regress. The proposed `list_product_category_ids_by_product_ids`
   helper is the fix.
5. **Restore-from-backup flow (Slice 12).** Pre-V4 backups need an
   adapter or re-migration. Proposal must call this out.
6. **CSV import back-compat.** Single-category CSV files keep working.
   Multi-category CSV import is deferred; proposal should explicitly opt
   out of it for this slice.
7. **Search backend shape.** `list_categories` returns the full set
   today; the picker needs server-side search at 200+ categories. The
   slice must include it. Decision: extend with `query`/`limit` args or
   add `list_categories_search` command.
8. **No popover primitive today.** New picker introduces a floating-UI
   primitive (mirrors the date picker). Keyboard contract must be
   enumerated in the design.
9. **Reviewer load.** A single ~1 400–2 300 LOC PR is well within budget
   but is on the larger end of typical PRs in this repo (most prior
   slices are < 1 000 LOC). Plan the review surface explicitly.
10. **Empty `<select>` removal is visible.** Removing the (already-broken)
    `<select>` is a user-visible change. The apply-progress ledger must
    record the equivalence mapping (chips render for selected; search
    results render for the picker; no more "all active categories as
    pills").

---

## 10. Substrate the proposal phase needs

### Hard constraints (already user-confirmed or canonical)

- Platforms: Linux + Windows (Tauri 2 WebKitGTK / WebView2).
- Architecture: local-first desktop, no network calls. No new frontend
  test harness.
- Verify gates: `npx svelte-check --workspace . --threshold error`,
  `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml
  --lib` with the existing 2-pre-existing-failure baseline
  (`services::reports::tests::preview_report_in_alert_window_returns_alert_lots`,
  `services::reports::tests::preview_report_next_30_days_returns_30d_lots`).
- Manual smoke matrix on Linux WebKitGTK and Windows WebView2, recorded
  in the apply-progress ledger.
- Spec language: `en` (project config).
- Empty → `null` semantics preserved at every consumer for the **old**
  field (e.g., when `category_ids: None` is passed, behave like today).
  New convention: empty list `[]` is valid and means "unassigned".
- Duplicate rule: case-insensitive at the service layer; SQLite `UNIQUE`
  constraint stays as the safety net.
- Archive-only: no hard delete from this slice. (`update_category({ is_active:
  false })` is already wired.)
- Review budget for this change: **3 000 changed lines** (user-typed
  session override; project default 800; canonical threshold 400).

### Open product questions (defer to proposal / parent round)

1. **Schema shape: Option A (drop legacy FK) vs. Option B (hybrid, keep FK
   as "primary category").** Default recommendation: A. Proposal must
   confirm.
2. **ANY-of vs. ALL-of filter semantics.** Default: ANY-of. Proposal must
   confirm.
3. **Backfill / drop order in V4.** Single transaction vs. separate
   statements; idempotency approach (`INSERT ... WHERE NOT EXISTS`).
4. **Existing-data case-fold duplicates.** What to do if any exist
   (dedupe as part of V4, accept + surface error, or `COLLATE NOCASE`).
5. **CSV import.** Stay single-category per row for this slice, or grow
   to multi-category (semicolon-separated, separate columns). Default:
   stay single-category.
6. **Restore-from-backup.** Adapter that translates pre-V4 backups, or
   require re-migration. Default: adapter.
7. **Search backend shape.** Extend `list_categories` with `query`/
   `limit` vs. new `list_categories_search`. Default: extend.
8. **Inline-create UX in the picker.** Hide it, route through a
   confirmation step, or keep as-is. UX brief: "controlled category
   creation should be intentional."
9. **Archived categories in the picker.** Hide entirely (picker = active
   only; restore is admin work), or surface as a "Show archived"
   toggle. Default: hide.
10. **`category_id` deprecation in the wire shape.** Single release cycle
    with both fields (legacy `category_id` is always equal to the first
    of `category_ids`), or hard rename. Default: hard rename (the app is
    pre-1.0 and no external consumer exists).

### Codebase constraints the proposal should mirror

- **No popover / portal / floating UI primitive.** Existing modals are
  page-scoped overlays (`role="dialog"`, `aria-modal="true"`,
  `.modal-overlay` / `.modal-box`). The multi-select combobox is a
  non-modal floating popover — a genuinely new primitive, but a popover
  can be implemented with `position: absolute` relative to the trigger
  via a `bind:this` ref + `getBoundingClientRect` (mirrors the existing
  date picker popover).
- **No CSS framework.** Hand-written CSS in `<style>` per component.
- **No `tsconfig.json` strict mode** at repo root (frontend relies on
  `svelte-check`).
- **`strictTdd: false`** — manual smoke is the gate, recorded in the
  apply-progress ledger.
- **Migration runner uses `MigrationType::Simple` with `no_tx: false`**;
  V4 should follow the same pattern.

### Build / verify command set

Frontend:

- `npx svelte-check --workspace . --threshold error`
- `npm run build`
- Manual smoke: Linux WebKitGTK + Windows WebView2 with 0, 50, 200 active
  categories and products with 1, 3, 5 categories each — covers empty
  selection, single selection, multi selection, chip removal, duplicate
  error path, archived-category behaviour.

Backend:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib` — must remain
  GREEN with the same 2 pre-existing failures (not new failures).
- New unit tests for the case-fold guard (`create_category` + rename
  path).
- New unit tests for the multi-category write/read round-trip
  (`create_product({ category_ids: vec!["a","b"] })`).
- New unit tests for the report filter any-of semantics and
  no-double-count.
- New V4 migration tests (back-fill correctness, idempotency).

---

## 11. Recommended next phase

Proceed to **`proposal.md`** with `deliveryStrategy: ask-on-risk` round on
the 3–5 product questions in §10. The must-ask questions are (in priority
order):

1. Schema shape: drop legacy FK (Option A) or hybrid (Option B)?
   (Binary, blocks everything else.)
2. ANY-of vs. ALL-of filter semantics? (Blocks report filter UI and SQL.)
3. Existing-data case-fold duplicates — what to do if any exist? (Blocks
   the migration story.)
4. CSV import: single-category per row for this slice, or grow now?
   (Blocks CSV-import tasks.)
5. Restore-from-backup: adapter or re-migration? (Blocks the
   apply-progress plan.)

After locking those, write the proposal + spec delta under
`## Capability: Categories` (one `MODIFIED` requirement
`editable category list` absorbing the multi-category model + one new
`ADDED` requirement for any-of filter semantics), plus a
`MODIFIED` requirement under `## Capability: Reports` for the new filter
vocabulary. Then `design.md` and `tasks.md`.

The slice stays a single PR within the 3 000-line session budget. No
`chainedPrStrategy` choice required; no `size:exception` needed unless the
final LOC count exceeds 3 000 after design locks.

## Key Learnings

- The DB schema today is **single FK**: `products.category_id TEXT
  REFERENCES categories(id)`, no `product_categories` junction. Multi-
  category is structurally impossible without a V4 migration.
- **Every DTO is single-id-shaped** at the Rust and TS layers:
  `ProductCreate.category_id`, `ProductUpdate.category_id`,
  `ProductResponse.category_id`, `ProductSearchResult.category_id`,
  `ProductDetailResponse.category: Option<CategoryResponse>`,
  `ReportFilters.category_id`. `DashboardFilters` does not currently
  carry a category field at all.
- The **report category filter** is a post-filter in
  `services::reports::filter_by_category` that does a per-row `get_product`
  call to compare `product.category_id == target` — already N+1; the
  multi-category work should replace it with a SQL-level `EXISTS`
  subquery against the new junction.
- **Dashboard filter is store/location only** today. No `category_id`
  field on `DashboardFilters`. The slice is an opportunity to add
  `category_ids` at the SQL level (no N+1).
- **ProductForm has a dead `<select>`** (lines ~326–328) that renders
  only `— None —`; the real interactivity is the `<ul
  class="category-pills">` (lines ~330–344) with one button per active
  category. Both go away when the picker is replaced with a multi-select
  combobox.
- **`update_category({ is_active: false })` already supports archive**;
  no new archive command needed. The slice just needs the UI and the FK
  `ON DELETE RESTRICT` semantics on `product_categories`.
- **SQLite `UNIQUE` on `categories.name` is case-sensitive.** Form and
  CSV import have a semantic gap (CSV folds case via
  `resolve_category_name`; form does not). The case-fold guard lands at
  the service layer in this slice.
- **Existing 2-pre-existing-failure baseline** in
  `services::reports::tests` is scoped to alert-window and next-30-days
  fixtures; the slice's verify report must preserve the baseline and
  not introduce new failures.
- **No frontend test harness**, no `vitest`, no `playwright`. Manual
  smoke on Linux WebKitGTK and Windows WebView2 is the canonical gate.
- **No popover / portal primitive** exists. The new multi-select
  combobox introduces a floating-UI primitive — same family as the date
  picker popover. Keyboard contract must be enumerated in the design.
- **No `tsconfig.json` strict mode** at repo root. Frontend relies on
  `svelte-check` only.
- **Review budget is 3 000 lines** for this change (user-typed session
  override). Project default is 800; canonical threshold is 400. The
  recommended scope lands at ~1 400–2 300 LOC, well under all three.
- **V3 unit-catalog migration** is the in-repo pattern to follow for
  V4: ALTER TABLE add nullable FK + INSERT OR IGNORE seed + idempotent
  back-fill. The same shape covers `product_categories`.
- **Restore-from-backup** (Slice 12) must handle pre-V4 backups that
  carry `products.category_id`. Proposal must specify an adapter path.
- **ANY-of is the default user expectation** for multi-category
  filters; ALL-of is rare and must be confirmed. SQL is `EXISTS` for
  ANY-of and `COUNT(*) = N` for ALL-of.
- **Duplicate-count risk** is real for join-based filters. Use
  `EXISTS`, not `JOIN`, to keep `DashboardLotRow` rows distinct by
  `lot_id`. SQL-level filter is mandatory — replacing the existing
  per-row post-filter is the same change.
- **Empty-list semantics:** `category_ids: Some(vec![])` ≠ `None`.
  `None` (or empty `Vec` per Rust idiom) means "no filter"; explicit
  empty list means "unassigned". Proposal must clarify.
- **Tests that seed `category_id: None` in fixtures** (`services::
  products.rs`, `services::expiry_lots.rs`, `services::notifications.rs`,
  `services::unit_definitions.rs`, `services::csv_io.rs`,
  `pdf/report_pdf.rs`) keep compiling under the rename to
  `category_ids: vec![]`. Fixtures that exercise the single-FK write
  path need to switch to junction seeding or to the service-layer API.

## Artifact store

`both` (per project config; session honors OpenSpec artifacts in repo).
Persisted to:

- `openspec/changes/caduxo-category-management/explore.md` (this file)
- Engram mirror: `mem_save` with
  `topic_key: "sdd/caduxo-category-management/explore"`,
  `type: "architecture"`, `project: "Caduxo"`,
  `capture_prompt: false`.

## skill_resolution

`none` — the parent did not inject `## Skills to load before work` paths
for this phase, and the project/user skill registry was not independently
consulted per the executor contract. No fallback skill loading was
performed.
