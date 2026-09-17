# Explore — caduxo-lot-movement-ledger

## Scope of this exploration

Caduxo today is a local-first product-expiry tracker: each lot is one row in
`expiry_lots`, with a single `quantity` and a single optional `location_id`.
A user that receives 30 units of an ice-cream batch and wants to keep 20 on
display ("exhibición") and 10 in the warehouse ("bodega") cannot express
that. A user that throws 3 units away because they expired, sells 5, and
returns 2 to the supplier cannot express that either — the only stock-out
concept today is `lot_resolution_events`, which is a partial-resolution audit
trail for one quantity reduction per lot, with no source/destination
location, no standardized reason vocabulary, and no UI for managing
intermediate movements.

This change introduces a **lot movement ledger**: every change in a lot's
quantity is recorded as an event with a movement kind, an optional reason,
and source/destination locations (when applicable). Stock exits carry a
reason drawn from a small fixed vocabulary. Lot creation auto-emits the
initial "Entrada inicial" movement. Lot creation also auto-generates a
readable date-based `batch_code` only when the user leaves it blank. Sale is
one of several stock-out reasons and remains strictly non-financial.

This document is **substrate for the proposal phase**. It does not propose
the schema, the wire format, or the UI — those are the proposal's job. It
captures the product problem, the current-state gap, the proposed domain
vocabulary, the user workflows, the non-goals, the implementation options,
the risks, and the open decisions that must be settled before the proposal
is written.

---

## 1. Repository state

Working tree is clean per parent. No active OpenSpec changes are open; the
most recent archived change (`2026-09-15-caduxo-category-management`) settled
the multi-category product model and explicitly noted that **product
creation no longer uses `category_id`** — it uses `category_ids: Vec<String>`
via a `product_categories` junction. The new change must coexist with that
contract and not regress category filters anywhere.

### Project config (`openspec/config.yaml`)

| Knob                          | Value     |
|-------------------------------|-----------|
| `project`                     | Caduxo    |
| `artifactStore` (project)     | `both`    |
| `sdd.executionMode`           | `interactive` |
| `sdd.reviewBudgetChangedLines`| `800`     |
| `sdd.chainedPrStrategy`       | `auto-forecast` |
| `sdd.strictTdd`               | `false`   |
| `product.priorityPlatforms`   | `windows`, `linux` |
| `product.nonGoals`            | POS, billing, payments, accounting, full inventory |
| `product.summary`             | Local-first desktop application for product expiry tracking only. |

Session preflight overrides (current change):

- `artifactStore`: `openspec` (project config says `both`; session says `openspec`).
  → Persist only to `openspec/changes/caduxo-lot-movement-ledger/` this session.
- `reviewBudgetChangedLines`: `400` (session override; canonical is 800).
  → Any forecast above 400 changed lines MUST trigger the
  `deliveryStrategy: ask-on-risk` decision round before `tasks.md`.
- `chainStrategy`: `deferred`. No chained split is planned at this stage.
- `executionMode`: `interactive`. This phase completes only `explore`; the
  proposal phase must wait for explicit user approval.

### Frontend / backend split

- Frontend: Svelte 5 + Vite + TypeScript, mounted via Tauri 2
  (`src/main.ts`, `src/App.svelte`). No frontend test harness (`strictTdd:
  false`, `svelte-check` is the type gate).
- Backend: Rust + Tauri in `src-tauri/` (SQLite, command/handler layer,
  services + repositories). Test harness: `cargo test --manifest-path
  src-tauri/Cargo.toml --lib`.

### Canonical spec surface

`openspec/specs/caduxo-expiry-tracker/spec.md` is the only domain. Existing
capabilities that the new change will touch (likely) or that constrain it:

