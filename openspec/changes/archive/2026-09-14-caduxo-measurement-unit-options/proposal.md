# Proposal: caduxo-measurement-unit-options

## Intent

Improve measurement units in Caduxo along two axes in the same product slice:

1. **Preset catalog.** Replace the current free-text `default_unit` field with a
   typed catalog that ships with a sensible default set (mass, volume, count
   families) and lets users add their own custom units inline from the product
   form. Once a custom unit exists, it becomes reusable across products.
2. **Integer vs decimal classification.** Each catalog unit carries a single
   classifier — `integer` (count-like, e.g. `units`, `boxes`) or `decimal`
   (mass/volume, e.g. `kg`, `g`, `L`, `ml`). The classifier drives how quantity
   controls render and how quantities are displayed in this slice; storage and
   downstream stock math are explicitly out of scope.

The user-visible outcome: products get a richer, less error-prone unit
selector; lots surface integer quantities naturally (`4 units`) versus decimal
quantities (`0.5 kg`); the app stops silently coercing unrecognized units.

## Scope

### In scope (this slice)

- A `unit_definitions` SQLite table seeded with preset units covering mass,
  volume, and count families. Each row carries:
  - `id` (PK), `key` (stable symbol/code, e.g. `kg`),
  - `display_name` (full localized label, e.g. `kilogramo`),
  - `kind` (`integer` | `decimal`),
  - `is_preset` (boolean — preset vs user-created),
  - `archived_at` (soft-delete timestamp; nullable),
  - `created_at`, `updated_at`.
- New `unit_definitions` rows are created **inline from the product form**
  when a user types a unit not already in the catalog. Inline-created units
  become selectable from any product form from then on.
- A `unit_type` column on `products` derived from the chosen unit's `kind`,
  stored alongside `default_unit_id` (FK to `unit_definitions`). Existing
  `products.default_unit` text column is preserved as a back-compat fallback
  for unrecognized rows.
- ProductForm UX:
  - A `<datalist>`-backed unit input populated from `unit_definitions`
    (display name, with `key` available internally for matching).
  - An inline "Create new unit" affordance inside the form for units not in
    the list, scoped to the catalog (no separate Settings page in this
    slice).
  - Display labels show the full `display_name` (per UX decision), not the
    symbol/shortcode.
- LotForm UX:
  - The `quantity` input renders with `step="1" min="1"` when the product's
    unit kind is `integer`, and `step="0.01"` when it is `decimal`.
  - Storage stays `REAL` for both kinds (per the product-level-only
    decision). The input rules are presentation, not enforcement.
  - Display in dashboard, reports, and PDF uses the full display name.
- Migration audit (first run on existing databases):
  - Query `products.default_unit` for values that don't match any preset
    `key` and are not already known safe aliases (e.g. `units`, `pcs`,
    `cajas`, `kg`, `g`, `L`, `ml`, `box`, `boxes`).
  - Surface a non-blocking dashboard banner with the count and a "Review"
    action. Existing values are **not** rewritten.
- CSV import:
  - Preview rows flag units that don't match any catalog preset with a
    non-blocking per-row badge. The import can still proceed; the user
    decides.
- Canonical spec delta in `openspec/specs/caduxo-expiry-tracker/spec.md`:
  add a new requirement to the `## Capability: Product catalog` capability
  (e.g. `### Requirement: unit catalog and integer/decimal classification`)
  and update `## Capability: Expiry lots` `Required lot fields` only as needed
  to point at the catalog.

### Out of scope (this slice)

- Lot-level quantity validation against product unit kind (no rejection of
  `0.5 pcs` even when product is `integer`).
- Stock, movement, or aggregate math that depends on unit kind.
- Translation of unit display names (single canonical label per preset; full
  i18n follow-up).
- A dedicated Settings page for catalog management (rename, archive,
  re-classify). Inline creation from the product form is the v1 path.
- Removing or re-classifying user-created units. Deletion is blocked when
  referenced; rename is allowed; re-classification (`integer` ↔ `decimal`)
  is not exposed in this slice.
- Changing the `quantity REAL` storage type or introducing a separate
  `integer_quantity` column.
- Any change to the `"pcs"` hardcoded fallback in `resolve_unit` beyond
  what is strictly required by the catalog introduction. The fallback is
  replaced with a catalog lookup that prefers the product's catalog unit
  and only falls back to the `units` preset when the product has none.

