# Delta for Caduxo Expiry Tracker

## ADDED Requirements

### Capability: Lot movements

#### Requirement: append-only movement ledger

The system MUST record every change in a lot's quantity or per-location distribution as an immutable row in a `lot_movements` table. The ledger is the canonical source of truth for a lot's movement history; the lot's total quantity is a denormalized column maintained inside the same transaction as every ledger write, and per-location balances are derived on demand.

- `expiry_lots.quantity` SHALL remain the denormalized canonical total. It MUST equal `SUM(quantity WHERE destination_location_id IS NOT NULL) − SUM(quantity WHERE source_location_id IS NOT NULL)` for the affected lot after every ledger write.
- The system MUST NOT expose any runtime path that `UPDATE`s or `DELETE`s an existing `lot_movements` row. Corrections SHALL be made by inserting a new compensating movement row; never by editing or removing a prior row.
- No `lot_movements` row SHALL carry any monetary or financial field (no price, no payment method, no customer, no tax, no invoice, no margin, no cost). "Sale" remains a stock-out reason only.

#### Scenario: ledger write and lot-total update are atomic

- GIVEN a lot has `quantity = 10`, with 5 at `Bodega` and 5 at `Exhibición`
- WHEN the user records a `Registrar salida` of 2 units with reason `Venta` from `Bodega`
- THEN a new `exit:sale` row is inserted into `lot_movements`
- AND `expiry_lots.quantity` is updated to `8` in the same transaction
- AND the per-location balance for `Bodega` becomes `3`
- AND the per-location balance for `Exhibición` remains `5`

#### Scenario: correction is a compensating movement, never an edit

- GIVEN a `Venta` of 2 units was recorded against the wrong lot by mistake
- WHEN the user records a compensating `Venta` of 2 units against the correct lot's same source location
- THEN a new `exit:sale` row is inserted (no `UPDATE` or `DELETE` is issued against the prior row)
- AND the original row remains in the ledger unchanged

#### Requirement: movement kind vocabulary

The system MUST use a fixed movement kind vocabulary. Each kind binds to a fixed source/destination nullability pattern, and the binding MUST be enforced both at the service layer and by a database `CHECK` constraint on `lot_movements`.

| Kind                       | `source_location_id` | `destination_location_id` | Free-text note |
|----------------------------|----------------------|---------------------------|----------------|
| `entry:initial`            | `NULL`               | set                       | not applicable (system-emitted) |
| `transfer`                 | set                  | set (≠ source)            | optional |
| `exit:sale`                | set                  | `NULL`                    | optional |
| `exit:waste`               | set                  | `NULL`                    | optional |
| `exit:expired`             | set                  | `NULL`                    | optional |
| `exit:damaged`             | set                  | `NULL`                    | optional |
| `exit:internal_consumption`| set                  | `NULL`                    | optional |
| `exit:return_to_supplier`  | set                  | `NULL`                    | optional |
| `exit:inventory_adjustment`| set                  | `NULL`                    | REQUIRED |
| `exit:other`               | set                  | `NULL`                    | REQUIRED |
| `inventory_adjustment`     | follows direction    | follows direction         | REQUIRED |

The `inventory_adjustment` kind is the unified count-correction movement emitted by the `Ajustar conteo` flow. It carries a directional sign so a single kind encodes both an increase (physical count exceeds the prior system balance) and a decrease (physical count falls below it). The source/destination nullability follows the sign: an increase sets `destination_location_id` only (entry-shaped), and a decrease sets `source_location_id` only (exit-shaped). The magnitude stored in `quantity` is always the absolute difference and SHALL be `> 0` for any persisted row.

#### Scenario: vocabulary is exhaustive and the kind/source/destination contract is enforced

- GIVEN any persisted `lot_movements` row
- WHEN the row is read
- THEN its `movement_kind` is one of the eleven values listed in the table
- AND its source/destination nullability matches the table
- AND rows of kind `exit:other`, `exit:inventory_adjustment`, or `inventory_adjustment` carry a non-blank `notes` value
- AND rows of any other `exit:*` kind carry either a null or a non-blank `notes` value (optional)

#### Requirement: initial entry on lot creation

When a lot is created, the system MUST insert a single `entry:initial` movement in the same database transaction as the `INSERT INTO expiry_lots`.

