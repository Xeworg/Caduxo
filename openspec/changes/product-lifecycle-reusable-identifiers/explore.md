# Explore — product-lifecycle-reusable-identifiers

Read-only exploration note for the `product-lifecycle-reusable-identifiers` SDD
change. No source code was edited in this phase.

Branch: `feat/product-lifecycle-reusable-identifiers`
Artifact store: both (OpenSpec + Engram)
Engram topic: `sdd/product-lifecycle-reusable-identifiers/explore`
Engram observation id: 1515

## 1. Problem statement evidence

The user goal is to add a "safe terminal" lifecycle state for products
*distinct* from today's soft-archive, such that SKU and UPC/barcode values can
be released for reuse while preserving the original product row and all
downstream history (`expiry_lots`, `lot_movements`, `lot_resolution_events`,
`notification_log`, `product_categories` junctions).

Evidence that the current codebase does NOT yet support that outcome:

- `openspec/specs/caduxo-expiry-tracker/spec.md` line 18 (`mandatory unique
  SKU`) and line 54 (`barcode uniqueness`) describe the *current* one-state
  uniqueness constraint, with no lifecycle carve-out.
- `src-tauri/src/db/migrations.rs` V2 schema (lines 84, 101) declares both
  `products.sku TEXT NOT NULL UNIQUE` and
  `product_barcodes.barcode TEXT NOT NULL UNIQUE` — there is no partial /
  functional index that excludes any terminal state today.
- `services::products::archive_product` (`src-tauri/src/services/products.rs`
  line 444) and `repo::archive_product` (`src-tauri/src/db/repositories/
  products.rs` line 350) only flip `is_active = 0` and bump `updated_at`.
  No additional `status`/`lifecycle`/`retired_at` column is recorded.
- The frontend i18n key `products.unarchive` exists at
  `src/i18n/en/index.ts:365` ("Unarchive"), but a search of `src/` and
  `src-tauri/src/` returns zero consumers — no backend command, no
  Svelte component, no test. The string is currently dead.
- The only path today that can flip `is_active = 1` is the full edit form
  (`update_product`), which is awkward for a recovery flow.

So the gap is real and well-bounded: there is no "recoverable-archive" vs.
"released-terminal" distinction in either the schema, the service layer,
or the i18n tree, and the existing `Unarchive` string is unreferenced.

## 2. Current archive / unarchive behavior (evidence)

### 2.1 Backend

| Surface | File | Behaviour |
|---|---|---|
| `archive_product` IPC | `commands/products.rs:111` (`#[tauri::command]`) | Thin wrapper over `services::products::archive_product`. |
| `services::products::archive_product` | `services/products.rs:443` | Calls `repo::archive_product`; returns `DomainError::NotFound` if row missing. |
| `repo::archive_product` | `repositories/products.rs:349` | `UPDATE products SET is_active = 0, updated_at = $1 WHERE id = $2`. Single row, no transaction, no audit row, no related-row cleanup. |
| `add_barcode` guard | `services/products.rs:454` | Rejects new barcodes on archived products via `UserMessage::ProductArchivedForBarcode`. |
| `find_product_by_scan` (dashboard) | `services/products.rs:465` | Returns archived products so the catalog can locate them. |
| `find_active_product_by_barcode_exact` / `…by_sku_exact` (scanner) | `repositories/products.rs:625`, `:660` | Hard-filters `is_active = 1`. Archived products never resolve via the scanner. |

There is **no** `unarchive_product` IPC, service, or repository. The only
way to re-activate a product today is to walk the user through
`update_product` with `is_active: true`.

### 2.2 Frontend

| Surface | File | Behaviour |
|---|---|---|
| `ProductDetailPage.svelte` archive UI | `ProductDetailPage.svelte:156` (`confirmArchive`), `:305–:319` (banner, confirmation copy) | Single-step archive with localized `yesArchive` button. No mirror action for recovery. |
| `ProductCatalogPage.svelte` list | `ProductCatalogPage.svelte:354` (`hideArchived` filter), `:507` (`.archived` row class), `:539–:545` (`Archived` badge) | Hide-archived toggle persisted in `localStorage` key `caduxo.products.catalog.hideArchived.v1` (line 65). No terminal-state column. |
| Dashboard product modal | `DashboardPage.svelte:435–:444`, `:795` | Renders `active` / `archived` status; no third state. |
| Product create form | `ProductForm.svelte:101` (line 92 in source) | `isActive` defaults `true`; the user can flip `is_active` in edit mode via a checkbox. |

