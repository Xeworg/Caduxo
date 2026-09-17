# Tasks — caduxo-lot-movement-ledger

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~990 net additions + ~60 deletions (project total) |
| 400-line budget risk | Medium (single-PR aggregate exceeds the 800-line session budget; per-phase slices land below) |
| Chained PRs recommended | Yes (Path A from design §10.2: split Phase 1 into 1a schema + 1b logic) |
| Suggested split | Phase 1a (schema/migration) → Phase 1b (domain/repo/service) → Phase 2 (lot creation integration) → Phase 3 (movements UI) → Phase 4 (settings menu) → Phase 5 (optional polish, deferred) |
| Delivery strategy | ask-on-risk (three decisions deferred to parent before apply) |
| Chain strategy | pending (parent to ratify Path A vs Path B; either stacked-to-main or feature-branch-chain works against the project's single-domain git layout) |

```text
Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: Medium
```

The three ask-on-risk points the parent must decide before any code lands:

1. **Phase 1 slicing.** Path A (1a schema + 1b logic) vs Path B (single Phase 1 PR at ~430 lines). Path A is the recommended default; the design §10.2 default already leans Path A.
2. **Review budget.** Project total (~990) exceeds the 800-line session budget. Either commit to Path A plus per-PR discipline (<250 lines each), or accept `size:exception` (path B single PR at ~430 is the only candidate).
3. **Design reconciliation edits.** The reconciliation listed below is implementation work; the parent must ratify that `tasks.md` can land those edits on `design.md` as part of the change (not as a separate proposal) before Phase 1a begins.

---

## Design Reconciliation: Locked Spec vs Current design.md

Two material conflicts between `openspec/changes/caduxo-lot-movement-ledger/specs/caduxo-expiry-tracker/spec.md` and `design.md` were identified while preparing this task list. The spec is the locked authority (per proposal §Open questions #2 and #4). `design.md` must be reconciled before Phase 1a begins. Both reconciliation edits sit on the allowed edit surface (`design.md`).

### Conflict 1 — Cross-store transfers

- **design.md §0 row 10** ("Same-store transfers only in v1" with validation that rejects `source.store_id != destination.store_id`).
- **design.md §1.3 invariant 4** (`source.store_id == destination.store_id == expiry_lots.store_id`).
- **Spec** (requirement `transfers within and across stores`): *"The system MUST allow the destination store to differ from the source store. The lot's `expiry_lots.store_id` anchor remains the original store."*

**Resolution:** Spec wins. Remove the same-store rejection in §0 row 10 and §1.3 invariant 4. Replace with "service rejects a transfer whose source or destination location is inactive (`is_active = 0`); the destination location MAY belong to a different store than the source; `expiry_lots.store_id` is not updated by a transfer."

### Conflict 2 — Single `inventory_adjustment` kind with directional sign

The spec's vocabulary table is **eleven** kinds but the eleventh is `inventory_adjustment` (a single kind for both increase and decrease via a `direction` column), not the design's `entry:inventory_adjustment` + `exit:inventory_adjustment` pair.

Concrete mismatches in design.md:

- **§1.1 schema check constraint** (the LIKE-pattern for `entry:%` / `exit:%`) does not match the bare `inventory_adjustment` kind. The CHECK must be rewritten to handle the new kind via a `direction` column.
- **§1.1 column list** lacks a `direction` column. Add `direction TEXT NULL` with CHECK `(movement_kind = 'inventory_adjustment' AND direction IN ('increase','decrease')) OR (movement_kind <> 'inventory_adjustment' AND direction IS NULL)`.
- **§1.2 vocabulary table** lists `entry:inventory_adjustment` + `exit:inventory_adjustment`. Replace the pair with a single `inventory_adjustment` row whose source/destination columns read "follows direction."
- **§1.3 invariant 7** (notes required) must add `inventory_adjustment` to the required-notes list.
- **§3.3 pseudocode `match movement.kind`** treats `entry:*` and `exit:*` as the only delta signifiers. Add an explicit arm for `inventory_adjustment` whose delta is `+q` when `direction='increase'` and `-q` when `direction='decrease'`.
- **§3.5** "reactivation via `entry:inventory_adjustment`" must read `inventory_adjustment` with `direction='increase'`.
- **§6.4 client routing** must emit a single `inventory_adjustment` row with the `direction` value rather than splitting between two kinds.
- **§9.1–§9.2 test names** referencing `count_adjustment_increase_uses_entry_inventory_adjustment` and `_decrease_uses_exit_inventory_adjustment` must collapse to `count_adjustment_*_writes_inventory_adjustment_with_direction`.
- **§11 open item** "The `entry:inventory_adjustment` vocabulary addition requires parent confirmation" is **closed** by the spec; remove it.
- **§Key Learnings** entry referencing `entry:inventory_adjustment` is now stale; rewrite.

**Resolution:** Spec wins. All ten misalignment sites above must be edited as the first implementation task (before any code lands). The schema migration block in §2 then picks up the new `direction` column.

### Reconciliation work unit (lands first)

- [x] Edit `openspec/changes/caduxo-lot-movement-ledger/design.md` to remove the same-store validation rule (§0 row 10, §1.3 invariant 4) and replace `entry:inventory_adjustment` + `exit:inventory_adjustment` with the unified `inventory_adjustment` kind + `direction` column at every site listed above (Conflict 2 bullets). No code change. <!-- sdd-owner: implementation -->

Verification: a single reviewer can `grep` design.md for `Same-store transfers only` (must return zero matches) and inspect the active contract sites (§1.1 schema CHECK, §1.2 vocabulary table rows, §1.3 invariant 7, §3.3 pseudocode arm, §3.5 reactivation, §6.4 client routing) — none of them may reference `entry:inventory_adjustment` as an active kind. Any remaining textual mentions of `entry:inventory_adjustment` are limited to explanatory/closed-item rationale (e.g., "rejected in favor of the unified kind") and are acceptable. The spec's vocabulary table is the authority and remains unchanged.

---

## Phase 1a — V5 Schema + Migration (backend only, no FE)

Strict-TDD ordering is followed where tests exist (backend `rust` tests, `strictTdd: false` per `openspec/config.yaml` so the order is recommended, not gated). UI slices later in this document skip the RED step because no FE test harness exists.

- [x] RED — Add `migrations.rs` tests (`fresh_test_pool`) asserting `v5_applies_on_fresh_db`, `v5_migration_is_idempotent`, `v5_relaxes_expiry_lots_quantity_check_to_zero_or_more`, `v5_repoint_null_lot_locations_to_sentinel`, `v5_backfill_creates_entry_initial_for_every_pre_existing_lot`, `v5_backfill_idempotent_on_rerun`, `v5_legacy_resolution_migration_uses_otro_with_note_for_unknown_values`, `v5_legacy_resolution_migration_idempotent`, `v5_reconcile_sets_resolved_lot_quantity_to_zero`. All tests must fail before the migration SQL lands. <!-- sdd-owner: implementation -->
- [x] GREEN — Append the V5 migration entry to `MIGRATIONS` in `src-tauri/src/db/migrations.rs` executing §1.1 schema (with `direction` column per design reconciliation), the §2.3 sentinel `INSERT OR IGNORE`, the §2.3 `UPDATE expiry_lots ... WHERE location_id IS NULL`, the back-fill INSERTs, the `lot_resolution_events → lot_movements` migration with the lookup table, the §2.4 reconcile UPDATE, and the `expiry_lots` CHECK relaxation via the §2.2 12-step recreate. Tests turn green. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE — Cover the pre-V5 backup restore path: add `restore_from_pre_v5_backup_applies_v5_backfill_in_situ` to `src-tauri/src/services/backup_restore.rs::tests`, mirroring the V4 precedent (`restore_from_pre_v4_backup_applies_v4_backfill_in_situ`). Asserts schema, back-fill, legacy-event migration, lot-total reconcile, default `require_initial_location_on_lot_create='1'`, and that `lot_resolution_events` remains untouched. <!-- sdd-owner: implementation -->
- [x] REFACTOR — Strip migration SQL into a multi-line `const MIGRATION_V5: &str = ...` block for readability; extract the legacy-resolution lookup table into a `fn legacy_resolution_to_kind(text: &str) -> &'static str` constant map. No behavior change. <!-- sdd-owner: implementation -->

**Acceptance / evidence**

- `cargo test --manifest-path src-tauri/Cargo.toml --lib v5_` returns 10 tests passing.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib restore_from_pre_v5_` passes.
- Re-running migration on an already-migrated test pool leaves row counts unchanged (asserted by the idempotency tests).
- `expiry_lots_check_quantity_positive` legacy test removed (or adapted) because the constraint is relaxed; the new `expiry_lots_check_quantity_zero_or_more` test asserts the new shape.

**Files / discovery targets**

- `src-tauri/src/db/migrations.rs` (append V5 + tests).
- `src-tauri/src/services/backup_restore.rs` (pre-V5 restore test only; no production change).

**Forecast:** ~220 net additions. **Rollback:** V5 cannot be down-migrated cleanly in SQLite; rollback means "ship forward, document in release notes that V5 is irreversible; if V5 lands in error, the previous release's backup remains the only path back."

---

## Phase 1b — Backend Domain + Repository + Service

- [x] RED — Add `domain/lot_movements.rs` unit tests covering `compute_movement_delta` (every kind plus `inventory_adjustment` with both directions), `validate_movement_kind`, `derive_batch_prefix` (SKUs of lengths 0/2/3/5/10, mixed case, non-alphanumeric, None), `extract_batch_nnn`, `next_batch_candidate`, `legacy_resolution_to_kind` (six known + three unknown), `notes_required` (must include `inventory_adjustment`). All tests fail before the functions exist. <!-- sdd-owner: implementation -->
- [x] GREEN — Implement the pure helpers in `src-tauri/src/domain/lot_movements.rs`. Include the unified `inventory_adjustment` arm in `compute_movement_delta` so `inventory_adjustment` + `direction='increase'` returns `+1` and `direction='decrease'` returns `-1`. Tests turn green. <!-- sdd-owner: implementation -->
- [x] RED — Add `repositories/lot_movements.rs` integration tests against `fresh_test_pool`: `insert_movement_persists_row_with_kind_and_quantities`, `list_movements_by_lot_orders_newest_first`, `location_balances_for_lot_matches_ledger_sum`, `find_max_nnn_for_prefix_on_date_returns_latest`, `has_initial_entry_true_after_first_insert_false_before`. All fail before the file exists. <!-- sdd-owner: implementation -->
- [x] GREEN — Implement the repository in `src-tauri/src/db/repositories/lot_movements.rs` with the §1.1 schema mapping, the §3.1 per-location balance SQL, the `find_max_nnn_for_prefix_on_date` SQL, and the `has_initial_entry` short-circuit. Tests turn green. <!-- sdd-owner: implementation -->
- [x] RED — Add `services/lot_movements.rs` integration tests: `create_lot_movement_emits_initial_entry_on_lot_creation`, `create_lot_movement_transfer_preserves_lot_total`, `create_lot_movement_exit_reduces_lot_total_and_resolves_on_zero`, `create_lot_movement_exit_rejected_when_source_balance_insufficient`, `create_lot_movement_exit_rejected_when_notes_blank_for_inventory_adjustment`, `create_lot_movement_transfer_accepts_cross_store_destination` (Conflict 1 acceptance test), `create_lot_movement_transfer_rejected_when_source_or_destination_inactive`, `create_lot_movement_count_adjustment_increase_writes_inventory_adjustment_with_direction_increase`, `create_lot_movement_count_adjustment_decrease_writes_inventory_adjustment_with_direction_decrease`, `create_lot_movement_count_adjustment_zero_delta_writes_no_row`, `create_lot_movement_count_adjustment_blank_note_rejected`, `list_movements_by_lot_orders_newest_first`, `location_balances_for_lot_matches_ledger_sum` (service level, also asserts reactivation path), `auto_batch_blank_input_generates_prefix_date_nnn`, `auto_batch_collision_appends_dash_two_three`, `auto_batch_manual_input_preserved_verbatim`, `lot_total_invariant_holds_after_random_sequence_of_movements` (fuzz: 50 mixed sequences). All fail before the file exists. <!-- sdd-owner: implementation -->
- [x] GREEN — Implement `services/lot_movements.rs` with `create_lot_movement`, `list_lot_movements`, `get_lot_location_balances`, `derive_auto_batch_code`, and the public `migrate_legacy_resolution_events` (called from V5 only). Use one transaction per `create_lot_movement` call. Apply the §3.3 pseudocode with the unified-`inventory_adjustment` arm. Reject transfer when source or destination `is_active = 0`. Reject cross-store === false (allow cross-store per Conflict 1 resolution). Reject blank `notes` for `inventory_adjustment` (any direction) and for `exit:other`, `exit:inventory_adjustment`. Tests turn green. <!-- sdd-owner: implementation -->
- [x] REFACTOR — Extract the kind-`delta` calculation into a single private helper that both `inventory_adjustment` and the historic kinds share; collapse the validation into a single `MovementInput::validate` that returns `Result<(), DomainError>`. No behavior change. <!-- sdd-owner: implementation -->

**Acceptance / evidence**

- `cargo test --manifest-path src-tauri/Cargo.toml --lib lot_movements` green (≥22 service + ≥9 domain + ≥6 repo tests).
- Cross-store acceptance test asserts `Bodega-A` (store A) → `Bodega-B` (store B) inserts a single `transfer` row, leaves `expiry_lots.store_id` unchanged.
- `inventory_adjustment` insertion with `direction='increase'` and `source_location_id = X` is rejected at the service layer (the DB CHECK and service validation are mutually enforcing).

**Files / discovery targets**

- `src-tauri/src/domain/lot_movements.rs` (new).
- `src-tauri/src/db/repositories/lot_movements.rs` (new).
- `src-tauri/src/services/lot_movements.rs` (new).

**Forecast:** ~180 net additions. **Rollback:** each module is reversible by deleting the file; Phase 1a schema remains intact.

---

## Phase 2 — Lot Creation Integration + Settings DTO + LotForm

This phase is the lot-creation hand-off from the new ledger to the existing lot creation flow.

- [x] Modify `src-tauri/src/services/expiry_lots.rs::create_expiry_lot` to (a) resolve the destination location: prefer the explicit input, else if `require_initial_location_on_lot_create` is off and the picker is empty, lazily create the per-store sentinel and use its id; (b) generate or preserve `batch_code` via `lot_movements::derive_auto_batch_code` (blank-only, transactional collision retry); (c) insert the `entry:initial` movement via `lot_movements::create_lot_movement` in the same transaction as `INSERT INTO expiry_lots`. Reject when the toggle is on and no location was chosen. The wire shape of `ExpiryLotCreate` does not change. <!-- sdd-owner: implementation -->
- [x] Extend `src-tauri/src/dto/stores.rs` `SettingsResponse` and `SettingsUpdate` with `require_initial_location_on_lot_create: bool` (default `true` when the key is absent). The `update_settings` command path adds an optional setter for the new key via the existing `upsert_setting`/`get_setting` helpers. No new schema. <!-- sdd-owner: implementation -->
- [x] Extend `src/lib/stores.ts` so `getSettings` returns the new field and `updateSettings` accepts the new field; the FE mirror of the backend DTO. <!-- sdd-owner: implementation -->
- [x] Modify `src/components/LotForm.svelte` so that on mount it reads `require_initial_location_on_lot_create`; when `true` and the picker is empty, `submit()` rejects with `errorMsg = "Selecciona una ubicación"` (matches the spec scenario wording). When `false`, an empty picker is acceptable; the batch echo chip "Lote generado: `<code>`" appears next to the batch input after a successful create when the manual input was blank. <!-- sdd-owner: implementation -->

**Acceptance / evidence**

- Rust service tests for `create_expiry_lot`: new tests `create_lot_emits_initial_entry_with_blank_batch_generates_code`, `create_lot_with_blank_location_and_toggle_off_uses_sentinel`, `create_lot_with_blank_location_and_toggle_on_rejects`, `create_lot_with_manual_batch_preserves_verbatim`.
- `svelte-check --workspace . --threshold error` clean.
- Manual smoke (covered in Phase 4 verify gate) confirms the FE guard and the batch echo chip.

**Files / discovery targets**

- `src-tauri/src/services/expiry_lots.rs`, `src-tauri/src/services/stores.rs`.
- `src-tauri/src/dto/stores.rs`, `src-tauri/src/commands/stores.rs` (settings DTO wire + command).
- `src-tauri/src/commands/expiry_lots.rs` (if needed for response passthrough).
- `src/lib/stores.ts`, `src/components/LotForm.svelte`.

**Forecast:** ~190 net additions. **Rollback:** revert the four files; the ledger still works for any lot created before the integration land; lots created with the new logic keep their `entry:initial`.

---

## Phase 3 — Movements UI (Historial tab + three modals)

- [x] Add `src/lib/lot_movements.ts` exporting `createLotMovement`, `listLotMovements`, `getLotLocationBalances`, typed against the new DTOs. Calls the new Tauri commands. <!-- sdd-owner: implementation -->
- [x] Create `src-tauri/src/commands/lot_movements.rs` exposing `create_lot_movement`, `list_lot_movements`, `get_lot_location_balances` and register them in `src-tauri/src/lib.rs::invoke_handler`. <!-- sdd-owner: implementation -->
- [x] Add `src-tauri/src/dto/lot_movements.rs` DTOs: `MovementKind`, `LotMovementCreate` (includes `direction: Option<Direction>`), `LotMovementResponse`, `LotLocationBalanceResponse`. The frontend lib wrapper maps `kind` ↔ `direction`. <!-- sdd-owner: implementation -->
- [x] Add `src/components/LotMovementsPanel.svelte`: header (lot total + per-location breakdown), three buttons (*Mover stock*, *Registrar salida*, *Ajustar conteo*), and the chronological list with newest-first rows. Each row renders the Spanish kind label, reason label, source/destination arrows, magnitude, and notes when present; `inventory_adjustment` rows prefix the magnitude with `+` or `−`. Newest first. <!-- sdd-owner: implementation -->
- [x] Add `src/components/MoveStockModal.svelte`: source select (defaults to highest-balance location), destination select (excludes source, includes other stores per Conflict 1 resolution), quantity input, submit calls `createLotMovement({ kind: 'transfer', ... })`. <!-- sdd-owner: implementation -->
- [x] Add `src/components/RegisterExitModal.svelte`: source select, motivo select from the eight exit kinds (renders `Ajuste de inventario` and `Otro` with the required-notes textarea), quantity input, submit calls `createLotMovement({ kind: 'exit:<motivo>', ... })`. <!-- sdd-owner: implementation -->
- [x] Add `src/components/AdjustCountModal.svelte`: location select, real physical quantity input, notes textarea (required). On submit, computes the delta; emits one `createLotMovement` call with `kind: 'inventory_adjustment'`, `direction: 'increase' | 'decrease'` (or `null` direction → no-op when delta is `0`). The FE computes delta and submits the already-signed sign once, matching the unified kind. No split between two kinds on the wire. <!-- sdd-owner: implementation -->
- [x] Hook the panel into the existing lot detail modal opened from `ProductDetailPage.svelte` and `DashboardPage.svelte`: add a third *Historial* tab alongside the existing detail/edit surfaces. NO per-lot movement widget on the dashboard itself (per spec). <!-- sdd-owner: implementation -->

**Acceptance / evidence**

- `npx svelte-check --workspace . --threshold error` clean.
- `npm run build` succeeds.
- Manual smoke (verify gate) covers: lot creation → Historial shows `entry:initial`; Mover stock 5 from Bodega to Exhibición updates balances; Registrar salida with each of the eight reasons (only Venta, Merma, Vencido, Dañado, Consumo interno, Devolución a proveedor success without notes; Ajuste de inventario and Otro rejected without notes); Ajustar conteo with `+2` writes a single `inventory_adjustment` row with `direction='increase'`; Ajustar conteo with `-2` writes a single row with `direction='decrease'`; Ajustar conteo with zero delta writes nothing; reactivating a resolved lot via `direction='increase'` flips `status` back to `active`.

**Files / discovery targets**

- `src-tauri/src/commands/lot_movements.rs`, `src-tauri/src/dto/lot_movements.rs`, `src-tauri/src/lib.rs` (handler registration).
- `src/lib/lot_movements.ts`.
- `src/components/LotMovementsPanel.svelte`, `src/components/MoveStockModal.svelte`, `src/components/RegisterExitModal.svelte`, `src/components/AdjustCountModal.svelte`.
- `src/components/ProductDetailPage.svelte`, `src/components/DashboardPage.svelte` (tab integration only; no widget added).

**Forecast:** ~280 net additions. **Rollback:** revert the new components; the lot detail modal renders without the Historial tab.

---

## Phase 4 — Settings Menu (`Configuración`)

- [x] Add `src-tauri/src/dto/stores.rs::SettingsResponse` already carries `require_initial_location_on_lot_create` from Phase 2; Phase 4 wires it into a page-level toggle. <!-- sdd-owner: implementation -->
- [x] Create `src/components/ConfigurationPage.svelte`: one `Lotes` section with a single `Ubicación inicial obligatoria al crear lote` toggle. Bind to `settings.require_initial_location_on_lot_create`. Save on toggle (auto-save, rollback on failure). Reads settings on mount. <!-- sdd-owner: implementation -->
- [x] Register `Configuración` in `src/components/App.svelte` navigation alongside Dashboard / Calendar / Products / Reports / CSV / Stores / Backup. <!-- sdd-owner: implementation -->

**Acceptance / evidence**

- `npx svelte-check --workspace . --threshold error` clean.
- `npm run build` succeeds.
- Manual smoke: toggle OFF → create a lot with the picker empty → lot lands against `Sin ubicación` and the initial entry's destination is the sentinel; toggle ON → empty picker rejects with `Selecciona una ubicación`.

**Files / discovery targets**

- `src/components/ConfigurationPage.svelte` (new).
- `src/components/App.svelte` (one entry).
- `src/lib/stores.ts` already wired in Phase 2.

**Forecast:** ~90 net additions. **Rollback:** revert the page and the navigation entry.

---

## Phase 5 — Optional Polish (Deferred / Out of v1)

- [x] No-op. Recent-activity dashboard widget, bulk movement entry, and the print-friendly ledger are deferred per the proposal's "Open questions" round (decisions 5 and 6). Reopen only on user request. <!-- sdd-owner: implementation -->

---

## Cross-cutting Constraints (apply to every phase)

- No `lot_movements` row carries any monetary or financial field. No price, payment method, customer, tax, invoice, margin, or cost column exists or is added during any phase. The `Registrar salida` form surfaces stock-out reasons only.
- The `actor` column on every `lot_movements` row is the literal string `system` until auth exists. No code path in any phase writes another value.
- The CSV export shape is unchanged in v1; `batch_code` reflects whatever the lot stores (manual or auto).
- Reports do not query `lot_movements` in v1; `Movimientos por motivo` is deferred.

---

## Verify Gate (every phase must clear before the next begins)

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
npx svelte-check --workspace . --threshold error
npm run build
```

Plus manual smoke on **Linux + Windows** covering:

1. Lot creation (manual + auto batch, with and without location).
2. Mover stock (same-store and cross-store).
3. Registrar salida for each of the eight reasons, including the blank-notes rejection paths for `Ajuste de inventario` and `Otro`.
4. Ajustar conteo for `+`, `−`, and zero delta; the reactivation path.
5. Toggle `Ubicación inicial obligatoria` ON / OFF and create lots in both settings.
6. CSV import / export.
7. Pre-V5 backup restore → movements are present after migration.

---

## Parent / Lifecycle Actions (post-implementation; bounded review and gates)

- [x] Collapse the original five-PR chained delivery plan into the user-selected local-commit-then-verify path. Local commits exist on `feat/caduxo-lot-movement-ledger`; PR creation is deferred until after SDD verify. <!-- sdd-owner: parent -->
- [x] Run the full verify gate (cargo test, svelte-check, npm run build, manual smoke on Linux + Windows) and confirm the canonical reference cited in `caduxo-lot-movement-ledger/proposal.md` is satisfied before archiving the change. Automated gates passed; manual Windows smoke is explicitly deferred by parent until VM setup / Windows compile. <!-- sdd-owner: parent -->
- [x] Open one aggregate PR for `feat/caduxo-lot-movement-ledger` after verify is green. PR #8 opened against `main`: https://github.com/Xeworg/Caduxo/pull/8 <!-- sdd-owner: parent -->
- [ ] Archive the change via `openspec archive caduxo-lot-movement-ledger` once the verify gate is green and delivery policy allows archive. <!-- sdd-owner: parent -->
