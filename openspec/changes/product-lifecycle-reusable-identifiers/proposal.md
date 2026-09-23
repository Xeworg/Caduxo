# Proposal — product-lifecycle-reusable-identifiers

## Change metadata

- **Change ID**: `product-lifecycle-reusable-identifiers`
- **Domain**: `caduxo-expiry-tracker` (single-domain project)
- **Artifact store**: `both` (OpenSpec + Engram; persisted to OpenSpec file
  and Engram topic key `sdd/product-lifecycle-reusable-identifiers/proposal`)
- **Review budget**: 400 changed lines (session-confirmed canonical;
  session override vs. project default 800 — chosen because the existing
  scanner/i18n/lifecycle chain precedent landed well below 800 once split)
- **Delivery strategy**: `auto-chain` (deferred until chaining is selected);
  any phase whose forecast exceeds 400 lines MUST trigger
  `deliveryStrategy: ask-on-risk` before `tasks.md`
- **Chain strategy**: `deferred` — chained split will be proposed at the
  design phase once schema/UI/CSV sub-surfaces are sized; the explore
  evidence supports a 3-PR chain similar to `scanner-quick-operations`
  (backend → UI/i18n → CSV import)
- **Strict TDD**: `false` (project default); backend lifecycle service
  tests are strongly encouraged but not TDD-mandatory
- **Execution mode**: `interactive` — this phase completes only `proposal`;
  `spec`, `design`, and `tasks` must wait for explicit user approval

## Problem statement

Caduxo's product catalog has a single off-state — soft archive via
`is_active = 0` — which is simultaneously the recovery state and the
release blocker. The current model has three concrete failure modes:

1. **SKU / UPC reuse is impossible by design.** `products.sku UNIQUE` and
   `product_barcodes.barcode UNIQUE` (V2 schema) span every row including
   archived ones, so a discontinued product permanently holds its
   identifiers. Operators cannot re-register a product under the same SKU
   even when the original product has not been sold in years.
2. **Archive is overloaded.** "Archive" is the only off-state and is
   expected to be reversible. Operators are forced to either keep using
   `is_active = 1` (so the SKU can never be reused) or archive and lose
   the reversible recovery flow that the existing UX implies. The
   localised `Unarchive` i18n string exists in `src/i18n/en/index.ts:365`
   but has no consumer — confirming the intent was never finished.
3. **No lifecycle audit trail.** Product state changes are not recorded
   anywhere except `updated_at`. Operators cannot answer "who retired
   this product and why" or "when was this SKU released", which matters
   for compliance and history reconstruction.

These three failure modes collapse into a single product gap: **Caduxo
has no terminal lifecycle state that preserves history while releasing
identifiers for reuse.** This change introduces that terminal state
distinct from the existing reversible archive, and the audit trail to
prove when the transition happened.

## Outcomes (success criteria)

After this change ships, a Caduxo user can:

1. **Archive a product and reverse it.** The existing archive flow
   continues to work and remains reversible. Archive keeps the SKU/UPC
   locked.
2. **Retire a product as a one-way terminal transition.** Retired
   products are preserved on disk with their full history intact
   (`expiry_lots`, `lot_movements`, `lot_resolution_events`,
   `notification_log`, `product_categories`, `product_barcodes`), but
   the SKU and UPC are released for reuse by a future product row.
3. **Re-register a previously used SKU/UPC.** After a product is
   retired, a new product can be created with the same SKU or attached
   with the same UPC value; the retired product remains in history
   independently.
4. **Inspect lifecycle history.** Every archive, unarchive, and retire
   transition is recorded in a dedicated `product_lifecycle_events`
   table with event type, timestamps, and reason.
5. **Distinguish operational from history-only state in the UI.**
   Archived products show an "Archived" badge and remain fully
   recoverable. Retired products show a distinct "Retired" / "Deleted"
   badge, are hidden from operational scanner and product-creation
   quick paths, and only surface via explicit history affordances.
6. **Export and report history including retired products.**
   `export_products_csv` and any history/reporting surfaces continue to
   include retired products so reports and backups stay complete.
7. **See clear duplicate-SKU/duplicate-UPC feedback during CSV
   import.** The import preview distinguishes "blocked by an active or
   archived product" from "released product — proceeding".

## Scope (in scope)

- A new `product_lifecycle_events` audit table recording every
  archive, unarchive, and retire transition with timestamp, event
  type, and reason.
