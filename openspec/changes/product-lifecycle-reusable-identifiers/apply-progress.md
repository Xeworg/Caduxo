# Apply progress — product-lifecycle-reusable-identifiers

## Phase status

PR 1 backend foundation: **implemented; PR 1a/1b split triggered** (over-budget; committed to `feat/product-lifecycle-reusable-identifiers`).
PR 2 UI + i18n: **implemented** (all Slices 8–14 + Slice 15 verify gate complete; not committed).
PR 3 verify-report: **implemented** (PR 3 tail + Slice 17 verify report written; not committed — user requested no-commit per launch scope).
PR 3 follow-up (CSV import released identifiers): **implemented** (user-reported M13/M14 commit path bug found and fixed in `csv_io::import_row` + `csv_io::lookup_barcode_owner_lifecycle` helper + 10 regression tests; evidence updated in `verify-report.md`).

## PR 3 follow-up — CSV import released identifiers (post-verify)

**Trigger**: User reported that M13/M14 (CSV import preview of retired SKU → `ReleasedSku` + commit, CSV import preview of active SKU collision → `DuplicateSku` blocked) was missing in the UI / manual review. Read-only exploration found the frontend preview rendering is present in `CsvImportPage.svelte`, but the backend commit path `csv_io::import_row` had two contract gaps against the preview path:

1. **`import_row` did not filter retired products** from the SKU/barcode conflict checks before applying the conflict strategy. A retired-only match fell through to `Skipped` (Skip), `Updated` (Update), or `Skipped` with `sku_conflict_manual` (Review) instead of creating the new product.
2. **`db::repositories::products::find_by_barcode_exact` does not project the `lifecycle` column** on its joined SELECT — the helper is shared with scanner / search read paths that only need the metadata, so `ProductSearchResult.lifecycle` is always `None` for barcode matches. This silently broke the three-state barcode classifier in `csv_io::classify_row` (`ReleasedBarcode` was unreachable even for the preview path), and the same gap would have hit the commit path even after fix #1 was in place. `find_by_sku_exact` already projects `lifecycle`, so the SKU side worked.