## Affected areas

### Data layer (Rust)

- `src-tauri/src/db/migrations.rs` — new migration (V3 or later) creating
  `unit_definitions`, adding `products.default_unit_id` (FK) and
  `products.unit_type` (TEXT, derived). Existing `default_unit` column
  remains.
- `src-tauri/src/db/repositories/products.rs` — `insert_product` and
  `update_product` accept catalog-aware fields; `get_product` returns the
  joined unit metadata.
- New module `src-tauri/src/db/repositories/unit_definitions.rs` (or
  equivalent) — list, create, find-by-key, archive operations.
- `src-tauri/src/services/products.rs` — validation that the chosen unit
  exists, derivation of `unit_type` from the chosen unit.
- `src-tauri/src/services/expiry_lots.rs` — `resolve_unit` consults the
  product's catalog unit; the silent `"pcs"` fallback is removed in favor
  of the `units` preset from the catalog.
- `src-tauri/src/services/csv_io.rs` and CSV import preview DTO — per-row
  unit match status surfaced.
- `src-tauri/src/pdf/report_pdf.rs::format_qty` — display name comes from
  the joined catalog, not from raw `default_unit` text.

### Domain types

- `src-tauri/src/dto/products.rs` — `ProductCreate`, `ProductUpdate`,
  `ProductResponse` gain catalog-aware fields (`default_unit_id`,
  `unit_type`); `default_unit` text remains as a back-compat echo of
  `display_name` for legacy callers.
- `src-tauri/src/dto/expiry_lots.rs` — `ExpiryLotResponse` may include
  the resolved display name; storage shape unchanged.

### Frontend

- `src/lib/products.ts` — product types carry the new fields and unit
  metadata.
- `src/lib/expiry_lots.ts` — quantity rendering uses `unit_type` to pick
  format and to align with the catalog `display_name`.
- `src/components/ProductForm.svelte` — datalist, inline-create affordance.
- `src/components/LotForm.svelte` — quantity `step`/`min` driven by product
  `unit_type`.
- `src/components/DashboardPage.svelte`, `src/components/ReportsPage.svelte`,
  `src/components/ResolveQuantityDialog.svelte` — display uses full display
  name from the catalog.
- `src/components/CsvImportPage.svelte`, `src/components/ColumnMapper.svelte`
  — surface per-row unit match status.
- New component (suggested) `src/components/UnitReviewBanner.svelte` (or
  equivalent) shown on the dashboard when the migration audit finds
  unrecognized units. Offers a "Review" action that navigates to a new
  review page (see below).
- New route/screen (suggested) `src/components/UnitReviewPage.svelte` — one
  row per unrecognized `default_unit` with the same actions the audit
  promised: map to a catalog preset, keep as custom, leave for later.

### Documentation

- `openspec/specs/caduxo-expiry-tracker/spec.md` — new requirement under
  Product catalog; minor edits to Expiry lots required-fields section.
- `docs/prd.md` — lines 226 / 261 / 273 / 530 / 563 should be aligned to
  point at the catalog (specific edits are design-phase work).

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Existing `default_unit` values don't match any preset after migration | Some products become "unrecognized" and show in the audit banner | Migration only adds the catalog table and product columns; existing text is preserved untouched. Audit banner surfaces the count and a "Review" action; nothing is rewritten silently. |
| Catalog FK changes break existing wire format | JS API consumers break | `default_unit_id` and `unit_type` are additive. `default_unit` text is preserved and re-served as a back-compat echo. Documented in design phase. |
| Inline creation produces noisy unit names ("caja de pan 1", "caja de pan 2") | Catalog pollutes quickly | Inline creation normalizes trimmed input, dedupes by case-insensitive key, and surfaces the proposed key for confirmation before insert. |
| CSV import carries unrecognized units | Bad data enters product catalog as inline-created | Preview flags unrecognized units per row; user can opt to skip unrecognized rows via a per-row toggle. Default behavior is "warn and continue", not silent insert. |
| `unit_type` is `integer` but a lot stores `2.5 pcs` | Display inconsistency | Per the confirmed decision, validation is product-level only. Documented gap; tracked as follow-up SDD (lot-level enforcement). Storage stays `REAL`. |
| `quantity` formatting regression in PDF vs Reports page | Trailing zero behavior drifts | Keep `format_qty` semantics intact in both layers; tests accompany the change. |
| Frontend form changes have no harness today | Manual-frontend gap is the project's accepted post-MVP posture | Recorded explicitly in `tasks.md`; tracked per the canonical spec's `Engineering safety > tests accompany implementation` requirement. |
| Banner re-appears on every launch even after user dismisses | Annoyance | Banner persists a dismissed state (e.g. `unit_audit_dismissed_at` in app settings) and only re-appears if a **new** unrecognized value is introduced after dismissal. |