- A third lifecycle state for products distinct from "active" and
  "archived", expressed as a `lifecycle` / `status` column on
  `products` (recommendation: `lifecycle TEXT NOT NULL DEFAULT 'active'
  CHECK(lifecycle IN ('active','archived','retired'))`).
- Replacement of the current whole-table `UNIQUE` constraints on
  `products.sku` and `product_barcodes.barcode` with partial
  `UNIQUE INDEX … WHERE lifecycle != 'retired'` indexes so that the
  identifiers of a retired product can be reused without dropping the
  uniqueness guarantee for active and archived rows.
- A new `retire_product` IPC plus a parallel `unarchive_product` IPC
  (closing the gap that the unused `Unarchive` i18n string implies).
- Service-layer enforcement: `add_barcode`, scanner resolver
  (`resolve_scanner_code`), scanner quick operations, dashboard
  product-create quick modal, and CSV import preview all become
  lifecycle-aware.
- UI affordances in `ProductDetailPage.svelte`,
  `ProductCatalogPage.svelte`, and `DashboardPage.svelte`:
  - Distinct "Archived" and "Retired" badges.
  - Unarchive action on archived rows (consuming the existing
    `Unarchive` i18n key).
  - Retire action with explicit two-step confirmation that requires
    a free-text reason.
  - Catalog list: a parallel "Show retired" toggle persisted in
    `localStorage` (default off), so the catalog stays focused on
    operational products by default.
- History affordances: a "Show retired" toggle and a per-product
  history pane that surfaces the `product_lifecycle_events` for that
  product.
- `export_products_csv` continues to include archived AND retired
  products; export semantics are documented in the spec.
- Spec delta under `Capability: Product catalog` covering the new
  lifecycle requirements, the partial-uniqueness carve-out, and the
  audit-trail contract.
- i18n parity: new keys `products.retire`, `products.unarchive`,
  `products.retired`, `products.retireReason`, `products.retired`,
  `products.lifecycleEvents`, and any dashboard/catalog variants in
  both `src/i18n/en/index.ts` and `src/i18n/es/index.ts`.

## Non-goals

This change does NOT implement:

- Hard delete (`DELETE FROM products …`). Retired products stay on
  disk; only the identifiers are released.
- Per-field audit trails. Audit captures lifecycle events only; field
  edits remain governed by `updated_at`.
- Re-binding a released SKU to a previous product. When a new product
  claims a released SKU/UPC, the new product is an independent row.
  Linking the new product to the previous row's history is a future
  product feature.
- Bumping an existing migration. The Rust migration harness refuses
  checksum drift (`migrations.rs` V15 comment). Any schema change
  ships as a fresh additive migration at the next available index.
- Multi-store or multi-tenant SKU scoping. SKU uniqueness remains
  global within the local database.
- Migration of legacy `is_active = 0` rows. Existing archived rows
  stay archived (`lifecycle = 'archived'`); the backfill is part of
  the new migration and is a literal projection.
- UI redesign of the catalog list. Lifecycle state is additive; the
  existing hide-archived toggle keeps its localStorage key.
- Reorganisation of unrelated tables (`units`, `categories`,
  `locations`). Lifecycle is a product-only concept in this change.

## Confirmed product decisions

| # | Decision | Source |
|---|----------|--------|
| 1 | Terminal state is irreversible. `retired` (a.k.a. "deleted" in user copy) preserves history on disk, releases SKU/UPC for reuse, and cannot be reactivated. | User confirmation |
| 2 | `archived` remains a reversible state and continues to block SKU/UPC reuse. | User confirmation |
| 3 | Retired products remain visible in history, export, and reporting surfaces. They are NOT visible in operational scanner resolver paths or the product-creation quick path. | User confirmation |
| 4 | Add a lifecycle audit trail, preferably a dedicated `product_lifecycle_events` table for archive / unarchive / retire events. | User confirmation |
| 5 | The UI separates the recoverable "Archived" status from the terminal "Retired / Deleted" status. Retired rows are history-visible only via an explicit show/history affordance. | User confirmation |

## Proposed capabilities

This proposal adds a third product lifecycle state, an audit trail, and
the partial-uniqueness carve-out that lets SKU/UPC values be reused
after retirement.

### New capability: Product lifecycle states