| Capability         | Requirement                  | Why this change cares |
|--------------------|------------------------------|-----------------------|
| `Expiry lots`      | `lot registration`           | New initial-movement emission on creation; batch-code auto-gen on blank; location is required for stock movement to make sense. |
| `Expiry lots`      | `lot resolution` (uses `lot_resolution_events`) | Overlap with the new exit-side vocabulary; see §3 and §7. |
| `Stores & locations` | `editable store + location list` | Internal-location split (bodega vs exhibición) needs to read as a first-class concept in product detail and dashboard. |
| `Dashboard`        | `operational main screen`    | One row per lot today; with per-location stock, dashboard semantics may shift (see §6). |
| `Reports`          | `report filters`, `report export` (PDF, CSV) | Filters may need a movement-date range; CSV export shape changes. |
| `Engineering safety` | `tests accompany implementation`, `safe log content`, `data backup & restore` | Backwards compat with pre-V5 backups is mandatory; no migration may strand data. |

No current requirement covers the movement ledger. The proposal will need to
add (at least) one new capability — most likely
`## Capability: Lot movements` — and modify `lot registration` and
`lot resolution`.

---

## 2. Product problem in plain language

The user is a small-business owner (bodega, café, kiosk, restaurant, etc.)
tracking perishable stock on a desktop or laptop. Their recurring,
product-shaped pain today is **how stock actually flows**:

- A delivery arrives with 30 units. They put 20 on display and keep 10 in
  the back room. Caduxo has no way to record that 10 are *not* on display.
- During the day, they sell some, throw some away (merma), give some to
  staff (consumo interno), and occasionally return damaged or wrong
  deliveries to the supplier. Caduxo can mark a lot as "resolved" once but
  cannot keep a per-event audit trail of why stock left.
- They want to scan/recall a batch later ("what happened to batch L-20251015
  of Yogurt Frutilla?") and see a clean ledger.
- When they create a lot, they often leave `batch_code` blank because the
  supplier wrote nothing legible. They want Caduxo to fill in something
  human-readable, but only when they really left it blank — never to clobber
  a deliberate manual batch.

Caduxo's product non-goals (POS, billing, payments, accounting, full
inventory) are not negotiable: a "sale" in this context is a stock-out
reason, not a financial event. No price, no tax, no invoice, no customer, no
margin, no payment method, no POS register, no chart of accounts. The
ledger must be expressible without any monetary concept, and the schema must
not invite future drift toward one.

---

## 3. Current-state gap (code-level)

What is already on disk that the new change will reuse, coexist with, or
have to retire:

### 3.1 What already exists and is reusable

- `store_locations` (`db/migrations.rs` V2) — internal location per store,
  with `name`, `notes`, `is_active`. CRUD is already wired through
  `src-tauri/src/services/stores.rs` and `src/components/StoresPage.svelte`.
  Filter wiring already exists in Dashboard (`DashboardPage.svelte:163`)
  and Reports (`ReportsPage.svelte:128`).
- `expiry_lots.location_id` (nullable FK to `store_locations`) — current
  lots have at most one location. The new model will make the lot's total
  quantity the canonical number and derive per-location balances from the
  movement ledger (see §5 for the design options).
- `lot_resolution_events` (V2) — `quantity`, `resolution`, `notes`,
  `created_at`. Only used for partial resolution today. Has a single CHECK
  constraint `quantity > 0`.
- `category_ids: Vec<String>` and `UNCATEGORIZED_SENTINEL` (V4 from the
  recent category-management change) — new code MUST use this multi-cat
  contract and not reintroduce `category_id`.

### 3.2 What is missing today

- **No movement ledger table.** There is no `lot_movements` (or equivalent).
  Every quantity change beyond the lot's initial creation is recorded only
  via `apply_partial_resolution` (a one-quantity-reducer).
- **No per-location stock view.** A lot cannot be split across locations.
  The single `location_id` column implies a lot is *entirely* at one
  location, which contradicts the bodega/exhibición example.
- **No reason vocabulary.** `lot_resolution_events.resolution` is a free
  text field. There is no constrained set of reasons; no UI to pick one.
