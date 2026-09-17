# Design — caduxo-lot-movement-ledger

## 0. Scope and alignment

This design is the technical companion to `proposal.md`. It commits to
Option A (ledger-as-truth, `expiry_lots.quantity` canonical, per-location
balance derived), the eight-term exit vocabulary, the manual expired flow,
the auto-batch `PREFIX-YYYYMMDD-NNN` blank-only policy, and the
initial-location-required setting. Anything in this document that conflicts
with the proposal is a bug in this document; the proposal is parent
authority.

`openspec/changes/caduxo-lot-movement-ledger/proposal.md` lists the eleven
locked product decisions. The relevant restatements for design:

| Decision | Design consequence |
|---|---|
| Ledger is canonical; `expiry_lots.quantity` is canonical total | Per-location balance is a derived SUM; lot total stays a denormalized column maintained in the same transaction as the ledger write |
| `lot_resolution_events` migrate into the ledger | V5 back-fill converts legacy rows into `exit:*` movements; legacy table stays as a read-only anchor for one release |
| Full eight-term vocabulary | New `MovementKind` enum with exactly the kinds in proposal §Reason vocabulary |
| Manual expired exits | No scheduled job, no auto-emit on launch; the user chooses *Registrar salida* with reason *Vencido* |
| Auto batch: `PREFIX-YYYYMMDD-NNN`, blank-only | Generator runs only when `batch_code` is null/blank after trim; manual codes are preserved verbatim |
| Initial entry on lot creation | One `entry:initial` movement emitted in the same DB transaction as `INSERT INTO expiry_lots` |
| Initial-location-required setting (default on) | New app setting `require_initial_location_on_lot_create: bool = true`; LotForm reads it and applies a frontend guard |
| *Otro* and *Ajuste de inventario* require a note | Service-layer validation rejects the insert when `notes` is blank for those kinds |
| Actor stored as `system` until auth exists | `actor TEXT NOT NULL DEFAULT 'system'` column on `lot_movements`; no runtime path emits anything else |
| Cross-store transfers allowed in v1 (spec reversal) | Service MUST allow `destination_location.store_id != source_location.store_id`; rejects only when source or destination is inactive (`is_active = 0`); `expiry_lots.store_id` is NOT updated by a transfer |
| Unified `inventory_adjustment` kind for Ajustar conteo (spec reversal) | One `inventory_adjustment` kind with a `direction` column (`'increase'` or `'decrease'`); schema CHECK ties source/destination nullability to direction; service routes both signs through one insert |
| Compensating movements (no edit/delete) | No `update_lot_movement` or `delete_lot_movement` API; corrections are new rows |

The review budget for this change is **400 changed lines** (session override;
canonical is 800). Phase forecasts in §11.

## 1. Data model

### 1.1 New table — `lot_movements`

Append-only ledger. The row shape:

```
lot_movements
  id              TEXT PRIMARY KEY                   -- uuid v4 string
  expiry_lot_id   TEXT NOT NULL REFERENCES expiry_lots(id) ON DELETE CASCADE
  movement_kind   TEXT NOT NULL                     -- enum string; see §1.2
  direction       TEXT NULL                         -- 'increase' | 'decrease'; set IFF movement_kind = 'inventory_adjustment'
  quantity        REAL NOT NULL                      -- > 0 always; absolute magnitude for inventory_adjustment
  source_location_id      TEXT NULL REFERENCES store_locations(id) ON DELETE RESTRICT
  destination_location_id TEXT NULL REFERENCES store_locations(id) ON DELETE RESTRICT
  reason          TEXT NULL                          -- free text for legacy 'Otro' only; null otherwise
  notes           TEXT NULL
  actor           TEXT NOT NULL DEFAULT 'system'
  created_at      TEXT NOT NULL                      -- RFC3339
  CHECK(quantity > 0)
  CHECK(direction IS NULL OR direction IN ('increase', 'decrease'))
  CHECK(
    (movement_kind = 'inventory_adjustment' AND direction IS NOT NULL)
    OR
    (movement_kind <> 'inventory_adjustment' AND direction IS NULL)
  )
  CHECK(movement_kind IN (
    'entry:initial',
    'transfer',
    'exit:sale',
    'exit:waste',
    'exit:expired',
    'exit:damaged',
    'exit:internal_consumption',
    'exit:return_to_supplier',
    'exit:inventory_adjustment',
    'exit:other',
    'inventory_adjustment'
  ))
  CHECK(
    (movement_kind = 'entry:initial'      AND source_location_id IS NULL     AND destination_location_id IS NOT NULL)
    OR
    (movement_kind = 'transfer'          AND source_location_id IS NOT NULL AND destination_location_id IS NOT NULL AND source_location_id <> destination_location_id)
    OR
    (movement_kind LIKE 'exit:%'          AND source_location_id IS NOT NULL AND destination_location_id IS NULL)
    OR
    (movement_kind = 'inventory_adjustment' AND direction = 'increase' AND source_location_id IS NULL     AND destination_location_id IS NOT NULL)
    OR
    (movement_kind = 'inventory_adjustment' AND direction = 'decrease' AND source_location_id IS NOT NULL AND destination_location_id IS NULL)
  )
```

Indexes:

```
idx_lot_movements_lot_created       ON lot_movements(expiry_lot_id, created_at DESC)
idx_lot_movements_source_location   ON lot_movements(source_location_id)     WHERE source_location_id IS NOT NULL
idx_lot_movements_dest_location     ON lot_movements(destination_location_id) WHERE destination_location_id IS NOT NULL
idx_lot_movements_kind              ON lot_movements(movement_kind)
```

The first index serves the Historial panel (newest-first per lot). The two
location indexes serve the per-location balance SUM query in §4.

### 1.2 Movement kinds and the kind ↔ source/destination contract

The CHECK constraint in §1.1 enforces the contract:

| Kind                       | source_location_id | destination_location_id | Note required |
|----------------------------|--------------------|-------------------------|---------------|
| `entry:initial`            | NULL               | set                     | n/a (system)  |
| `transfer`                 | set                | set (≠ source)          | no            |
| `exit:sale`                | set                | NULL                    | no            |
| `exit:waste`               | set                | NULL                    | no            |
| `exit:expired`             | set                | NULL                    | no            |
| `exit:damaged`             | set                | NULL                    | no            |
| `exit:internal_consumption`| set                | NULL                    | no            |
| `exit:return_to_supplier`  | set                | NULL                    | no            |
| `exit:inventory_adjustment`| set                | NULL                    | **yes**       |
| `exit:other`               | set                | NULL                    | **yes**       |
| `inventory_adjustment`     | follows `direction` | follows `direction`    | **yes**       |