The current UX makes `archived` the *only* off-state and offers no UX for
recovery or release.

## 3. SKU / UPC uniqueness surface

### 3.1 Schema (V2, `migrations.rs`)

```text
products.sku TEXT NOT NULL UNIQUE                              -- line 84
product_barcodes.barcode TEXT NOT NULL UNIQUE                  -- line 101
```

No partial / functional index. Constraints span active + archived rows.

### 3.2 Read paths that depend on those constraints

| Layer | Query | Uses |
|---|---|---|
| `repo::insert_product` | `products INSERT … sku` | UNIQUE `products.sku` |
| `repo::update_product` | `products UPDATE … sku` | UNIQUE `products.sku` (rejects duplicate SKU on edit) |
| `repo::insert_barcode` | `product_barcodes INSERT … barcode` | UNIQUE `product_barcodes.barcode` |
| `repo::search_products` | `LIKE` over description / sku / barcode | both unique tables |
| `repo::find_by_barcode_exact` / `…by_sku_exact` | exact-match, includes archived | both unique tables |
| `repo::find_active_product_by_barcode_exact` / `…by_sku_exact` | exact-match, active only | both unique tables |
| `services::scanner::resolve_scanner_code` | priority-ordered lookup | both active-only helpers |
| `services::products::find_product_by_scan` | dashboard search surface | both exact helpers |
| `commands::csv_io::import_product_csv` | `DuplicateSku` / `DuplicateBarcode` strategies | both UNIQUE errors |
| `services::csv_io::export_products_csv` | every product by SKU ASC | both tables (archived rows included) |

Any partial uniqueness carve-out therefore touches both lookups and writes.

### 3.3 Error translation (existing)

`services::products::product_unique_error` (line 65) and
`barcode_unique_error` (line 78) convert `UNIQUE constraint failed: …
products.sku` / `…product_barcodes.barcode` into a
`DomainError::DuplicateField { field, value }` so the UI renders a localised
"SKU already exists" / "barcode already exists" notice. These translations
rely on the SQLite error string prefix and will keep working as long as the
column name in the violation message stays `products.sku` or
`product_barcodes.barcode`.

## 4. Product-history dependencies

`ProductDetailPage.svelte` and `DashboardPage.svelte` already assume the
archive-first norm: archiving a product leaves its lots intact. Any
lifecycle change MUST preserve that.

| Dep | File | On archive (today) | On release (proposed) |
|---|---|---|---|
| `expiry_lots` | `migrations.rs:108` | Rows stay. `expiry_lots` has FK `product_id REFERENCES products(id) ON DELETE CASCADE` — but archive is *not* delete. | Must stay readable for the released product. |
| `lot_movements` | `migrations.rs:413` (V9–V17) | Rows stay. | Must stay readable. |
| `lot_resolution_events` | `migrations.rs:139` | Rows stay. | Must stay readable. |
| `notification_log` | `migrations.rs:149` (UNIQUE(expiry_lot_id, notification_date)) | Rows stay. | Must stay readable. |
| `product_categories` junction | `migrations.rs:247` (V4) | `ON DELETE CASCADE` from products, but archive is not delete. | Must stay readable. |
| `product_barcodes` | `migrations.rs:99` | Rows stay. | Must stay readable. |
| CSV export | `services/csv_io.rs:547` (`export_products_csv`) | Archived rows included. | A released product still belongs in the export because its history does. Decision needed in design phase. |
| CSV import | `services/csv_io.rs:737` | `Skip` and `UpdateExisting` strategies use UNIQUE collisions. | Must distinguish "blocked, see archived" from "blocked, see released". This is the most subtle UX risk. |
| Backup / restore | `services/backup_restore.rs:30` (`REQUIRED_TABLES`) | All tables backed up wholesale; `VACUUM INTO` snapshot. | Released-product rows must round-trip via VACUUM INTO without schema drift. |

## 5. Backup / restore implications