- **No auto-batch generation.** `batch_code` is nullable free text. The
  user types whatever they want (or nothing). No format guarantee.
- **No initial-movement emission.** `create_expiry_lot` inserts the lot
  row with `quantity` set and that's the end of the audit trail — the
  fact that the qty "moved in" to that location on day X is not recorded.
- **No movement history view.** Even with `lot_resolution_events`, there is
  no page that shows the chronological list of events for a lot.
- **No "where did it go" queries.** A user cannot ask "how many units of
  Yogurt Frutilla did I throw away last month?" — there's no
  by-reason-by-time aggregation.

### 3.3 The `lot_resolution_events` overlap (decision needed)

Today, the only stock-out concept is `lot_resolution_events`. The user's
desired exit-reason vocabulary (sale, waste, expired, damaged, internal
consumption, return to supplier, inventory adjustment, other) overlaps with
that table. The proposal must resolve one of three directions (see §7):

1. **Retire `lot_resolution_events`** and replace it with `lot_movements`,
   whose `movement_kind IN ('exit:sale', 'exit:waste', ...)`. A V5
   back-fill must migrate any pre-existing resolution events into the new
   table so historical audit data survives.
2. **Keep `lot_resolution_events`** as a legacy audit table (read-only,
   hidden in UI) and add a parallel `lot_movements` table for new
   transfer/exit events. Two parallel concepts is the most confusing
   option; rejected unless a real reason appears.
3. **Generalize `lot_resolution_events`** by adding `source_location_id`,
   `dest_location_id`, `movement_kind`. Cheapest migration path but
   pollutes the existing table's meaning and breaks the current "audit
   trail" framing.

This is the **first decision the proposal round must settle**.

---

## 4. Proposed domain vocabulary

These are the names the proposal will commit to. Capture them here so the
proposal round can correct or ratify them.

### Entities

- **Lot** (`expiry_lots`) — unchanged concept: a batch of one SKU at one
  expiry date, optionally one batch code. **Quantity is the canonical total**
  remaining across all locations.
- **LotMovement** (new) — an event that changes a lot's remaining quantity
  or redistributes it across locations. Recorded as a row in the new
  `lot_movements` table. Has a `movement_kind`, `quantity` (always > 0),
  `source_location_id` (nullable for entries), `destination_location_id`
  (nullable for exits), `reason` (constrained vocabulary or null), `notes`,
  `actor` (future; v1 can leave null/derived), `created_at`.
- **LotMovementKind** — an enum:
  - `entry:initial` — auto-emitted on lot creation, full `quantity` into the
    chosen initial location. Single row, never user-authored.
  - `transfer` — move qty from one internal location to another in the same
    store. Both source and destination are required.
  - `exit:sale` — quantity left the lot for a sale.
  - `exit:waste` — `merma` / discard (unspecified reason).
  - `exit:expired` — quantity past expiry, removed.
  - `exit:damaged` — physical damage.
  - `exit:internal_consumption` — staff consumption, sampling, etc.
  - `exit:return_to_supplier` — sent back.
  - `exit:inventory_adjustment` — count correction. Should require a
    note (proposal round decision).
  - `exit:other` — escape hatch with a mandatory free-text reason.
- **ExitReason** — the constrained vocabulary for the `exit:*` kinds above.
  The exact set is the second decision for the proposal round (see §7).
- **LotLocationBalance** (derived view, not necessarily a table) — sum of
  inbound movements minus sum of outbound movements for each `(lot_id,
  location_id)` pair. Powers dashboard / reports.

### Important vocabulary invariants

- "Sale" is a stock-out reason, **never** a sale-of-goods event.
- "Inventory" in this app means "expiry-tracked stock", **not** general
  stock-keeping. The non-goal "full inventory management" still binds.
- "Batch" is `batch_code`, a label on a lot. It is not an entity. Two lots
  may share a batch code (supplier batch spans multiple deliveries); one
  lot may have no batch code.