The `inventory_adjustment` kind is a single kind shared by the *Ajustar conteo*
flow for both sign variants. The `direction` column encodes the sign
(`'increase'` for entry-shaped rows, `'decrease'` for exit-shaped rows) and the
CHECK constraint in §1.1 ties source/destination nullability to `direction`. It
is system-emitted by the count-correction flow only and never appears in the
manual picker. The schema adopts this shape because the proposal's open
questions round (decision 4, §Open questions) recorded the user-approved
reversal of the design's earlier two-kind proposal
(`exit:inventory_adjustment` + `entry:inventory_adjustment`) in favor of the
single unified kind; the spec is the authoritative source for vocabulary.

### 1.3 Schema invariants (enforced by the DB and the service)

1. **`expiry_lots.quantity >= 0` (post-V5).** V5 relaxes the existing
   `CHECK(quantity > 0)` to `CHECK(quantity >= 0)` so a fully-resolved lot
   can carry `quantity = 0`. The ledger enforces positive `quantity` on
   every movement; the relaxed constraint lets the denormalized lot total
   reach zero without the legacy "clamp to 1.0" workaround.
2. **Lot total = ledger sum.** After any ledger write, for each affected
   lot:
   `expiry_lots.quantity == SUM(quantity WHERE destination_location_id IS NOT NULL) - SUM(quantity WHERE source_location_id IS NOT NULL)`.
   This is the canonical invariant. The service enforces it by computing the
   delta inside the same transaction as the `INSERT INTO lot_movements` and
   applying it as `UPDATE expiry_lots SET quantity = quantity + delta,
   updated_at = now`.
3. **Append-only.** No `UPDATE` or `DELETE` is ever emitted against
   `lot_movements` by runtime paths. Backups and migrations are the only
   writers besides the runtime inserts.
4. **Transfer destination may be in another store.** The service rejects a
   transfer whose source or destination location is inactive (`is_active = 0`).
   The destination location MAY belong to a different store than the source
   location. `expiry_lots.store_id` is NOT updated by a transfer; the lot's
   store anchor remains its creation store, and per-location balances MAY
   include locations of other stores via the ledger. This is the user-ratified
   reversal of the proposal's earlier same-store-only non-goal.
5. **Source balance coverage.** The service validates
   `SUM(quantity WHERE destination_location_id = X AND expiry_lot_id = L)
   - SUM(quantity WHERE source_location_id = X AND expiry_lot_id = L)
   >= requested_qty` before any non-entry insert.
6. **Lot status transition.** When an exit reduces a lot's quantity to
   zero, the service sets `status = 'resolved'`,
   `resolution = '<last_exit_kind>'`, `resolved_at = now`, leaves
   `quantity = 0`. Reactivation requires a new `inventory_adjustment` row with
   `direction = 'increase'` (compensating event), never an `UPDATE expiry_lots`.
7. **Notes required.** `exit:other`, `exit:inventory_adjustment`, and
   `inventory_adjustment` (any direction) MUST carry non-blank `notes`. Service
   rejects blank with `DomainError::Validation`. For `inventory_adjustment`,
   the direction and the chosen source/destination location are mutually
   derived from `direction`; the service MUST also verify that the
   `direction` value is set and matches the chosen source/destination shape.
8. **actor = system.** No runtime path writes anything other than the literal
   string `system` into `actor` until a future change introduces auth.

### 1.4 New app settings keys

`app_settings` already exists (V2) and supports arbitrary key/value rows.
Two new keys land in V5:

| Key | Value | Default | Lifetime |
|---|---|---|---|
| `require_initial_location_on_lot_create` | `true` / `false` (stored as `0`/`1`) | `1` | Persistent until changed |

The settings page reads/writes via the existing `upsert_setting` /
`get_setting` repository functions. No new schema needed.

### 1.5 Sentinel location — *Sin ubicación*

Each store gets at most one sentinel location per migration. The sentinel:

- `name = 'Sin ubicación'` (Spanish label; matches proposal UX)
- `is_active = 1`
- `created_at`/`updated_at` = migration timestamp
- `store_id` = the store it belongs to
- ID is a deterministic UUID derived from the store id
  (e.g. `format!("loc-sentinel-{}", store_id)`) so re-runs are idempotent
  via `INSERT OR IGNORE`.

Created lazily:

- During V5 migration, for every store that has at least one
  `expiry_lots` row with `location_id IS NULL`, the migration ensures the
  sentinel exists for that store. Pre-existing lots with `NULL` get their
  `location_id` repointed to the sentinel in the same migration.
- During new lot creation, when `require_initial_location_on_lot_create`
  is off AND the user leaves the picker empty, the service ensures the
  sentinel exists for the chosen store and emits the initial movement
  with `destination_location_id = sentinel.id`. The lot row itself stores
  `location_id = sentinel.id` so the dashboard's `location_name` join
  resolves to "Sin ubicación".

The sentinel is treated as a regular active location everywhere (filters,
joins, dashboard). It is not hidden from pickers in v1 — adding a
"do not show this in the picker" affordance is out of scope for this
slice. The Spanish label "Sin ubicación" makes the intent obvious.

## 2. Migration V5

### 2.1 SQL plan (single migration block)

The V5 entry appended to the `MIGRATIONS` const in
`src-tauri/src/db/migrations.rs`. The block is structured in this order so
each step's invariants hold before the next runs:

```sql
-- 1. Drop the old quantity-positive CHECK on expiry_lots.
--    SQLite stores named CHECKs only when defined inline; the V2 schema
--    inlined CHECK(quantity > 0). SQLite cannot DROP a CHECK directly,
--    so the migration recreates the table without the CHECK constraint.
--    See §2.2 for the full recreate sequence (preserves data, FKs, indexes).

-- 2. Create lot_movements table + indexes (per §1.1).

-- 3. Ensure per-store sentinel locations exist.
INSERT OR IGNORE INTO store_locations (id, store_id, name, notes,
                                       is_active, created_at, updated_at)
SELECT 'loc-sentinel-' || s.id, s.id, 'Sin ubicación', NULL, 1,
       CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
FROM stores s
WHERE EXISTS (
  SELECT 1 FROM expiry_lots el
  WHERE el.store_id = s.id AND el.location_id IS NULL
);

-- 4. Repoint pre-existing lots with NULL location_id to the sentinel.
UPDATE expiry_lots
SET location_id = 'loc-sentinel-' || store_id,
    updated_at  = CURRENT_TIMESTAMP
WHERE location_id IS NULL;

-- 5. Back-fill entry:initial for every pre-existing lot.
INSERT INTO lot_movements (
  id, expiry_lot_id, movement_kind, quantity,
  source_location_id, destination_location_id,
  reason, notes, actor, created_at
)
SELECT
  'mvmt-init-' || el.id, el.id, 'entry:initial', el.quantity,
  NULL, el.location_id,
  NULL, NULL, 'system', el.created_at
FROM expiry_lots el
WHERE NOT EXISTS (
  SELECT 1 FROM lot_movements lm
  WHERE lm.expiry_lot_id = el.id AND lm.movement_kind = 'entry:initial'
);

-- 6. Migrate lot_resolution_events into exit movements.
--    See §2.3 for the lookup table and full SQL.

-- 7. Reset quantity on legacy-resolved lots (where the existing
--    'clamp to 1.0' workaround hid the truth). For any lot whose
--    current quantity + sum of legacy resolution events does not match
--    the original quantity, set quantity = 0 to reflect resolved state.
--    See §2.4 for the reconciliation logic.
```

