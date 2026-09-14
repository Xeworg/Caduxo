# Explore: caduxo-measurement-unit-options

## Executive summary

The current Caduxo product/lot model treats units as unstructured free text: `products.default_unit TEXT` and `expiry_lots.unit TEXT NOT NULL`. There is no canonical unit catalog, no validation, and no distinction between integer (count) units and decimal (mass/volume) units. The LotForm quantity input is `step="0.01"` regardless of unit, and `format_qty` in the PDF layer quietly drops trailing zeros when the value is whole.

This SDD change should:

- Expand the unit-of-measure surface so products can pick from a prefixed catalog
  (mass, volume, count, and similar groups) instead of typing free text.
- Introduce a unit "kind" (`integer` vs `decimal`) so a `pcs` / `cajas` product
  shows integer quantities while a `kg` / `L` product shows decimals.
- Treat the integer/decimal split as a product-level (display + form input)
  concern in this slice. Lot-level validation and stock/movement math are
  explicitly out of scope per the confirmed product answers.
- Audit existing rows. When migration runs on a populated database the system
  must surface unrecognized units as a list (e.g. a "needs review" report or an
  inline pre-form warning) rather than silently coercing or rewriting data.

The exploration does not block the proposal step. Two product decisions still
need a dedicated question round (custom-catalog persistence and the exact
integer-vs-decimal question: hard two-bucket taxonomy vs richer axis).

## Confirmed product decisions (from preflight Q&A)

| Area | Decision |
|------|----------|
| Scope of this slice | Both expanded preset catalog **and** integer-vs-decimal unit type |
| Validation scope | Product-level display/UX only — no lot/stock/movement quantity enforcement |
| Unit catalog | Customizable. Allow prefixed units plus custom units from product form OR configuration if practical |
| Migration of existing data | Detect unrecognized units and surface a "needs manual decision" list rather than silently mutating DB rows |

## Existing data model — relevant facts

- `products` table (`src-tauri/src/db/migrations.rs`, V2 schema, ~line 87)
  - `default_unit TEXT` — nullable, no default, no CHECK, no FK.
- `expiry_lots` table (~line 116)
  - `unit TEXT NOT NULL` — required; backend pre-fills `"pcs"` if blank
    (`src-tauri/src/services/expiry_lots.rs::resolve_unit`, line 64).
  - `quantity REAL NOT NULL` — f64, `CHECK(quantity > 0)` only.
- DTOs (`src-tauri/src/dto/products.rs`, `src-tauri/src/dto/expiry_lots.rs`)
  - `ProductCreate.default_unit: Option<String>`, `ProductUpdate` same,
    `ProductResponse.default_unit: Option<String>`.
  - `ExpiryLotCreate.unit: Option<String>` (pre-fills from product default).
  - `ExpiryLotUpdate.unit: String` (required on edit).
- Repository (`src-tauri/src/db/repositories/products.rs`)
  - `insert_product` / `update_product` / `get_product` all pass `default_unit`
    through with no validation.
- CSV import (`src-tauri/src/services/csv_io.rs`)
  - `canonical_field_name` maps `unit`, `defaultunit`, `uom` → `default_unit`.
  - `CsvImportPreviewRow` carries `default_unit` into the product writer with
    no shape validation.
- PDF rendering (`src-tauri/src/pdf/report_pdf.rs`)
  - `format_qty` already drops trailing zero (e.g. `4.0 → "4"`,
    `4.5 → "4.5"`, `0.125 → "0.125"`). Has a unit test
    `format_qty_strips_trailing_zero`.

## Existing UI — relevant facts

- `src/components/ProductForm.svelte`
  - `defaultUnit = ""` (free text), `<input type="text" bind:value={defaultUnit}
    placeholder="e.g. kg, L, unit" />` at lines 328–334.
  - Submit sets `default_unit: defaultUnit.trim() || null`. No datalist, no
    validation, no type selector.
- `src/components/LotForm.svelte`
  - `unit = ""` (free text, pre-fill from product default).
  - `quantity` field is `<input type="number" min="0.01" step="0.01" required>`
    — decimals only today, regardless of unit.
- `src/components/DashboardPage.svelte`
  - Renders `{lot.quantity} {lot.unit}` and the product detail block
    `Default unit: {detailProduct.product.default_unit ?? "—"}`.
- `src/components/ReportsPage.svelte`
  - `formatQty` (line 203) preserves numeric shape; `4 → "4"`, `4.5 → "4.5"` —
    differs from the PDF version only in that it does not strip trailing zeros
    on `.0` like the PDF version does (the number never has them in TS).