- "Movement" is a ledger row. The user does not see "stock movements" as
  a noun — they see "events" or "history" on a lot detail page.

---

## 5. Conceptual design options (for the proposal to choose between)

These are the three plausible shapes for the lot → movement relationship.
The proposal must pick one (or a documented hybrid) before `design.md`.

### Option A — Append-only ledger, total quantity is canonical

`expiry_lots.quantity` remains the single source of truth for remaining
quantity. A `lot_movements` ledger records every event that changed it.
Per-location balance is **always computed**: `SUM(in to L) − SUM(out from L)`
across all movements for that `(lot_id, location_id)`.

Pros: minimal schema change to `expiry_lots`; one canonical number; simple
backup restore. Cons: per-location "stock on hand" is a join query; small
risk of drift if the ledger is edited without updating the lot.

### Option B — Per-location stock lines, ledger is delta

`expiry_lots` becomes a header row with no `quantity`. A new
`lot_location_stock` table holds one row per `(lot_id, location_id)` with
its own `quantity`. The `lot_movements` table records deltas between those
rows (transfer) or terminal events (exit).

Pros: per-location stock is a primary row, not a join; matches the
"warehouse vs display" mental model. Cons: bigger schema change; backup
restore more complex; reconciliation logic needed on partial failure.

### Option C — Hybrid: keep `expiry_lots.quantity` total, introduce

`lot_location_stock` as a materialized view-style table maintained by the
ledger on each write

Pros: best read ergonomics; can survive the join entirely on dashboard
reads. Cons: requires a transactional invariant "ledger write +
stock-table update must be atomic"; more code paths.

**The proposal round must pick A, B, or C**, with reasoning. The user's
example (30 in, split 20/10) is expressible under all three; the difference
is mostly in dashboard reporting shape and backup complexity.

---

## 6. Likely affected areas (high level only)

Per the user's instruction, this section is intentionally shallow — the
proposal and design phases will do the deep audit.

- **Backend — domain**: new `lot_movements` module under `domain/` (or
  similar), shared with the existing `domain/lot_resolution.rs` (which may
  be retired or renamed).
- **Backend — repositories**: a new `db/repositories/lot_movements.rs`
  with insert/list/by-lot queries. `db/repositories/expiry_lots.rs`
  changes for: required `location_id` on creation (debatable; see §7),
  initial-movement emission inside the same transaction as lot insert,
  and any reconciliation logic for Option B/C.
- **Backend — services**: a new `services/lot_movements.rs` (or merged
  into `services/expiry_lots.rs`). `services/backup_restore.rs` gets a
  V5-aware test fixture similar to the V4 precedent.
- **Backend — migrations**: a new V5 migration. Must be idempotent and
  back-fill (a) initial-movement rows for any pre-existing lots, (b) any
  pre-existing `lot_resolution_events` rows that the proposal decides to
  migrate.
- **Backend — commands** (Tauri): new `create_lot_movement`, `list_lot_
  movements`, possibly `cancel_lot_movement` (if undo is in scope; see
  §7).
- **Frontend — DTOs**: new `MovementCreate`, `MovementResponse`,
  `MovementKind` in `src/lib/expiry_lots.ts` (or a new `src/lib/movements
  .ts`). `LotDetailResponse` gains an embedded `movements: Movement[]` or
  a separate fetch.
- **Frontend — pages**: new `LotMovementsPanel.svelte` (or section in
  `ProductDetailPage.svelte` / `LotDetailPage.svelte`) showing the
  chronological ledger. A new "Add movement" modal/page with reason
  picker, source/destination location picker, qty input. A new "Transfer
  between locations" flow (or merged into "Add movement").
- **Frontend — LotForm**: `batchCode` becomes optional with a "leave blank
  for auto" hint; on submit, if blank, the backend generates a readable
  date-based code and echoes it back. UI must display the generated code
  after creation.