### 2.2 Recreating `expiry_lots` without the old CHECK

SQLite (≤ 3.35) cannot `ALTER TABLE ... DROP CONSTRAINT`. Two options:

1. **12-step recreate** (V4 precedent, V3 used a similar pattern).
   - `BEGIN;`
   - `PRAGMA foreign_keys = OFF;`
   - Create `expiry_lots_new` with the same column set minus the
     `CHECK(quantity > 0)` and plus `CHECK(quantity >= 0)`.
   - `INSERT INTO expiry_lots_new SELECT ... FROM expiry_lots;`
   - `DROP TABLE expiry_lots;`
   - `ALTER TABLE expiry_lots_new RENAME TO expiry_lots;`
   - Recreate indexes.
   - `PRAGMA foreign_keys = ON;`
   - `COMMIT;`
2. **Skip the CHECK entirely** and rely on the application-level guards
   plus the new ledger. This is the cleaner path: the ledger already
   enforces `quantity > 0` on every movement, and the service enforces
   the `quantity >= 0` invariant on lot totals. SQLite will let us add a
   table-level CHECK on a new table, not retroactively on an existing one.

The chosen path is **option 1, recreate with `CHECK(quantity >= 0)`**.
Reasons: (a) symmetry with V4's recreate pattern, (b) defense in depth at
the DB layer, (c) the 12-step procedure is already exercised by V4 tests
in the same file and we can mirror that coverage.

The recreate also preserves the legacy `resolution` and `resolved_at`
columns. After V5 lands, those columns still serve legacy-resolved lots
(and are populated by the new service path on first full resolution via the
ledger); see §1.3 invariant 6.

### 2.3 Migrating `lot_resolution_events`

The legacy rows map to v1 exit kinds via a case-insensitive lookup:

| Legacy `resolution` text | New kind | Notes |
|---|---|---|
| `consumed` | `exit:internal_consumption` | — |
| `sold` | `exit:sale` | — |
| `discarded` | `exit:waste` | — |
| `donated` | `exit:other` | `notes` carries `"legacy: donated"` |
| `transferred` | `exit:other` | `notes` carries `"legacy: transferred (destination unknown)"` |
| `other` | `exit:other` | — |
| anything else | `exit:other` | `notes` carries `"legacy: <original text>"` |

For every migrated row, the new movement:

- `quantity` = legacy `quantity`
- `source_location_id` = `expiry_lots.location_id` at migration time
  (which is now the sentinel if it was originally NULL)
- `destination_location_id` = NULL
- `created_at` = legacy `created_at` (preserves chronology)
- `actor` = `'system'`
- `id` = deterministic `format!("mvmt-legacy-{}", legacy_event_id)` so the
  migration is idempotent (`INSERT OR IGNORE` would also work; the
  deterministic id keeps re-runs observable)

Idempotency guard at the top:

```sql
INSERT INTO lot_movements (...)
SELECT ... FROM lot_resolution_events lre
WHERE NOT EXISTS (
  SELECT 1 FROM lot_movements lm
  WHERE lm.id = 'mvmt-legacy-' || lre.id
);
```

The legacy table stays in place. No `DROP TABLE lot_resolution_events` in
V5. A future change retires it after one release cycle.

### 2.4 Reconciling legacy "clamp to 1.0" lots

The existing `apply_partial_resolution` clamps `quantity` to `1.0` when
remaining reaches zero. After V5 we want `quantity = 0` to mean fully
resolved. The migration recalculates every lot's `quantity` as:

```
quantity = SUM(lm.quantity WHERE destination IS NOT NULL AND lm.expiry_lot_id = lot.id)
         - SUM(lm.quantity WHERE source IS NOT NULL      AND lm.expiry_lot_id = lot.id)
```

For lots that were resolved under the legacy clamp (quantity = 1.0, status
= 'resolved'), the back-fill sum equals the original pre-resolution
quantity minus the resolution events' quantities — which is exactly zero.
The migration sets them to `0`. For lots that still have remaining
quantity, the sum matches `expiry_lots.quantity` exactly because step 5
inserted the initial-entry with the lot's full quantity. Reconciliation is
a no-op for active lots.

The reconcile SQL:

```sql
UPDATE expiry_lots
SET quantity = COALESCE((
  SELECT SUM(CASE WHEN destination_location_id IS NOT NULL THEN lm.quantity
                  WHEN source_location_id      IS NOT NULL THEN -lm.quantity
                  ELSE 0 END)
  FROM lot_movements lm WHERE lm.expiry_lot_id = expiry_lots.id
), 0),
updated_at = CURRENT_TIMESTAMP;
```

This runs after steps 5 and 6, so it sees all back-filled movements.

### 2.5 Idempotency

V5 is safe to re-run on a partially-applied V5 database (manual testing,
interrupted restore, etc.) via:

- Deterministic IDs for sentinels and back-fill rows.
- `INSERT OR IGNORE` for the sentinel insert.
- `WHERE NOT EXISTS` guards on every INSERT INTO `lot_movements`.
- The reconcile UPDATE is a no-op when invariants already hold.

The standard pattern (`v4_backfill_is_idempotent`, `v3_migration_is_idempotent`)
in `migrations.rs` tests is mirrored for V5 with two new tests:
`v5_backfill_is_idempotent` and `v5_legacy_resolution_migration_is_idempotent`.

## 3. Derived balances and stock model

### 3.1 Per-location balance query

For a single lot, the per-location breakdown is computed in SQL:

```sql
SELECT location_id, SUM(direction) AS balance
FROM (
  SELECT destination_location_id AS location_id, quantity AS direction
  FROM lot_movements
  WHERE expiry_lot_id = ?1 AND destination_location_id IS NOT NULL
  UNION ALL
  SELECT source_location_id, -quantity
  FROM lot_movements
  WHERE expiry_lot_id = ?1 AND source_location_id IS NOT NULL
) v
GROUP BY location_id
HAVING balance > 0;          -- exclude fully-departed locations
```