- `src/components/CsvImportPage.svelte` and `src/components/ColumnMapper.svelte`
  - Carry `default_unit` through column mapping. No unit-aware UI.

## Spec target

Canonical spec: `openspec/specs/caduxo-expiry-tracker/spec.md`.

The relevant capabilities today:

- `## Capability: Product catalog` (line 16 onwards) — where SKU, multiple
  barcodes, and `default_unit` semantics live.
- `## Capability: Expiry lots` (line 192 onwards) — `Required lot fields`
  already names `unit`; the proposal may add a `uom_catalog` requirement there
  or in Product catalog.

The `local-first` / `non-POS` / `nonGoals` framing in
`openspec/specs/caduxo-expiry-tracker/spec.md` plus the project
`openspec/config.yaml` does **not** need to change.

## PRD expectations

- `docs/prd.md` line 261: `Default unit | No | Example: units, kg, g, L, ml, boxes`
- `docs/prd.md` line 273: `Unit | Yes | Pre-filled from product default when available`
- `docs/prd.md` line 530 (schema doc): `default_unit | TEXT | No | Example: units, box, kg, g, L, ml`
- `docs/prd.md` line 563: `unit | TEXT | Yes | Unit for this lot`

PRD already names example units but never specifies a canonical catalog or a
type axis. The PRD table needs a follow-up edit to remove the free-text
characterization or to point at the catalog managed in the canonical spec.

## Recommended target files (read-only inventory)

| Concern | Path |
|--------|------|
| Migration authoring | `src-tauri/src/db/migrations.rs` (append new V3 migration) |
| Products DTO | `src-tauri/src/dto/products.rs` (`ProductCreate`, `ProductUpdate`, `ProductResponse`) |
| Lot DTO | `src-tauri/src/dto/expiry_lots.rs` (`ExpiryLotCreate.unit` already optional; shape may need to match catalog) |
| Products repo | `src-tauri/src/db/repositories/products.rs` (`insert_product`, `update_product`, `get_product`, query helpers) |
| Lots repo | `src-tauri/src/db/repositories/expiry_lots.rs` (insert/update unchanged for now; per-lot audit if any) |
| Products service | `src-tauri/src/services/products.rs` (`create_product`, `update_product`, validation helpers) |
| Lots service | `src-tauri/src/services/expiry_lots.rs::resolve_unit` (currently falls back to `"pcs"`) |
| CSV import | `src-tauri/src/services/csv_io.rs`, `src-tauri/src/dto/csv_io.rs` |
| PDF format | `src-tauri/src/pdf/report_pdf.rs::format_qty` |
| Frontend types | `src/lib/products.ts`, `src/lib/expiry_lots.ts` |
| Product form | `src/components/ProductForm.svelte` |
| Lot form | `src/components/LotForm.svelte` |
| Dashboard | `src/components/DashboardPage.svelte` (display only, no behavior change required) |
| Reports | `src/components/ReportsPage.svelte` (`formatQty` + display) |
| Resolve dialog | `src/components/ResolveQuantityDialog.svelte` (display only for now) |
| PRD | `docs/prd.md` lines 226 / 261 / 273 / 530 / 563 |
| Canonical spec | `openspec/specs/caduxo-expiry-tracker/spec.md` |
| New docs | New `openspec/changes/caduxo-measurement-unit-options/{proposal,design,tasks}.md` etc. |

## Domain logic — what stays, what moves

Stays:

- `unit` on a lot is still a free-text value for storage and display.
- `quantity REAL` storage and the `validate_quantity(qty > 0)` guard.

Moves:

- `default_unit` becomes either a reference to a catalog unit id OR a validated
  string constrained by the catalog. (Decision pending question round.)
- A new `unit_type` (or equivalent) classifier drives `LotForm` quantity input
  rules (`step="1"` and `min="1"` for `integer`, `step="0.01"` for `decimal`),
  *display-only* per the preflight Q&A.
- The `"pcs"` fallback in `resolve_unit` is replaced with whatever the catalog
  default is, and a missing unit on the product surfaces a validation error in
  the lot form rather than silently filling `"pcs"`.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Existing free-text `default_unit` values don't match the catalog after migration | Some products become "unrecognized" | Run a pre-migration audit script; surface unrecognized units as a "needs review" list; never overwrite existing values silently |