- The initial movement's `quantity` SHALL equal the lot's `quantity` at creation.
- The initial movement's `destination_location_id` SHALL equal the lot's `location_id` after location resolution (the chosen location, or the per-store sentinel `Sin ubicación` when the `require_initial_location_on_lot_create` setting is off and the user left the picker empty).
- The initial movement's `actor` SHALL be the literal string `system`.

#### Scenario: lot creation writes both lot and initial movement atomically

- GIVEN a product P and a chosen location L exist
- WHEN the user submits the lot creation form with `quantity = 12` and `location_id = L`
- THEN the `expiry_lots` row is inserted with `quantity = 12` and `location_id = L`
- AND a corresponding `entry:initial` movement is inserted in the same transaction
- AND the lot detail's Historial tab shows the initial entry as the oldest row

#### Requirement: transfers within and across stores

The system MUST allow moving stock from one internal location to another internal location. The destination location MAY belong to the same store as the source location or to a different store.

- A transfer is one user action that emits a single `transfer` row with `source_location_id`, `destination_location_id` (both non-null and distinct), and a positive `quantity`.
- The service MUST reject a transfer whose `quantity` exceeds the current source balance for the lot at the source location.
- The service MUST reject a transfer whose source or destination location is inactive (`is_active = 0`).
- The service MUST allow the destination store to differ from the source store. The lot's `expiry_lots.store_id` anchor remains the original store; per-location balances MAY include locations of other stores via the ledger.