Returned as `Vec<(location_id, balance)>`. The repository function
`list_location_balances_for_lot(pool, lot_id)` wraps this.

### 3.2 Transfer as a single row

A transfer moves `q` units from `src` to `dst`. The service writes ONE
`lot_movements` row with `source_location_id = src`, `destination_location_id = dst`,
`quantity = q`. The per-location balance query reads it as `q` out of `src`
and `q` into `dst`, preserving the lot total (`q - q = 0` net change).
This matches the E4 research evidence (paired effect, two inventory
transactions but a single user-visible journal line).

### 3.3 Lot total reconciliation

The service maintains `expiry_lots.quantity` as the canonical total
(per proposal Option A). Every ledger write computes the delta (positive
for entries, negative for exits) and applies it in the same transaction.
For a `transfer`, the delta is zero and the lot total stays.

Pseudocode (executed inside a transaction):

```text
    fn insert_movement(pool, movement):
        match movement.kind:
            entry:initial         → delta = +movement.quantity
            transfer              → delta = 0
            exit:*                → delta = -movement.quantity
            inventory_adjustment  → delta = match movement.direction:
                                     Direction::Increase → +movement.quantity
                                     Direction::Decrease → -movement.quantity

    INSERT INTO lot_movements (...)

    UPDATE expiry_lots
       SET quantity = quantity + delta,
           updated_at = now,
           status = CASE
             WHEN quantity + delta = 0 THEN 'resolved'
             WHEN quantity + delta > 0 AND status = 'resolved' THEN 'active'
             ELSE status
           END,
           resolution  = CASE WHEN quantity + delta = 0 THEN movement.kind ELSE resolution END,
           resolved_at = CASE WHEN quantity + delta = 0 THEN now ELSE resolved_at END
     WHERE id = movement.lot_id
```

### 3.4 Auto-resolution semantics

When an exit reduces `quantity` to exactly zero:

- `status = 'resolved'`
- `resolution = '<exit kind>'` (e.g., `exit:sale`)
- `resolved_at = now`
- `quantity = 0` (no clamp)

When a subsequent `inventory_adjustment` with `direction = 'increase'`
(compensating count correction) brings the total above zero, status returns to
`active`, `resolution = NULL`, `resolved_at = NULL`. The ledger preserves the
full audit trail.

### 3.5 Reactivation of a resolved lot

A resolved lot (`status = 'resolved'`, `quantity = 0`) can be reactivated
only via:

- An `inventory_adjustment` row with `direction = 'increase'` and the
  required note (compensating count correction), or
- A new `entry:initial` (not allowed — a lot has at most one initial
  entry; this is rejected at the service layer).

The service path: any insert that increases the lot total past zero
flips status back to active. This is rare and intentional (someone
discards expired stock, then realizes they had a stock-take error); the
ledger makes it recoverable.

## 4. Auto batch code generation

### 4.1 Format and prefix derivation

Format: `PREFIX-YYYYMMDD-NNN` (12 chars + 1 separator + 3 digits when
`NNN <= 999`; longer with the `-M` collision suffix when NNN > 999).

Prefix derivation:

```
fn derive_prefix(product: &ProductResponse) -> String {
    if let Some(sku) = &product.sku {
        let trimmed = sku.trim();
        // Take alphanumeric chars only, uppercase, first 3..=6 chars.
        let alnum: String = trimmed
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_uppercase())
            .take(6)
            .collect();
        if alnum.len() >= 3 { return alnum; }
        // Pad with X if SKU is too short (e.g. "AB" → "ABX").
        let mut padded = alnum;
        while padded.len() < 3 { padded.push('X'); }
        return padded;
    }
    "LOT".to_string()
}
```

`YYYYMMDD` is the **local** creation date, formatted with
`chrono::Local::now().format("%Y%m%d")`. UTC is intentionally not used —
the user reads batches in their own timezone.

### 4.2 Counter and collision resolution

The counter is per-day-per-prefix. Lookup:

```sql
SELECT batch_code
FROM expiry_lots
WHERE batch_code LIKE ?1 || '-%'
  AND substr(batch_code, length(?1) + 10, 8) = ?2   -- matches YYYYMMDD
ORDER BY batch_code DESC
LIMIT 1;
```

The service extracts `NNN` from the highest matching row. If none, start
at `001`. If the candidate `PREFIX-YYYYMMDD-NNN` is taken (only possible
if the user manually typed a generated-style code), try `NNN+1`,
`NNN+2`, etc. If `NNN > 999`, fall back to `PREFIX-YYYYMMDD-NNN-M`
where `M = 2, 3, ...` until a free slot is found.

The lookup happens inside the same transaction as the lot insert, so two
concurrent creates with the same prefix and date both succeed (one takes
`001`, the other takes `002`). SQLite's serialised writer avoids the race
within a single process.

### 4.3 Manual batch preservation

The blank-only invariant is enforced in the service:

```text
fn generate_or_preserve(input: ExpiryLotCreate) -> String {
    if let Some(raw) = &input.batch_code {
        let trimmed = raw.trim();
        if !trimmed.is_empty() { return trimmed.to_string(); }
    }
    generate_auto(input)
}
```

The generator never sees a non-blank value. The frontend never sends a
non-blank value when the user typed something; the JS guard in `LotForm`
matches. This is a two-layer defense; one test covers each layer:

- Backend: `auto_batch_blank_only_enforced_when_input_blank` (service
  test).
- Frontend: manual smoke per the canonical verify gate.

### 4.4 Pre-existing lots without a batch_code

Per the proposal, no back-fill of `batch_code` for pre-existing lots.
Their audit trail is the migration event itself. The Historial tab shows
their initial-entry with `notes = "Migrated from pre-V5 database"` to
make the provenance visible (optional, design-level decision; deferred
to `tasks.md` for the exact wording).

## 5. Application settings menu

### 5.1 Storage

Reuse the existing `app_settings` table and the
`upsert_setting(pool, key, value)` / `get_setting(pool, key)` repository
functions. The new key is
`require_initial_location_on_lot_create` with value `'1'` (true) or
`'0'` (false).

The `SettingsResponse` DTO in `dto/stores.rs` gains one field:

```rust
pub struct SettingsResponse {
    pub last_selected_store_id: Option<String>,
    pub require_initial_location_on_lot_create: bool,
}
```

A new DTO module `dto/lot_movements.rs` (or extension of
`dto/stores.rs`) carries the request shape for updates; the design
favours extending `SettingsUpdate` with the new bool field, mirroring
the existing `last_selected_store_id` pattern.

### 5.2 UI shape — *Configuración*

A new top-level menu entry **Configuración** in the existing navigation
(alongside Dashboard / Calendar / Products / Reports / CSV / Stores /
Backup). The page renders a single section "Lotes" with one toggle row:

```
┌─────────────────────────────────────────────────────────────┐
│ Configuración                                               │
├─────────────────────────────────────────────────────────────┤
│ Lotes                                                       │
│                                                             │
│ Ubicación inicial obligatoria al crear lote        [●━━○]  │
│ Cuando está activado, el formulario de lote no permite      │
│ continuar sin elegir una ubicación interna.                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

The page is designed to host additional setting rows under additional
section headings (e.g. "Notificaciones", "Respaldo") without a layout
change. Each row binds to a boolean DTO field. Save semantics:
auto-save on toggle (no separate "Save" button). The toggle reads the
current value on mount, flips it on click, calls
`update_settings({...})`, and rolls back on failure.

### 5.3 Default and behaviour

Default: `require_initial_location_on_lot_create = true`.

When ON and the user submits the LotForm with no location selected, the
form shows an inline error "Selecciona una ubicación" and blocks
submission. When OFF, the form allows submission with no location; the
service resolves to the per-store sentinel (§1.5).

The setting affects lot creation validation only. It does not rewrite
existing lots, does not affect transfers, and does not affect exits
(those always require a source location).

## 6. UI architecture

### 6.1 Lot detail — *Historial* tab

The existing lot detail modal (rendered from the Dashboard's row
`👁` action or from the lot edit flow) gains a third surface: the
**Historial** tab, alongside the existing detail/edit views. The tab
fetches two payloads in parallel on mount:

1. The lot response (`get_expiry_lot`) for the header (quantity,
   expiry, batch, store).
2. The movement list (`list_lot_movements(lot_id)`) plus the
   per-location balance summary (`get_lot_location_balances(lot_id)`).

Header layout:

```
┌──────────────────────────────────────────────────────┐
│ Yogurt Frutilla — Lote #abc12345                    │
├──────────────────────────────────────────────────────┤
│ Cantidad restante:  7 L                              │
│ Vence:             15/10/2026                        │
│ Lote:              SKU-A-20260115-001                │
├──────────────────────────────────────────────────────┤
│ Por ubicación:                                       │
│   Bodega         3 L                                 │
│   Exhibición     4 L                                 │
├──────────────────────────────────────────────────────┤
│  [Mover stock]  [Registrar salida]  [Ajustar conteo]│
├──────────────────────────────────────────────────────┤
│ Historial (más reciente primero)                     │
│ ─────────────────────────────────────────────────── │
│ 14/10/2026 14:32  Salida · Venta          1 L  Bodega│
│ 13/10/2026 09:11  Traslado        2 L  Exhib→Bodega   │
│ 12/10/2026 18:05  Entrada inicial        10 L  Bodega│
└──────────────────────────────────────────────────────┘
```

Movements render with kind label (Spanish), reason when applicable,
source/destination arrows, quantity, and notes when present. Newest
first. No pagination for v1 (a single lot rarely exceeds a few dozen
movements).

### 6.2 *Mover stock* — transfer flow

Modal form with three fields:

- **Origen** (select, locations of the lot's store; defaults to the
  location with the highest current balance).
- **Destino** (select, locations of the lot's store, excludes origin).
- **Cantidad** (number; min 0.01 / 1, drives off the lot's unit kind).

Submit handler calls `create_lot_movement({ kind: 'transfer', ... })`.
On success, the modal closes and the Historial panel reloads.

Validation handled server-side; the form displays the server's
`Validation` error message inline if rejected.

### 6.3 *Registrar salida* — exit flow

Modal form with:

- **Origen** (select; defaults to the highest-balance location).
- **Motivo** (select from the eight exit kinds; *Ajuste de inventario*
  and *Otro* trigger an extra required *Notas* textarea).
- **Cantidad** (number).
- **Notas** (textarea; required when *Motivo* is *Ajuste de inventario*
  or *Otro*; optional otherwise).

Submit handler calls
`create_lot_movement({ kind: 'exit:<motivo>', source_location_id, quantity, notes })`.
On success: close, refresh dashboard + historial.

### 6.4 *Ajustar conteo* — count correction flow

Modal form with:

- **Ubicación** (select; locations of the lot's store, including locations of
  other stores when cross-store transfers are allowed — see §1.3 invariant 4).
- **Cantidad real** (number; min 0; the physical count).
- **Notas** (textarea; required).

Submit handler computes the signed delta on the client and routes to a single
backend call:

- `create_lot_movement({ kind: 'inventory_adjustment', direction: 'increase', destination_location_id, quantity: |delta|, notes })` if `delta > 0`
- `create_lot_movement({ kind: 'inventory_adjustment', direction: 'decrease', source_location_id,      quantity: |delta|, notes })` if `delta < 0`
- No-op if `delta == 0` (the system already matches)

The single wire shape matches the spec's unified-kind decision; no two-kind
split is introduced on the FE. The modal fetches the lot's current balance
before submitting so the direction is unambiguous. On success: close, refresh
dashboard + historial.

### 6.5 LotForm integration

Two changes to the existing `LotForm.svelte`:

1. **Settings-aware validation.** On mount, fetch the
   `require_initial_location_on_lot_create` value. When true, add a
   JS guard `if (!selectedLocationId) errorMsg = "Selecciona una
   ubicación"`.
2. **Generated batch echo-back.** The current form keeps the typed
   `batchCode` in its local state. After a successful create, the
   server returns the lot with the final `batch_code` (manual or
   auto). The form displays a small chip near the batch input:
   "Lote generado: `<code>`" when the manual input was blank and the
   server returned a generated value. The chip persists on screen
   until the form closes.

The LotForm itself stays focused on creation; the new flows
(Mover/Salida/Ajustar) live in the Historial tab.

## 7. Backend boundaries

### 7.1 Domain (pure functions, no I/O)

New module: `src-tauri/src/domain/lot_movements.rs`. Pure helpers:

- `compute_movement_delta(kind: &MovementKind) -> i32` — returns +1, 0,
  or −1 (multiplied by `quantity` at the service layer).
- `validate_movement_kind(kind: &str) -> Option<MovementKind>` — typed
  parse, returns `None` for unknown.
- `location_balance_direction(movement: &MovementRow) -> (Option<&str>, f64)`
  — returns `(location_id, signed_delta)` for the per-location balance
  accumulator.
- `derive_batch_prefix(sku: Option<&str>) -> String` — per §4.1.
- `extract_batch_nnn(batch_code: &str, prefix: &str, date: &str) -> Option<u32>`.
- `next_batch_candidate(prefix: &str, date: &str, last_nnn: u32) -> String`.
- `legacy_resolution_to_kind(text: &str) -> MovementKind` — maps the
  proposal's lookup table for migration.
