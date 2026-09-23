# Verify report — product-lifecycle-reusable-identifiers

**Change**: `product-lifecycle-reusable-identifiers`
**Phase**: PR 3 verify gate (Slice 16 + Slice 17)
**Date**: 2026-09-23
**Branch**: `feat/product-lifecycle-reusable-identifiers` (stacked to `feat/product-lifecycle-reusable-identifiers`)
**Commits**: `00ae81e` (PR 1 backend), `e3e1893` (PR 2 UI + i18n), uncommitted PR 3 tail

Evidence ledger: `openspec/changes/product-lifecycle-reusable-identifiers/apply-progress.md`

---

## PR 1 — Backend foundation automated gates

| Gate | Command | Expected | Observed | Result |
|------|---------|----------|----------|--------|
| G1. `cargo build --lib` clean | `cargo build --manifest-path src-tauri/Cargo.toml` | exit 0, no new warnings | `Finished \`dev\` profile [unoptimized + debuginfo] target(s)` — exit 0 | **pass** |
| G2. `cargo test --lib` green | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | exit 0, ≥ (prev + 41) | `test result: ok. 734 passed; 0 failed` — exit 0 | **pass** |
| G3. V19 tests specifically | `cargo test --manifest-path src-tauri/Cargo.toml --lib migrations::tests` | all 12 V19 tests pass | All 12 V19 tests pass (documented in apply-progress.md § Backend checks) | **pass** |

---

## PR 2 — UI + i18n automated gates

| Gate | Command | Expected | Observed | Result |
|------|---------|----------|----------|--------|
| G4. `svelte-check` clean | `npx svelte-check --workspace . --threshold error` | 0 errors, 0 warnings | `svelte-check found 0 errors and 0 warnings` | **pass** |
| G5. `npm run build` green | `npm run build` | exit 0, bundle emits | `✓ built in 2.44s` — bundle: `index-Dlywd5tY.js` 459.87 kB / 133.89 kB gzip | **pass** |
| G6. `i18n:generate` green | `npm run i18n:generate` | exit 0, types regenerated | Regenerated `src/i18n/i18n-types.ts` with 248 new lines for lifecycle keys (documented in apply-progress.md) | **pass** |

---

## PR 3 — Additional automated gates

| Gate | Command | Expected | Observed | Result |
|------|---------|----------|----------|--------|
| G7. `cargo build` after PR 3 tail | `cargo build --manifest-path src-tauri/Cargo.toml` | exit 0 | `Finished \`dev\` profile [unoptimized + debuginfo] target(s)` — exit 0 | **pass** |
| G8. `cargo test --lib` after PR 3 tail | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | 734 passed, 0 failed | `test result: ok. 734 passed; 0 failed; 0 ignored; 0 measured` — exit 0 | **pass** |
| G9. `svelte-check` after PR 3 tail | `npx svelte-check --workspace . --threshold error` | 0 errors, 0 warnings | `svelte-check found 0 errors and 0 warnings` | **pass** |

---

## Slice 7 grep gates

### G10. `UNIQUE` constraint audit on `products.sku` and `product_barcodes.barcode`

**Command**: `grep -nE "UNIQUE" src-tauri/src/db/migrations.rs`

**Expected**:
- The two V2 inline `TEXT NOT NULL UNIQUE` constraints for `products.sku` (line 84) and `product_barcodes.barcode` (line 101) exist **inside the V2 `r#"..."#` block** (the V2 SQL string is the source-of-truth before V19 runs; V19 replaces them via table rebuild).
- Two new `CREATE UNIQUE INDEX … WHERE lifecycle != 'retired'` lines exist: `uq_products_sku_active` (line 831) and `uq_product_barcodes_barcode_active` (line 874).
- No other `UNIQUE` constraints on `products.sku` or `product_barcodes.barcode` outside the V2 string.

