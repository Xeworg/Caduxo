# Proposal — caduxo-lot-movement-ledger

## Change metadata

- **Change ID**: `caduxo-lot-movement-ledger`
- **Domain**: `caduxo-expiry-tracker` (single-domain project)
- **Artifact store**: `openspec` (per session preflight; project config is `both`)
- **Review budget**: 400 changed lines (session override)
- **Delivery strategy**: `ask-on-risk` for phases whose forecast exceeds the budget
- **Chain strategy**: `deferred` — no chained split planned at proposal time

## Problem statement

Caduxo today records each expiry-tracked lot as a single row with one quantity
and one optional location. The user has no way to express how stock actually
flows through their business:

- A delivery of 30 units that splits 20 onto the display shelf
  (*exhibición*) and 10 into the back room (*bodega*) is recorded as a single
  lot at one location.
- A day in which 5 are sold, 3 expire, 1 is given to staff, and 2 are returned
  to the supplier is recorded as one or more free-text "partial resolution"
  events with no standardized reason and no source location.
- When the user receives a delivery, they often leave `batch_code` blank
  because the supplier wrote nothing legible; today Caduxo stores nothing.
- When the user later asks "what happened to batch L-20251015 of Yogurt
  Frutilla?", they get nothing.

The only existing audit concept (`lot_resolution_events`) is a one-quantity
reducer with a free-text `resolution` column. It cannot represent stock
moving between internal locations, it cannot represent the *entry* of a lot
into its first location, and it cannot be filtered by reason.

## Outcomes (success criteria)

After this change ships, a small-business user running Caduxo locally can:

1. **Split a lot across internal locations** (*bodega*, *exhibición*, etc.)
   and see the split both in the lot detail and on the dashboard.
2. **Record any stock exit** with a reason drawn from a fixed vocabulary
   (*Venta*, *Merma*, *Vencido*, *Dañado*, *Consumo interno*,
   *Devolución a proveedor*, *Ajuste de inventario*, *Otro*), with the
   location it left from.
3. **Move stock between two locations of the same store** as a single user
   action that preserves the lot's total quantity.
4. **Read a chronological movement history** for any lot, including the
   initial entry event and every subsequent transfer or exit.
5. **Leave `batch_code` blank at lot creation** and have Caduxo auto-generate
   a readable date-based code without overwriting a manual one.
6. **Restore a pre-change backup** and have it open in the new version with
   historical data intact and a back-filled initial movement row per
   existing lot.
7. **Trust the audit trail**: corrections never silently rewrite or delete a
   prior movement row.

## Scope (in scope)

- A new append-only **lot movement ledger** as the canonical source of truth
  for how a lot's quantity reached its current state.
- An initial **entry event** auto-emitted when a lot is created, for the
  full quantity into the initial location.
- **Transfers** between two internal locations (same store OR across
  stores in v1 — the original same-store-only restriction was reversed
  by the user), modeled as one user action with paired source/destination
  effects.
- **Exits** with a fixed reason vocabulary; the **other** exit requires a
  free-text note; the **inventory adjustment** exit requires a free-text
  note (see proposal question round).
- **Auto batch code generation** when `batch_code` is left blank at lot
  creation, in a readable `PREFIX-YYYYMMDD-NNN` format.
- A new **app configuration menu** hosting the initial-location-required
  toggle (default: required). The menu is designed to host future options
  but ships with this single setting to avoid scope creep.
- **Migration of pre-existing partial resolutions** into the new ledger as
  `exit:*` movements with their original reason text mapped to the closest
  vocabulary term (or *Otro* + note when unmappable).
- **Backfill of an initial entry movement** for every pre-existing lot, into
  its current `location_id` (or into a sentinel location if the toggle is
  off — see configuration).
- **Update of dashboard, reports, and lot detail** to surface the movement
  history; the dashboard lot detail gains a *Historial* tab.
- **Backup / restore round-trip** verified for pre-change backups.

## Non-goals (reaffirmed and tightened)