- `notes_required(kind: &MovementKind) -> bool` — true for `exit:other`,
  `exit:inventory_adjustment`, and `inventory_adjustment` (any direction).

All functions are unit-testable in isolation (no pool required).

### 7.2 Service

New module: `src-tauri/src/services/lot_movements.rs`. Public API:

- `create_lot_movement(pool, input) -> Result<LotMovementResponse>`
  — validates, computes delta, inserts the row + updates `expiry_lots`,
  all in one transaction. Returns the new movement.
- `list_lot_movements(pool, lot_id) -> Result<Vec<LotMovementResponse>>`
  — newest-first by `created_at DESC`.
- `get_lot_location_balances(pool, lot_id)
  -> Result<Vec<LotLocationBalance>>` — runs the §3.1 SQL.
- `migrate_legacy_resolution_events(pool) -> Result<MigrationStats>` —
  called from V5 migration only; not exposed as a command.
- `derive_auto_batch_code(pool, product_id) -> Result<String>` — used by
  `services::expiry_lots::create_expiry_lot` when input is blank.
- `validate_direction(kind: &MovementKind, direction: &Option<Direction>)`
  — `inventory_adjustment` requires a `Direction` value; every other kind
  must carry `None`. Returns `Result<(), DomainError>`.

Modified: `services/expiry_lots.rs::create_expiry_lot` gains a call to
`derive_auto_batch_code` (when input is blank) and a call to
`lot_movements::create_lot_movement` (system-emitted `entry:initial`)
inside the same transaction. The function signature and wire shape
remain unchanged (the response already carries the lot row).

### 7.3 Repository

New module: `src-tauri/src/db/repositories/lot_movements.rs`. Public API:

- `insert_movement(pool, movement: &NewMovement) -> Result<LotMovementRow>`
- `list_movements_by_lot(pool, lot_id: &str) -> Result<Vec<LotMovementRow>>`
- `location_balances_for_lot(pool, lot_id: &str) -> Result<Vec<(String, f64)>>`
- `find_max_nnn_for_prefix_on_date(pool, prefix: &str, date: &str)
   -> Result<Option<u32>>` — used by the batch generator.
- `has_initial_entry(pool, lot_id: &str) -> Result<bool>` — used by V5
  migration's idempotency guard.
- `list_unmigrated_legacy_events(pool) -> Result<Vec<LegacyRow>>` — V5 use.

No new transaction abstraction is introduced; the service uses
`pool.begin()` directly when it needs atomicity across the ledger insert
and the lot total update.

### 7.4 Tauri commands

New module: `src-tauri/src/commands/lot_movements.rs`. Commands:

- `create_lot_movement(input: LotMovementCreate) -> LotMovementResponse`
- `list_lot_movements(lot_id: String) -> Vec<LotMovementResponse>`
- `get_lot_location_balances(lot_id: String)
   -> Vec<LotLocationBalanceResponse>`

The `SettingsUpdate` DTO and the existing `update_settings` command gain
one optional bool field; the command shape stays. The `SettingsResponse`
DTO gains one bool field; existing consumers ignore unknown fields but
the FE will read it.

The legacy `commands::expiry_lots::resolve_expiry_lot` and
`list_lot_resolution_events` remain available but the new UI surfaces
use the ledger commands. They are not deleted in V5.

### 7.5 DTOs

New module: `src-tauri/src/dto/lot_movements.rs`:

- `MovementKind` (enum, derives `Serialize`/`Deserialize`/`sqlx::FromRow`
  via a `#[serde(rename_all = "snake_case")]` and a `#[sqlx(type_name =
  "TEXT")]`).
- `LotMovementCreate` — input DTO with `kind`, `lot_id`, `quantity`,
  optional `source_location_id`, optional `destination_location_id`,
  optional `notes`.
- `LotMovementResponse` — full row.
- `LotLocationBalanceResponse` — `{ location_id, location_name?,
  quantity }`.

The `SettingsResponse` and `SettingsUpdate` DTOs in `dto/stores.rs`
gain `require_initial_location_on_lot_create: bool`.

## 8. Compatibility surfaces

### 8.1 Dashboard

The existing `list_dashboard_lots` query (single row per lot) is
unchanged. The dashboard does NOT break out one row per `(lot,
location)` in v1; the per-location breakdown lives in the Historial tab
behind the existing `👁` action.

The dashboard's `location_name` join continues to use
`expiry_lots.location_id`. After V5, that column reflects the
initial-entry destination (possibly the sentinel). The dashboard's
single "Bodega 10, Exhibición 20" breakdown is replaced by the
Historial tab's per-location summary; no new column on the dashboard
table.

The new "Configuración" entry slots into the existing navigation. No
other navigation changes.

### 8.2 Reports

The existing reports surface (`reports.rs`, `ReportsPage.svelte`) is
unchanged in v1. The proposal's "Movimientos por motivo" report is
deferred to a follow-up. The CSV export shape adds no new columns for
v1; the `batch_code` column continues to carry whatever is stored on
the lot, whether manual or auto-generated.

The report filter for `category_ids` and the new movement reason filter
are not coupled in v1 — the reports service does not query
`lot_movements` yet.

### 8.3 CSV import/export

The product CSV import path is untouched. The lot CSV export path
(handled inside `csv_io::export_report_csv` for now) gains no new
columns; `batch_code` continues to be the per-lot string.

A future slice can add a "Movements by reason" CSV export that joins
`lot_movements` grouped by `movement_kind`. Out of scope for v1.

### 8.4 Backup and restore

The V4 precedent (`restore_from_pre_v4_backup_applies_v4_backfill_in_situ`)
is mirrored: `restore_from_pre_v5_backup_applies_v5_backfill_in_situ`.

The test creates a pre-V5 database (V4 schema with sample lots and
`lot_resolution_events` rows), runs the V5 migration on it, and asserts:

- `lot_movements` table exists with the expected columns and CHECKs.
- Every pre-existing `expiry_lots` row has exactly one `entry:initial`
  movement in `lot_movements`.
- Every pre-existing `lot_resolution_events` row has a corresponding
  `exit:*` movement (id starts with `mvmt-legacy-`).
- `expiry_lots.quantity` for each lot matches the ledger-derived total.
- The legacy `lot_resolution_events` table is still present.
- `require_initial_location_on_lot_create` defaults to `'1'`.

The backup export (`export_backup`) does not need changes — it dumps
the SQLite file as-is and V5 applies on restore.

## 9. Test strategy

### 9.1 Domain unit tests (`src-tauri/src/domain/lot_movements.rs`)

Pure functions, no pool. Cases:

- `compute_movement_delta` for every kind (10 entries × 1 result).
- `validate_movement_kind` for valid kinds, unknown kinds, trailing
  whitespace.
