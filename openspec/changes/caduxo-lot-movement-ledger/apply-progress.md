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

*Generated: Phase 1b completion*

---

### Phase 1b — Backend Domain + Repository + Service

**Status:** COMPLETE

#### Completed Tasks

- [x] RED — Add `domain/lot_movements.rs` unit tests (39 tests)
- [x] GREEN — Implement pure helpers in `domain/lot_movements.rs`
- [x] RED — Add `repositories/lot_movements.rs` integration tests
- [x] GREEN — Implement repository with schema mapping, per-location balance SQL, batch code helpers
- [x] RED — Add `services/lot_movements.rs` integration tests (13 tests)
- [x] GREEN — Implement service with `create_lot_movement`, `list_lot_movements`, `get_lot_location_balances`, `derive_auto_batch_code`
- [x] REFACTOR — Validation collapsed into helper functions

#### Files Changed

- `src-tauri/src/domain/lot_movements.rs` — Pure domain functions (MovementKind enum, delta computation, batch code derivation)
- `src-tauri/src/domain/mod.rs` — Added lot_movements module
- `src-tauri/src/dto/lot_movements.rs` — DTOs (LotMovementCreate, LotMovementResponse, LotLocationBalance, Direction)
- `src-tauri/src/dto/mod.rs` — Added lot_movements module
- `src-tauri/src/db/repositories/lot_movements.rs` — Repository functions (insert_movement, list_movements_by_lot, location_balances_for_lot, find_max_nnn, has_initial_entry)
- `src-tauri/src/db/repositories/mod.rs` — Added lot_movements module
- `src-tauri/src/services/lot_movements.rs` — Service (create_lot_movement, list_lot_movements, get_lot_location_balances, derive_auto_batch_code, ensure_sentinel_location, is_initial_location_required)
- `src-tauri/src/services/mod.rs` — Added lot_movements module
- `src-tauri/src/db/migrations.rs` — Fixed CHECK constraint (quantity >= 0)

#### Test Results

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib domain::lot_movements
test result: ok. 39 passed; 0 failed
```

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib services::lot_movements
test result: ok. 22 passed; 0 failed
```

#### Full Test Suite

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib
test result: ok. 376 passed; 0 failed
```

#### Key Design Decisions

1. **entry:initial does not update lot quantity**: The lot already has the correct quantity from INSERT. The entry:initial movement records history but doesn't update the denormalized lot total.
2. **inventory_adjustment direction required**: The direction must be specified; the service validates this.
3. **Reactivation clears resolution**: When a resolved lot is reactivated via inventory_adjustment increase, the resolution and resolved_at fields are cleared.
4. **Location active validation**: Both source and destination locations must be active for transfers and exits.
5. **Notes required for specific kinds**: exit:other, exit:inventory_adjustment, and inventory_adjustment all require notes.

## Next Steps

**Phase 2:** Integrate lot_movements with expiry_lots service (lot creation emits initial entry), add settings DTO, update LotForm.

---

### Phase 2 — Lot Creation Integration + Settings DTO + LotForm

**Status:** COMPLETE

#### Completed Tasks

- [x] Extend `dto/stores.rs` `SettingsResponse` and `SettingsUpdate` with `require_initial_location_on_lot_create: bool`
- [x] Extend `db/repositories/settings.rs` with `get_require_initial_location_on_lot_create` and `set_require_initial_location_on_lot_create` helpers; updated `get_settings` to include new field
- [x] Extend `services/settings.rs` `update_settings` to handle new field
- [x] Extend `src/lib/stores.ts` `SettingsResponse` and `SettingsUpdate` interfaces with new field
- [x] Modify `services/expiry_lots.rs::create_expiry_lot` to: (a) check `require_initial_location_on_lot_create` setting, (b) resolve location (sentinel if setting off + no location, reject if setting on + no location), (c) auto-generate or preserve `batch_code`, (d) emit `entry:initial` movement atomically in same transaction as lot insert
- [x] Add `ensure_sentinel_for_store` helper in expiry_lots service
- [x] Add `resolve_batch_code` helper in expiry_lots service
- [x] Modify `LotForm.svelte` to fetch `require_initial_location_on_lot_create` on mount; validate location required when true; show (required)/(optional) label
- [x] Update all existing `expiry_lots` service tests to handle new `seed_product` signature (3-tuple with location); disable setting by default in tests

#### Files Changed

- `src-tauri/src/dto/stores.rs` — Added `require_initial_location_on_lot_create` to `SettingsResponse` and `SettingsUpdate`
- `src-tauri/src/db/repositories/settings.rs` — Added `get_require_initial_location_on_lot_create`, `set_require_initial_location_on_lot_create`; updated `get_settings`
- `src-tauri/src/services/settings.rs` — Updated `update_settings` to handle new field
- `src-tauri/src/services/expiry_lots.rs` — Full `create_expiry_lot` rewrite with transaction, location resolution, batch code, and entry:initial emission
- `src/lib/stores.ts` — Extended `SettingsResponse` and `SettingsUpdate` interfaces
- `src/components/LotForm.svelte` — Fetch settings on mount; validate location required when setting is on

#### Test Results

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib -- services::expiry_lots
test result: ok. 28 passed; 0 failed
```

