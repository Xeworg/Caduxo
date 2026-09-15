# Tasks — caduxo-category-management

> **Scope.** Real multi-category support for products, including the V4
> `product_categories` migration, the case-fold guard, the
> `CategoryPicker.svelte` primitive, multi-category filters on
> `ProductCatalogPage` and `ReportsPage`, and the new category search API.
> Single PR within the user-typed **3,000-line** session review budget.
> No chained-PR split, no `size:exception`. The legacy
> `products.category_id` column stays in the schema as ignored legacy
> data — runtime never reads or writes it after V4.
> The plan follows design §15 and proposal §"Confirmed product decisions".

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1,400–2,400 (backend ~700–1,100 + frontend ~500–800 + tests ~200–320 + spec delta ~50–90) |
| 400-line budget risk | Low (canonical 400-line threshold would be High; the user's typed 3,000-line session override moves it to Low) |
| Chained PRs recommended | No |
| Suggested split | Single PR (PR 1 = full slice) |
| Delivery strategy | single-pr |
| Chain strategy | stacked-to-main |

```text
Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: stacked-to-main
400-line budget risk: Low
```

---

## Pre-flight (apply-phase gate)

- [x] Confirm `openspec/config.yaml` still pins `sdd.executionMode: interactive`, `artifactStore: both`, `strictTdd: false`, and that the session preflight carries `reviewBudgetChangedLines: 3000`. <!-- sdd-owner: implementation -->
- [x] Run baseline: `cargo test --manifest-path src-tauri/Cargo.toml --lib` and capture the existing 2-failure baseline (`preview_report_in_alert_window_returns_alert_lots`, `preview_report_next_30_days_returns_30d_lots`) so the slice can prove it does not regress. <!-- sdd-owner: implementation -->
- [x] Run baseline: `npx svelte-check --workspace . --threshold error` and `npm run build` to confirm a clean starting state. <!-- sdd-owner: implementation -->

---

## WU-1 — Backend: V4 migration `product_categories`

- [x] Add the `V4 = "add_product_categories_v4"` entry in `src-tauri/src/db/migrations.rs::MIGRATIONS` following the existing `MigrationType::Simple` + `no_tx: false` pattern from V3. DDL: `CREATE TABLE product_categories (product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE, category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE RESTRICT, created_at TEXT NOT NULL, PRIMARY KEY (product_id, category_id))` plus `idx_product_categories_category` and `idx_product_categories_product`. <!-- sdd-owner: implementation -->
- [x] Implement the idempotent back-fill from `products.category_id` with the `INSERT … WHERE NOT EXISTS` shape from design §3.3. <!-- sdd-owner: implementation -->
- [x] Implement the case-fold dedup step (design §3.4) using subquery-based approach (no CTE window functions to avoid SQLite compatibility issues); reassign junction rows and legacy `products.category_id` rows to the canonical id; archive (`is_active = 0`) younger duplicates. <!-- sdd-owner: implementation -->
- [x] RED — add the migration test surface in `db::migrations::tests` (per design §3.5): `v4_applies_on_fresh_db`, `v4_migration_is_idempotent`, `v4_backfill_copies_legacy_category_id_into_junction`, `v4_backfill_is_idempotent`, `v4_backfill_skips_null_category_id`, `v4_case_fold_dedup_keeps_oldest`, `v4_case_fold_dedup_remaps_legacy_column`, `v4_legacy_column_is_left_intact_for_non_duplicate_categories`. <!-- sdd-owner: implementation -->
- [x] GREEN / TRIANGULATE — run `cargo test --manifest-path src-tauri/Cargo.toml --lib db::migrations::tests` and capture the output in the apply-progress ledger. <!-- sdd-owner: implementation -->
- [x] REFACTOR — add a comment in `migrations.rs` and `repositories/products.rs` enforcing the invariant "runtime never reads or writes `products.category_id` after V4". <!-- sdd-owner: implementation -->

## WU-2 — Backend: DTOs (Rust)

- [x] Update `src-tauri/src/dto/products.rs` per design §4.1: `ProductCreate.category_ids: Option<Vec<String>>`, `ProductUpdate.category_ids: Option<Vec<String>>`, `ProductResponse.category_ids: Vec<String>` (drop the legacy `category_id` field), `ProductDetailResponse.categories: Vec<CategoryResponse>` (drop the legacy `category` field), `ProductSearchResult.category_ids: Vec<String>` (drop the legacy `category_id` field). <!-- sdd-owner: implementation -->
- [x] Update `src-tauri/src/dto/reports.rs`: `ReportFilters.category_ids: Option<Vec<String>>` (replaces `category_id`). <!-- sdd-owner: implementation -->
- [x] Update `src-tauri/src/dto/dashboard.rs`: add `DashboardFilters.category_ids: Option<Vec<String>>`. <!-- sdd-owner: implementation -->
- [x] Add `CategorySearchPage { items: Vec<CategoryResponse>, total: usize, has_more: bool }` and `CategorySearchInput { query: String, limit: Option<usize> }` in `src-tauri/src/dto/products.rs` (co-located per design §4.1 — project convention). <!-- sdd-owner: implementation -->
- [x] Add `pub const UNCATEGORIZED_SENTINEL: &str = "__uncategorized__";` next to the new types. <!-- sdd-owner: implementation -->
- [x] GREEN — run `cargo build --manifest-path src-tauri/Cargo.toml` (compile-only) so the rename's downstream compile errors surface as a single batch; record the failing files in the apply-progress ledger. <!-- sdd-owner: implementation -->

## WU-3 — Backend: Junction repository helpers

- [x] Add `list_product_category_ids_by_product_ids(pool, ids: &[String]) -> Result<HashMap<String, Vec<String>>, sqlx::Error>` in `src-tauri/src/db/repositories/products.rs` per design §5.1 (single batched read, ordered by `product_id, category_id`). <!-- sdd-owner: implementation -->
- [x] Add `find_category_by_name_ci(pool, normalized: &str)` next to the existing `find_by_key` for units: `SELECT id, name, is_active, created_at, updated_at FROM categories WHERE lower(name) = lower($1) LIMIT 1`. <!-- sdd-owner: implementation -->
- [x] Add a per-product junction SELECT helper used by the new read paths (one `SELECT category_id FROM product_categories WHERE product_id = $1`). <!-- sdd-owner: implementation -->
- [x] RED — repository tests: `list_product_category_ids_by_product_ids_empty_input_returns_empty_map`, `list_product_category_ids_by_product_ids_groups_by_product_id`. <!-- sdd-owner: implementation -->
- [x] GREEN / TRIANGULATE — `cargo test --manifest-path src-tauri/Cargo.toml --lib db::repositories::products::tests` and capture output. <!-- sdd-owner: implementation -->

## WU-4 — Backend: Repository write paths (insert / update product)

- [x] Rewrite `insert_product` in `src-tauri/src/db/repositories/products.rs` to drop the `category_id` column from `INSERT INTO products` and write through the junction inside a transaction (delete-then-insert on update, replace single-FK write). <!-- sdd-owner: implementation -->
- [x] Rewrite `update_product` to perform `UPDATE products` (without `category_id`), then `DELETE FROM product_categories WHERE product_id = $1`, then `write_junction(&mut tx, &id, &category_ids)`; commit the transaction; return `Ok(None)` if zero rows affected. <!-- sdd-owner: implementation -->
- [x] Add a private `write_junction` helper that inserts each category id, skipping duplicates (the composite PK protects integrity either way). <!-- sdd-owner: implementation -->
- [x] RED — repository tests per design §13.1: `insert_product_writes_junction_rows`, `insert_product_with_empty_category_ids_writes_no_junction`, `insert_product_with_three_category_ids_writes_three_junction_rows`, `update_product_replaces_full_junction_set`, `update_product_from_two_to_zero_categories_removes_junction`. <!-- sdd-owner: implementation -->
- [x] GREEN / TRIANGULATE — `cargo test --manifest-path src-tauri/Cargo.toml --lib db::repositories::products::tests`; capture output. <!-- sdd-owner: implementation -->

## WU-5 — Backend: Repository read paths (get / search / find)

- [x] Update `get_product` to populate `category_ids: Vec<String>` from the per-product junction SELECT helper. <!-- sdd-owner: implementation -->
- [x] Update `search_products`, `find_by_barcode_exact`, `find_by_sku_exact` to add `category_ids: Vec<String>` from the same per-product SELECT (no name join at the search-hit level — wire shape stays narrow). <!-- sdd-owner: implementation -->
- [x] Update `list_all_products_for_export` to fetch all products, fetch all junction rows in one batched query, build a `HashMap<String, Vec<String>>` for `category_ids` and a paired CSV-only `category_names_csv` field (off the wire shape). <!-- sdd-owner: implementation -->
- [x] RED — repository tests: `get_product_returns_category_ids_from_junction`, `search_products_returns_category_ids_from_junction`, `find_by_barcode_exact_returns_category_ids_from_junction`, `find_by_sku_exact_returns_category_ids_from_junction`, `list_all_products_for_export_includes_csv_only_category_names_string`, `delete_product_cascades_junction_rows`, `delete_category_with_junction_is_rejected`. <!-- sdd-owner: implementation -->
- [x] GREEN / TRIANGULATE — `cargo test --manifest-path src-tauri/Cargo.toml --lib db::repositories::products::tests`; capture output. <!-- sdd-owner: implementation -->

## WU-6 — Backend: Dashboard filter SQL (EXISTS, no JOIN)

- [x] Update `src-tauri/src/db/repositories/dashboard.rs::list_dashboard_lots` per design §5.2: accept `DashboardFilters.category_ids: Option<Vec<String>>`; build the optional `EXISTS` clause (not JOIN) for any-of real ids; build `NOT EXISTS` for the `UNCATEGORIZED_SENTINEL`; compose both with `OR` when the selection contains real ids and the sentinel; preserve "empty selection = no filter" semantics. <!-- sdd-owner: implementation -->
- [x] RED — dashboard repository tests per design §13.1: `list_dashboard_lots_with_no_category_filter_returns_all_lots`, `list_dashboard_lots_with_empty_category_ids_returns_all_lots`, `list_dashboard_lots_with_one_category_id_returns_any_of_lots`, `list_dashboard_lots_with_two_category_ids_returns_any_of_lots_without_double_count`, `list_dashboard_lots_with_uncategorized_sentinel_returns_products_with_zero_junction`, `list_dashboard_lots_with_real_id_and_uncategorized_composes_correctly`, `list_dashboard_lots_with_archived_category_id_in_selection_returns_no_lots_after_archive`. <!-- sdd-owner: implementation -->
- [x] GREEN / TRIANGULATE — `cargo test --manifest-path src-tauri/Cargo.toml --lib db::repositories::dashboard::tests`; capture output. <!-- sdd-owner: implementation -->

## WU-7 — Backend: Category search service + command

- [x] Add `src-tauri/src/services/categories.rs::search(pool, input) -> CategorySearchPage` per design §8 (empty query → active list ordered by name; non-empty → prefix first, substring fallback; active only; `limit` default 50, clamped to `[1, 500]`; `has_more` from total). <!-- sdd-owner: implementation -->
- [x] Add `commands::products::list_categories_search` Tauri command wired to the new service. <!-- sdd-owner: implementation -->
- [x] Register `commands::products::list_categories_search` in `src-tauri/src/lib.rs` alongside the existing `list_categories` registration. <!-- sdd-owner: implementation -->
- [x] RED — search service tests: `search_empty_query_returns_active_categories`, `search_prefix_match_for_partial_query`, `search_falls_back_to_substring_when_no_prefix`, `search_excludes_archived_categories`, `search_respects_limit`, `search_has_more_true_when_total_exceeds_limit`. <!-- sdd-owner: implementation -->
- [x] GREEN / TRIANGULATE — `cargo test --manifest-path src-tauri/Cargo.toml --lib services::categories::tests`; capture output. <!-- sdd-owner: implementation -->

## WU-8 — Backend: Service write paths (products + category case-fold guard)

- [x] Update `src-tauri/src/services/products.rs::create_product` / `update_product` per design §6.1: validate `category_ids` (empty `Vec` / `None` = unassigned; each id must exist (`SELECT id FROM categories WHERE id = ANY($1)`); archived ids rejected by default; de-duplicate input before write); call the repository's transactional write. <!-- sdd-owner: implementation -->
- [x] Update `services::products::get_product` to resolve all category ids to `Vec<CategoryResponse>` via one `SELECT … WHERE id = ANY($1)` (small set; not N+1); assemble `ProductDetailResponse { product, barcodes, categories }`. <!-- sdd-owner: implementation -->
- [x] Add the case-fold guard to `create_category` and `update_category` per design §6.1: check `find_category_by_name_ci` before insert / update; reject with `DuplicateField { field: "name", value: <typed> }`; keep `category_unique_error` translation as the canonical race-condition safety net. <!-- sdd-owner: implementation -->
- [x] RED — service tests: `create_product_rejects_nonexistent_category_id`, `create_product_rejects_archived_category_id_by_default`, `create_category_rejects_case_collision_with_existing` ("Dairy" then "dairy"), `update_category_rename_rejects_case_collision`, `update_category_rename_to_own_current_name_succeeds`, `get_product_resolves_category_ids_to_categories_vec`, `get_product_with_no_categories_returns_empty_categories_vec`. <!-- sdd-owner: implementation -->
- [x] GREEN / TRIANGULATE — `cargo test --manifest-path src-tauri/Cargo.toml --lib services::products::tests`; capture output. <!-- sdd-owner: implementation -->

## WU-9 — Backend: Reports service (remove post-filter, forward `category_ids`)

- [x] Remove `services::reports::filter_by_category` post-filter entirely. <!-- sdd-owner: implementation -->
- [x] Update `services::reports::to_dashboard_filters` to pass `category_ids` through from `ReportFilters` to `DashboardFilters`. <!-- sdd-owner: implementation -->
- [x] Update `src-tauri/src/services/csv_io.rs::import_row` to call `create_product` / `update_product` with a one-element `category_ids: vec![resolved_id]` (single-column CSV unchanged; pass-through to the service-layer signature). <!-- sdd-owner: implementation -->
- [x] Update fixture `services/reports/tests::seed_report_fixture` to seed via the service-layer API with `category_ids: Some(vec![dairy.id.clone()])` instead of the legacy `category_id: Some(...)`. <!-- sdd-owner: implementation -->
- [x] RED — reports service tests: `preview_report_custom_with_multiple_categories_returns_any_of_lots`, `preview_report_custom_with_uncategorized_returns_zero_category_lots`, `preview_report_custom_with_real_and_uncategorized_composes`. <!-- sdd-owner: implementation -->
- [x] Update existing tests `preview_report_custom_with_category_filter` and `preview_report_metadata_captures_effective_filters` to use `category_ids` and assert against `filters_used.category_ids`. <!-- sdd-owner: implementation -->
- [x] GREEN / TRIANGULATE — `cargo test --manifest-path src-tauri/Cargo.toml --lib services::reports::tests`; confirm the 2-pre-existing-failure baseline is preserved (`preview_report_in_alert_window_returns_alert_lots`, `preview_report_next_30_days_returns_30d_lots`) and that no new failures appear. <!-- sdd-owner: implementation -->

## WU-10 — Backend: Existing test fixture update sweep

- [x] Audit fixtures in `src-tauri/src/services/products.rs`, `services/expiry_lots.rs`, `services/notifications.rs`, `services/unit_definitions.rs`, `services/csv_io.rs`, `pdf/report_pdf.rs` that seed `category_id: None` (or `category_id: Some(...)`) and update to the new `category_ids` shape: `category_ids: vec![]` (or service-layer API call) where they only meant "no category"; switch fixtures that exercise the single-FK write path to use the service-layer API. <!-- sdd-owner: implementation -->
- [x] GREEN / TRIANGULATE — full backend test run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`. Record the result and confirm the 2-pre-existing-failure baseline is unchanged. <!-- sdd-owner: implementation -->

## WU-11 — Backend: Restore-from-backup smoke (no new code)

- [x] RED — add `services/backup_restore::tests::restore_from_pre_v4_backup_applies_v4_backfill_in_situ` per design §13.1: seed a V3-only DB fixture with `products.category_id` rows; restore via `VACUUM INTO`; re-open the pool; assert V4 applies, junction rows are populated, and the picker-facing service returns the right ids. <!-- sdd-owner: implementation -->
- [x] GREEN / TRIANGULATE — `cargo test --manifest-path src-tauri/Cargo.toml --lib services::backup_restore::tests`; capture output. <!-- sdd-owner: implementation -->

## WU-12 — Frontend: TS DTOs + `src/lib/categories.ts`

- [x] Update `src/lib/products.ts`: `ProductCreate.category_ids?: string[]`, `ProductUpdate.category_ids?: string[]`, `ProductResponse.category_ids: string[]` (drop legacy `category_id`), `ProductDetailResponse.categories: CategoryResponse[]` (drop legacy `category`), `ProductSearchResult.category_ids: string[]` (drop legacy `category_id`). <!-- sdd-owner: implementation -->
- [x] Update `src/lib/dashboard.ts`: add `DashboardFilters.category_ids?: string[]`. <!-- sdd-owner: implementation -->
- [x] Update `src/lib/reports.ts`: replace `ReportFilters.category_id` with `ReportFilters.category_ids?: string[]`. <!-- sdd-owner: implementation -->
- [x] Add `src/lib/categories.ts` (new) with `UNCATEGORIZED_SENTINEL`, `CategorySearchInput`, `CategorySearchPage`, and the `listCategoriesSearch({ query, limit })` wrapper around the new Tauri command. <!-- sdd-owner: implementation -->
- [x] GREEN — `npx svelte-check --workspace . --threshold error` (compile-only gate) and capture the failing components in the apply-progress ledger. <!-- sdd-owner: implementation -->

## WU-13 — Frontend: `CategoryPicker.svelte` primitive

- [x] Create `src/components/inputs/CategoryPicker.svelte` per design §9 with the exact prop contract: `bind:value` (`string[]`), `categories` (optional initial list), `placeholder` (default `"Search categories…"`), `includeUncategorized` (default `false`), `on:create` event. <!-- sdd-owner: implementation -->
- [x] Implement the WAI-ARIA combobox markup (`role="combobox"`, `aria-expanded`, `aria-controls`, `aria-activedescendant` on the trigger; `role="listbox"` on the result list; `role="option"` per row with `aria-selected`). <!-- sdd-owner: implementation -->
- [x] Implement the chip row above the input with per-chip `aria-label="Remove {name}"` × buttons and a `Clear all` link (`aria-label="Clear all selected categories"`). <!-- sdd-owner: implementation -->
- [x] Implement the popover positioning action that mirrors `caduxo-custom-date-picker` (absolute positioning via `getBoundingClientRect`, flip-up when `< 200px` below). <!-- sdd-owner: implementation -->
- [x] Implement the inline-create row: when the typed query has no match in the result list, render `Create "<query>"` button; clicking calls `createCategory({ name: trimmedQuery })`, appends the new id, dispatches `create`, and shows an inline error on `DuplicateField { field: "name" }`. No silent-create path. <!-- sdd-owner: implementation -->
- [x] When `includeUncategorized={true}`, render the `Uncategorized` pseudo-row at the top of the result list with a distinct style; toggling it pushes/pops `UNCATEGORIZED_SENTINEL` into `value`. <!-- sdd-owner: implementation -->
- [x] Wire keyboard ergonomics per design §9.4 (`Tab` enter/leave, `Esc` close without committing typed text, `ArrowDown`/`ArrowUp` move active descendant, `Enter` select, `Backspace` in empty input removes last chip, click-outside closes). <!-- sdd-owner: implementation -->
- [x] GREEN — `npx svelte-check --workspace . --threshold error` after the picker is consumed by at least one host page (otherwise the file has unused-prop warnings). <!-- sdd-owner: implementation -->

## WU-14 — Frontend: Host adoption `ProductForm.svelte`

- [x] Replace `let categoryId: string = ""` (line ~43) with `let categoryIds: string[] = []`; initialize from `initial.category_ids ?? []` in edit mode. <!-- sdd-owner: implementation -->
- [x] Remove the empty `<select bind:value={categoryId}>` (~line 326) and the `<ul class="category-pills">` (~line 330). <!-- sdd-owner: implementation -->
- [x] Replace the inline-category `<div class="inline-category">` block (~lines 133–149) with the picker's inline-create row. <!-- sdd-owner: implementation -->
- [x] Render `<CategoryPicker bind:value={categoryIds} {categories} includeUncategorized={false} on:create={onCategoryCreated} />`. <!-- sdd-owner: implementation -->
- [x] Submit payload uses `category_ids: categoryIds` (line ~247). <!-- sdd-owner: implementation -->

## WU-15 — Frontend: Host adoption `ProductDetailPage.svelte` and `DashboardPage.svelte`

- [x] `src/components/ProductDetailPage.svelte` (~line 207): render multiple `.category-badge` chips or a comma-joined label per design §10.2; muted `Uncategorized` badge when the array is empty. <!-- sdd-owner: implementation -->
- [x] `src/components/DashboardPage.svelte` (~line 606, detail modal): render `detailProduct.categories.map(c => c.name).join(", ")` (or a list); `Uncategorized` when empty. <!-- sdd-owner: implementation -->

## WU-16 — Frontend: Host adoption `ProductCatalogPage.svelte`

- [x] Add a `<CategoryPicker bind:value={categoryIds} includeUncategorized={true} placeholder="Filter by category…" />` at the top of the page. <!-- sdd-owner: implementation -->
- [x] Wire the filter to the existing client-side catalog match (`category_ids` intersect against the selection); empty selection = no filter (no explicit All chip). <!-- sdd-owner: implementation -->

## WU-17 — Frontend: Host adoption `ReportsPage.svelte`

- [x] Replace `let categoryId: string` state with `let categoryIds: string[] = []`. <!-- sdd-owner: implementation -->
- [x] Replace the `<select bind:value={categoryId}>` (~line 353) with `<CategoryPicker bind:value={categoryIds} includeUncategorized={true} />`. <!-- sdd-owner: implementation -->
- [x] Update `buildFilters()` (~line 143) to use `category_ids: categoryIds.length > 0 ? categoryIds : null`. <!-- sdd-owner: implementation -->
- [x] Update the filter summary (~line 265) to render `categories (N): <id1.slice(0,4)>, <id2.slice(0,4)>, …` with a `categories=uncategorized` segment when the sentinel is included; no raw id leak. <!-- sdd-owner: implementation -->

## WU-18 — Verify gates (frontend + backend + manual smoke)

- [x] Run `npx svelte-check --workspace . --threshold error` and capture the result (must pass). <!-- sdd-owner: implementation -->
- [x] Run `npm run build` and capture the result (must pass). <!-- sdd-owner: implementation -->
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml --lib` and confirm: (a) the 2-pre-existing-failure baseline is unchanged, (b) all new tests pass. <!-- sdd-owner: implementation -->

### Manual smoke matrix

> Per the canonical `Engineering safety > tests accompany implementation`
> requirement, the slice has no frontend test harness
> (`strictTdd: false`). The matrix is the verification gate.

| # | Step | Expected | Evidence |
|---|------|----------|----------|
| 1 | `ProductForm`: type `Dai` | `Dairy` appears in the result list | screenshot / note |
| 2 | `ProductForm`: select `Bak` → `Bakery`; select `Dai` → `Dairy` | Two chips above the input | screenshot / note |
| 3 | `ProductForm`: click × on `Dairy` chip | Chip disappears from selection; chip list updates without refetch | screenshot / note |
| 4 | `ProductForm`: click `Clear all` | All chips clear; `value === []` | screenshot / note |
| 5 | `ProductForm`: type `abc` (no match) | `Create "abc"` row appears | screenshot / note |
| 6 | `ProductForm`: click `Create "abc"` | New chip appears; backend `categories` row created | screenshot / note |
| 7 | `ProductCatalogPage`: select `Dairy` | Only products in `Dairy` show | screenshot / note |
| 8 | `ProductCatalogPage`: select `Uncategorized` pseudo-row | Only products with zero active categories show | screenshot / note |
| 9 | `ReportsPage`: select `Dairy, Bakery` | Lots whose product is in either appear; no double count for products in both | screenshot / note |
| 10 | `ReportsPage`: filter summary | `categories (2): <id…>, <id…>` (no raw id leak) | screenshot / note |
| 11 | `ReportsPage`: empty selection | Filter summary omits categories; report unfiltered | screenshot / note |
| 12 | Archive a category (existing `update_category({ is_active: false })` path) | Picker no longer surfaces it; lots only in that category shift to `Uncategorized` | screenshot / note |
| 13 | `ProductDetailPage` for a product with two categories | Two `.category-badge` chips render | screenshot / note |
| 14 | Dashboard detail modal | Comma-joined names (or list) render in `<dd>` | screenshot / note |
| 15 | Calendar tab smoke | Inherits the new `DashboardFilters` shape without UI changes | screenshot / note |
| 16 | Restore from a pre-V4 fixture (`services::backup_restore::tests` shape) | Pool reopens; V4 applies; junction rows populated from legacy column | screenshot / note |
| 17 | CSV round-trip | Export of multi-category product emits comma-joined names; re-import parses first name | screenshot / note |
| 18 | Keyboard ergonomics | Tab → ArrowDown → Enter → Esc; chip appears, popover closes, focus returns to trigger | screenshot / note |

## WU-19 — Spec delta (`openspec/specs/caduxo-expiry-tracker/spec.md`)

- [x] MODIFY `## Capability: Categories > editable category list` in place: products may belong to one, many, or zero categories; unassigned is valid; archived categories are hidden from pickers but preserved in history. <!-- sdd-owner: implementation -->
- [x] ADD `## Capability: Categories > multi-category product model`: covers the `product_categories` junction table, composite uniqueness, ON DELETE CASCADE / RESTRICT semantics, and the service-layer case-fold guard. <!-- sdd-owner: implementation -->
- [x] ADD `## Capability: Categories > category_filter_sentinel_uncategorized` scenario: lots whose product has zero junction rows to active categories are included when the picker selects the `Uncategorized` pseudo-row. <!-- sdd-owner: implementation -->
- [x] MODIFY `## Capability: Product catalog > mandatory unique SKU`: clarify that `category_ids` is a list of zero or more ids (not a single nullable FK). <!-- sdd-owner: implementation -->
- [x] MODIFY `## Capability: Reports > report filters`: `category_ids` (any-of), `Uncategorized` pseudo-row, SQL-level `EXISTS` filter, default `None` / `Some(vec![])` = no filter. <!-- sdd-owner: implementation -->
- [x] ADD `## Capability: Reports > multi-category filter any-of semantics` scenario: lots whose product belongs to **any** of the selected categories appear exactly once. <!-- sdd-owner: implementation -->
- [x] MODIFY `## Capability: Dashboard > operational main screen` (or most-fitting dashboard requirement): add `category_ids` filter (any-of, default no filter, `Uncategorized` allowed). <!-- sdd-owner: implementation -->
- [x] MODIFY `## Capability: Data persistence and safety > migrations`: note V4 introduces `product_categories` and the case-fold dedup step. <!-- sdd-owner: implementation -->
- [x] Update `docs/prd.md` with the multi-category / picker / filter references (≤ 30 LOC). <!-- sdd-owner: implementation -->

## WU-20 — Apply-progress ledger and verify report

- [x] Record baseline test counts and the final test counts in `openspec/changes/caduxo-category-management/apply-progress.md`; confirm the 2-pre-existing-failure baseline is unchanged. <!-- sdd-owner: implementation -->
- [x] Record `npx svelte-check` and `npm run build` outputs in the apply-progress ledger. <!-- sdd-owner: implementation -->
- [x] Record the manual smoke matrix evidence (per WU-18) in the apply-progress ledger. <!-- sdd-owner: implementation -->
- [x] Record the V4 migration apply log (counts: `duplicates_found=N, remapped_junction=M, remapped_legacy=L, archived_duplicate=D`) for visibility on existing data. <!-- sdd-owner: implementation -->

---

## Parent actions (post-apply)

- [x] Bounded review of the slice per the project's review-workload discipline; confirm the apply-progress ledger captures all verify evidence and that the 2-pre-existing-failure baseline is preserved. Post-apply regression fixes were independently verified and committed as `83807f8` and `876084f`; user manual smoke confirmed the category picker now works. <!-- sdd-owner: parent -->
- [x] Decision: confirm `Decision needed before apply: No` is honored (single PR, no chained split, no `size:exception`); final slice remains within the user-typed 3,000-line override. <!-- sdd-owner: parent -->

## Key Learnings

- The legacy `products.category_id` column stays in the schema after V4 as ignored legacy data; **no runtime path reads or writes it** (design §2.2 split-brain prevention).
- Junction is `product_categories` with composite PK `(product_id, category_id)`; `product_id` is `ON DELETE CASCADE`, `category_id` is `ON DELETE RESTRICT` (archive-first).
- Filter semantics is **ANY-of with `EXISTS`** (never `JOIN`) so a product in two selected categories is matched exactly once; `ALL-of` is out of scope.
- `Uncategorized` is a sentinel id `__uncategorized__` rendered by the picker; the backend translates it to `NOT EXISTS (… product_categories JOIN categories WHERE c.is_active = 1)`.
- Empty `category_ids` (`None` or `Some(vec![])`) means **no filter**; the picker never renders an explicit All chip.
- Case-fold dedup is resolved inside V4 by `ROW_NUMBER() OVER (PARTITION BY lower(name) ORDER BY created_at ASC, id ASC)` — oldest wins; younger duplicates are archived (`is_active = 0`), junction rows and the legacy column are remapped to the canonical id.
- The runtime never fans out to two category-read paths: junction helper `list_product_category_ids_by_product_ids` for batches, single-product junction SELECT for per-product reads, SQL-level `EXISTS` for catalog/report filters.
- CSV import stays single-category per row; CSV export emits a comma-joined `category` cell so existing parsers still read the first name.
- Restore-from-backup is automatic via the migrator on `open_pool` — no new code in `services/backup_restore.rs`; pre-V4 backups pick up V4 + back-fill + case-fold dedup on the restored pool.
- Review budget override is **3,000 changed lines** (user-typed for this change); canonical threshold is 400. The slice lands at ~1,400–2,400 LOC. No chained-PR split; no `size:exception`.
- No frontend test harness (`strictTdd: false`); manual smoke on Linux/WebKitGTK + Windows/WebView2 is the verification gate per canonical `Engineering safety > tests accompany implementation`.
- The 2-pre-existing-failure baseline in `services::reports::tests` is preserved across the slice; the verify report captures before/after counts.