- **POS, billing, payments, accounting, full inventory management** remain
  non-goals. *Venta* is a stock-out reason only. No price, no payment, no
  customer, no tax, no invoice, no margin, no cash register, no chart of
  accounts. The schema and the UI must not introduce any column or hook
  that would invite future drift toward any of these.
- **Cross-store transfers** are IN scope for v1 (the original
  same-store-only recommendation was reversed by the user during the
  spec-phase decision round). The lot remains anchored to its original
  store via `expiry_lots.store_id`; per-location balances MAY include
  locations of other stores via the ledger.
- **Auth, users, login.** Movement *actor / source* is stored as a
  `system` / local placeholder until users exist.
- **Frontend unit tests.** No frontend test harness exists
  (`strictTdd: false`). UI changes rely on manual smoke per the canonical
  spec's `tests accompany implementation` clause.
- **Bulk movement entry, print-friendly ledgers, recent-activity widget**
  on the dashboard are out of scope unless the user explicitly asks later.

## Confirmed product decisions (locked)

Recapping the orchestrator-confirmed choices; the proposal commits to them
and they are not open for re-litigation in this round:

| # | Decision                                                              | Source                    |
|---|-----------------------------------------------------------------------|---------------------------|
| 1 | Caduxo remains product-expiry tracking only                          | Project config + user     |
| 2 | *Venta* is a stock-out reason, not a financial event                 | User                      |
| 3 | The ledger is the canonical source of truth                           | User                      |
| 4 | Existing `lot_resolution_events` migrate into the ledger             | User                      |
| 5 | V1 reason vocabulary is the full 8-term set                          | User                      |
| 6 | Expired stock exits are manual (user confirms discard + location)    | User                      |
| 7 | Auto batch code: readable `PREFIX-YYYYMMDD-NNN`, blank-only         | User                      |
| 8 | Lot creation emits initial *Entrada inicial* for full qty            | User                      |
| 9 | Initial-location-required is a new app setting (default: required)   | User                      |
| 10 | *Otro* exit requires a note                                          | User                      |
| 11 | Actor / source stored as `system` until users/login exist            | User                      |

## Proposed capabilities

The proposal adds **one new capability** and modifies **two existing
capabilities** in `openspec/specs/caduxo-expiry-tracker/spec.md`.

### New capability: *Lot movements*

- Every change in a lot's quantity or location distribution is recorded as a
  ledger row with a movement kind, a quantity, source/destination
  locations, a reason (when applicable), notes (when required), an actor
  placeholder, and a timestamp.
- Movement kinds in v1:
  - `entry:initial` — system-emitted on lot creation.
  - `transfer` — paired source/destination in the same store.
  - `exit:sale`, `exit:waste`, `exit:expired`, `exit:damaged`,
    `exit:internal_consumption`, `exit:return_to_supplier`,
    `exit:inventory_adjustment`, `exit:other`.
- The ledger is append-only. Corrections are compensating movements, not
  edits or deletes of prior rows.
- Initial-location-required is configurable in the new *Configuración* menu.
  When required and a lot form is submitted without a location, the form
  rejects the submission. When not required, lots may be created with no
  location and their initial entry is recorded against a sentinel
  *Sin ubicación* location.
- Movement *actor / source* is stored as the literal string `system` until
  users/login exist; the column exists so no migration is needed later.

### Modified capability: *Expiry lots → lot registration*

- On creation, the lot is inserted **and** its initial `entry:initial`
  movement is inserted in the same transaction.
- If `batch_code` is blank, the backend generates a `PREFIX-YYYYMMDD-NNN`
  code where `PREFIX` is the product's SKU short code (or `LOT` if the
  product has no SKU), `YYYYMMDD` is the local creation date, and `NNN` is
  a per-day counter scoped to the prefix; the generated code is echoed
  back so the UI can display it after creation.
- If `batch_code` is non-blank, the value is preserved verbatim. The
  generator is never invoked.

### Modified capability: *Expiry lots → lot resolution*

- The previous *partial resolution* concept is replaced by exit movements
  drawn from the v1 reason vocabulary.