- `derive_batch_prefix` for SKUs of length 0/2/3/5/10/uppercase
  /mixed-case / non-alphanumeric (`SKU-001` → `SKU001`; `AB` → `ABX`;
  ` ` → `LOT`).
- `extract_batch_nnn` for `SKU-A-20260115-001`, `-002-2`,
  `SKU-A-20260115-001` (collision suffix), malformed.
- `legacy_resolution_to_kind` for all 6 known legacy values and 3
  unknown values.
- `notes_required` for every kind.

### 9.2 Service / repo integration tests

Using `fresh_test_pool()`. Cases:

- `create_lot_movement_emits_initial_entry_on_lot_creation`
- `create_lot_movement_transfer_preserves_lot_total`
- `create_lot_movement_exit_reduces_lot_total_and_resolves_on_zero`
- `create_lot_movement_exit_rejected_when_source_balance_insufficient`
- `create_lot_movement_exit_rejected_when_notes_blank_for_inventory_adjustment`
- `create_lot_movement_transfer_rejected_for_cross_store`
- `create_lot_movement_count_adjustment_increase_writes_inventory_adjustment_with_direction_increase`
- `create_lot_movement_count_adjustment_decrease_writes_inventory_adjustment_with_direction_decrease`
- `create_lot_movement_count_adjustment_zero_delta_writes_no_row`
- `create_lot_movement_inventory_adjustment_rejected_when_direction_missing`
- `create_lot_movement_inventory_adjustment_rejected_when_direction_mismatches_source_destination_shape`
- `create_lot_movement_inventory_adjustment_blank_note_rejected`
- `create_lot_movement_transfer_accepts_cross_store_destination`
- `list_movements_by_lot_orders_newest_first`
- `location_balances_for_lot_matches_ledger_sum`
- `auto_batch_blank_input_generates_prefix_date_nnn`
- `auto_batch_collision_appends_dash_two_three`
- `auto_batch_manual_input_preserved_verbatim`
- `lot_total_invariant_holds_after_random_sequence_of_movements`
  (fuzz: 50 random sequences of mixed movements, including
  `inventory_adjustment` rows with both directions, assert
  `SUM(lm WHERE dest IS NOT NULL) - SUM(lm WHERE source IS NOT NULL)
  == expiry_lots.quantity` after each)
- `inventory_adjustment_direction_sign_matches_per_location_balance`
  (asserts the §1.1 CHECK binds `direction` to source/destination).

### 9.3 Migration safety

In `src-tauri/src/db/migrations.rs::tests`:

- `v5_applies_on_fresh_db` — applied_count becomes 5.
- `v5_migration_is_idempotent` — re-running adds zero rows.
- `v5_backfill_creates_entry_initial_for_every_pre_existing_lot`
- `v5_backfill_creates_sentinel_locations_for_stores_with_null_lot_locations`
- `v5_repoints_null_lot_locations_to_sentinel`
- `v5_migrates_each_lot_resolution_event_to_exit_movement`
- `v5_legacy_resolution_migration_uses_otro_with_note_for_unknown_values`
- `v5_relaxes_expiry_lots_quantity_check_to_zero_or_more`
- `v5_reconcile_sets_resolved_lot_quantity_to_zero`

In `src-tauri/src/services/backup_restore.rs::tests`:

- `restore_from_pre_v5_backup_applies_v5_backfill_in_situ` — V4
  precedent, mirrored for V5.

### 9.4 Manual smoke (no FE test harness)

Linux + Windows. Steps:

1. Create a store and two locations (Bodega, Exhibición).
2. Create a lot with a manual batch; verify the batch is preserved.
3. Create a lot with no batch; verify the auto-generated
   `PREFIX-YYYYMMDD-NNN` is echoed back.
4. Open Historial; verify the `entry:initial` row is shown.
5. Move 5 units from Bodega to Exhibición; verify per-location
   balances update.
6. Register a `Venta` exit; verify lot total and balance update.
7. Register a `Vencido` exit on an expired lot; verify the lot
   transitions to `resolved` with `quantity = 0`.
8. Use `Ajustar conteo` to increase stock by 1; verify status returns
   to `active`.
9. Toggle `Ubicación inicial obligatoria` OFF; create a lot with no
   location; verify the sentinel is used and the lot displays
   "Sin ubicación".
10. Export a backup, restore on a fresh database; verify all
    movements survive.
11. Open a pre-V5 backup (a fixture file in `tests/fixtures/`); verify
    V5 applies and back-fills initial entries.

## 10. Phasing and review workload forecast

### 10.1 Phase breakdown

Five phases from the proposal, each forecast against the 400-line
budget. Counts are estimates of **net additions + deletions** in the
final diff for the change as a whole.

| Phase | Scope | Forecast lines | Verdict |
|---|---|---|---|
| **Phase 1 — Backend ledger skeleton** | V5 migration SQL, `lot_movements` table + indexes, `domain/lot_movements.rs`, `repositories/lot_movements.rs`, `services/lot_movements.rs` (insert/list/balances), reconcile SQL, `migrations.rs` tests, `backup_restore.rs` pre-V5 test. No FE changes. | **~430** | **At budget ceiling** |
| **Phase 2 — Lot creation integration** | `services/expiry_lots.rs` change to emit initial entry + auto-batch in the same transaction; `dto/stores.rs` settings extension; `services/settings.rs` reads new key; LotForm settings-aware JS guard + batch echo chip; LotForm hint text. | ~190 | OK |
| **Phase 3 — Movements UI** | `LotMovementsPanel.svelte`, `MoveStockModal.svelte`, `RegisterExitModal.svelte`, `AdjustCountModal.svelte`, `lot_movements.ts` lib wrappers, Historial tab integration into the existing lot detail modal, per-location balance renderer. | ~280 | OK |
| **Phase 4 — Settings menu** | `ConfigurationPage.svelte`, navigation entry, settings DTO extension wire to FE, auto-save toggle wiring, manual smoke for the toggle behaviour. | ~90 | OK |
| **Phase 5 (deferred / optional)** | Recent-activity widget, bulk movement entry, print-friendly ledger. | — | Not in v1 |

Phase 1 alone is at the ceiling. The 400-line budget is the
**session override**; the project canonical is 800. Forecasts above
trigger the `deliveryStrategy: ask-on-risk` round.

### 10.2 Recommended slicing if Phase 1 exceeds 400

If Phase 1's actual diff lands above 400 (likely, given the V5
migration is dense), two slicing paths:

**Path A — Split Phase 1 by layer:**

- Phase 1a — Schema only: V5 migration SQL + reconcile + migration
  tests + backup-restore test. ~220 lines. Landed as a single PR with
  no runtime behavior change (the new table is created, sentinels
  inserted, back-fills applied, but no code path reads/writes the new
  table yet).
