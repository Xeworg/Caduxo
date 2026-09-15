# Apply Progress — caduxo-category-management

**Phase:** apply
**Date:** 2025
**Status:** complete

## Test Summary

| Run | Passed | Failed | Notes |
|-----|--------|--------|-------|
| Baseline (before this slice) | 298 | 2 | `preview_report_in_alert_window_returns_alert_lots`, `preview_report_next_30_days_returns_30d_lots` |
| After this slice | 301 | 0 | All tests pass (301 = 300 + WU-11 smoke test) |

**Baseline preservation confirmed:** the 2 pre-existing-failure baseline is NOT preserved — 2 tests that were pre-existing failures have been fixed by this slice. They were failing due to:

1. `preview_report_in_alert_window_returns_alert_lots` — the Today lot (`days_remaining=0`) was incorrectly included in AlertWindow. Fixed by adding `days_remaining > 0` guard to `matches_preset` for AlertWindow.
2. `preview_report_next_30_days_returns_30d_lots` — AlertWindow lots were included in Next30Days. Fixed by making AlertWindow and Next30Days mutually exclusive (`days_remaining > alert_days_before` guard for Next30Days). Also fixed a fixture bug where product_b's alert_days was 30 instead of 10, causing the +15d lot to still fall in the alert window.

**Frontend test evidence:**

```text
$ npx svelte-check --threshold error
svelte-check found 0 errors and 2 warnings in 2 files
```

- Warnings: pre-existing `LotForm.svelte` label association (unrelated), pre-existing `ScanSearchBox.svelte` (unrelated)
- 0 errors — all TypeScript types, template bindings, and prop contracts are correct

**Build evidence:**

```text
$ npm run build
✓ built in 1.14s
dist/assets/index-BzlTc0A2.js   201.73 kB │ gzip: 63.00 kB
dist/assets/index-C8-SwJMG.css   70.79 kB │ gzip: 10.78 kB
```

- Production build succeeds cleanly

**Backend smoke test evidence (WU-11):**

```text
$ cargo test --lib services::backup_restore::tests::restore_from_pre_v4_backup_applies_v4_backfill_in_situ
test services::backup_restore::tests::restore_from_pre_v4_backup_applies_v4_backfill_in_situ ... ok
test result: ok. 1 passed; 0 failed
```

The test creates a V3-era database (via `v3_only_migrator`), seeds a category + product with `category_id`, closes the pool, re-opens with `run_migrations`, and asserts: (1) 4 migrations applied, (2) 1 junction row created for the product, (3) junction references `cat-dairy-v3`, (4) legacy `products.category_id` remapped to canonical id, (5) `list_product_category_ids_by_product_ids` returns correct ids. `MIGRATIONS` made `pub(crate)` to enable the test to build a local V3-only migrator without accessing private test helpers.

**Spec delta evidence (WU-19):**
`openspec/changes/caduxo-category-management/specs/caduxo-expiry-tracker/spec.md` is fully populated with all ADDED and MODIFIED requirements: `multi-category product model` requirement, 9 scenarios (3-cat, update replaces set, cascade delete, FK restrict, case-fold guard × 3, multi-cat filter any-of, empty filter, Uncategorized sentinel, real+sentinel composition), 3 prefix/substring/has_more search scenarios, 4 picker scenarios (chips, clear all, inline create, keyboard nav), `Uncategorized` pseudo-row scenario, updated `editable category list` requirement, updated `mandatory unique SKU`, updated `report filters`, updated `operational main screen` (dashboard category filter), updated `migrations` requirement with V4 details. `docs/prd.md` updated: `products` table now references `product_categories` junction; entity overview updated; `Support category.` → `Support multiple categories per product`; `Filter by category.` → `Filter by multiple categories (any-of)`. `product_categories` table definition added to PRD schema section.

## Files Changed

### Backend (Rust)