- Pre-existing `lot_resolution_events` rows are back-filled as exit
  movements during the v1 migration; after migration the table remains in
  place as a read-only historical anchor for one release, then can be
  retired in a follow-up. (See *Migration policy*.)

## UX at high level

### Lot detail — *Historial* tab

The lot detail view (modal from the dashboard or section in the product
detail) gains a new **Historial** tab. It shows, newest-first:

- Each movement row with: timestamp, kind label (Spanish), reason label
  when applicable, source location → destination location when applicable,
  quantity, and notes when present.
- The current total remaining quantity and a per-location breakdown
  (*Bodega 10, Exhibición 20*).
- Two primary actions: **Mover stock** and **Registrar salida**.

### Move stock (*Mover stock*)

- Form fields: source location (defaults to the location with most stock),
  destination location (excludes the source), quantity.
- Validation: quantity > 0, source ≠ destination, source balance ≥
  quantity, destination and source belong to the same store.
- On submit, emits one `transfer` movement; the ledger updates, the UI
  refreshes.

### Register exit (*Registrar salida*)

- Form fields: source location, reason (radio or select from the v1
  vocabulary), quantity, notes (required when reason is *Otro* or
  *Ajuste de inventario* — see proposal question round).
- Validation: quantity > 0, source balance ≥ quantity.
- On submit, emits one `exit:*` movement; the ledger updates, the UI
  refreshes, and the dashboard count reflects the new balance.

### Adjust count (*Ajustar conteo*)

- A separate flow under the *Historial* tab for the user to record a
  physical count correction.
- The user enters the actual physical quantity at a chosen location; the
  system computes the delta and emits a single `exit:inventory_adjustment`
  movement (or an `entry` compensating movement when the physical count
  exceeds the system balance — see proposal question round on direction).
- A free-text note is required and recorded as the justification.

### Configuration menu (*Configuración*)

- A new app menu item opens a settings page.
- v1 ships exactly one setting: **Ubicación inicial obligatoria al crear
  lote** (toggle, default on). The page header, section structure, and
  save UX are designed to host additional settings later, but no other
  setting is added in this change.
- Toggling the setting affects lot creation validation only; it does not
  retroactively rewrite existing lots.

## Data migration / business policy

### Backfill of initial entries

- During the v1 migration, every pre-existing lot receives an
  `entry:initial` movement row for its current `quantity` into its current
  `location_id`.
- If the lot has no `location_id` and the initial-location-required setting
  is on, the migration attaches the initial movement to a new sentinel
  *Sin ubicación* location and the lot is flagged for the user to assign
  one on next edit.
- The backfill is idempotent: re-running it on a database that already
  contains initial entries for those lots is a no-op.

### Migration of `lot_resolution_events`

- Each pre-existing resolution row is converted to an exit movement:
  - If the original `resolution` text matches a v1 vocabulary term
    (case-insensitive), the new movement uses that term.
  - Otherwise, the new movement uses *Otro* and the original `resolution`
    text plus the original `notes` are concatenated into the new
    `notes` field.
- The original table is kept in place for one release as a read-only
  historical anchor (so older builds remain able to read backups if
  needed). A follow-up change retires it.

### Batch code policy

- Auto-generation runs only when the form submits an empty `batch_code`.
- Format: `{SKU_SHORT}-{YYYYMMDD}-{NNN}` where:
  - `SKU_SHORT` is a sanitized short prefix from the product's SKU
    (uppercased, alphanumeric, first 3–6 chars), or `LOT` if the product
    has no SKU.
  - `YYYYMMDD` is the local creation date.
  - `NNN` is a 3-digit zero-padded counter that increments per day per
    prefix; collision is resolved by suffixing `-2`, `-3`, etc.
- Manual `batch_code` values are preserved verbatim. The generator is
  never invoked on a non-blank input. This invariant is enforced
  backend-side and tested.
- Pre-existing lots without a `batch_code` are **not** back-filled with an
  auto-generated code at migration time (their audit is the migration
  event itself); auto-generation applies only to **new** creations.

### Configuration settings

- Stored as key/value rows in a new `app_settings` table or a single
  settings row, design to be settled in `design.md`.