- **Frontend — Dashboard**: depends on Option A/B/C. At minimum, the
  detail modal gains a "movements" tab. With Option B/C, the main list
  may gain a per-location column or expand to one row per
  `(lot, location)`.
- **Frontend — Reports**: filters gain a date-range and a reason filter
  for the "movements by reason" report (if in scope). PDF export shape
  changes if the report type gains new columns.
- **Frontend — StoresPage**: no new surface, but `stores.ts` may gain a
  small "stock at location" summary widget for each location.
- **Backup / Restore**: must round-trip the new ledger. Pre-V5 backups
  restore cleanly (apply V5 in situ and back-fill initial movements); this
  is mandatory. The V4 precedent (`caduxo-category-management`) sets the
  testing pattern: `restore_from_pre_v5_backup_applies_v5_backfill_in_situ`.
- **CSV import/export**: if the ledger is exported, the CSV column list
  changes. If only lots are exported, `batch_code` may now be auto-filled
  for legacy rows on first save — verify this does not break import
  idempotency.

---

## 7. Open product decisions (proposal round)

These are the **business / product** questions the proposal phase must
settle with the user before `design.md`. They are deliberately not
implementation details (test commands, line budgets, PR shape) — those
are the parent's domain.

1. **Lot→movement shape (A, B, or C in §5).** This is foundational. The
   user's example works under all three, but the dashboard reads
   differently and the schema size differs. Pick one with reasoning.

2. **Existing `lot_resolution_events` policy.** Retire / keep-as-legacy /
   generalize-in-place (see §3.3). Each has a migration cost and a UX
   consequence (do historical events show in the new ledger?).

3. **Scope of stock-out reason vocabulary in v1.** All eight (sale, waste,
   expired, damaged, internal consumption, return to supplier, inventory
   adjustment, other), or a slimmer MVP (e.g., sale + waste + expired +
   other) with the rest as a follow-up? Tradeoff: more reasons = more
   UI options and more aggregation reports; fewer = faster to ship, easier
   to test.

4. **"Expired" — auto or manual?** When a lot crosses its expiry date,
   should Caduxo auto-emit a `exit:expired` event for the remaining
   quantity, or require the user to log each expiry manually? Auto-emit
   is more accurate for batch-expiry contexts but inflates the ledger
   and may surprise the user; manual is honest but tedious. A
   background-job emission on app launch is a third option.

5. **Auto-batch format.** Propose 2–3 candidate formats (e.g.,
   `L-YYYYMMDD`, `LOT-YYYY-MM-DD-NN`, `YYYY-MM-DD-<sku>`) and confirm.
   Per-product unique or globally unique? Local time or UTC?

6. **Initial movement: full qty, or user-declared?** Today the user types
   `quantity` and `location_id` together. Is the initial movement always
   for the full qty into that location (i.e., the user cannot create a
   lot with 30 units and say "only 20 are in bodega, the rest is still
   in transit from the supplier")? Or does the lot creation form support
   a "received into bodega" sub-quantity for cases where the delivery
   was split on arrival?

7. **Cross-store transfers — in scope or deferred?** The user said "in
   the same store". Confirm: v1 is same-store only; cross-store is a
   later change. Schema should not preclude it (a `transfer` could in
   theory cross stores if both ends reference the same `expiry_lots.id`
   — but lots are store-bound today via `expiry_lots.store_id`).

8. **Edit / undo policy for movements.** Append-only (with a
   "correcting entry" as the only fix), or in-place edit (with audit log),
   or full undo (only for the latest event within N minutes)? Each has UX
   and audit implications. The non-negotiable: the audit trail must
   never silently lose an event.

9. **`inventory_adjustment` justification.** If included in v1, must it
   always carry a non-empty `notes`? This is the most error-prone reason
   and a free-text note turns "I clicked the wrong button" into a
   recoverable record. Confirm whether it's a hard requirement or just
   recommended.

10. **Movement history visibility on Dashboard.** Does the dashboard
    surface today's movements anywhere (a small "Recent activity" card,
    or a per-lot detail tab only), or is the movement ledger strictly a
    per-lot detail feature in v1?