- Phase 1b — Domain + repo + service: domain functions, repository
  functions, service functions, no Tauri commands yet. ~180 lines.
  Tested via direct service calls; commands arrive in Phase 3.

**Path B — Single PR but explicit ask-on-risk:**

Pause before `tasks.md`, ask the user to ratify the chain strategy or
accept a single 600–800 line PR against the canonical budget. Given
that V5 + reconcile + tests are tightly coupled (you can't land the
service without the schema; you can't land the tests without the
schema and the service), splitting purely for review focus is harder
than splitting purely for size.

The design recommends **Path A** as the default, with Path B as the
fallback if the user prefers single-PR simplicity.

### 10.3 Chained-PR strategy

`chainStrategy: deferred` per the proposal. Once the parent selects
Phases, the chain config lands in `tasks.md` along with the
`deliveryStrategy: ask-on-risk` decision round.

### 10.4 Risks the parent should be aware of

1. ~~**The `entry:inventory_adjustment` vocabulary addition (§1.2)**
   needs an explicit parent confirmation.**~~ **CLOSED by the spec
   (`specs/caduxo-expiry-tracker/spec.md`, requirement `count adjustment with
   directional sign`):** the unified `inventory_adjustment` kind with a
   `direction` column is the authoritative shape (proposal decision 4). No
   confirmation gate remains.

2. **The `expiry_lots.quantity` CHECK relaxation (§1.3 invariant 1,
   §2.2)** is a backward-incompatible schema change. The recreate is
   straightforward but means V5 cannot be reverted without a manual
   data-fix script (no `down` migration in SQLite). Mitigated by V5's
   idempotency and the backup-restore test.

3. **Phase 1's line count** is likely to exceed 400. Path A in §10.2
   is the recommended mitigation; the parent should confirm before
   `tasks.md` locks the slices.

4. **Frontend has no test harness** per `strictTdd: false`. The
   *Historial* tab, modals, and toggle rely on manual smoke. The
   canonical spec's `tests accompany implementation` clause carries
   the burden.

5. **Lot detail modal complexity.** Adding a Historial tab inside an
   existing modal grows the modal significantly. Consider promoting
   the lot detail to a dedicated page if the modal grows past
   ~600 lines. Defer to `tasks.md` for the exact cut.

6. **Auto-batch readability vs. uniqueness.** A purely date-based
   `NNN` is human-readable but caps at 999 per day per prefix. The
   design's `-M` collision suffix handles overflow but inflates
   readability. If users routinely create > 999 lots per day per
   prefix, the format may need a wider counter (e.g. `NNNN`). No data
   currently suggests this is a problem; flagged for a future review.

## 11. Open items deferred to the next phase

- ~~`Ajustar conteo` for positive deltas — confirmed design extension
  (`entry:inventory_adjustment`) requires parent confirmation.~~ **CLOSED by
  the spec** — the unified `inventory_adjustment` kind with `direction` is the
  authoritative shape; `entry:inventory_adjustment` no longer exists.
- **Phase 1 slicing** — Path A (1a + 1b) vs. Path B (single PR).
  Decision lands in the `deliveryStrategy: ask-on-risk` round before
  `tasks.md`.
- **Lot detail modal vs. page** — UX decision for where the Historial
  tab lives. Design assumes modal extension; page promotion is an
  alternative.
- **Sentinel display in location pickers** — whether the sentinel
  shows up with a special marker (e.g., `(por defecto)`) in the
  LotForm picker. Out of scope for v1 if the Spanish label is enough.
- **Notification preferences in *Configuración*** — the settings page
  is designed to host additional toggles (notifications, backup
  cadence). No additional settings land in v1.
- **Cross-store transfer UX signals** — when a transfer's destination is in
  a different store, the Historial row already shows both location names; no
  extra UI affordance (e.g., a "Cross-store" badge) lands in v1.

## Key Learnings

- Caduxo's backend follows a strict layer split: domain (pure) →
  service (orchestration + transactions) → repository (SQL) → Tauri
  command (IPC). The new ledger must slot into this stack without
  inventing a new layer or crossing boundaries.
- The existing `apply_partial_resolution` "clamp to 1.0" workaround
  exists because `expiry_lots.quantity` carries a `CHECK(quantity > 0)`.
  V5's ledger-as-truth model lets us relax the check to `>= 0` and
  retire the clamp.
- The `lot_movements` table's CHECK constraint that ties `movement_kind`
  to source/destination nullability is the single source of truth for
  the kind contract; the service layer validates the same rule before
  inserting so the DB never sees an inconsistent row.
- `fresh_test_pool()` writes to `/tmp/caduxo_test_<pid>_<rand>.db`,
  which is what the V4 precedent relies on for migration tests. V5
  mirrors this exact pattern for `restore_from_pre_v5_*` tests.
- `app_settings` already supports arbitrary key/value rows via
  `upsert_setting`/`get_setting`. The new
  `require_initial_location_on_lot_create` key reuses these helpers
  with zero schema change.
- The V4 precedent (`restore_from_pre_v4_backup_applies_v4_backfill_in_situ`)
  in `services/backup_restore.rs` is the template for V5's
  pre-backup restore test.
- `expiry_lots.location_id` remains the dashboard's single-row
  "where is this lot" hint. Per-location stock distribution lives in
  the ledger and surfaces only on the Historial tab in v1. Mixing the
  two would cause drift; keeping them separate is the design's
  strongest invariant.
- Manual `batch_code` must be preserved verbatim. The two-layer
  defense (frontend guard + service guard) is required because the
  LotForm and the service both see the input, and either could be the
  source of a regression.
- The legacy `lot_resolution_events` table stays in place as a
  read-only anchor for one release. Hard-deleting it would strand any
  pre-V5 backup that lands on a V5 build that hasn't yet hidden the
  legacy surface.
- Phase 1 alone is at the 400-line ceiling. Path A (1a schema +
  1b logic) is the recommended slicing; the parent should ratify
  before `tasks.md`.
- The unified `inventory_adjustment` kind with a `direction` column is the
  smallest design change that honors the user-confirmed *Count adjustment flow
  can increase or decrease stock* rule (proposal decision 4). A single `direction`
  column of `'increase' | 'decrease'` collapses the two predecessor kinds
  (`entry:inventory_adjustment`, `exit:inventory_adjustment`) into one wire
  shape; the schema CHECK ties `direction` to the source/destination
  nullability so the DB never sees an inconsistent row.
- Cross-store transfers were removed from v1's non-goals by the user. The
  reverse direction (re-introducing a same-store-only restriction) would
  require dropping the new spec scenarios; if the user wants to revert, the
  spec is updated first and the design follows.