- v1 ships one key: `require_initial_location_on_lot_create` (bool,
  default `true`).
- Reading is cheap and the UI binds to it on settings load; no live
  reactivity required.

## Reason vocabulary (Spanish-facing UI)

| Internal kind                | Spanish label            | English label           | Notes required |
|------------------------------|--------------------------|-------------------------|----------------|
| `entry:initial`              | *Entrada inicial*        | Initial entry           | n/a (system)   |
| `transfer`                   | *Traslado*               | Transfer                | no             |
| `exit:sale`                  | *Venta*                  | Sale                    | no             |
| `exit:waste`                 | *Merma*                  | Waste                   | no             |
| `exit:expired`               | *Vencido*                | Expired                 | no             |
| `exit:damaged`               | *Dañado*                 | Damaged                 | no             |
| `exit:internal_consumption`  | *Consumo interno*        | Internal consumption    | no             |
| `exit:return_to_supplier`    | *Devolución a proveedor* | Return to supplier      | no             |
| `exit:inventory_adjustment`  | *Ajuste de inventario*   | Inventory adjustment    | **yes** (see proposal question round) |
| `exit:other`                 | *Otro*                   | Other                   | **yes** (confirmed) |

UI labels are Spanish; the artifact text (this proposal, design, tasks)
remains in English.

## Risks and mitigations

1. **Shape decision (A vs. B vs. C from explore §5) is foundational.**
   The proposal commits to **Option A**: `expiry_lots.quantity` stays the
   canonical total; per-location balance is derived from the ledger.
   Mitigation: the schema is the smallest possible change, and a future
   shift to B/C remains possible without rewriting the ledger itself.

2. **Backup / restore regression.** Pre-change backups must open in the
   new version. Mitigation: a new `restore_from_pre_v1_backup_*` test,
   following the v4 (`caduxo-category-management`) precedent, is
   mandatory.

3. **Review budget overrun.** v1 migration + ledger skeleton + lot creation
   integration may approach or exceed the 400-line budget. Mitigation:
   before `tasks.md`, the parent runs `deliveryStrategy: ask-on-risk` to
   pause if a phase forecast is too large.

4. **Manual batch overwritten by accident.** Mitigation: backend enforces
   the blank-only invariant; a test covers the scenario explicitly.

5. **Auto-batch collisions.** Same-day, same-prefix lots would collide on
   `NNN=001`. Mitigation: counter increments per day per prefix; collisions
   append `-2`, `-3`, etc. The counter is a small per-day sequence scoped
   to the prefix.

6. **Edit / undo policy ambiguity.** The proposal commits to
   **append-only with compensating movements** (no edit, no delete; a
   correction is a new movement). This matches the canonical audit-trail
   pattern from the research phase (E3). v1 ships no "undo" affordance for
   a mistaken movement; the user enters a compensating movement with a
   clear note.

7. **Non-goal drift.** *Venta* could grow a price field "for context".
   Mitigation: the proposal names the invariant explicitly and the new
   requirement forbids price/payment/customer/tax columns in the
   `lot_movements` table and in the *Registrar salida* form.

8. **Frontend has no test harness.** UI changes rely on manual smoke.
   Mitigation: the verify gate lists Windows + Linux manual smoke covering
   lot creation, transfer, each exit reason, count adjustment, settings
   toggle, and CSV import/export.

## Rollout and phasing

The phases are illustrative; `tasks.md` will forecast each one against the
400-line budget before locking.

- **Phase 0 — Confirm proposal.** Lock this proposal with the user. No
  code. Resolves the proposal question round.
- **Phase 1 — Backend ledger skeleton.** V1 migration (table +
  initial-entry backfill + `lot_resolution_events` migration), domain
  module, repository, service, and the pre-backup restore test. No UI.
  Verify: `cargo test --lib` green.
- **Phase 2 — Lot creation integration.** Initial-movement emission in the
  same transaction, batch-code auto-generation, LotForm hint and
  post-create confirmation. Verify: `cargo test` + `svelte-check` +
  manual smoke.