---

## 8. Phased implementation options

For the proposal round's discussion. Each phase is intentionally rough;
`tasks.md` will refine.

- **Phase 0 — Confirm domain shape.** Lock §5 (A/B/C) and §7 decisions
  with the user. No code. Produces: ratified proposal.md.

- **Phase 1 — Backend ledger skeleton.**
  - V5 migration: `lot_movements` table with the chosen shape; back-fill
    of pre-existing lots (one initial movement each, full qty into current
    `location_id`); idempotent.
  - Domain module + repository (insert + list-by-lot).
  - Service layer with transactional invariants.
  - Backup-restore smoke test (pre-V5 → V5 in situ).
  - No UI yet. Verify: cargo test green.

- **Phase 2 — Lot creation integration.**
  - `create_expiry_lot` emits the initial movement in the same
    transaction.
  - `batch_code` auto-generation logic (format chosen in §7).
  - Echoes the generated batch back to the caller so the form can
    display it.
  - LotForm UI: hint "leave blank for auto" + post-create confirmation.
  - Verify: cargo test + svelte-check + manual smoke.

- **Phase 3 — Movements UI.**
  - Lot detail page: chronological movement ledger view.
  - "Add movement" form (depends on Option A/B/C and reason vocabulary
    chosen in §7).
  - Transfer flow (same store, different locations).
  - Verify: cargo test + svelte-check + manual smoke on Windows + Linux.

- **Phase 4 — Dashboard / Reports integration.**
  - Dashboard per-lot detail: movements tab.
  - Reports: new "Movements by reason" report (if in scope per §7).
  - CSV export updated for the new shape.
  - Verify: full gates + manual smoke.

- **Phase 5 (optional) — Polish.**
  - Recent-activity dashboard widget.
  - Bulk movement entry (e.g., "today's waste for these 5 lots").
  - Print-friendly movement ledger.
  - Out of scope unless user explicitly asks.

This phased view is illustrative; the **proposal round** may collapse or
reorder phases based on user priorities and on the size of each phase
against the 400-line review budget.

---

## 9. Risks the parent should know before proposal

1. **Foundational shape decision (A/B/C) drives everything.** Picking
   wrong now means redoing migrations, dashboards, and reports later.
   The proposal round must invest time here.

2. **`lot_resolution_events` retirement must not strand historical data.**
   Any pre-existing user data must survive the migration, even if the
   table is later renamed or hidden in the UI. The V4 precedent shows
   this is tractable but requires a tested back-fill path.

3. **Backup / restore is a hard gate.** Pre-V5 backups must restore
   cleanly. The V5 migration must apply on an opened pre-V5 pool and
   back-fill initial movements idempotently. A new
   `restore_from_pre_v5_backup_*` test is mandatory, mirroring the V4
   pattern.

4. **Non-goal drift.** "Sale" must remain a non-financial stock-out
   reason. The schema must not introduce any column or UI hook that
   would invite future POS/accounting features. The proposal must name
   this explicitly in the new requirement.

5. **Review budget.** Session override is 400 changed lines; canonical is
   800. Even Phase 1 + Phase 2 combined (migration + domain + repository
   - service + form integration + back-fill test) may approach or
   exceed 400 lines. If the forecast is close, the parent MUST use the
   `deliveryStrategy: ask-on-risk` decision before locking `tasks.md`.

6. **Auto-batch readability vs. uniqueness.** A purely date-based code
   is human-readable but collides across same-day lots. A counter or
   SKU suffix solves uniqueness but reduces readability. The proposal
   must name the chosen format and its conflict-resolution rule.

7. **Movement reason vocabulary expansion over time.** Whatever set ships
   in v1 will be asked to grow. The schema should make the vocabulary
   a constant table (not a hardcoded enum) so adding "donation" or
   "transfer to sister store" later is a code change, not a schema
   change. Caveat: that complicates i18n if labels are DB-driven.