| CSV import carries unrecognized values too | Same risk, bigger surface | CSV row preview flags unrecognized units per-row with a non-blocking badge; user can remap before commit |
| Product `unit_type` is `integer` but lot is stored with decimals (`2.5 pcs`) | Display inconsistency | Per the confirmed Q&A, validation is product-level only; lot stays free for now. Document as accepted gap; track lot-level enforcement as a follow-up SDD |
| Removing the `"pcs"` fallback breaks legacy data | Lot creation fails on legacy products without `default_unit` | Migration assigns a synthetic catalog entry to every existing product that has a recognized unit; unrecognized rows go to the review list; lot creation behavior must degrade gracefully |
| Custom unit catalog storage (table vs settings vs hardcoded) decides backend complexity | Affects Settings page scope | Resolve in question round before proposal |
| Custom unit rename/remove conflicts with existing product references | Stale labels in reports | Defer rename to this slice is enough; removal is blocked when referenced; document in design.md |
| Tests cover only Rust side; frontend form behavior change has no harness today | Engineering safety requirement gap | The canonical spec's `Engineering safety > tests accompany implementation` requirement states this kind of manual-frontend gap is tracked as post-MVP. Record the gap explicitly in tasks.md |
| `product.default_unit` becoming FK or backed by id changes wire format | Breaking change for the JS API | Add the new field as additive and keep `default_unit` text backwards-compatible for one release; document behavior |

## Product questions for the proposal round

Three to five focused questions before drafting `proposal.md`:

1. **Custom unit catalog persistence.** Where does the catalog live?
   - (a) Hardcoded preset list shipped in the app, no user addition.
   - (b) Hardcoded preset list plus a `unit_definitions` SQLite table managed
     from a Settings page (add/disable/rename).
   - (c) Hardcoded preset list plus inline custom unit creation in the Product
     form, stored in the same `unit_definitions` table.
   - (d) Hardcoded preset list plus both (b) and (c).
2. **Unit type taxonomy.** Is "integer vs decimal" the only axis?
   - (a) Two buckets: `integer` (count) and `decimal` (everything else).
   - (b) Multi-axis: `count | mass | volume | length | time | custom`, with
     each preset carrying default `step` and display hints.
3. **Product-level vs lot-level enforcement.** The preflight Q&A says
   product-level only. Confirm:
   - The lot form shows integer controls (`step="1"`) when product unit is
     `integer`, but if the user types `0.5` we accept it (no rejection).
   - Lot storage stays `REAL`; we do not introduce `INTEGER quantity` in this
     slice.
4. **Migration audit UX.** What does the user see on first launch after
   upgrade, when a non-canonical `default_unit` is detected?
   - (a) One-time modal with the list of affected products + per-row "Choose
     catalog match / Leave as custom / Save current text".
   - (b) Inline dashboard banner with a link to a "Needs review" page.
   - (c) Silent — log only, surface later in a future task.
5. **Prefix vs full word in the catalog.** Should presets expose both the
   symbol (`kg`, `L`) and the localized word (`kilogramo`, `litro`)?
   - The Spanish request hints at prefixed words (`prefijadas`). Confirm the
     catalog should seed at minimum `units/pcs`, `boxes/cajas`, `kg`, `g`,
     `L`, `ml`, with extension points for prefixes (`caja`/`cajas`, etc.).

## Open assumptions to call out in the proposal

- Custom units are stored as plain strings, **not** translated; the catalog
  ships with one canonical label per preset.
- Renaming a custom unit is allowed; deleting is blocked when referenced.
- The catalog is read-only preset + add-only custom for this slice; presets
  cannot be removed by users.
- The CSV import preview surfaces per-row unit warnings but does not block
  commit; user can opt-in to a "skip unrecognized" toggle if needed.

## Recommendation

Proceed to the proposal phase. A focused product question round on the five
items above is necessary to lock down the catalog model and the migration UX
before drafting `proposal.md`. After answers, the proposal can be written with
high confidence.

`status.md` should record `explore: complete` and the canonical spec delta
target is `openspec/specs/caduxo-expiry-tracker/spec.md` (Product catalog
and, optionally, Expiry lots capabilities).

## Suggested change directory

`openspec/changes/caduxo-measurement-unit-options/`

Naming follows the existing pattern (`caduxo-{feature-slug}` seen in
`caduxo-dashboard-expiry-filters`, `caduxo-create-product-upc`,
`caduxo-product-identity`). `caduxo-measurement-unit-options` covers both
presets and the integer/decimal split without overspecifying implementation.