Key new tests:

- `create_lot_emits_entry_initial_movement` — verifies entry:initial is written atomically
- `create_lot_requires_location_when_setting_is_on` — setting=true + no location = error
- `create_lot_uses_explicit_location_when_provided` — setting=true + explicit location = success
- `create_lot_auto_generates_batch_code` — blank batch_code generates PREFIX-YYYYMMDD-NNN
- `create_lot_preserves_explicit_batch_code` — manual batch_code preserved verbatim
- `settings_require_initial_location_defaults_to_true` — fresh DB defaults to true
- `settings_update_toggles_require_initial_location` — toggle round-trips correctly

#### Full Test Suite

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib
test result: ok. 383 passed; 0 failed
```

```text
npx svelte-check --output machine-readable
svelte-check found 0 errors and 2 warnings in 2 files
```

#### Key Design Decisions

1. **Default: require=true**: `require_initial_location_on_lot_create` defaults to `true` when absent (per spec scenario requirement)
2. **Sentinel naming**: `loc-sentinel-{store_id}` ensures one sentinel per store
3. **Batch collision retry**: up to 100 attempts before falling back to UUID suffix
4. **Entry:initial delta**: `direction=NULL`, `quantity=lot.quantity`, `destination_location_id=location_id`
5. **Transaction boundary**: lot INSERT + entry:initial INSERT are in the same `pool.begin()` / `tx.commit()` transaction
6. **Test fixture**: `seed_product` disables the requirement and returns a location for test isolation

## Next Steps

**Phase 3:** Movements UI — `LotMovementsPanel`, `MoveStockModal`, `RegisterExitModal`, `AdjustCountModal`, Historial tab on lot detail.

---

### Phase 3 — Movements UI (Historial tab + three modals)

**Status:** COMPLETE

#### Completed Tasks

- [x] Add `src/lib/lot_movements.ts` exporting `createLotMovement`, `listLotMovements`, `getLotLocationBalances`, typed against the new DTOs
- [x] Create `src-tauri/src/commands/lot_movements.rs` exposing `create_lot_movement`, `list_lot_movements`, `get_lot_location_balances` and register them in `src-tauri/src/lib.rs::invoke_handler`
- [x] Add `src-tauri/src/dto/lot_movements.rs` DTOs (already existed from Phase 1b: `MovementKind`, `LotMovementCreate`, `LotMovementResponse`, `LotLocationBalanceResponse`, `Direction`)
- [x] Add `src/components/LotMovementsPanel.svelte`: header (lot total + per-location breakdown), three buttons (*Mover stock*, *Registrar salida*, *Ajustar conteo*), and the chronological list with newest-first rows
- [x] Add `src/components/MoveStockModal.svelte`: source select (defaults to highest-balance location), destination select (excludes source, includes other stores per Conflict 1 resolution), quantity input
- [x] Add `src/components/RegisterExitModal.svelte`: source select, motivo select from the eight exit kinds, quantity input
- [x] Add `src/components/AdjustCountModal.svelte`: location select, real physical quantity input, notes textarea (required). Computes delta and emits one movement with `direction='increase'` or `'decrease'`
- [x] Hook the panel into the existing lot detail modal opened from `DashboardPage.svelte` and `CalendarPage.svelte`: add a third *Historial* tab alongside the existing detail surface

#### Files Changed

**Backend:**

- `src-tauri/src/commands/lot_movements.rs` (new) — Tauri command handlers
- `src-tauri/src/commands/mod.rs` — Added lot_movements module
- `src-tauri/src/lib.rs` — Registered new commands in invoke_handler

**Frontend:**

- `src/lib/lot_movements.ts` (new) — TypeScript API wrapper with DTOs and helper functions
- `src/components/LotMovementsPanel.svelte` (new) — Main panel with header, action buttons, and movement list
- `src/components/MoveStockModal.svelte` (new) — Transfer modal
- `src/components/RegisterExitModal.svelte` (new) — Exit registration modal with eight reasons
- `src/components/AdjustCountModal.svelte` (new) — Inventory adjustment modal with delta preview
- `src/components/DashboardPage.svelte` — Added Historial tab to lot detail modal
- `src/components/CalendarPage.svelte` — Added Historial tab to lot detail modal

#### Verification Results

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib
test result: ok. 383 passed; 0 failed
```