## Rollback

Rollback is a forward-only additive migration up to a point: the migration
is additive (`CREATE TABLE unit_definitions` plus nullable columns on
`products`). Removing the catalog is therefore safe at the schema level.

Application-level rollback:

1. Revert the schema migration only if the new behavior has not been used.
   Existing `default_unit` text remains valid in all rollback paths.
2. If the new behavior has been used:
   - Stop using `default_unit_id` and `unit_type` in the DTOs (revert to
     plain `default_unit: Option<String>`).
   - Remove inline-creation UI and the datalist; restore the free-text
     input on ProductForm.
   - Restore the `resolve_unit` `"pcs"` fallback as the unconditional
     default when `default_unit` is blank.
3. The migration audit banner is removed by the application revert; the
   underlying unrecognized values are still preserved on disk in
   `default_unit` text and become visible again if the user re-runs the
   audit manually.

The canonical spec requirement added in this slice can be reverted by
removing the corresponding block from
`openspec/specs/caduxo-expiry-tracker/spec.md`.

## Success criteria

- A new product can be created with a unit chosen from a preset catalog
  with full-name labels, or by typing a custom unit inline that becomes
  selectable from any product form afterwards.
- For an `integer` unit, LotForm's quantity input uses integer controls; for
  a `decimal` unit, it uses decimal controls. The same product page
  reflects the chosen unit kind without a manual refresh.
- Existing products whose `default_unit` text matches a preset or known
  alias continue to render the same label as before. Nothing on disk is
  rewritten silently.
- On first launch after upgrade, the dashboard surfaces a non-blocking
  banner with the count of unrecognized units and a "Review" action;
  resolving each row keeps the user's intent or maps to a catalog
  preset.
- The canonical spec gains a `Product catalog > unit catalog and
  integer/decimal classification` requirement with at least one scenario
  per rule (catalog seeded, inline creation, classification, migration
  audit, display).
- CSV import preview flags unrecognized units without blocking the import.
- No regression in `format_qty` for whole-number vs decimal values in PDF
  reports.

## Proposal question round (residual assumptions to review)

The orchestrator already collected the five product Q&A items from the
exploration. The following residual assumptions are the only places where
this proposal makes a specific call that may need user confirmation
during design review:

1. **Alias policy.** The migration audit uses a small alias set (`units`,
   `pcs`, `cajas`, `kg`, `g`, `L`, `ml`, `box`, `boxes`) to recognize
   legacy free-text values without rewriting them. This proposal assumes
   that list is acceptable for v1; an extended alias set (e.g. regional
   Spanish abbreviations) is a follow-up if needed.
2. **Banner persistence.** The banner remembers a dismissal until a
   **new** unrecognized unit is introduced. Alternative: re-show on every
   launch. The proposal assumes dismissal-with-staleness.
3. **Inline-creation UX.** Inline creation prompts for the catalog key
   (short symbol) and a full display name. The proposal assumes both
   fields are exposed; alternative is to auto-derive one from the other.
4. **CSV import behavior.** Per-row unrecognized units are flagged but
   still imported. Alternative: block commit when any row has an
   unrecognized unit. The proposal assumes warn-and-continue as the
   default.
5. **Backend surface.** No Settings page in this slice; catalog
   management happens inline from the product form only. Renaming an
   existing custom unit is allowed; re-classifying (`integer` ↔
   `decimal`) is not exposed.

These are documented here for review during the design phase; they are
not expected to reopen the Q&A round the orchestrator already ran.

## Next steps (after this proposal is approved)

1. Move to design (`design.md`): concrete schema migration, DTO shapes,
   the exact preset seed list, the alias set, and the review-page wire
   flow.
2. Move to tasks (`tasks.md`): ordered work units, with the manual-frontend
   test gap explicitly recorded.
3. Spec delta on `openspec/specs/caduxo-expiry-tracker/spec.md` (Product
   catalog capability; Expiry lots required fields if needed).
4. PRD follow-up edits on `docs/prd.md`.