(Previously: cross-store transfers were excluded from v1; the proposal's "Cross-store transfers are out of scope for v1" non-goal is reversed in this delta.)

#### Scenario: same-store transfer preserves the lot total

- GIVEN a lot has 10 units at `Bodega-A` and 0 units at `Exhibición-A`, both in store A
- WHEN the user records a Mover stock of 3 units from `Bodega-A` to `Exhibición-A`
- THEN a single `transfer` row is inserted
- AND the lot total remains `10`
- AND `Bodega-A`'s per-location balance becomes `7`
- AND `Exhibición-A`'s per-location balance becomes `3`

#### Scenario: cross-store transfer moves stock across stores

- GIVEN a lot L was created in store A with `store_id = A`
- AND a location `Bodega-B` exists in store B
- WHEN the user records a Mover stock of 5 units from `Bodega-A` to `Bodega-B`
- THEN the transfer is accepted
- AND a single `transfer` row is inserted
- AND the lot total remains unchanged
- AND `Bodega-A`'s balance decreases by `5`
- AND `Bodega-B`'s balance increases by `5`
- AND `expiry_lots.store_id` remains `A`

#### Scenario: transfer rejected when source balance is insufficient

- GIVEN a lot has only 3 units at `Bodega`
- WHEN the user records a Mover stock of 5 units from `Bodega`
- THEN the insert is rejected with a validation error
- AND no movement row is written
- AND the per-location balances are unchanged

#### Requirement: exit movements with reason vocabulary

The system MUST accept stock-out movements drawn from the v1 reason vocabulary. Each `Registrar salida` submission emits a single `exit:*` movement with the selected reason, a chosen source location, and a positive `quantity`.

- The reason MUST be one of: `Venta`, `Merma`, `Vencido`, `Dañado`, `Consumo interno`, `Devolución a proveedor`, `Ajuste de inventario`, `Otro`. Each maps to one of the eight `exit:*` kinds in the vocabulary table.
- The system MUST reject the insert when the reason is `Otro` or `Ajuste de inventario` (i.e., `exit:other` or `exit:inventory_adjustment`) and the supplied `notes` is blank.
- The system MUST reject the insert when the chosen source location's per-location balance for the lot is less than `quantity`.

#### Scenario: exit with optional note succeeds

- GIVEN an active lot has 10 units at `Bodega`
- WHEN the user records a `Registrar salida` of 4 units with reason `Venta` from `Bodega` and an empty `notes` field
- THEN a single `exit:sale` row is inserted with `quantity = 4`
- AND the lot total becomes `6`
- AND the lot remains active
- AND the Historial panel lists the `exit:sale` movement

#### Scenario: exit with reason Otro requires a non-blank note

- GIVEN an active lot has 10 units at `Bodega`
- WHEN the user records a `Registrar salida` of 4 units with reason `Otro` and a blank `notes` field
- THEN the service returns a validation error
- AND no movement row is written
- AND the lot total and per-location balance are unchanged

#### Scenario: exit with reason Ajuste de inventario requires a non-blank note

- GIVEN an active lot has 10 units at `Bodega`
- WHEN the user records a `Registrar salida` of 4 units with reason `Ajuste de inventario` and a blank `notes` field
- THEN the service returns a validation error
- AND no movement row is written

#### Requirement: count adjustment with directional sign

The system MUST provide an `Ajustar conteo` flow that records a single count-correction movement of kind `inventory_adjustment` with a directional sign and a required justification note.

- The kind is `inventory_adjustment` (a single kind, distinct from the eight `exit:*` kinds). The movement carries a `direction` of `increase` or `decrease` and a `quantity` equal to the absolute difference between the physical count and the prior system balance for the lot at the chosen location. `quantity` SHALL be `> 0`; a zero-delta submission is a no-op and writes no row.
- When `direction = increase`, the row is entry-shaped: `source_location_id IS NULL`, `destination_location_id = chosen_location`.
- When `direction = decrease`, the row is exit-shaped: `source_location_id = chosen_location`, `destination_location_id IS NULL`.
- The `notes` field is REQUIRED. The service MUST reject the insert when `notes` is blank.
- The `quantity` magnitude MUST equal the absolute difference between the physical count and the prior system balance for the lot at the chosen location.

#### Scenario: Ajustar conteo records a single increase movement

- GIVEN a lot has 7 units at `Bodega` and the user performs a physical count that finds 9 units at `Bodega`
- WHEN the user submits Ajustar conteo with physical count `9`, location `Bodega`, and a non-blank justification
- THEN a single `inventory_adjustment` row is inserted with `direction = increase`, `destination_location_id = Bodega`, `quantity = 2`, and the supplied `notes`
- AND `expiry_lots.quantity` becomes `9`
- AND the per-location balance for `Bodega` becomes `9`
- AND no separate `entry:*` kind is emitted

#### Scenario: Ajustar conteo records a single decrease movement

- GIVEN a lot has 7 units at `Bodega` and the user performs a physical count that finds 5 units at `Bodega`
- WHEN the user submits Ajustar conteo with physical count `5`, location `Bodega`, and a non-blank justification
- THEN a single `inventory_adjustment` row is inserted with `direction = decrease`, `source_location_id = Bodega`, `quantity = 2`, and the supplied `notes`
- AND `expiry_lots.quantity` becomes `5`

#### Scenario: Ajustar conteo with zero delta is a no-op

- GIVEN a lot has 7 units at `Bodega` and the user performs a physical count that finds 7 units at `Bodega`
- WHEN the user submits Ajustar conteo with physical count `7`, location `Bodega`, and any note
- THEN no `lot_movements` row is inserted
- AND `expiry_lots.quantity` and the per-location balances are unchanged

#### Scenario: Ajustar conteo with blank note is rejected

- GIVEN the user submits Ajustar conteo with a non-zero delta and a blank justification
- WHEN the service validates the insert
- THEN the insert is rejected with a validation error
- AND no movement row is written

#### Requirement: reactivation of a resolved lot via a compensating increase

A lot whose `status = 'resolved'` and `quantity = 0` MAY be reactivated by a single `inventory_adjustment` movement with `direction = increase` (the count-correction flow). The service SHALL flip `status` back to `active`, clear `resolution` and `resolved_at`, and increase `expiry_lots.quantity` by the magnitude in the same transaction.

#### Scenario: resolved lot is reactivated by a compensating count increase

- GIVEN a lot is fully resolved with `quantity = 0`, `status = 'resolved'`, and `resolution = 'exit:expired'`
- WHEN the user submits Ajustar conteo with physical count `3` at `Bodega` and a non-blank justification
- THEN a single `inventory_adjustment` row is inserted with `direction = increase`, `destination_location_id = Bodega`, `quantity = 3`, and the supplied `notes`
- AND `expiry_lots.quantity` becomes `3`
- AND `expiry_lots.status` becomes `'active'`
- AND `expiry_lots.resolution` becomes `NULL`
- AND `expiry_lots.resolved_at` becomes `NULL`

#### Requirement: derived per-location balance

The system MUST compute a lot's per-location balance from the ledger on demand. The balance for a `(lot_id, location_id)` pair SHALL be `SUM(quantity WHERE destination_location_id = location_id) − SUM(quantity WHERE source_location_id = location_id)`; a zero or negative balance is treated as fully depleted for that location.

- The system MUST NOT persist a per-location balance as a primary column. The only persisted state is the ledger; balances are derived at read time.
- The derivation MUST be computed against a consistent snapshot so concurrent inserts cannot leak half-applied state to a reader.

#### Scenario: per-location balance reflects the full ledger

- GIVEN a lot has the following movements: entry `10` at `Bodega`, transfer `4` from `Bodega` to `Exhibición`, exit:sale `1` from `Exhibición`
- WHEN the lot detail requests per-location balances
- THEN `Bodega` reports `6`
- AND `Exhibición` reports `3`
- AND the lot total reports `9`

#### Requirement: unified movement history on the per-lot detail

The system MUST surface a chronological movement history on each lot's detail view, rendered as the `Historial` tab inside the existing lot detail surface.

- The Historial tab MUST list every movement for the lot (including the initial entry, all exits and transfers, all `Ajustar conteo` rows, and any migrated legacy events), newest first.
- Each row MUST display the timestamp, the kind label in Spanish, the reason label when applicable, the source and/or destination location when applicable, the `quantity` as a positive magnitude (with an explicit `+` / `−` sign for `inventory_adjustment` rows), and the `notes` when present.
- The Historial tab MUST also render the lot's current total remaining quantity and a per-location breakdown.
- The dashboard MUST NOT introduce a separate "recent activity" or movement widget for active lots in v1. Movements remain visible only via the per-lot detail.

#### Scenario: Historial panel shows initial and subsequent movements

- GIVEN a lot has an `entry:initial` of `10` at `Bodega`, a `transfer` of `4` from `Bodega` to `Exhibición`, and an `exit:sale` of `2` from `Exhibición`
- WHEN the user opens the Historial tab for that lot
- THEN the panel lists three rows, newest first
- AND the panel header shows lot total `8`
- AND the per-location breakdown shows `Bodega 6, Exhibición 2`
- AND no entry is duplicated or missing

#### Scenario: dashboard does not show a movement widget in v1

- GIVEN the dashboard renders with active lots
- WHEN the dashboard list is shown
- THEN no per-lot recent-movement widget is rendered for any lot
- AND the dashboard continues to show one row per active lot, as before

#### Requirement: legacy resolution events migrate into the unified ledger

The system MUST back-fill pre-existing `lot_resolution_events` rows as `lot_movements` rows during the v5 schema migration. Each migrated row is converted to a single exit movement drawn from the v1 vocabulary and appears in the same Historial panel as native ledger movements.

- If the original `resolution` text matches a v1 vocabulary term (case-insensitive), the new movement uses that term.
- If the original `resolution` text does not match any v1 vocabulary term, the new movement uses `exit:other` and the original text is preserved in the new `notes` field (with any original `notes` appended).
- The legacy `lot_resolution_events` table SHALL remain in place for one release as a read-only anchor; no runtime path SHALL read or write it after migration.

#### Scenario: known legacy resolution maps to v1 vocabulary

- GIVEN a `lot_resolution_events` row with `resolution = 'sold'`, `quantity = 2`, and `notes = 'staff lunch'`
- WHEN the v5 migration applies
- THEN a new `exit:sale` movement is inserted with `quantity = 2`, `notes = 'staff lunch'`, and the original `created_at`
- AND the original row in `lot_resolution_events` is left untouched

#### Scenario: unknown legacy resolution maps to Otro with note

- GIVEN a `lot_resolution_events` row with `resolution = 'donated'` and `quantity = 1`
- WHEN the v5 migration applies
- THEN a new `exit:other` movement is inserted with `notes = 'donated'`
- AND the original row in `lot_resolution_events` is left untouched
- AND the Historial panel renders this row under reason `Otro` with the note visible

#### Requirement: backfill of initial entries for pre-existing lots

The system MUST insert one `entry:initial` movement for every pre-existing lot during the v5 schema migration, into the lot's current `location_id`.

- Lots with `location_id IS NULL` at migration time SHALL be repointed to the per-store sentinel `Sin ubicación` location before the initial entry is inserted.
- The backfill is idempotent: re-running the migration adds zero additional rows.

#### Scenario: pre-existing lot receives an initial entry on migration

- GIVEN a database with one lot L having `quantity = 8` and `location_id = 'bodega-id'`
- WHEN the v5 migration applies
- THEN exactly one `entry:initial` row exists for L with `destination_location_id = 'bodega-id'` and `quantity = 8`
- AND re-running the migration does not insert a second row

#### Scenario: pre-existing lot with NULL location points to the sentinel

- GIVEN a database with one lot L having `quantity = 5` and `location_id IS NULL` in store S
- WHEN the v5 migration applies
- THEN a sentinel `Sin ubicación` location exists for store S
- AND L's `location_id` is repointed to the sentinel
- AND exactly one `entry:initial` row exists for L with `destination_location_id = sentinel.id` and `quantity = 5`

#### Requirement: sentinel location for unassigned stock

The system MUST guarantee that every store has at most one sentinel `Sin ubicación` location. The sentinel is a regular active location used as the destination for the initial entry when a lot is created without a chosen location and the `require_initial_location_on_lot_create` setting is off.

- The sentinel is created lazily during migration and during lot creation.
- The sentinel's id SHALL be deterministic for the store so re-runs are idempotent.
- The sentinel SHALL be treated as a regular active location everywhere (filters, joins, dashboard) and is not hidden from pickers in v1; the Spanish label `Sin ubicación` makes the intent obvious.

#### Scenario: lot creation without location uses the sentinel when the setting is off

- GIVEN a store S exists with a sentinel `Sin ubicación`
- AND the `require_initial_location_on_lot_create` setting is off
- WHEN the user creates a lot in store S with `quantity = 6` and leaves the location picker empty
- THEN the lot's `location_id` is set to the sentinel id
- AND the initial entry's `destination_location_id` is the sentinel id

#### Requirement: actor placeholder is `system`

The system MUST store the actor of every movement as the literal string `system` until users or login exist. No runtime path SHALL emit any other actor value in v1.

#### Scenario: actor is recorded as `system`

- GIVEN any user action creates a movement via `Registrar salida`, `Mover stock`, or `Ajustar conteo`
- WHEN the new `lot_movements` row is persisted
- THEN the `actor` column contains the string `system`

## MODIFIED Requirements

### Capability: Expiry lots

#### Requirement: lot registration

(Previously: the requirement listed required, conditionally required, and optional fields. It specified the `unit` resolution and the `expiry date` picker shape. It did not mention initial-movement emission, batch-code auto-generation, settings-aware location validation, or the sentinel location for unassigned stock.)

The lot registration contract is extended as follows:

- On lot creation, the system MUST emit an `entry:initial` movement in the same database transaction as the `INSERT INTO expiry_lots` (per the new `initial entry on lot creation` requirement).
- The lot's `quantity` field SHALL remain the canonical total. Per-location balances are derived from the ledger (per the new `derived per-location balance` requirement).
- When the user leaves `batch_code` blank, the system MUST auto-generate a `PREFIX-YYYYMMDD-NNN` code where:
  - `PREFIX` is the sanitized alphanumeric SKU short prefix (3–6 chars, uppercased), padded with `X` if the source has fewer than three alphanumeric characters, or the literal `LOT` if the product has no SKU.
  - `YYYYMMDD` is the local creation date.
  - `NNN` is a per-day per-prefix counter starting at `001`. If `PREFIX-YYYYMMDD-NNN` already exists, the system SHALL try `NNN+1`, `NNN+2`, etc.; if `NNN > 999`, the system SHALL fall back to `PREFIX-YYYYMMDD-NNN-M` where `M = 2, 3, ...` until a free slot is found.
- When the user enters a non-blank `batch_code`, the system MUST preserve it verbatim. The auto-generator MUST NOT be invoked for non-blank input. This invariant SHALL be enforced at the service layer and SHALL also be enforced by the LotForm frontend guard.
- The lot registration form MUST respect the `require_initial_location_on_lot_create` setting:
  - When ON (default) and the user submits without a location, the form rejects the submission with an inline error and no lot or movement row is written.
  - When OFF and the user submits without a location, the lot is created against the per-store sentinel `Sin ubicación` (per the new `sentinel location for unassigned stock` requirement).
- The remaining fields and constraints from the prior version of this requirement are preserved:

  Required lot fields:

  - product
  - quantity
  - unit — the resolved free-text label displayed on the lot. When the product references a catalog unit via `default_unit_id`, the value SHALL be the catalog unit's `display_name`. When no catalog unit is set, the value SHALL be the legacy `products.default_unit` text, or, if that is empty, the seeded `units` preset's `display_name` (`Unidades`). The lot form SHALL render the `unit` field as a read-only chip when the product has a catalog unit, and SHALL keep it editable for legacy lots whose product has `unit_type = null`.
  - expiry date
  - alert days before expiry

  Conditionally required:

  - store/local when multiple stores exist

  Optional:

  - internal location
  - batch/lot code
  - notes

  The `expiry date` field uses the `Date input` picker with `clearable={false}`. The host `required` JS guard (`if (!expiryDate) errorMsg = "Expiry date is required";`) remains the source of truth for blocking empty submits. No `<input type="date">` is rendered for this field.

#### Scenario: lot creation with auto-generated batch

- GIVEN a product P has `sku = "YOG-001"` and the user creates a lot with `batch_code = ""` on `2026-10-15`
- WHEN the lot creation submit succeeds
- THEN the returned lot has `batch_code = "YOG001-20261015-001"` (or `...-002` if `...-001` already exists for that prefix and date)
- AND the LotForm displays a confirmation chip with the generated code
- AND an `entry:initial` movement is written in the same transaction

#### Scenario: manual batch is preserved verbatim

- GIVEN the user creates a lot with `batch_code = "SUPPLIER-XYZ-2026"`
- WHEN the lot creation submit succeeds
- THEN the returned lot has `batch_code = "SUPPLIER-XYZ-2026"`
- AND the auto-generator is not invoked
- AND an `entry:initial` movement is written in the same transaction

#### Scenario: blank batch with the required-location toggle off uses the sentinel

- GIVEN the `require_initial_location_on_lot_create` setting is off
- WHEN the user submits a lot creation form with no location selected and `batch_code = ""`
- THEN the lot is created against the per-store sentinel `Sin ubicación`
- AND the initial entry's `destination_location_id` is the sentinel id
- AND no validation error is raised

#### Scenario: blank batch with the required-location toggle on is rejected

- GIVEN the `require_initial_location_on_lot_create` setting is on
- WHEN the user submits a lot creation form with no location selected
- THEN the form rejects the submission with the message `Selecciona una ubicación`
- AND no `expiry_lots` row is created
- AND no `lot_movements` row is created

#### Requirement: partial resolution

(Previously: the requirement was a one-paragraph statement with a single scenario. It did not specify the data model or the reason vocabulary. The audit trail was a `lot_resolution_events` row with a free-text `resolution` and `notes`.)

The `partial resolution` concept is now realized as an exit movement drawn from the v1 reason vocabulary. The user-facing behavior is preserved (resolve part of a lot, keep the rest active), but the data model and reason contract change.

- Resolving part of a lot quantity SHALL emit a single `exit:*` movement with the user's selected reason, source location, and positive `quantity`.
- The reason MUST be drawn from the v1 vocabulary (`Venta`, `Merma`, `Vencido`, `Dañado`, `Consumo interno`, `Devolución a proveedor`, `Ajuste de inventario`, `Otro`).
- The `notes` field SHALL be REQUIRED when the reason is `Ajuste de inventario` or `Otro`; OPTIONAL otherwise.
- Pre-existing `lot_resolution_events` rows SHALL be migrated into `lot_movements` rows during the v5 migration (per the new `legacy resolution events migrate into the unified ledger` requirement) and the legacy table becomes read-only.

#### Scenario: partial exit with reason preserves the remaining quantity

- GIVEN an active lot has `quantity = 10` and the user selects reason `Venta`
- WHEN the user resolves 4 units via `Registrar salida` with reason `Venta`
- THEN the lot total becomes `6`
- AND the lot remains active
- AND the Historial panel lists the `exit:sale` movement with `quantity = 4`

#### Scenario: legacy resolution data is migrated into the ledger

- GIVEN a pre-V5 `lot_resolution_events` row with `resolution = 'consumed'`, `quantity = 2`, and `notes = 'staff lunch'`
- WHEN the v5 migration applies
- THEN a new `exit:internal_consumption` movement is inserted with `quantity = 2` and `notes = 'staff lunch'`
- AND the legacy row remains in place (read-only)