```text
npx svelte-check --output machine-readable
svelte-check found 0 errors and 6 warnings in 4 files
```

```text
npm run build
✓ built in 1.11s
```

#### Key Implementation Details

1. **Movement kind labels**: Spanish labels for all movement kinds (`Entrada inicial`, `Transferencia`, `Venta`, etc.)
2. **Quantity display**: `inventory_adjustment` rows prefix magnitude with `+` or `−`
3. **Balance tracking**: Per-location balances shown in header; defaults source to highest-balance location
4. **Tab integration**: Lot detail modal now has "Detalle" and "Historial" tabs
5. **Modal styling**: Wide modal (720px) for movements panel, consistent with existing modal patterns
6. **Exit reasons**: Eight reasons from spec, with notes required for `Ajuste de inventario` and `Otro`
7. **Adjustment delta preview**: Shows preview of adjustment (positive/negative) before submission

## Next Steps

**Phase 4:** Settings menu (`Configuración`) — wire the toggle into a page-level toggle, create ConfigurationPage.svelte.

---

*Generated: Phase 4 completion*

---

### Phase 4 — Settings Menu (`Configuración`)

**Status:** COMPLETE

#### Completed Tasks

- [x] Wire `SettingsResponse.require_initial_location_on_lot_create` from Phase 2 into a page-level toggle (already present in backend DTO from Phase 2)
- [x] Create `src/components/ConfigurationPage.svelte`: one "Lotes" section with `Ubicación inicial obligatoria al crear lote` toggle; reads settings on mount; auto-saves on toggle with optimistic update and rollback on failure
- [x] Register `Configuración` nav entry in `src/App.svelte` alongside Dashboard / Stores / Products / Calendar / Reports / Import / Backup

#### Files Changed

- `src/components/ConfigurationPage.svelte` (new) — Settings page with CSS-only toggle switch, optimistic update, rollback on error
- `src/App.svelte` — Added `ConfigurationPage` import, `settings` tab variant, nav button, and `{:else if}` branch

#### Verification Results

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib
test result: ok. 383 passed; 0 failed
```

```text
npx svelte-check --output machine-readable
svelte-check found 0 errors and 6 warnings in 4 files
```

(6 warnings are pre-existing from Phase 3; none introduced by Phase 4)

```text
npm run build
✓ built in 1.09s
```

#### Key Implementation Details

1. **CSS-only toggle**: no Tailwind; matches the existing app's visual style
2. **Optimistic update**: toggle flips immediately; reverts on API error
3. **Spanish labels**: matches the spec scenario wording (`Ubicación inicial obligatoria al crear lote`)
4. **No backend changes needed**: DTO and service layer were fully implemented in Phase 2
5. **Auto-save**: `updateSettings` is called on every toggle change; no separate save button

#### Pending Tasks

Phase 5 (optional polish, deferred) — no work planned unless user requests it.

#### Verify Gate — Manual Smoke (deferred to parent)

- Toggle OFF → create a lot with the picker empty → lot lands against `Sin ubicación` sentinel and initial entry's destination is the sentinel
- Toggle ON → empty picker on LotForm rejects with `Selecciona una ubicación` (already covered by Phase 2 tests)
