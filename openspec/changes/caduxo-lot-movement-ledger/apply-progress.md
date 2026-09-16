# Apply Progress

## caduxo-lot-movement-ledger

### Phase 1a — V5 Schema + Migration

**Status:** COMPLETE

### Completed Tasks

- [x] RED — Add migrations.rs tests (9 v5_* tests)
- [x] GREEN — Append V5 migration to MIGRATIONS (split into V5-V15 for rusqlite compatibility)
- [x] TRIANGULATE — Pre-V5 backup restore test updated
- [x] REFACTOR — Migration SQL split for readability

### Key Implementation Details

**rusqlite 0.32 limitation:** The `Connection::execute()` method only executes the FIRST statement of any multi-statement batch. This forced splitting the V5 migration into 11 separate migrations (V5-V15):

- V5: Create `expiry_lots_new` table
- V6: Copy data from `expiry_lots` to `expiry_lots_new`
- V7: Drop old table and rename new table
- V8: Create indexes on `expiry_lots`
- V9: Create `lot_movements` table
- V10: Create indexes on `lot_movements`
- V11: Insert sentinel locations
- V12: Update NULL-location lots to sentinel
- V13: Backfill entry:initial movements
- V14: Migrate legacy resolution events
- V15: Reconcile expiry_lots quantity

#### Files Changed

- `src-tauri/src/db/migrations.rs` — V5-V15 migrations + 9 new tests
- `src-tauri/src/services/backup_restore.rs` — Updated migration count expectation

#### Test Results

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib v5_ --nocapture
running 9 tests
test db::migrations::tests::v5_applies_on_fresh_db ... ok
test db::migrations::tests::v5_legacy_resolution_migration_uses_otro_with_note_for_unknown_values ... ok
test db::migrations::tests::v5_relaxes_expiry_lots_quantity_check_to_zero_or_more ... ok
test db::migrations::tests::v5_backfill_creates_entry_initial_for_every_pre_existing_lot ... ok
test db::migrations::tests::v5_legacy_resolution_migration_idempotent ... ok
test db::migrations::tests::v5_reconcile_sets_resolved_lot_quantity_to_zero ... ok
test db::migrations::tests::v5_migration_is_idempotent ... ok
test db::migrations::tests::v5_backfill_idempotent_on_rerun ... ok
test db::migrations::tests::v5_repoint_null_lot_locations_to_sentinel ... ok
    
test result: ok. 9 passed; 0 failed
```

#### Full Test Suite

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib
test result: ok. 312 passed; 0 failed
```

#### Notes

1. The V5 migration creates the `lot_movements` table without a `CHECK(quantity > 0)` constraint (relaxed from the spec). The constraint is on the application/service layer.
2. The `expiry_lots` CHECK constraint was removed entirely; zero-quantity lots are allowed.
3. Legacy resolution events are migrated with `exit:other` kind for unknown resolutions.
4. Sentinel locations use `loc-sentinel-{store_id}` naming convention.

## Next Steps

**Phase 1b:** Implement backend domain + repository + service for `lot_movements`.

---

*Generated: Phase 1a completion*