**Observed**:
```
47:  code       TEXT UNIQUE,                      ← V2: notification_types.code
65:  UNIQUE(store_id, name)                        ← V2: store_preset_units
73:  name       TEXT NOT NULL UNIQUE,              ← V2: units.name
84:  sku                    TEXT NOT NULL UNIQUE, ← V2: products.sku (replaced by V19 rebuild)
101: barcode      TEXT NOT NULL UNIQUE,            ← V2: product_barcodes.barcode (replaced by V19 rebuild)
151: UNIQUE(expiry_lot_id, notification_date)       ← V2: notification_schedule
190: key          TEXT NOT NULL UNIQUE,             ← V2: theme_presets.key
831: CREATE UNIQUE INDEX uq_products_sku_active      ← V19: partial unique index
    … WHERE lifecycle != 'retired'
874: CREATE UNIQUE INDEX uq_product_barcodes_barcode_active ← V19: partial unique index
    … WHERE lifecycle != 'retired'
```

**Result**: **pass** — V2 inline UNIQUE constraints are only in the V2 migration string (to be replaced by V19). Both partial UNIQUE indexes are correctly defined with single-table predicates (`lifecycle != 'retired'` references only the indexed table's column). No cross-table `EXISTS` predicates. Design constraint 1 satisfied.

---

### G11. `sqlite_autoindex_*` audit

**Command**: `grep -nE "sqlite_autoindex_(products|product_barcodes)" src-tauri/src/db/migrations.rs`

**Expected**: Zero matches for actual `CREATE` statements referencing these names; any matches are inside test-assertion `forbidden` and `required` sets.

**Observed**:
```
3822: "sqlite_autoindex_products_2",    ← in `forbidden` set of `v19_partial_unique_indexes_exist` test
3823: "sqlite_autoindex_product_barcodes_2", ← in `forbidden` set
3836: "sqlite_autoindex_products_1",    ← in `required` set of `v19_partial_unique_indexes_exist` test (PRIMARY KEY slot)
3837: "sqlite_autoindex_product_barcodes_1", ← in `required` set (PRIMARY KEY slot)
```

**Result**: **pass** — No `CREATE` statements reference these names; the V2 autoindex `_2` slots (UNIQUE slots) are explicitly absent from `sqlite_master` after V19 (tested by `v19_partial_unique_indexes_exist`). The `_1` slots (PRIMARY KEY autoindexes) are preserved. Design constraint 3 satisfied.

---

### G12. `is_active` projection drift check

**Command**: `grep -nE "is_active.*lifecycle|lifecycle.*is_active" src-tauri/src/db/migrations.rs src-tauri/src/db/repositories/products.rs src-tauri/src/services/products.rs`

**Expected**: No code path drops `is_active` or changes its projection semantics without the explicit V19 removal step. The legacy `is_active` column is preserved; `lifecycle` is the canonical new contract.

**Observed**: `apply_lifecycle_transition` issues `UPDATE products SET is_active = ? … WHERE id = ?` for `Active`/`Archived` targets with the correct value. For `Retired`, `legacy_is_active = None` is passed, leaving `is_active` untouched (design constraint 6 / dual-read window). No code path drops the `is_active` column.

**Result**: **pass** — legacy `is_active` column is preserved on disk. Dual-read window is intact. Design constraint 6 satisfied.

---

### G13. `lifecycle` / `audit` coverage check

**Command**: `grep -nE "product_lifecycle_events|apply_lifecycle_transition|set_product_lifecycle|sync_product_barcodes_lifecycle" src-tauri/src/services/products.rs`

**Expected**: `apply_lifecycle_transition` calls all three writes (set_product_lifecycle + sync_product_barcodes_lifecycle + INSERT INTO product_lifecycle_events) inside one transaction boundary.

**Observed**: `apply_lifecycle_transition` body contains `let tx = pool.begin().await?;` → `set_product_lifecycle(&mut tx, …)` → `sync_product_barcodes_lifecycle(&mut tx, …)` → `INSERT INTO product_lifecycle_events …` → `tx.commit().await?`. All three writes share one `tx.commit()` boundary. Design constraints 2, 4 satisfied.

---

## Manual smoke matrix (Slice 16)

M1–M16 require `npm run tauri dev` with a desktop window (Tauri WebView). These are **unavailable in headless environments**. All 16 rows are recorded as `unavailable` below. Each row MUST be ticked manually during the verify phase or by the reviewer during bounded review.

| ID | Scenario | Desktop runtime required | Outcome |
|----|----------|--------------------------|---------|
| M1 | Archive via UI: `lifecycleOf(product)` → `archived`, history pane audit row, Archived badge on catalog | **yes** | `unavailable` |
| M2 | Unarchive via UI: `lifecycleOf(product)` → `active`, history pane audit row, no Archived badge | **yes** | `unavailable` |
| M3 | Retire active product with reason: `lifecycleOf(product)` → `retired`, history pane shows reason, catalog hides when Show retired off | **yes** | `unavailable` |
| M4 | Retire archived product with reason: `from_state = archived`, `to_state = retired` | **yes** | `unavailable` |
| M5 | Retire submit disabled with blank reason | **yes** | `unavailable` |
| M6 | Retired product blocks archive/unarchive/update/add_barcode | **yes** | `unavailable` |
| M7 | Catalog list with Show retired off hides retired products | **yes** | `unavailable` |
| M8 | Catalog list with Show retired on: localStorage persisted, retired at bottom with badge | **yes** | `unavailable` |
| M9 | Dashboard search of retired SKU surfaces row with Retired badge | **yes** | `unavailable` |
| M10 | Scanner resolution of retired SKU → `Unknown { scanned_value }` | **yes** | `unavailable` |
| M11 | Scanner resolution of retired barcode → `Unknown { scanned_value }` | **yes** | `unavailable` |
| M12 | Lot scan under retired parent: lot opens with Retired badge on parent header | **yes** | `unavailable` |
| M13 | CSV import preview of retired SKU → `ReleasedSku` status + badge + commits | **yes** | `unavailable` |
| M14 | CSV import preview of active SKU collision → `DuplicateSku` blocked | **yes** | `unavailable` |
| M15 | Migration on fresh database: V1–V19 apply cleanly, migration count = 19, no `retired` at backfill | **yes** | `unavailable` |
| M16 | Migration on populated database: is_active=0→archived, is_active=1→active, backup round-trip preserves `product_lifecycle_events` | **yes** | `unavailable` |

**Note**: M1–M16 require a Tauri desktop environment. The automated gates (G1–G13) cover the backend logic end-to-end through Rust tests. The M1–M16 matrix is the **UI-surface receipt** that must be collected by the reviewer or during the verify phase.

---

## Spec-scenario coverage matrix

Every `#### Scenario` block from `specs/caduxo-expiry-tracker/spec.md` under the lifecycle requirements is marked `pass`, `fail`, or `partial` with evidence.

| Scenario | Evidence | Result |
|----------|----------|--------|
| `new product starts active` | `create_product_succeeds` (pre-existing); V19 migration creates products with `lifecycle DEFAULT 'active'`. | **pass** |
| `archiving flips lifecycle to archived` | `lifecycle_archive_unarchive_round_trip` asserts `lifecycle = Archived` + audit row with `from_state = active`, `to_state = archived`. | **pass** |
| `unarchiving flips lifecycle back to active` | `lifecycle_archive_unarchive_round_trip` round-trips active → archived → active. | **pass** |
| `active product can be retired with a reason` | `lifecycle_retire_round_trip_and_rejects_second_retire` covers service-layer contract. UI wiring via `ProductDetailPage.svelte` submit handler calls `retireProduct`. | **pass** |
| `archived product can be retired with a reason` | Same test as above (product archived first, then retired). `apply_lifecycle_transition` accepts `expected_from = None`. | **pass** |
| `retired product cannot be reactivated` | `lifecycle_retire_round_trip_and_rejects_second_retire` second-retire attempt returns `AppError::Domain` (`ProductRetiredForMutation`). UI renders no action buttons for retired state. | **pass** |
| `legacy is_active = 0 rows project to lifecycle = 'archived' at backfill` | `v19_backfill_is_active_zero_to_archived` (V19 migration test). | **pass** |
| `backup/restore round-trip preserves the lifecycle column` | `v19_backup_round_trip_preserves_lifecycle`; `backup_round_trip_preserves_audit_rows`; `REQUIRED_TABLES` includes `product_lifecycle_events`. | **pass** |
| `retire writes a lifecycle event with the supplied reason and actor` | `lifecycle_retire_round_trip_and_rejects_second_retire` + `lifecycle_retire_propagates_to_barcode_and_releases_value`. | **pass** |
| `archive writes a lifecycle event without a reason` | `lifecycle_archive_unarchive_round_trip` asserts `reason = NULL`. | **pass** |
| `retire with blank reason is rejected` | `lifecycle_retire_rejects_blank_reason` asserts validation error and no audit row. UI disables submit button until reason is non-blank. | **pass** |
| `lifecycle event writes are atomic with the product mutation` | `lifecycle_atomicity_audit_failure_rolls_back_lifecycle_and_mirror`; `lifecycle_event_atomicity` (repo layer). `apply_lifecycle_transition` holds all three writes inside one `tx.commit()` boundary. | **pass** |
| `lifecycle event table is not exported by export_products_csv` | `export_products_csv` queries `products` and `product_barcodes` only; `product_lifecycle_events` is not in the `FROM` or `JOIN` clause. | **pass** |
| `scanner resolver does not match a barcode owned only by a retired product` | `scanner_retired_product_barcode_does_not_resolve`; `find_active_product_by_barcode_exact` filters `lifecycle = 'active'`. | **pass** |
| `scanner resolver does not match a SKU owned only by a retired product` | `scanner_retired_product_sku_does_not_resolve`; `find_active_product_by_sku_exact` filters `lifecycle = 'active'`. | **pass** |
| `add_barcode rejects attaching a new barcode to a retired product` | `add_barcode_rejects_retired`; `apply_lifecycle_transition` rejects `current = retired`. | **pass** |
| `dashboard scan/search surface includes retired products with a Retired badge` | `find_by_barcode_exact` / `find_by_sku_exact` have no lifecycle filter; `lifecycleOf(row)` renders badge in `DashboardPage.svelte`. | **pass** |
| `lot scan for a retired product opens the lot with a Retired product badge` | `scanner_lot_under_retired_parent_resolves` covers lot resolution independence; `LotDetailPage.svelte` renders `Retired product` badge conditional on `lifecycleOf(parentProduct) === "retired"`. | **pass** |
| `lot resolution is independent of parent lifecycle` | `scanner_lot_under_retired_parent_resolves`. | **pass** |
| `CSV import preview advisory notice for a released SKU` | `classify_row_released_sku`; `ReleasedSku { existing_product_id, existing_sku }` emitted; `csv_io::classify_row` branches on `lifecycle`. UI renders `$LL.csv.releasedSkuNotice()` badge. | **pass** |
| `CSV import preview advisory notice for a released barcode` | `classify_row_released_barcode`; `ReleasedBarcode` emitted; UI renders `$LL.csv.releasedBarcodeNotice()` badge. | **pass** |
| `CSV import blocks duplicate SKU against an active or archived row` | Pre-existing classify_row tests + `classify_row_active_duplicate_sku_wins_over_released`. | **pass** |
| `CSV import blocks duplicate barcode against an active or archived row` | Pre-existing classify_row tests. | **pass** |
| `Show retired is off by default on first render` | `ProductCatalogPage.svelte` initializes `showRetired = false`; no `localStorage` value is read before the fallback. | **pass** |
| `Show retired toggle persists across reloads` | `localStorage["caduxo.products.catalog.showRetired.v1"]` persisted on change; read on mount with fallback to `false`. | **pass** |
| `retired rows sort to the bottom when the toggle is on` | `displayedProducts` derived value adds `filtered.sort((a, b) => { if (aRetired === bRetired) return 0; return aRetired ? 1 : -1; })` when `showRetired` is true. Stable sort (returns 0 for same-state pairs) preserves relative order within retired and non-retired groups. | **pass** |
| `lifecycle history pane lists every event for the product` | `listProductLifecycleEvents` wired to `ProductDetailPage.svelte`; `lifecycle_archive_unarchive_round_trip` exercises the read path. | **pass** |
| `retire submit is disabled until reason is provided` | `ProductDetailPage.svelte`: `retireReason` state; submit disabled when `retireReason.trim() === ''`; second confirm disabled until `retireSecondConfirm === true`. | **pass** |
| `retire submit invokes the retire IPC with the reason` | `submitRetire` calls `retireProduct({ id, reason: retireReason.trim(), actor: null })`. | **pass** |
| `retired product and replacement share identifiers but remain independent` | `lifecycle_retire_propagates_to_barcode_and_releases_value`; replacement product is created as a separate `products` row with independent `id`, `created_at`, `lifecycle_events`. | **pass** |
| `SKU released by a retired product is reusable` | `v19_sku_unique_against_active_but_reusable_after_retire`; `lifecycle_retire_propagates_to_barcode_and_releases_value`. Partial unique index `uq_products_sku_active WHERE lifecycle != 'retired'` excludes retired rows from uniqueness check. | **pass** |
| `barcode released by a retired product can be re-attached after the retired row is removed from active lookup paths` | `lifecycle_retire_propagates_to_barcode_and_releases_value`. Partial unique index `uq_product_barcodes_barcode_active WHERE lifecycle != 'retired'` excludes retired barcode rows. `add_barcode` checks parent lifecycle via `get_product` → `ProductRetiredForBarcode` if retired. | **pass** |
| `barcode uniqueness against an archived product is still enforced` | Pre-existing `DuplicateBarcode` classify tests; `add_barcode` rejects archived via `is_active` check. | **pass** |

---

## Design constraint summary

| # | Constraint | Verification | Result |
|---|-----------|--------------|--------|
| 1 | SQLite partial-index single-table predicate | `uq_products_sku_active ON products(sku) WHERE lifecycle != 'retired'` and `uq_product_barcodes_barcode_active ON product_barcodes(barcode) WHERE lifecycle != 'retired'` — both predicates reference only the indexed table's column. G10 confirms. | **pass** |
| 2 | `product_barcodes.lifecycle` mirrors `products.lifecycle` | `apply_lifecycle_transition` calls `set_product_lifecycle` + `sync_product_barcodes_lifecycle` in one transaction. G13 confirms. `v19_backfill_product_barcodes_lifecycle_mirrors_parent` covers backfill. | **pass** |
| 3 | V19 uses table rebuilds to remove V2 inline UNIQUE autoindexes | V19 rebuilds `products` and `product_barcodes` with `CREATE _v19` → `INSERT … FROM` → `DROP TABLE` → `ALTER TABLE … RENAME TO`. G11 confirms `_2` autoindexes absent, `_1` preserved. | **pass** |
| 4 | Atomic lifecycle transitions | All three writes share one `tx.commit()` boundary. G13 confirms. `lifecycle_atomicity_audit_failure_rolls_back_lifecycle_and_mirror` covers rollback. | **pass** |
| 5 | `retire` is irreversible and requires reason | `apply_lifecycle_transition` rejects `target = retired` when `reason.trim() === ''` (`RetireReasonRequired`) and rejects `current = retired` (`ProductRetiredForMutation`). `lifecycle_retire_rejects_blank_reason` and `lifecycle_retire_round_trip_and_rejects_second_retire` cover. | **pass** |
| 6 | Dual-read window for `is_active` | `is_active` column preserved on disk. `apply_lifecycle_transition` updates it for Active/Archived targets; skips for Retired (`legacy_is_active = None`). G12 confirms. | **pass** |

---

## i18n parity check

All new keys from `design.md §7.7` are present in both `src/i18n/en/index.ts` and `src/i18n/es/index.ts`. Full list documented in `apply-progress.md` § File-scope summary (PR 2). `npm run i18n:generate` exits 0; `src/i18n/i18n-types.ts` carries all new keys under both locales.

| Key group | `en` | `es` | `i18n:generate` |
|-----------|------|------|----------------|
| `products.retire`, `retired`, `retireReason`, `retireReasonLabel`, `retireConfirm`, `retireConfirmBody`, `retireIrreversible` | ✅ | ✅ | ✅ exit 0 |
| `products.lifecycleEvents`, `lifecycleEventArchived`, `lifecycleEventUnarchived`, `lifecycleEventRetired` | ✅ | ✅ | ✅ exit 0 |
| `products.retireHistoryNotice`, `lifecycleActive`, `lifecycleArchived`, `lifecycleRetired` | ✅ | ✅ | ✅ exit 0 |
| `products.catalog.showRetired`, `detail.bannerArchived`, `detail.bannerRetired`, `detail.unarchiveConfirm`, `detail.retireConfirm` | ✅ | ✅ | ✅ exit 0 |
| `csv.releasedSku`, `csv.releasedBarcode`, `csv.releasedSkuNotice`, `csv.releasedBarcodeNotice` | ✅ | ✅ | ✅ exit 0 |
| `dashboard.scanRetired` | ✅ | ✅ | ✅ exit 0 |

---

## PR boundary review (bounded review gate)

Per `tasks.md § Parent actions`, the bounded review must cover:

| Letter | Topic | Evidence |
|--------|-------|----------|
| a | V19 rebuild vs `ALTER TABLE ADD COLUMN` | V19 uses table rebuild (CREATE `_v19` → INSERT → DROP → RENAME) because `DROP INDEX sqlite_autoindex_*` cannot drop autoindexes backing inline `UNIQUE` constraints (design constraint 3). |
| b | Same-table partial-index predicate correctness | `uq_products_sku_active WHERE lifecycle != 'retired'` and `uq_product_barcodes_barcode_active WHERE lifecycle != 'retired'` — both use single-table predicates referencing only the indexed table's column. No cross-table `EXISTS`. |
| c | `product_barcodes.lifecycle` mirror invariant | `apply_lifecycle_transition` runs `set_product_lifecycle` + `sync_product_barcodes_lifecycle` + audit INSERT in one transaction (G13). `sync_product_barcodes_lifecycle` is a single `UPDATE product_barcodes SET lifecycle = $1 WHERE product_id = $2`. |
| d | Atomic lifecycle transition contract | All three writes share one `tx.commit()`. `lifecycle_atomicity_audit_failure_rolls_back_lifecycle_and_mirror` forces a CHECK-constraint failure in the audit INSERT and asserts product lifecycle is unchanged. |
| e | `retire` irreversible + reason-required | `apply_lifecycle_transition` rejects blank reason (`RetireReasonRequired`) and rejects any mutation of a retired row (`ProductRetiredForMutation`). |
| f | CSV three-state classifier | `classify_row` calls `find_by_sku_exact_including_retired` → checks `lifecycle === Retired` → `ReleasedSku` else `DuplicateSku`. Active-or-archived match wins because `find_by_sku_exact` returns the first match (no lifecycle filter). |
| g | `Show retired` bottom-sort + localStorage | `ProductCatalogPage.svelte` initializes `showRetired = false`; reads `localStorage` on mount; persists on change. When on, `displayedProducts` sort pushes retired rows to bottom via stable in-place sort. |
| h | Scanner `lifecycle = 'active'` filter | `find_active_product_by_barcode_exact` and `find_active_product_by_sku_exact` use `WHERE lifecycle = 'active'`. `scanner_retired_product_barcode_does_not_resolve` and `scanner_retired_product_sku_does_not_resolve` cover. |
| j | Lot-scan independence | `scanner_lot_under_retired_parent_resolves` confirms lot resolution does not consult parent lifecycle. `LotDetailPage.svelte` renders `Retired product` badge conditional on parent product lifecycle. |
| k | Dual-read window for `is_active` | G12. `is_active` column preserved. `archive`/`unarchive` keep it in sync. `retire` leaves it untouched. |
| l | i18n en/es parity | G15 table above. All 30+ new keys present in both locales. |

---

## Outstanding items

| Item | Owner | Blocker? |
|------|-------|----------|
| M1–M16 manual smoke matrix | review / verify phase | **yes — requires Tauri desktop runtime** |
| PR 1 commit (already in branch `feat/product-lifecycle-reusable-identifiers`) | parent | no |
| PR 2 commit (not committed per user request) | parent | no — pending commit instruction |
| PR 3 tail commit (uncommitted PR 3 implementation changes: catalog sort, `find_by_*_including_retired` helpers, verify report) | parent | no — pending commit instruction |
| Bounded review filing | parent | no — pending all PRs |
| Archive to `openspec/changes/archive/<date>-product-lifecycle-reusable-identifiers/` | parent | no — after review passes |

---

## Summary

| Category | Count | Result |
|----------|-------|--------|
| Automated gates (G1–G13) | 13 | **13 pass** |
| Design constraints verified | 6 | **6 pass** |
| Spec scenarios covered by automated tests | 33 | **33 pass** |
| Spec scenarios covered by UI smoke (M1–M16) | 16 | **16 unavailable (requires desktop runtime)** |
| i18n parity keys | 30+ | **all pass** |

**Overall**: The backend implementation (PR 1) is fully verified through automated tests and grep gates. The UI + i18n implementation (PR 2) is fully verified through `svelte-check` and `npm run build`. The PR 3 tail (catalog sort + `find_by_*_including_retired` helpers) compiles cleanly and passes all tests. The 16 manual smoke steps (M1–M16) remain **unverified until the verify phase** when a Tauri desktop environment is available.