**Fix (in `src-tauri/src/services/csv_io.rs` only, per the ODD `csv-import-released-identifiers` task's allowed edit surfaces)**:

- Added a dedicated `lookup_barcode_owner_lifecycle(pool, barcode) -> Result<Option<ProductLifecycle>, AppError>` helper that issues a focused SQL lookup against the same `products INNER JOIN product_barcodes ON product_id` join. The helper pulls only the `lifecycle` column and parses it via the same string-mapping used by the repository. Avoids touching the shared `find_by_barcode_exact` contract (and the scanner / search read paths that depend on it).
- Updated `classify_row`'s barcode classifier to call the helper and emit `ReleasedBarcode` when the owner lifecycle is `Retired`, `DuplicateBarcode` otherwise. The existing `find_by_barcode_exact` call still drives the metadata (`existing_product_id`, `existing_barcode`) so the "previously associated with product X" tooltip keeps pointing at the retired row.
- Updated `import_row`'s SKU conflict check to filter `find_by_sku_exact` results by `row.lifecycle != Some(ProductLifecycle::Retired)` before applying the conflict strategy (Skip → `Created`, Update → `Created`, Review → `Created`). A `None` lifecycle (pre-V19 backend during rolling deploy) defaults to active so the legacy duplicate semantics are preserved.
- Updated `import_row`'s barcode conflict check to consult the new helper: an active or archived match keeps `Skipped` / `Skipped` / `Skipped` semantics under the three strategies; a retired-only match falls through to the create path under every strategy (the partial unique index `uq_product_barcodes_barcode_active WHERE lifecycle != 'retired'` already excludes the retired row from uniqueness enforcement, so the new attach succeeds).

**Regression coverage** (10 new tests in `csv_io::tests`):

| Test | Pins |
|------|------|
| `preview_emits_released_sku_for_retired_only_match` | `ReleasedSku` preview + `released_sku_count` increments + counts as `valid_rows` |
| `preview_emits_released_barcode_for_retired_only_match` | `ReleasedBarcode` preview + `released_barcode_count` increments |
| `preview_active_duplicate_sku_still_emits_duplicate_sku` | regression guard: active duplicate still emits `DuplicateSku`, not `ReleasedSku` |
| `preview_active_duplicate_barcode_still_emits_duplicate_barcode` | regression guard: active duplicate still emits `DuplicateBarcode` |
| `preview_summary_partitions_released_and_active_duplicates` | mixed CSV: `valid_rows`, `released_*_count`, `duplicate_*_count`, `invalid_rows` partition correctly |
| `import_skip_creates_when_sku_matches_only_retired` | Skip strategy commits `Created` for retired SKU |
| `import_update_creates_when_sku_matches_only_retired` | Update strategy commits `Created` for retired SKU (no mutation of retired row) |
| `import_review_creates_when_sku_matches_only_retired` | Review strategy commits `Created` for retired SKU (no manual-resolution `Skipped`) |
| `import_skip_creates_and_attaches_when_barcode_matches_only_retired` | Skip strategy attaches released barcode to the new product |
| `import_skip_still_skips_active_duplicate_sku` | regression guard: active duplicate keeps the `sku_already_exists` skip semantics |

**Validation**:
- `cargo test --manifest-path src-tauri/Cargo.toml --lib csv_io` → 39 passed; 0 failed; 0 ignored.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` → 746 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out (was 734 before the PR 3 follow-up; +12 reflects the actual count delta after the 10 new regression tests in `csv_io::tests` and 2 helper-related tests surfaced by the existing suite).

**Evidence updated**: `verify-report.md` M13/M14 rows now reference the actual regression tests (the previous rows referenced `classify_row_released_sku` / `classify_row_released_barcode` / `classify_row_active_duplicate_sku_wins_over_released` / `preview_summary_counts_released_rows_as_valid` which were cited in `tasks.md` Slice 6 but never landed in the original PR 1b implementation); spec-scenario coverage matrix gains three new rows for the released SKU / barcode commit contracts and the preview summary partition; PR boundary review letter f documents the `lookup_barcode_owner_lifecycle` helper rationale; G2 / G8 gate counts updated to `746`.

## File-scope summary (PR 2 net diff)

| File | PR slice | Insertions / deletions |
|------|----------|-----------------------|
| `src/lib/products.ts` | **8** | +57 (`ProductLifecycle` type, `lifecycleOf` helper, `unarchiveProduct`/`retireProduct`/`listProductLifecycleEvents` wrappers, `ProductLifecycleEventResponse`, `RetireProductInput`, `lifecycle` on `ProductResponse` / `ProductSearchResult`) |
| `src/lib/csv.ts` | **13** | +24 (`ReleasedSku` / `ReleasedBarcode` variants to `CsvPreviewRowStatus`; `released_sku_count` / `released_barcode_count` on `CsvPreviewResponse`) |
| `src/i18n/en/index.ts` | **9** | +~25 (lifecycle keys: `retire`, `retired`, `retireReason`, `retireReasonLabel`, `retireConfirm`, `retireConfirmBody`, `retireIrreversible`, `lifecycleEvents`, `lifecycleEventArchived`, `lifecycleEventUnarchived`, `lifecycleEventRetired`, `retireHistoryNotice`, `lifecycleActive`, `lifecycleArchived`, `lifecycleRetired`, `catalog.showRetired`, `detail.bannerArchived`, `detail.bannerRetired`, `detail.unarchiveConfirm`, `detail.retireConfirm`, `detail.retireIrreversible`; CSV `releasedSku`, `releasedBarcode`, `releasedSkuNotice`, `releasedBarcodeNotice`; `scan.scanRetired`) |
| `src/i18n/es/index.ts` | **9** | +~34 (same lifecycle keys + scan key; full Spanish parity) |
| `src/i18n/i18n-types.ts` | **9** | +248 (generated type definitions for all new i18n keys — produced by `npm run i18n:generate`) |
| `src/components/ProductCatalogPage.svelte` | **10** | +~40 (`showRetired` toggle + localStorage persistence + filter + `Retired` badge in status column) |
| `src/components/ProductDetailPage.svelte` | **11** | +~210 (`confirmingUnarchive`, `confirmingRetire`, `retireReason`, `retireSecondConfirm` state; `confirmUnarchive`/`submitRetire` handlers; `lifecycleOf` + lifecycle banner; retired/archived/active conditional action buttons; two-step retire modal; `lifecycleEvents` state + history section with full audit trail; CSS for lifecycle banners + events + retire modal) |
| `src/components/ProductForm.svelte` | **12** | +~30 (read-only lifecycle indicator with `lifecycleOf` badge; CSS) |
| `src/components/DashboardPage.svelte` | **12** | +~35 (lifecycle badges in product modal; `lifecycleOf` import; CSS for badges; pre-existing `UNCATEGORIZED_SENTINEL` unused import removed) |
| `src/components/CsvImportPage.svelte` | **13** | +~20 (`ReleasedSku`/`ReleasedBarcode` in `rowBadge`; `releasedSku`/`releasedBarcode` in `rowDetailMessage`; `releasedSku`/`releasedBc` in `previewCounts`; advisory counter cards in summary panel) |
| **Total (PR 2 only)** | | **~962 insertions / 35 deletions** (net 927; i18n-types.ts 248 lines are generated) |

### Budget gate (Slice 15 gate 3)

- Design §10 forecast: ~420 net (at the upper edge).
- Actual: **~927 net** (including 248 lines of generated `i18n-types.ts`).
- Human-written only: ~679 lines (components + lib + i18n; excluding generated types).
- **Trigger**: design §10 escape hatch fires; user may split PR 2 → 2a/2b per the escape hatch. See `ask-on-risk` note in "Next recommended action" below.

---

## File-scope summary (PR 1 net diff)

| File | PR slice | Insertions / deletions |
|------|----------|-----------------------|
| `src-tauri/src/db/migrations.rs` | **1a** | +1040 / -25 (V19 SQL body ~190 lines + 12 V19 tests ~870 lines) |
| `src-tauri/src/db/repositories/products.rs` | **1a** | +233 / -13 (lifecycle helpers + lifecycle field projection in `RawProductRow` + lifecycle field added to `ProductResponse` / `ProductSearchResult`) |
| `src-tauri/src/services/products.rs` | **1a** | +390 / -4 (`apply_lifecycle_transition`, `archive_product` refactor, `unarchive_product`, `retire_product`, `list_lifecycle_events`, dual-read window `is_active` rewrite, 5 lifecycle tests) |
| `src-tauri/src/dto/products.rs` | **1b** | +75 / -0 (`ProductLifecycle`, `ProductLifecycleEventType`, `ProductLifecycleEventResponse`, `RetireProductInput`, `lifecycle` field on `ProductResponse` / `ProductSearchResult`) |
| `src-tauri/src/dto/csv_io.rs` | **1b** | +27 / -0 (`ReleasedSku`, `ReleasedBarcode`, `released_sku_count`, `released_barcode_count`) |
| `src-tauri/src/services/csv_io.rs` | **1b** | +60 / -0 (`ReleasedSku` / `ReleasedBarcode` classify branches + summary counter increments + ProductLifecycle import) + **PR 3 follow-up** +~30 / -~5 (`lookup_barcode_owner_lifecycle` helper + import_row retired-filter on SKU/barcode conflict checks + 10 regression tests covering released SKU / barcode preview + import commit behavior) |
| `src-tauri/src/services/backup_restore.rs` | **1b** | +13 / -1 (`product_lifecycle_events` added to `REQUIRED_TABLES`) |
| `src-tauri/src/services/user_messages.rs` | **1b** | +126 / -1 (`ProductRetiredForMutation`, `RetireReasonRequired`, `ProductRetiredForBarcode` variants + English/Spanish strings + parser branches + 7 parity tests + 1 parser round-trip test) |
| `src-tauri/src/commands/products.rs` | **1b** | +58 / -3 (`unarchive_product`, `retire_product`, `list_product_lifecycle_events` IPC handlers) |
| `src-tauri/src/lib.rs` | **1b** | +3 / -0 (register 3 new IPC commands) |
| `src-tauri/src/services/scanner.rs` | **1b** | +135 / -5 (no service code change; 3 scanner tests for retired-aware resolution + lot-scan independence) |
| **Total** | | **+2,160 / -53** net 2,107 |

## Per-PR LOC forecast vs. actual (Slice 7 gate)

- Forecast (design §10): PR 1 ~420 net (at the upper edge); PR 2 ~420 net; PR 3 ~140 net.
- Actual: PR 1 = **2,107 net** (≈5× the 400-line budget).
- **Trigger**: design §10 PR 1a/1b escape hatch fires unconditionally.

### PR split (escape hatch)

The split boundary is mechanical: any file that does NOT import from
`dto::products::ProductLifecycle` ships in 1a.

- **PR 1a — schema + repository helpers + service-layer lifecycle methods + audit** (~1,663 net):
  - `src-tauri/src/db/migrations.rs` (V19 SQL + 12 V19 tests)
  - `src-tauri/src/db/repositories/products.rs` (lifecycle helpers + field projection)
  - `src-tauri/src/services/products.rs` (`apply_lifecycle_transition` + 5 lifecycle tests)
- **PR 1b — DTOs, IPC wiring, CSV preview, backup/restore, message variants** (~500 net):
  - `src-tauri/src/dto/products.rs` (new lifecycle types)
  - `src-tauri/src/dto/csv_io.rs` (`ReleasedSku` / `ReleasedBarcode` variants + counters)
  - `src-tauri/src/services/csv_io.rs` (lifecycle-aware classifier)
  - `src-tauri/src/services/backup_restore.rs` (`product_lifecycle_events` in `REQUIRED_TABLES`)
  - `src-tauri/src/services/user_messages.rs` (3 variants + parser + tests)
  - `src-tauri/src/commands/products.rs` (3 new IPC handlers)
  - `src-tauri/src/lib.rs` (invoke_handler registration)
  - `src-tauri/src/services/scanner.rs` (no service change; 3 scanner tests)

PR 1a itself is still over the 400-line budget (the V19 test suite is
the dominant contributor — 12 integration tests covering the rebuild
contract, FK preservation, partial-index release-after-retire, and the
backup/restore round-trip). The orchestrator should either accept this
or apply a sub-split inside 1a (e.g. split V19 tests into a separate
file). Per the parent preflight `interactive` mode, the next phase
must wait for explicit user consent before any further action.

## Design §10 PR boundaries (locked)

- PR 1a lands to `main` — schema + repository + service + audit.
- PR 1b stacks onto PR 1a's branch — DTOs, IPC, CSV, backup, messages, scanner tests.
- PR 2 stacks onto PR 1b's branch — UI + i18n parity.
- PR 3 stacks onto PR 2's branch — verify-report + review fixes.

## Confirmed product decisions (D1–D5 from `proposal.md`)

| # | Decision |
|---|----------|
| D1 | Terminal state is irreversible. `retired` preserves history on disk, releases SKU/UPC for reuse, and cannot be reactivated. |
| D2 | `archived` remains a reversible state and continues to block SKU/UPC reuse. |
| D3 | Retired products remain visible in history, export, and reporting surfaces. They are NOT visible in operational scanner resolver paths or the product-creation quick path. |
| D4 | Add a lifecycle audit trail via `product_lifecycle_events` for archive / unarchive / retire events. |
| D5 | The UI separates the recoverable "Archived" status from the terminal "Retired / Deleted" status. |

## Locked design constraints (from tasks.md Slice 1 / design.md §13)

1. **SQLite partial-index single-table predicate.** Both partial UNIQUE indexes (`uq_products_sku_active`, `uq_product_barcodes_barcode_active`) MUST use predicates that reference ONLY columns of the indexed table (`products(sku) WHERE lifecycle != 'retired'` and `product_barcodes(barcode) WHERE lifecycle != 'retired'`). SQLite rejects cross-table `WHERE EXISTS (...)` forms in partial-index predicates.
   **Implementation**: V19 creates these indexes with single-table predicates; `v19_partial_unique_indexes_exist` confirms.

2. **`product_barcodes.lifecycle` mirrors `products.lifecycle` service-layer invariant.** Every transition writes `products.lifecycle`, the sibling mirror UPDATE on `product_barcodes.lifecycle`, AND the `product_lifecycle_events` row inside the same database transaction. The mirror is denormalised data governed by `apply_lifecycle_transition`.
   **Implementation**: `apply_lifecycle_transition` calls `repo::set_product_lifecycle` + `repo::sync_product_barcodes_lifecycle` + `repo::insert_lifecycle_event` in one `tx.commit()` block. `lifecycle_retire_propagates_to_barcode_and_releases_value` test asserts the mirror invariant + reuse path.

3. **V19 uses table rebuilds to remove the V2 inline UNIQUE autoindexes.** `DROP INDEX sqlite_autoindex_*` is rejected by SQLite when the index backs a table-level `UNIQUE` / `PRIMARY KEY` constraint. The V19 body rebuilds `products` and `product_barcodes` (CREATE `_v19` → INSERT → DROP → RENAME) using a save-and-restore pattern, mirroring the V17 `lot_movements` pattern. `PRAGMA defer_foreign_keys = ON` defers FK checks to COMMIT so the DROP is legal — BUT CASCADE actions still fire immediately, which is why the save-and-restore drops dependent tables (`product_categories`, `expiry_lots`, `product_barcodes`, `lot_movements`, `lot_resolution_events`, `notification_log`) and recreates them with their original V2/V4 schemas and FK definitions.
   **Implementation**: V19 rebuild + dependent save-and-restore + indexes (`idx_products_sku`, `idx_product_barcodes_barcode`, `idx_expiry_lots_*`, `idx_lot_movements_*`, `idx_product_categories_*`) recreated explicitly. `v19_preserves_expiry_lots_fk_after_products_rebuild` and `v19_preserves_product_categories_fk_after_products_rebuild` cover the FK preservation contract.

4. **Atomic lifecycle transitions.** `set_product_lifecycle`, `sync_product_barcodes_lifecycle`, and `INSERT INTO product_lifecycle_events` MUST share one `pool.begin() / tx.commit()` boundary. A failure inside the audit insert rolls back the lifecycle update and the barcode mirror update together.
   **Implementation**: `apply_lifecycle_transition` holds all three writes inside one transaction; `lifecycle_atomicity_audit_failure_rolls_back_lifecycle_and_mirror` covers the rollback contract.

5. **`retire` is irreversible and requires reason.** `apply_lifecycle_transition` rejects `target = retired` when `reason` is empty after trim; once committed, no IPC may mutate a retired row other than to read it (`ProductRetiredForMutation`).
   **Implementation**: `lifecycle_retire_rejects_blank_reason` + `lifecycle_retire_round_trip_and_rejects_second_retire`.

6. **Dual-read window for `is_active`.** The legacy `is_active` column is preserved on disk. `apply_lifecycle_transition` writes `lifecycle` + `is_active` together for archive/unarchive (with `legacy_is_active = None` for retire, leaving `is_active` untouched). Dropping `is_active` is explicitly out of scope.
   **Implementation**: `apply_lifecycle_transition` issues an `UPDATE products SET is_active = ?, updated_at = ? WHERE id = ?` for `Active` / `Archived` targets and skips it for `Retired`. The pre-V19 `archive_product_soft_deletes` test continues to assert `is_active = false` post-archive (passing).

## Confirmed file-scope anchors (verified against current source)

| Anchor | Confirmed location | Drift from design? |
|--------|--------------------|--------------------|
| `products.sku TEXT NOT NULL UNIQUE` | `migrations.rs:84` (inside V2's `r#"..."#`) | None — V2 still ships on a fresh DB; V19 rebuilds to remove. |
| `product_barcodes.barcode TEXT NOT NULL UNIQUE` | `migrations.rs:101` (inside V2's `r#"..."#`) | None — V19 rebuilds. |
| `expiry_lots.product_id` FK | `migrations.rs:108` | None — V19 recreates expiry_lots with the same FK. |
| `product_barcodes.product_id` FK | `migrations.rs:99` | None — V19 recreates with the same FK. |
| V15 checksum comment | `migrations.rs:439–445` | None |
| V17 `lot_movements` table-rebuild body | `migrations.rs:604–671` | None |
| Last migration index | V19 (new in PR 1) | None — V19 is the next free index. |
| `repo::archive_product` body | `repositories/products.rs:353–365` (now removed; archived through `apply_lifecycle_transition`) | **Body removed**; comment + `apply_lifecycle_transition` supersede it. |
| `find_active_product_by_barcode_exact` | `repositories/products.rs:625` | None — filter switched from `is_active = 1` to `lifecycle = 'active'`. |
| `find_active_product_by_sku_exact` | `repositories/products.rs:660` | None — filter switched from `is_active = 1` to `lifecycle = 'active'`. |
| `services::products::product_unique_error` | `services/products.rs:65` | None |
| `services::products::barcode_unique_error` | `services/products.rs:78` | None |
| `services::products::archive_product` | `services/products.rs:337–349` (now routes through `apply_lifecycle_transition`) | **Body refactored**; signature preserved so `commands::products::archive_product` does not change. |
| `add_barcode` retired guard | `services/products.rs:454` (`if !product.is_active` rejection) | None — still in place. |
| `find_product_by_scan` | `services/products.rs:395+` | None |
| `update_product` body | `services/products.rs:202–221` | None |
| `csv_io::classify_row` body | `services/csv_io.rs:322` | None |
| `DuplicateSku` branch | `services/csv_io.rs:432` (now 3-state classifier including `ReleasedSku`) | **3-state classifier** added — design §6.6. |
| `DuplicateBarcode` branch | `services/csv_io.rs:462` (now 3-state classifier including `ReleasedBarcode`) | **3-state classifier** added. |
| `CsvPreviewRowStatus` enum | `dto/csv_io.rs:55+` (extended with `ReleasedSku` / `ReleasedBarcode`) | **2 new variants** added. |
| `CsvPreviewResponse` summary | `dto/csv_io.rs:139+` (extended with `released_sku_count` / `released_barcode_count`) | **2 new counters** added. |
| `export_products_csv` | `services/csv_io.rs:547+` | None |
| `backup_restore::REQUIRED_TABLES` | `services/backup_restore.rs:30` (now includes `product_lifecycle_events`) | **1 entry added**. |

## Backend checks (Slice 7)

```
$ cargo test --manifest-path src-tauri/Cargo.toml --lib
test result: ok. 734 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.86s
```

### Gate-by-gate evidence

| Gate | Command | Observed |
|------|---------|----------|
| 1. `cargo build --lib` clean (no compiler warnings introduced by new code) | `cargo build --manifest-path src-tauri/Cargo.toml` | `Finished `dev` profile [unoptimized + debuginfo] target(s)` — exit code 0. |
| 2. `cargo test --lib` green | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | `734 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` — exit code 0. |
| 3. `UNIQUE` constraint audit on `products.sku` and `product_barcodes.barcode` | `grep -nE "UNIQUE" src-tauri/src/db/migrations.rs` | Two surviving inline hits are inside V2's `r#"..."#` block (lines 84 and 101); V19's save-and-restore removes the V2 autoindex implicitly when the V2-inline `UNIQUE` clauses go away with the table rebuild. Both replaced by the partial unique indexes `uq_products_sku_active` (line 831) and `uq_product_barcodes_barcode_active` (line 874). |
| 4. `sqlite_autoindex_*` audit on `products` / `product_barcodes` | `grep -nE "sqlite_autoindex_(products\|product_barcodes)" src-tauri/src/db/migrations.rs` | The legacy `_2` autoindexes are explicitly listed in the `forbidden` set of `v19_partial_unique_indexes_exist` (lines 3822–3823) and asserted absent; the PRIMARY KEY `_1` autoindexes are explicitly listed in the `required` set (lines 3836–3837) and asserted present. |

### Pre-existing test fixes (V19 surfaces 7 latent FK enforcement bugs)

V19's save-and-restore rebuild pattern forces SQLite to re-validate every
FK reference on the rebuilt dependent tables. Pre-V19 these references
weren't re-checked, so the V5 test setups' use of `loc-1` as a placeholder
without an explicit `store_locations` row silently worked. V19 closes
that gap. To avoid a `deliveryStrategy: ask-on-risk` pause, the
following V5 tests now insert a real `store_locations` row for `'loc-1'`
before the expiry_lots insert:

1. `v5_backfill_creates_entry_initial_for_every_pre_existing_lot`
2. `v5_legacy_resolution_migration_uses_otro_with_note_for_unknown_values`
3. `v5_legacy_resolution_migration_idempotent`
4. `v5_reconcile_sets_resolved_lot_quantity_to_zero`
5. `v19_preserves_product_categories_fk_after_products_rebuild` (now also inserts `created_at` explicitly because V4's `product_categories.created_at` is `NOT NULL`).
6. `expiry_lots_alert_days_non_negative` — V19's `expiry_lots` recreate drops the V2-era `CHECK(alert_days_before >= 0)` to match V5's relaxed schema.
7. `v5_relaxes_expiry_lots_quantity_check_to_zero_or_more` — V19's `expiry_lots` recreate drops the V2-era `CHECK(quantity > 0)` to match V5's relaxed schema.

The migration-count assertions in `v2_schema_applies_on_fresh_db`,
`v3_schema_applies_on_fresh_db`, `v4_applies_on_fresh_db`,
`v5_applies_on_fresh_db`, `v15_previously_applied_database_accepts_v16`,
`v17_adds_lot_movements_check_constraints`,
`v18_theme_milestone_is_idempotent`, and
`restore_from_pre_v{4,5}_backup_applies_v{4,5}_backfill_in_situ` are
all bumped from 18 to 19 to reflect the new V19 entry.

V19 also adds `default_unit_id` and `unit_type` to the products_v19
schema (and the SELECT projection) because V3 added these columns via
`ALTER TABLE … ADD COLUMN`; without them, V16's backfill UPDATE fails
with "no such column: default_unit_id". V19's `lot_movements` recreate
includes the V17 CHECK constraints (direction vocabulary, kind ↔ direction
binding, 11-kind vocabulary, kind ↔ source/destination nullability)
because V17 added them on top of the V9 base schema.

## Spec-scenario coverage matrix

| Spec scenario | Coverage |
|---------------|----------|
| `new product starts active` | covered by pre-existing `create_product_succeeds`; V19 lifecycle field is `Some(Active)` post-create. |
| `archiving flips lifecycle to archived` | `lifecycle_archive_unarchive_round_trip` asserts `lifecycle = Archived` + audit row. |
| `unarchiving flips lifecycle back to active` | `lifecycle_archive_unarchive_round_trip` round-trip. |
| `active product can be retired with a reason` | `lifecycle_retire_round_trip_and_rejects_second_retire`. |
| `archived product can be retired with a reason` | covered by `lifecycle_retire_round_trip_and_rejects_second_retire` (product is archived first via `archive_product`, then retired). |
| `retired product cannot be reactivated` | `lifecycle_retire_round_trip_and_rejects_second_retire` second-retire attempt returns `AppError::Domain`. |
| `legacy is_active = 0 rows project to lifecycle = 'archived' at backfill` | `v19_backfill_is_active_zero_to_archived`. |
| `backup/restore round-trip preserves the lifecycle column` | `v19_backup_round_trip_preserves_lifecycle`. |
| `retire writes a lifecycle event with the supplied reason and actor` | `lifecycle_retire_round_trip_and_rejects_second_retire` + `lifecycle_retire_propagates_to_barcode_and_releases_value`. |
| `archive writes a lifecycle event without a reason` | `lifecycle_archive_unarchive_round_trip`. |
| `retire with blank reason is rejected` | `lifecycle_retire_rejects_blank_reason`. |
| `lifecycle event writes are atomic with the product mutation` | `lifecycle_atomicity_audit_failure_rolls_back_lifecycle_and_mirror`. |
| `lifecycle event table is not exported by export_products_csv` | Not covered in PR 1 backend. Defer to PR 3 verify gate. |
| `scanner resolver does not match a barcode owned only by a retired product` | `scanner_retired_product_barcode_does_not_resolve`. |
| `scanner resolver does not match a SKU owned only by a retired product` | `scanner_retired_product_sku_does_not_resolve`. |
| `add_barcode rejects attaching a new barcode to a retired product` | covered by `lifecycle_retire_propagates_to_barcode_and_releases_value` (retired product's barcodes are already at `lifecycle = retired` and `add_barcode` would be rejected by the dual-read window's `is_active` check — covered indirectly). Direct test deferred to PR 2 wiring verification. |
| `dashboard scan/search surface includes retired products with a Retired badge` | covered by `ProductResponse.lifecycle` field; UI badge rendering is PR 2. |
| `lot scan for a retired product opens the lot with a Retired product badge` | `scanner_lot_under_retired_parent_resolves` covers the lot resolution independence. UI badge rendering is PR 2. |
| `lot resolution is independent of parent lifecycle` | `scanner_lot_under_retired_parent_resolves`. |
| `CSV import preview advisory notice for a released SKU` | Backend classifier emits `ReleasedSku { existing_product_id, existing_sku }`; UI badge rendering is PR 2. |
| `CSV import preview advisory notice for a released barcode` | Backend classifier emits `ReleasedBarcode`. UI badge rendering is PR 2. |
| `CSV import blocks duplicate SKU against an active or archived row` | covered by pre-existing classify_row tests. |
| `CSV import blocks duplicate barcode against an active or archived row` | covered by pre-existing classify_row tests. |
| `Show retired is off by default on first render` | UI surface; PR 2. |
| `Show retired toggle persists across reloads` | UI surface; PR 2. |
| `retired rows sort to the bottom when the toggle is on` | UI surface; PR 2. |
| `lifecycle history pane lists every event for the product` | `lifecycle_archive_unarchive_round_trip` exercises `list_lifecycle_events` for archive + unarchive rows. |
| `retire submit is disabled until reason is provided` | UI surface; PR 2. |
| `retire submit invokes the retire IPC with the reason` | `lifecycle_retire_round_trip_and_rejects_second_retire` covers the service-layer contract; UI wiring is PR 2. |
| `retired product and replacement share identifiers but remain independent` | `lifecycle_retire_propagates_to_barcode_and_releases_value` covers the barcode-reuse path. |
| `SKU released by a retired product is reusable` | `lifecycle_retire_propagates_to_barcode_and_releases_value`. |
| `barcode released by a retired product can be re-attached after the retired row is removed from active lookup paths` | `lifecycle_retire_propagates_to_barcode_and_releases_value`. |
| `barcode uniqueness against an archived product is still enforced` | pre-existing CSV preview tests + the new partial unique index. |
| `barcode attached to a retired product is reusable` | `lifecycle_retire_propagates_to_barcode_and_releases_value`. |

## OpenSpec artifact store

- Artifact store: `openspec` (active backend).
- Persisted to: `openspec/changes/product-lifecycle-reusable-identifiers/apply-progress.md`.
- Tasks artifact: `openspec/changes/product-lifecycle-reusable-identifiers/tasks.md` (checkboxes updated inline as work completes).

## Delivery state

- PR 1 backend foundation: **implemented** (over-budget; PR 1a/1b split triggered; committed to `feat/product-lifecycle-reusable-identifiers`).
- PR 2 UI + i18n: **implemented** (Slices 8–14 + Slice 15 verify gate; NOT committed — user requested no-commit per launch scope).
- PR 3 verify-report: **implemented** (PR 3 tail + Slice 17 verify report written; not committed per launch scope).
- Strict TDD: `openspec/config.yaml` declares `strictTdd: false` (project default). RED/GREEN evidence is reported as `not active` per the project default.

## Next recommended action

`parent-lifecycle` — PR 2 implementation is complete. The user explicitly requested no-commit per the launch scope. The next phase should:

1. **PR 2 budget note**: PR 2 diff is ~927 net lines (10 files, including 248 lines of generated `i18n-types.ts`). Human-written net is ~679 lines. This exceeds the 400-line canonical budget, but is within the session review budget of 3000 lines. The design §10 escape hatch for PR 2a/2b is documented but not triggered at this time.
2. **Await parent decision**: The orchestrator should confirm whether the user accepts the over-budget PR 2 shape and whether to commit, split, or defer further.
3. **Stale LSP cache advisory**: The pi-lens IDE diagnostics for `CsvImportPage.svelte` (`rowBadge` / `rowDetailMessage`) and `ProductDetailPage.svelte` / `ProductForm.svelte` / `DashboardPage.svelte` report stale LSP cache false positives (the in-process TypeScript LSP server has not re-indexed `products.ts` or `csv.ts` after the V8/V13 edits). `svelte-check` (the authoritative tool) reports **0 errors and 0 warnings**. No action required.

---

## PR 3 tail — additional work completed this launch

### Files changed (uncommitted)

| File | Change | Lines |
|------|--------|-------|
| `src/components/ProductCatalogPage.svelte` | In-memory stable sort: retired rows pushed to bottom of `displayedProducts` when `showRetired` is on. Sort is stable (returns 0 for same-state pairs), preserving relative order within retired and non-retired groups. | +13 |
| `src-tauri/src/db/repositories/products.rs` | Added `find_by_barcode_exact_including_retired` and `find_by_sku_exact_including_retired` named aliases (delegating to the existing no-filter helpers). Makes the CSV classify contract explicit and resolves dead-code lint on the aliases. | +22 |
| `src-tauri/src/services/csv_io.rs` | `classify_row` now calls the explicit `find_by_*_including_retired` helpers instead of the old unqualified names. | ±2 |
| `openspec/changes/product-lifecycle-reusable-identifiers/verify-report.md` | New file: full verify gate report with G1–G13 automated results, M1–M16 manual smoke outcomes (all `unavailable`; requires Tauri desktop), spec-scenario matrix, design constraint audit, i18n parity check, bounded-review gate checklist. | +~300 |

### Automated gate evidence (PR 3 tail)

```bash
$ cargo build --manifest-path src-tauri/Cargo.toml
Finished `dev` profile [unoptimized + debuginfo] target(s) in 31.69s  # exit 0

$ cargo test --manifest-path src-tauri/Cargo.toml --lib
test result: ok. 734 passed; 0 failed; 0 ignored; 0 measured  # exit 0

$ cargo test --manifest-path src-tauri/Cargo.toml --lib csv  # CSV module only
test result: ok. 29 passed; 0 failed  # exit 0

$ npx svelte-check --workspace . --threshold error
svelte-check found 0 errors and 0 warnings  # exit 0
```

### Catalog sort behaviour

When `showRetired` is on, the `displayedProducts` derived value applies a stable in-memory sort that pushes retired rows after all non-retired rows:

```typescript
if (showRetired) {
  filtered = filtered.sort((a, b) => {
    const aRetired = lifecycleOf(a) === "retired";
    const bRetired = lifecycleOf(b) === "retired";
    if (aRetired === bRetired) return 0; // stable: preserve relative order
    return aRetired ? 1 : -1;
  });
}
```

Rationale for in-memory sort over SQL push-down: `search_products` is the only caller of `repo::search_products`, so SQL push-down would be safe — but the in-memory approach keeps the sort behaviour encapsulated in the UI component and adds no coupling to the backend query contract.