`export_backup` uses SQLite `VACUUM INTO` (line 96) — the entire DB is
copied as one consistent snapshot. Any new column added to `products`
or `product_barcodes` MUST either be added via a new sqlx migration
inside `MIGRATIONS` (next index `V19`), or the change must stay column-
free and use only `is_active` semantics.

Because every existing migration in `MIGRATIONS` already pins a SHA-384
checksum that sqlx uses to detect drift (`migrations.rs` V15 comment lines
439–445), bumping a migration is BLOCKED — only additive migrations at a
fresh index are safe. Any DDL change here MUST be `V19` or later.

## 6. Spec coverage gap

Canonical spec `openspec/specs/caduxo-expiry-tracker/spec.md` covers:

- Line 18: `mandatory unique SKU` — single-state contract.
- Line 54: `barcode uniqueness` — single-state contract.
- Line 196: `unit catalog and integer/decimal classification` — mentions
  archive semantics for units ("archiving a unit SHALL be blocked while
  any product still references it via `default_unit_id`").
- Line 267: category archive-first ("archive is preferred over hard
  delete").

There is **no product lifecycle requirement** beyond `is_active`. The
proposal must add a new top-level requirement(s) under `Capability:
Product catalog`. The existing categories/units/lots archive-first
precedent means the proposal should phrase a *parallel* rule for products
("retire is preferred over hard delete; retire is distinct from archive")
rather than overriding the existing single-state SKU/barcode uniqueness.

## 7. UI / surface inventory (focus files)

- `src/components/ProductCatalogPage.svelte` — list view, hide-archived
  toggle, badge "Archived".
- `src/components/ProductDetailPage.svelte` — archive button, history
  list, `confirmArchive` two-step confirm.
- `src/components/ProductForm.svelte` — create / edit, no lifecycle
  surface.
- `src/components/DashboardPage.svelte` — quick product create modal,
  product detail modal with status badge.
- `src/components/ScannerPage.svelte` — scanner tab; uses
  `resolve_scanner_code` (active-only).
- `src/components/CsvImportPage.svelte` — uses duplicate-strategy logic
  that should distinguish "archived" from "released".
- `src/lib/products.ts` — invoke wrappers; current `archiveProduct`
  (line 190), no `unarchiveProduct` or `retireProduct`.
- `src/i18n/en/index.ts`, `src/i18n/es/index.ts` — already-localised
  `unarchive` key (en line 365, no current consumer).

## 8. Open product / business questions to surface for the proposal round

These are the kinds of questions the orchestrator should pose before
the proposal is written. Capturing them in the explore artifact so
the proposal round can borrow from this evidence.

1. **Terminal-state semantics.** Should release be a one-way
   transition (retired is irreversible, even archive can not bring the
   SKU back), or should it allow a "re-bind" recovery flow that points
   the original product row at the new owner? Recommended default:
   one-way terminal, with audit row preserved.

2. **Barcode ownership at release time.** When a product is released
   and its UPC is re-registered by a new product row, what happens to
   historical `product_barcodes` rows pointing at the old product id?
   The schema has `product_barcodes.product_id … ON DELETE CASCADE`,
   which only matters on hard delete. Released rows are still on
   disk — keep them verbatim and let the new row claim the same
   `barcode` value via a partial UNIQUE index that excludes
   terminal-state rows.

3. **History / report exports.** `export_products_csv` writes archived
   and active rows. Should a *released* product appear in the export
   even though it is terminal? Yes is the safe answer (history
   preservation principle) but the spec needs to state it.

4. **Barcode attach guard.** `add_barcode` already rejects archived
   products via `ProductArchivedForBarcode`. Should released products
   also reject new barcode attach? Yes by symmetry; the new
   `BusinessRule` message would be `ProductReleasedForBarcode`.

5. **CSV import preview status.** `DuplicateSku` / `DuplicateBarcode`
   statuses today cannot distinguish "SKU blocked because archived" vs.
   "SKU blocked because released". The proposal should add a
   release-aware diagnostic so the import preview can show a
   non-blocking `ReleasedSku` / `ReleasedBarcode` notice ("the SKU is
   associated with a released product; the new row will succeed").

6. **What does the dashboard search surface return for a released
   SKU?** Today `find_product_by_scan` includes archived rows. Should
   it also include released rows? Recommendation: include released
   rows (they have history) but show a clear "Released" badge so
   the user is not confused. Scanner operations, in contrast,
   already ignore archived rows — the proposal needs to confirm
   that released rows are likewise ignored for the stock-mutation
   path (it would be unsafe to deduct stock against a released
   product).

7. **Migration V19 surface.** Any schema change must live at the
   next available migration index. The Rust migration harness
   refuses checksum drift (`migrations.rs` V15 comment). The
   explore recommendation is to add a single, additive migration
   that (a) introduces a `lifecycle` / `status` column on
   `products`, (b) introduces a sibling column on
   `product_barcodes` if needed, and (c) replaces the simple
   UNIQUE constraints with partial UNIQUE indexes scoped to
   `lifecycle != 'released'`. Schema-aware?

## 9. Risks (preliminary, for the proposal round)

1. **Schema migration risk.** Partial UNIQUE indexes in SQLite
   require `CREATE UNIQUE INDEX … WHERE …` — SQLite supports this but
   the existing migration harness renders DDL via `sqlx::query` (no
   transaction guarantee for `CREATE INDEX`). The V17 migration
   (lines 506–551) provides the right precedent (recreate table +
   copy + indexes, in one transaction per slice). Use the same
   pattern for any products-table rebuild.

2. **Backup checksum drift.** Any existing migration whose SQL bytes
   change will fail the SHA-384 check on every existing user DB.
   The explore evidence is clear (`migrations.rs` V15 comment): do
   NOT edit an existing migration. New columns / indexes go in
   `V19` (and onward).

3. **CSV import semantics.** Distinguishing "archived duplicate"
   from "released duplicate" in the import preview is non-trivial
   and must be designed carefully — both look like UNIQUE failures
   to the runtime. The proposal should reserve status enums on the
   `DuplicateSku` / `DuplicateBarcode` payloads rather than
   overloading existing ones.

4. **Scanned-lot vs. scanned-product interactions.** The scanner
   resolves a lot by `batch_code` first, then a product barcode
   second (PR 1 of `scanner-quick-operations`). A released
   product's lot scans still resolve on the lot branch because
   that lookup ignores `is_active`. The proposal should confirm
   this is the intended behaviour: released-product LOT scans
   still jump to history; released-product BARCODE / SKU scans do
   NOT.

5. **Audit-trail completeness.** If a product transitions
   `active → archived → released`, three audit events should land
   on disk (or at least the released event should, with a stored
   `released_at` and reason). The current schema has no audit
   table for product-lifecycle events — `lot_movements` only
   records stock events, not lifecycle. The proposal should pick:
   (a) add a dedicated `product_lifecycle_events` table, or
   (b) reuse `lot_movements`-style actor/reason rows in a new
   movements table. Option (a) is cleaner.

6. **Front-end surface area.** The catalog list, detail page,
   dashboard modal, and CSV import preview all surface product
   status. A new "released" badge must be added consistently,
   and the hide-archived toggle may need a sibling
   "show-released" toggle for users who want to *see* their
   released products in the catalog.

## 10. Recommended next step (proposal gate)

Stay in the **explore → proposal** hand-off only after the parent
explicitly approves the next phase. Recommended next phase proposal
shape (not implemented in this phase):

1. Carry this explore artifact forward as the scope-of-evidence basis.
2. Open a proposal-round question set on:
   (a) terminal vs. recoverable semantics, (b) barcode attach guard
   parity, (c) dashboard search surfacing, (d) audit-event table,
   (e) hide-archived vs. show-released UX.
3. After the user confirms / adjusts, generate `proposal.md`,
   `design.md`, `tasks.md`, and the spec delta targeting
   `Capability: Product catalog`.

Chained-PR strategy is recommended because the change touches
schema, services, multiple UI surfaces, and i18n parity. The
explore evidence supports a 3-PR chain similar to
`scanner-quick-operations`:

- PR 1: backend foundation (V19 migration, status enum,
  lifecycle service, retired IPC, audit events, tests).
- PR 2: catalog / detail / dashboard UI + i18n keys.
- PR 3: CSV import preview status + backup/restore notes.

The review budget is the project default 800-line cap (the session
cap is 3000). Each PR slice should sit comfortably under 600 net
human-edited LOC to stay clearly under both.