- Every product row carries a `lifecycle` value in
  `{active, archived, retired}`. The default for new products is
  `active`. Existing archived rows project to `archived`; existing
  active rows project to `active`. No row projects to `retired` at
  backfill time.
- Transitions allowed: `active → archived`, `archived → active`
  (unarchive), `active → retired`, `archived → retired`. No
  transition out of `retired`.
- Each transition writes one row to `product_lifecycle_events`.
- The Rust service layer rejects any IPC that tries to mutate a
  retired row other than read.

### Modified capability: SKU uniqueness

- The V2 schema constraint `products.sku UNIQUE` is replaced by
  `CREATE UNIQUE INDEX … ON products(sku) WHERE lifecycle !=
  'retired'`. The same treatment applies to
  `product_barcodes.barcode`.
- A new product can be created with a SKU that matches a retired
  product's SKU. The new row's lifecycle starts at `active`.
- A new barcode can be attached to an active or archived product
  with a value that matches a retired product's barcode.

### Modified capability: Barcode attach guard

- `add_barcode` rejects new barcodes on archived products (existing
  `ProductArchivedForBarcode`) and on retired products with a new
  `ProductRetiredForBarcode` user message.
- Scanner quick operations already filter active rows; retired
  rows must join the filter so the resolver hard-blocks a barcode or
  SKU that points only at retired products.

### New capability: Lifecycle audit trail

- A new `product_lifecycle_events` table records every transition.
  Recommended columns: `id`, `product_id`, `event_type`, `from_state`,
  `to_state`, `actor` (nullable for system events), `reason` (nullable;
  required for retire events), `created_at`.
- The history pane on `ProductDetailPage.svelte` reads this table for
  the selected product.
- CSV export does NOT export this table by default; the audit trail is
  a runtime/internal surface. (Export of the audit table can be added
  later if compliance requires it.)

### Modified capability: Product detail / catalog / dashboard UI

- `ProductDetailPage.svelte`:
  - Replace the single "Archived" banner copy with conditional
    copy keyed on lifecycle.
  - Add a "Unarchive" action for `archived` rows.
  - Add a "Retire" action for `active` and `archived` rows. Retire
    requires a free-text reason and an explicit second confirmation.
  - Render a "Lifecycle history" section listing
    `product_lifecycle_events` rows for the product.
- `ProductCatalogPage.svelte`:
  - Render a distinct `Retired` badge for retired rows.
  - Add a parallel "Show retired" toggle persisted in
    `localStorage` next to the existing `Hide archived` toggle.
  - Retired rows sort to the bottom by default when the toggle is
    enabled.
- `DashboardPage.svelte`:
  - The product modal continues to render both badges.
  - The product-create quick path stays retired-aware: it offers no
    way to start from a retired row's barcode (the scanner resolver
    returns "no match" for retired-only lookups).

## Risks and considerations

- **Schema migration risk.** The existing migration harness
  (`src-tauri/src/db/migrations.rs`) refuses checksum drift. The new
  lifecycle column and partial indexes MUST land as a fresh additive
  migration at the next available index. The proposal phase pins
  that requirement; the design phase selects the index number.
- **Partial-index transactionality.** `CREATE UNIQUE INDEX` outside a
  `BEGIN/COMMIT` is the standard SQLite pattern but the harness
  renders DDL via `sqlx::query`. The V17 migration
  (`migrations.rs:506–:551`) provides the table-rebuild precedent;
  the design phase must mirror that pattern.
- **Backup/restore round-trip.** `export_backup` uses
  `VACUUM INTO` and snapshots every table wholesale. Any new column
  on `products` or `product_barcodes`, plus the new
  `product_lifecycle_events` table, MUST round-trip through
  `VACUUM INTO` without schema drift. The design phase verifies
  `REQUIRED_TABLES` (`services/backup_restore.rs:30`) lists the new
  audit table.
- **CSV import preview.** Distinguishing "SKU blocked because
  archived" from "SKU blocked because retired" in the import preview
  is non-trivial. The runtime currently surfaces both as
  `DuplicateSku` / `DuplicateBarcode`. The design phase reserves
  distinct enum variants on the preview payload so the UI can render
  the appropriate notice.
- **Scanner resolver behaviour.** `resolve_scanner_code` uses
  `find_active_product_by_barcode_exact` /
  `find_active_product_by_sku_exact`. Those helpers hard-filter
  `is_active = 1`. The proposal confirms the new behaviour: the
  filter must additionally exclude `lifecycle = 'retired'`. Lot-code
  scans stay unchanged (lot resolution does not depend on product
  state) — the lot detail view shows the "Retired" badge so the
  operator knows the parent product is terminal.