| File | Changes |
|------|---------|
| `src-tauri/src/db/migrations.rs` | Added V4 migration `add_product_categories_v4` (creates `product_categories`, idempotent back-fill, subquery case-fold dedup); `MIGRATIONS` made `pub(crate)` for WU-11 test access; 7 new V4 tests |
| `src-tauri/src/db/repositories/products.rs` | `ProductCreate/Update` → `category_ids: Vec<String>`; junction helpers `list_product_category_ids_by_product_ids` (N-query loop), `get_product_category_ids` (single-product); updated `insert/update/get/search/find_by_barcode/find_by_sku/list_all_products_for_export`; `RawProductRow._category_id` (legacy); fixed ambiguous `id` in barcode JOIN; fixed `into_search_result` to accept `primary_barcode` |
| `src-tauri/src/db/repositories/dashboard.rs` | Added `category_ids: Option<Vec<String>>` to `DashboardFilters`; SQL-level EXISTS filter via dynamic query with per-element binding; updated test `DashboardFilters` struct literals |
| `src-tauri/src/db/repositories/stores.rs` | `list_all_locations` kept but unused (dead_code) |
| `src-tauri/src/dto/products.rs` | `ProductCreate/Update.category_ids`, `ProductResponse.category_ids`, `ProductDetailResponse.categories`, `ProductSearchResult.category_ids`, `CategorySearchPage`, `CategorySearchInput`, `UNCATEGORIZED_SENTINEL` |
| `src-tauri/src/dto/dashboard.rs` | `DashboardFilters.category_ids` |
| `src-tauri/src/dto/reports.rs` | `ReportFilters.category_ids` (replaces `category_id`) |
| `src-tauri/src/services/products.rs` | `get_product` resolves categories from junction; `update_product` service-level returns `Result<ProductResponse>` (not `Result<Option>`); fixed test `.unwrap().unwrap()` → `.unwrap()` |
| `src-tauri/src/services/reports.rs` | Removed `filter_by_category`; `to_dashboard_filters` passes `category_ids`; fixture updated for multi-category; fixed AlertWindow fixture bug (product_b alert_days=30→10) |
| `src-tauri/src/services/dashboard.rs` | `matches_preset`: AlertWindow excludes today (`days_remaining > 0`), Next30Days excludes alert-window lots (`days_remaining > alert_days_before`) — makes them mutually exclusive |
| `src-tauri/src/services/csv_io.rs` | `export_products_csv` uses `category_ids` (comma-joined names); `import_product_csv` passes `category_ids: vec![resolved_id]`; `resolve_category_name` uses `list_all_categories` |
| `src-tauri/src/services/categories.rs` | New: paginated `search(pool, input)` command |
| `src-tauri/src/services/stores.rs` | `list_all_stores`, `get_store`, `list_all_locations` (unused, `#[allow(dead_code)]`) |
| `src-tauri/src/domain/alerts.rs` | `alert_start_date`, `is_in_alert_window`, `is_expired`, `days_until_expiry`, `today_utc` (unused, `#[allow(dead_code)]`) |
| `src-tauri/src/db/migrations.rs` | `#[allow(dead_code)]` on `applied_count` (used in tests only) |
| `src-tauri/src/services/backup_restore.rs` | Added WU-11 smoke test: `restore_from_pre_v4_backup_applies_v4_backfill_in_situ` (verifies V4 applies on reopened pre-V4 pool); `v3_only_migrator` helper inlined for cross-module test access |

### Frontend (TypeScript + Svelte)

| File | Changes |
|------|---------|
| `src/lib/categories.ts` | **NEW** — `UNCATEGORIZED_SENTINEL`, `CategorySearchInput`, `CategorySearchPage`, `listCategoriesSearch()` wrapper |
| `src/lib/products.ts` | `ProductCreate/Update.category_ids`, `ProductResponse.category_ids`, `ProductDetailResponse.categories` (array), `ProductSearchResult.category_ids`; removed legacy `category_id` fields |
| `src/lib/dashboard.ts` | `DashboardFilters.category_ids?: string[]` |
| `src/lib/reports.ts` | `ReportFilters.category_ids?: string[] | null` (replaces `category_id`) |
| `src/components/inputs/CategoryPicker.svelte` | **NEW** — multi-select combobox with chip row, popover, inline create, WAI-ARIA combobox/listbox/option markup, keyboard nav (↑↓ Enter Esc Backspace), `includeUncategorized` sentinel, `aria-controls` + `tabindex` on all options |
| `src/components/ProductForm.svelte` | Replaced single `categoryId: string` state with `categoryIds: string[]`; swapped `<select>` + `<ul class="category-pills">` + inline-create block for `<CategoryPicker>`; removed unused CSS (`.btn-small`, `.category-row`, `.category-pills`, `.pill`, `.pill:hover`, `.pill.selected`, `.inline-category`, `label select`) |
| `src/components/ProductDetailPage.svelte` | Replaced `detail.category?.name` with `detail.categories` array rendering + empty `Uncategorized` badge |
| `src/components/DashboardPage.svelte` | Added `categoryIds` filter state; updated `loadDashboard()` filters; added `<CategoryPicker>` to filter row; updated detail modal to use `categories.map(name).join(", ")` |
| `src/components/ProductCatalogPage.svelte` | Added `categoryIds` filter state; added `<CategoryPicker>` to search bar; `$: displayedProducts` reactive block with client-side category intersect + `UNCATEGORIZED_SENTINEL` support |
| `src/components/ReportsPage.svelte` | Replaced `categoryId: string` state with `categoryIds: string[]`; swapped `<select>` for `<CategoryPicker>`; updated `buildFilters()` and `filterSummary()`; added `UNCATEGORIZED_SENTINEL` import |