- **Phase 3 — Movements UI.** *Historial* tab, *Mover stock*,
  *Registrar salida*, *Ajustar conteo*. Verify: `cargo test` +
  `svelte-check` + manual smoke on Linux + Windows.
- **Phase 4 — Dashboard / Reports / Settings.** Per-lot detail *Historial*
  surfacing, optional *Movimientos por motivo* report, settings menu with
  the initial-location toggle, CSV export shape update. Verify: full gates
  - manual smoke.
- **Phase 5 (deferred / optional) — Polish.** Recent-activity widget,
  bulk movement entry, print-friendly ledger. Out of scope unless the
  user explicitly requests.

The canonical verify gate is:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib`
- `npx svelte-check --workspace . --threshold error`
- `npm run build`
- Manual smoke on Linux + Windows covering lot creation, transfer, each
  exit reason, count adjustment, settings toggle, CSV import/export, and
  pre-v1 backup restore.

## Open questions (proposal question round) — RESOLVED

All items below were settled during the orchestrator-confirmed decision
round that preceded this change's spec phase. They are recorded here for
traceability; each one is now a locked decision that the spec, design,
and tasks phases implement as-is.

These are product/UX level only — no harness mechanics (test commands,
PR shape, line budgets) are raised here.

1. **Ledger shape commitment.** **RESOLVED — Option A.** Append-only
   ledger; `expiry_lots.quantity` remains the canonical total; per-location
   balances are derived from the ledger. The alternative shapes (Option B
   per-location stock lines; Option C hybrid materialized view) are
   rejected for v1 and remain open as future slices if read performance
   on derived balances ever becomes a concern.

2. **Cross-store transfers.** **RESOLVED — REVERSED.** Cross-store
   transfers ARE included in v1 (the original same-store-only
   recommendation was reversed by the user). The service MUST allow the
   destination store to differ from the source store and the lot's
   `expiry_lots.store_id` anchor remains the original store. The
   previously stated non-goal "Cross-store transfers are out of scope
   for v1" is REMOVED from the non-goals list for this change.

3. **`inventory_adjustment` note requirement.** **RESOLVED — REQUIRED.**
   The `Registrar salida` path with reason *Ajuste de inventario*
   (mapping to `exit:inventory_adjustment`) MUST require a non-blank
   free-text note, mirroring *Otro*. The service rejects blank notes
   with a validation error.

4. **`Ajustar conteo` direction.** **RESOLVED — unified kind with
   directional sign.** The `Ajustar conteo` flow emits a SINGLE movement
   of kind `inventory_adjustment` with a `direction` of `increase` or
   `decrease`. The `quantity` column stores the absolute magnitude
   (`> 0`). The source/destination nullability follows the direction
   (entry-shaped for increase, exit-shaped for decrease). The design's
   two-kind proposal (`exit:inventory_adjustment` + an additional
   `entry:inventory_adjustment`) is REJECTED in favor of this single
   kind. The justification note is REQUIRED. Zero-delta submissions are
   a no-op.

5. **Historical `lot_resolution_events` surfacing.** **RESOLVED —
   unified display.** Migrated rows appear in the *Historial* tab under
   their mapped vocabulary term (or under *Otro* + note). There is NO
   separate *Eventos legacy* filter or section in v1. Known legacy
   values map to the closest v1 vocabulary term; unknown values map to
   `exit:other` with the original text preserved in `notes`.

6. **Dashboard movement visibility.** **RESOLVED — per-lot history only.**
   Movements are surfaced ONLY on the per-lot *Historial* tab. The
   recent-activity dashboard widget is explicitly DEFERRED. The
   dashboard continues to show one row per active lot, as before; no
   per-lot recent-movement widget is added in v1.

Once these were answered, the proposal moved to `design.md` (schema,
wire format, UI primitives, migration plan), then to the spec phase
which now records these decisions as locked delta requirements under
`openspec/changes/caduxo-lot-movement-ledger/specs/caduxo-expiry-tracker/spec.md`,
and finally to `tasks.md` (forecasted phase slices) for implementation.