8. **Idempotency on partial failures.** If a `create_lot_movement` call
   inserts the movement row but fails to update derived state (per §5
   Option B/C), the ledger and the stock summary drift. The chosen
   approach must be transactional. The proposal must name the
   invariant.

9. **Manual batch must never be auto-overwritten.** If the user types
   `BATCH-XYZ-2025`, the system MUST NOT silently generate and replace
   it. This is explicit user instruction and must be a tested
   scenario.

10. **Frontend has no test harness.** `strictTdd: false`. UI changes for
    movements (forms, modals, pickers) will rely on manual smoke per
    the canonical spec's `tests accompany implementation` clause. The
    proposal should not promise frontend unit tests in this slice.

---

## 10. Recommended next phase

Proceed to **`proposal.md`** with the `deliveryStrategy: ask-on-risk`
round on the open product decisions in §7, in this order:

1. Pick §5 shape (A/B/C). (Blocking.)
2. Pick §3.3 `lot_resolution_events` policy. (Blocking.)
3. Pick §7.3 reason vocabulary scope for v1.
4. Pick §7.5 auto-batch format (2–3 candidates presented).
5. Pick §7.4 expired handling (auto / manual / background).

Five questions keeps the round within the canonical 3–5 limit. The
proposal should then lock the non-goal invariant (sale is not a financial
event), the review budget (400 lines), and the chained-PR decision
(deferred unless a phase forecast exceeds the budget).

After locking the proposal, `design.md` resolves the schema, the wire
format, the UI primitives, and the migration back-fill plan.
`tasks.md` slices by phase. The verify gate is the canonical:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib`
- `npx svelte-check --workspace . --threshold error`
- `npm run build`
- Manual smoke on Linux + Windows covering lot creation, transfer,
  each exit reason, undo/correct (if in scope), and CSV import/export.

---

## Key Learnings

- Caduxo is single-domain (`caduxo-expiry-tracker`); all new requirements
  live in or extend that spec.
- Project config says `artifactStore: both` and `reviewBudget: 800`;
  session preflight overrides to `openspec` and `400`. The override is
  authoritative for this session.
- `expiry_lots` is single-quantity, single-location today. Per-location
  split is the central product gap.
- `lot_resolution_events` exists but is partial-resolution only — a
  partial ledger, not a full movement ledger. It must be either retired,
  frozen, or generalized.
- `category_ids: Vec<String>` (V4) is the active contract for product
  categories. New code must not reintroduce `category_id`. New filters
  must use `category_ids: string[]`.
- `store_locations` and its filter wiring are already in place;
  Dashboard, Reports, and LotForm already expose locations. New work
  should slot into these existing sites, not introduce a parallel
  location system.
- `batch_code` is nullable free text today with no auto-generation. The
  user has explicitly asked for a date-based auto-fill ONLY when the
  user leaves it blank — manual batches must be respected.
- Non-goals (POS, billing, payments, accounting, full inventory) bind
  tightly. "Sale" stays a stock-out reason, never a financial event.
- The V4 backup-restore test pattern (`restore_from_pre_vN_backup_*`)
  is the template for the V5 back-fill.
- No frontend test harness. Manual smoke is the contract for UI changes.
- Linux + Windows are the target webviews; both Tauri builds matter for
  manual smoke.
- 400-line review budget means phases must be forecasted before
  `tasks.md`; the parent will use `deliveryStrategy: ask-on-risk` to
  pause if a forecast is too large.

## Artifact store

`openspec` (per session preflight; project config says `both` but session
preflight overrides). Persisted to
`openspec/changes/caduxo-lot-movement-ledger/explore.md`.

## skill_resolution

`none` — the parent did not inject any `## Skills to load before work`
paths in the user message; this phase was executed without project/user
skill discovery per the contract. (`gentle-ai` is loaded by default; no
domain-specific SDD executor skill exists for this harness.)