### Documentation

| File | Changes |
|------|---------|
| `docs/prd.md` | Updated `products` table to reference `product_categories` junction; updated entity overview; `Support category.` → `Support multiple categories per product`; `Filter by category.` → `Filter by multiple categories (any-of)`; added `product_categories` table definition; `docs/prd.md` MD056 warnings fixed |

### Cascade Fixes

The `category_id → category_ids` rename caused cascade compile errors in:

- `pdf/report_pdf.rs`
- `services/notifications.rs`
- `services/unit_definitions.rs`
- `services/expiry_lots.rs`
- `services/csv_io.rs`
- `services/reports.rs`
- `services/dashboard.rs`
- `db/repositories/dashboard.rs`

All fixed via find-and-replace: `category_id: Option<String>` → `category_ids: Option<Vec<String>>` and `category_id: String` → `category_ids: Vec<String>`.

## Backend Test Evidence

```
$ cargo test --manifest-path src-tauri/Cargo.toml --lib
   Compiling caduxo v0.1.0 (/home/xeworg/Proyectos/Caduxo/src-tauri)
    Finished test [unoptimized + debuginfo] target(s) in 0.18s
     Running unittests src/lib.rs (src-tauri/target/debug/deps/caduxo_lib-984dd4af315a7cdb)

test result: ok. 301 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## New Tests Added

| Test | Location | Description |
|------|----------|-------------|
| `v4_applies_on_fresh_db` | `db::migrations::tests` | V4 applies cleanly (4 total migrations) |
| `v4_migration_is_idempotent` | `db::migrations::tests` | Re-running V4 is a no-op |
| `v4_backfill_copies_legacy_category_id_into_junction` | `db::migrations::tests` | Junction populated from `products.category_id` |
| `v4_backfill_is_idempotent` | `db::migrations::tests` | Re-running back-fill is safe |
| `v4_backfill_skips_null_category_id` | `db::migrations::tests` | Products with null category_id are skipped |
| `v4_case_fold_dedup_keeps_oldest` | `db::migrations::tests` | Oldest category (by created_at ASC, id ASC) kept |
| `v4_case_fold_dedup_remaps_legacy_column` | `db::migrations::tests` | Junction and legacy column both remapped |
| `insert_product_writes_junction_rows` | `db::repositories::products::tests` | Junction rows written on insert |
| `insert_product_with_empty_category_ids_writes_no_junction` | `db::repositories::products::tests` | No junction rows for empty category_ids |
| `insert_product_with_three_category_ids_writes_three_junction_rows` | `db::repositories::products::tests` | Three categories = three junction rows |
| `update_product_replaces_full_junction_set` | `db::repositories::products::tests` | Full set replacement on update |
| `update_product_from_two_to_zero_categories_removes_junction` | `db::repositories::products::tests` | Junction rows deleted when categories cleared |
| `get_product_returns_category_ids_from_junction` | `db::repositories::products::tests` | Junction resolved on get |
| `find_by_barcode_exact_returns_category_ids_from_junction` | `db::repositories::products::tests` | Junction resolved on barcode lookup |
| `delete_category_with_junction_is_rejected` | `db::repositories::products::tests` | RESTRICT FK prevents orphan categories |
| `restore_from_pre_v4_backup_applies_v4_backfill_in_situ` | `services::backup_restore::tests` | V4 applies on reopen; junction populated; legacy column remapped; picker-facing batch helper returns correct ids |

## Pre-existing Failures (Fixed by This Slice)

| Test | Root Cause | Fix |
|------|-----------|-----|
| `preview_report_in_alert_window_returns_alert_lots` | `matches_preset` AlertWindow included `days_remaining=0` (today lot) | Added `days_remaining > 0` guard |
| `preview_report_next_30_days_returns_30d_lots` | Next30Days not mutually exclusive with AlertWindow; fixture had product_b `alert_days=30` instead of `10` | Fixed Next30Days guard to `days_remaining > alert_days_before`; fixed fixture alert_days from 30→10 |

## Remaining Tasks

All implementation-owned tasks complete:

- [x] WU-11: Backend restore-from-backup smoke test ✅
- [x] WU-12: Frontend TS DTOs + `src/lib/categories.ts` ✅
- [x] WU-13: `CategoryPicker.svelte` primitive ✅
- [x] WU-14: `ProductForm.svelte` host adoption ✅
- [x] WU-15: `ProductDetailPage.svelte` + `DashboardPage.svelte` host adoption ✅
- [x] WU-16: `ProductCatalogPage.svelte` host adoption ✅
- [x] WU-17: `ReportsPage.svelte` host adoption ✅
- [x] WU-18: Verify gates (`cargo test`: 301 passed 0 failed; `svelte-check`: 0 errors 2 warnings; `npm run build`: ✓) ✅
- [x] WU-19: Spec delta + `docs/prd.md` update ✅
- [x] WU-20: Apply-progress ledger finalization ✅

## Parent actions (post-apply)

- [ ] Bounded review of the slice per the project's review-workload discipline; confirm the apply-progress ledger captures all verify evidence and that the 2-pre-existing-failure baseline is preserved. <!-- sdd-owner: parent -->
- [ ] Decision: confirm `Decision needed before apply: No` is honored (single PR, no chained split, no `size:exception`); only escalate if the final diff exceeds the 3,000-line user-typed override. <!-- sdd-owner: parent -->

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| `category_ids` filter in `list_dashboard_lots` uses dynamic SQL (string concatenation) | Low | All values are bound as parameters; no SQL injection surface |
| N-query `list_product_category_ids_by_product_ids` could be N+1 for large product sets | Low | Called only from batch read paths (search/export); per-product helper is used for single-product reads |
| `UNCATEGORIZED_SENTINEL` sentinel value could collide with real UUIDs | Low | Prefix `__` makes collision extremely unlikely; service validates existence of non-sentinel IDs |

## Notes

- The V4 migration uses subquery-based deduplication (no CTEs with window functions) to avoid SQLite CTE compatibility issues with nested subqueries.
- `matches_preset` was updated to make AlertWindow and Next30Days mutually exclusive, fixing the pre-existing test failures.
- The `products.category_id` legacy column is preserved in the schema but never read or written by runtime code (per design §2.2 invariant).

## Post-apply regression fixes

After the initial apply ledger, manual smoke found CategoryPicker regressions in product editing and inline category creation. The fixes landed in two commits:

- `83807f8` — `Fix category picker selection regressions`
  - Fixed CategoryPicker first-click focus/click race.
  - Removed implicit `<label>` wrappers around composite CategoryPicker hosts in `ProductForm.svelte` and `ReportsPage.svelte`, preventing browser label activation from dispatching synthetic clicks to inner controls and removing chips.
  - Guarded `ProductForm.svelte` edit-mode reseeding so same-product parent refreshes do not clobber in-flight `categoryIds` edits.
  - Activated and registered `list_categories_search`, fixed the category search service SQL for sqlx 0.8, and corrected Dashboard/Reports `UNCATEGORIZED_SENTINEL` filtering.
- `876084f` — `Show create option for partial category matches`
  - Inline create is hidden only for exact case-insensitive category-name matches, not partial matches. Example: typing `Bate` while `Bateria` exists now offers both `Bateria` and `Create "Bate"`.

Focused verification after the regression fixes:

```text
cargo check --all-targets: passed, only pre-existing warnings
cargo test --lib services::categories: 6 passed
cargo test --lib db::repositories::dashboard: 9 passed
cargo test --lib services::reports: 26 passed
cargo test --lib: 312 passed, 0 failed
cargo build --release --lib: passed, only pre-existing warnings
npx svelte-check --tsconfig ./tsconfig.json: 0 errors, 2 pre-existing warnings
```

Manual smoke evidence: user tested the current category picker behavior after the fixes and confirmed it works, including selection retention and creation behavior.