- **Dashboard `find_product_by_scan`.** The dashboard search
  surface today includes archived rows. The proposal confirms the
  retired rows stay in this surface (they have history) and the
  badge makes the state clear.
- **i18n parity.** Both `src/i18n/en/index.ts` and
  `src/i18n/es/index.ts` need new keys. The `Unarchive` key in
  `src/i18n/en/index.ts:365` already exists and is unreferenced; the
  implementation can wire it to the new unarchive IPC.
- **Hide-archived + Show-retired UX.** Adding a second persisted
  toggle next to `Hide archived` increases settings surface. The
  design phase keeps the localStorage keys stable and named
  (`caduxo.products.catalog.hideArchived.v1` and
  `caduxo.products.catalog.showRetired.v1`).
- **Forecast.** The change touches schema, services, multiple UI
  surfaces, i18n parity, and CSV import preview. The chain strategy
  is `deferred`; the design phase MUST size each slice and propose
  the chain.

## Open product decisions to confirm

These are the second-order product gaps surfaced by the explore
artifact that the user has not yet explicitly decided. Each carries a
recommended default based on the confirmed decisions and existing
precedent. The proposal assumes these defaults; if any default is
rejected, the spec delta and the design phase will revisit before
`tasks.md`.

1. **CSV import preview — released-SKU behaviour.** When a CSV import
   encounters a row whose SKU matches only a retired product, the
   import should: (a) silent pass-through, (b) pass-through with an
   advisory notice ("this SKU was previously associated with a
   retired product"), or (c) hard block. Recommended: **(b)** —
   preserves the operator's right to proceed while making the prior
   ownership visible.
2. **Catalog list — retired-product visibility.** The catalog
   already has a `Hide archived` toggle. The proposed addition is a
   parallel `Show retired` toggle (default off). Alternative: always
   show retired rows with a distinct badge, or move them to a
   dedicated sub-tab. Recommended: **`Show retired` toggle (default
   off)** — keeps the catalog focused on operational products by
   default while making retired products discoverable.
3. **`product_lifecycle_events` schema fields.** The recommended
   column set is `id`, `product_id`, `event_type`, `from_state`,
   `to_state`, `actor` (nullable), `reason` (nullable, **required for
   retire events**), `created_at`. The user is invited to confirm
   the `reason` requirement for retire events; optional for archive
   and unarchive.
4. **Lot-code scans for retired products.** A scanner that resolves
   a lot barcode whose parent product is retired should: (a) show
   the lot and its history with a `Retired product` badge, or (b)
   block the lot scan. Recommended: **(a)** — lot history belongs to
   the operator; the badge makes the parent state clear.
5. **Retired-product ↔ new-product relationship.** When a new
   product claims a released SKU/UPC, the new row should be: (a)
   independent (no link to the retired row), or (b) optionally
   "rebindable" so the user can explicitly link the new row to the
   retired row's history. Recommended: **(a)** — keeps the change
   scope bounded; rebinding is a separate future feature.

The proposal above already uses these defaults. If the user accepts
them as written, the design phase proceeds without revisiting
product framing. If any default is rejected, the proposal will be
updated before the spec delta is drafted.

## Next phase

After this proposal is approved:

1. Draft the spec delta under `Capability: Product catalog`,
   adding:
   - `Requirement: product lifecycle states` (active / archived /
     retired semantics, transitions, reversibility).
   - `Requirement: SKU uniqueness excludes retired` (partial-index
     carve-out).
   - `Requirement: barcode uniqueness excludes retired` (partial-
     index carve-out).
   - `Requirement: lifecycle audit trail` (the new table and
     transition contract).
   - `Requirement: retired product history visibility` (export,
     reporting, lot scans, dashboard search, history pane).
2. Write `design.md` sizing each slice and proposing the chain.
3. Generate `tasks.md` only after the chain and budget risk are
   confirmed against the 400-line session cap.

## Persistence

- **OpenSpec**: this file at
  `openspec/changes/product-lifecycle-reusable-identifiers/proposal.md`.
- **Engram**: topic key `sdd/product-lifecycle-reusable-identifiers/proposal`,
  type `architecture`, project `caduxo`, scope `project`.
