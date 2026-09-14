# Design: caduxo-measurement-unit-options

## Overview

This design locks the implementation shape for the catalog + integer/decimal classification slice confirmed in the proposal. It targets the existing `packages/coding-agent` workspace (Rust backend in `src-tauri/`, Svelte frontend in `src/`). The artifact store is hybrid (OpenSpec file + Engram observation); both are written from this phase.

The slice is intentionally narrow:

- Add a typed `unit_definitions` SQLite table seeded with a preset list.
- Allow inline custom unit creation from `ProductForm`.
- Carry an `integer` / `decimal` classifier from the catalog into the product and drive LotForm quantity input rules and display labels from it.
- Surface unrecognized legacy units through a banner + review page; never silently rewrite `products.default_unit`.
- Keep `quantity REAL` storage and the existing PDF `format_qty` semantics unchanged.

Out of scope for this slice: lot-level quantity validation, stock/movement math by unit kind, i18n of unit names, dedicated Settings page, re-classification of existing units, removal of units that are still referenced.

## Architecture decisions (locked)

| # | Decision | Rationale |
|---|----------|-----------|
| AD-1 | Unit taxonomy is exactly `integer` vs `decimal` in this version. | Confirmed product decision. Keeps LotForm input rules to two cases and avoids a multi-axis classifier in v1. |
| AD-2 | Catalog persistence lives in a new SQLite table `unit_definitions`. No presets are hardcoded in code; the table is seeded at migration time. | Lets future unit management live in SQL only (no rebuild required), and lets inline creation re-use the same code path. |
| AD-3 | Products gain `default_unit_id` (FK) and `unit_type` (TEXT `integer`/`decimal`, derived). `products.default_unit` TEXT is preserved and re-served as a back-compat echo of `display_name`. | Wire-format additive (proposal §Back-compat). `default_unit` text remains the safety net for legacy callers and unrecognized values. |
| AD-4 | Lot storage stays `REAL`; `lot.unit` remains free text. Quantity input `step`/`min` is presentation-only and driven by `products.unit_type`. | Confirmed product-level-only scope. Avoids a migration of every existing lot value. |
| AD-5 | Custom units are inline-only in this slice. No Settings page, no re-classify, no delete. Rename is allowed via a dedicated command. | Proposal §Out of scope. The inline path keeps the scope to ProductForm + a couple of catalog commands. |
| AD-6 | Legacy aliases are recognised for the audit only; they are not silently applied to `default_unit_id`. | The alias set exists to decide what is "recognized" vs "needs review"; it never overwrites legacy data. |
| AD-7 | Banner dismissal is sticky until a *new* unrecognized unit appears. | Proposal §Banner persistence. Avoids noise after the user explicitly opts out. |
| AD-8 | CSV import preview flags unknown units per row but does not block commit. | Proposal §CSV import. Existing `CsvPreviewRowStatus` gains an `UnknownUnit { suggested_keys }` variant; warn-and-continue is the default. |
| AD-9 | Quantity formatting in PDF and Reports stays exactly as it is today (`format_qty` strips `.0`). Lot storage and PDF text shape are unchanged. | Proposal §Keep storage shape. Avoids a regression in the report PDF when an `integer` unit is rendered. |

## Data layer

### Migration V3

The migration is appended to the `MIGRATIONS` array in `src-tauri/src/db/migrations.rs`. It runs inside sqlx's transaction wrapper (`no_tx: false`) and must be idempotent for the already-applied path (sqlx's `_sqlx_migrations` row tracking covers this; explicit `IF NOT EXISTS` clauses are belt-and-suspenders).

```sql
-- V3 — unit catalog + product catalog linkage.
-- See openspec/changes/caduxo-measurement-unit-options/design.md.

CREATE TABLE unit_definitions (
    id            TEXT PRIMARY KEY,
    key           TEXT NOT NULL UNIQUE,
    display_name  TEXT NOT NULL,
    kind          TEXT NOT NULL CHECK(kind IN ('integer','decimal')),
    is_preset     INTEGER NOT NULL DEFAULT 0,
    archived_at   TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE INDEX idx_unit_definitions_kind_active
    ON unit_definitions(kind)
    WHERE archived_at IS NULL;

-- Additive product linkage. Existing rows keep their default_unit text
-- untouched; default_unit_id is back-filled by the seed step below for any
-- product whose default_unit matches a preset key.
ALTER TABLE products ADD COLUMN default_unit_id TEXT REFERENCES unit_definitions(id);
ALTER TABLE products ADD COLUMN unit_type TEXT;

-- Seed presets (idempotent — keyed on UNIQUE key).
INSERT OR IGNORE INTO unit_definitions (id, key, display_name, kind, is_preset, created_at, updated_at)
VALUES
    ('ud-units',   'units',  'Unidades',      'integer', 1, :now, :now),
    ('ud-pcs',     'pcs',    'Piezas',        'integer', 1, :now, :now),
    ('ud-cajas',   'cajas',  'Cajas',         'integer', 1, :now, :now),
    ('ud-box',     'box',    'Caja',          'integer', 1, :now, :now),
    ('ud-boxes',   'boxes',  'Cajas',         'integer', 1, :now, :now),
    ('ud-bottles', 'bottles','Botellas',      'integer', 1, :now, :now),
    ('ud-bags',    'bags',   'Bolsas',        'integer', 1, :now, :now),
    ('ud-packs',   'packs',  'Paquetes',      'integer', 1, :now, :now),
    ('ud-kg',      'kg',     'Kilogramo',     'decimal', 1, :now, :now),
    ('ud-g',       'g',      'Gramo',         'decimal', 1, :now, :now),
    ('ud-mg',      'mg',     'Miligramo',     'decimal', 1, :now, :now),
    ('ud-tn',      'tn',     'Tonelada',      'decimal', 1, :now, :now),
    ('ud-l',       'L',      'Litro',         'decimal', 1, :now, :now),
    ('ud-ml',      'mL',     'Mililitro',     'decimal', 1, :now, :now);

-- Back-fill default_unit_id / unit_type for existing rows whose default_unit
-- matches a preset key (case-insensitive). Rows that don't match remain in
-- default_unit text only and surface in the audit banner.
UPDATE products
SET default_unit_id = (
        SELECT ud.id FROM unit_definitions ud
        WHERE lower(ud.key) = lower(products.default_unit)
        LIMIT 1
    ),
    unit_type = (
        SELECT ud.kind FROM unit_definitions ud
        WHERE lower(ud.key) = lower(products.default_unit)
        LIMIT 1
    )
WHERE default_unit IS NOT NULL
  AND default_unit_id IS NULL;
```

Notes:

- The id format (`ud-<key>`) is stable and human-readable. It lets us hand-pick well-known ids in tests without a UUID lookup.
- `kind` is enforced by `CHECK` so future migrations cannot accidentally introduce a third bucket.
- The seed uses `INSERT OR IGNORE` so re-applying the migration after a manual catalog tweak is safe.
- The back-fill `UPDATE` deliberately uses `lower()` on both sides. SQLite's default `LIKE` is already case-insensitive for ASCII, but `=` is not; `lower()` is the safe choice and lets us also recognise `KG` vs `kg`.

### Audit / banner query

The banner is fed by a single read-only query exposed as `list_unrecognized_units`:

```sql
SELECT
    p.default_unit         AS raw_value,
    COUNT(*)               AS product_count,
    MIN(p.id)              AS sample_product_id
FROM products p
WHERE p.default_unit_id IS NULL
  AND p.default_unit IS NOT NULL
  AND TRIM(p.default_unit) <> ''
GROUP BY p.default_unit
ORDER BY product_count DESC, raw_value ASC;
```

The "known safe aliases" set (for the audit's `needs_review` boundary only — never for FK back-fill) is:

- integer: `units`, `pcs`, `cajas`, `box`, `boxes`, `bottles`, `bags`, `packs`, `u`, `pza`, `pzas`, `und`
- decimal: `kg`, `g`, `mg`, `tn`, `l`, `ml`, `lb`, `oz`

The alias set is defined as a Rust constant `UNIT_AUDIT_ALIASES: &[(&str, &str)]` (alias → canonical kind) in the unit-definitions module so the audit can recognise values the user may have typed that simply differ in casing or in the Spanish singular/plural form.

> Note: `l` and `ml` are intentionally case-sensitive in the alias table; uppercase `L` and `mL` are already preset keys and the back-fill in V3 handles them. `l` lowercase is an audit-only alias.

### Banner dismissal state

A new key is added to the `app_settings` key-value table:

```
key   = 'unit_audit.dismissed_signature'
value = '<hex hash of the current unrecognized-value list>'
```

The banner query result is hashed (sorted concatenation, then SHA-256 truncated to 16 hex chars) and compared with the stored signature. If they match, the banner is hidden; if they differ (or the key is missing), the banner is shown. The dismissal action writes the current signature.

This means:

- Dismissing the banner after reviewing all rows keeps it hidden.
- Any new unrecognized value introduced via CSV import or a new product bumps the signature and the banner reappears.

## Domain types (Rust DTOs)

### `unit_definitions` module

New DTOs in `src-tauri/src/dto/unit_definitions.rs`:

```rust
pub struct UnitDefinitionResponse {
    pub id: String,
    pub key: String,
    pub display_name: String,
    pub kind: UnitKind,
    pub is_preset: bool,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnitKind {
    Integer,
    Decimal,
}

pub struct UnitDefinitionCreateInput {
    pub key: String,
    pub display_name: String,
    pub kind: UnitKind,
}

pub struct UnitDefinitionRenameInput {
    pub id: String,
    pub display_name: String,
}
```

`UnitKind` is shared with the new `products.unit_type` column via `#[serde(rename_all = "lowercase")]` so the JSON wire shape is the string `integer` / `decimal`.

### Product DTO additions

In `src-tauri/src/dto/products.rs`, every existing `Product*` struct gains two new fields:

```rust
pub default_unit_id: Option<String>,
pub unit_type: Option<UnitKind>,
```

The existing `default_unit: Option<String>` is preserved as a back-compat echo. The server-side contract:

- When a product has `default_unit_id = Some(id)` and the unit still exists (not archived), `default_unit` reflects `display_name`.
- When `default_unit_id = None`, `default_unit` is the raw legacy text the user originally typed.
- `unit_type` is `Some(UnitKind)` when the product is bound to a catalog unit and `None` otherwise. The frontend treats `None` as "decimal" for quantity input purposes (current behavior).

Input DTOs (`ProductCreate`, `ProductUpdate`) accept either `default_unit_id` directly **or** the existing `default_unit` string. The service layer resolves whichever is supplied into the FK + `unit_type` pair.

### Expiry lot DTO

`ExpiryLotResponse` and `ExpiryLotCreate` are unchanged at the wire level. The service layer keeps resolving `unit` text from the product default. A future PR can add `unit_id` to lots; this slice explicitly does not.

### CSV DTO

`CsvPreviewRow` gains no new fields. `CsvPreviewRowStatus` gains:

```rust
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CsvPreviewRowStatus {
    Ok,
    DuplicateSku { ... },
    DuplicateBarcode { ... },
    MissingRequired { ... },
    Invalid { ... },
    UnknownUnit {
        raw_value: String,
        suggested_keys: Vec<String>,
    },
}
```

`UnknownUnit` is non-blocking: the row remains `Ok` for create-time semantics unless another failure applies. The frontend renders a per-row badge; import still commits.

## Domain logic (Rust services)

### `unit_definitions` service (`src-tauri/src/services/unit_definitions.rs`)

Public surface:

- `list_active_units(pool) -> Vec<UnitDefinitionResponse>` — all non-archived units ordered by `kind`, then `display_name`.
- `find_by_key(pool, key) -> Option<UnitDefinitionResponse>` — case-insensitive lookup; used by `resolve_unit`.
- `create_custom_unit(pool, input) -> Result<UnitDefinitionResponse, AppError>` — rejects empty/duplicate key, case-insensitive uniqueness on `key`, kind must be `integer` or `decimal`. Returns `DomainError::DuplicateField { field: "key", value }` on collision.
- `rename_unit(pool, input) -> Result<UnitDefinitionResponse, AppError>` — presets and custom units alike can rename `display_name` only; `key` is immutable in this slice to keep wire formats stable.
- `archive_unit(pool, id) -> Result<(), AppError>` — blocked when any product still references the unit via `default_unit_id`; returns `DomainError::BusinessRule { message: "Unit is still referenced by N product(s)" }`.

### Product service changes (`src-tauri/src/services/products.rs`)

`create_product` and `update_product` gain a unit-resolution step:

1. If input provides `default_unit_id`, look it up; reject if missing or archived.
2. Else if input provides `default_unit` text, call `find_by_key` (case-insensitive). On hit, populate both `default_unit_id` and `default_type` from the catalog row. On miss, leave `default_unit_id = None`, `unit_type = None`, and store the raw text in `default_unit` so the audit can find it.
3. Else (no unit info at all), `default_unit_id = None`, `unit_type = None`, `default_unit = NULL`.

`get_product` and the search queries select the joined catalog columns so `ProductDetailResponse.product.unit_type` is populated without an extra round-trip.

### Lot service changes (`src-tauri/src/services/expiry_lots.rs`)

The `resolve_unit` helper is replaced with a catalog-aware resolver:

```rust
fn resolve_unit_from_catalog(
    user_unit: Option<&str>,
    product_unit_id: Option<&str>,
    fallback: &UnitDefinitionResponse,
) -> String {
    match user_unit {
        Some(u) if !u.trim().is_empty() => u.trim().to_string(),
        _ => fallback.display_name.clone(),
    }
}
```

The hardcoded `"pcs"` literal disappears. When the product has no catalog unit, `fallback` is the seeded `units` preset; this preserves the current default behaviour for legacy products.

### CSV service changes (`src-tauri/src/services/csv_io.rs`)

`classify_row` gains a unit-match step after the existing validations:

```rust
let unit_status = match default_unit.as_deref() {
    Some(raw) => match units_repo::find_by_key(pool, raw.trim()).await? {
        Some(_) => None, // recognized; no extra status
        None => {
            let suggested = units_repo::suggest_similar(pool, raw.trim(), 3).await?;
            Some(UnknownUnit {
                raw_value: raw.trim().to_string(),
                suggested_keys: suggested.into_iter().map(|u| u.key).collect(),
            })
        }
    },
    None => None,
};
```

If `unit_status` is `Some(UnknownUnit { .. })`, `classify_row` returns that status instead of `Ok`. `suggest_similar` is a small helper (case-insensitive prefix match, then contains match) that returns up to 3 candidate keys for the frontend to render.

### Audit / banner service (`src-tauri/src/services/unit_audit.rs`)

Public surface:

- `unrecognized_units(pool) -> Vec<UnrecognizedUnitGroup>` — runs the audit query from §"Audit / banner query" and merges with the alias table so values that map to a kind but lack a preset key are still grouped as "known alias, no catalog entry".
- `current_signature(pool) -> String` — SHA-256 hex of the sorted `raw_value` list.
- `is_banner_dismissed(pool) -> Result<bool, AppError>` — compares `current_signature` against `app_settings` value.
- `dismiss_banner(pool) -> Result<(), AppError>` — writes the current signature.
- `review_action(pool, action: UnitReviewAction) -> Result<(), AppError>` — accepts `MapToPreset { raw_value, preset_id }` (assigns `default_unit_id` + `unit_type` for all matching products), `KeepAsCustom { raw_value }` (creates a custom unit with that key + display name derived from the value, then assigns it to all matching products), `LeaveForLater { raw_value }` (no-op).

The review page wires these together but lives in a separate Tauri command (see §"Backend API surface").

## Backend API surface (Tauri commands)

New commands in `src-tauri/src/commands/unit_definitions.rs`, wired into `commands/mod.rs`:

| Command | Input | Output | Notes |
|---------|-------|--------|-------|
| `list_unit_definitions` | — | `Vec<UnitDefinitionResponse>` | Frontend caches on ProductForm mount. |
| `create_unit_definition` | `UnitDefinitionCreateInput` | `UnitDefinitionResponse` | Used by ProductForm inline-create. |
| `rename_unit_definition` | `UnitDefinitionRenameInput` | `UnitDefinitionResponse` | Out-of-UI in v1; surfaced via the review page. |
| `list_unrecognized_units` | — | `Vec<UnrecognizedUnitGroup>` | Drives the banner + review page. |
| `unit_audit_banner_state` | — | `{ visible: bool, signature: String, group_count: usize }` | Cheap call, used by Dashboard mount. |
| `dismiss_unit_audit_banner` | — | `()` | Persists the current signature. |
| `apply_unit_review_action` | `UnitReviewAction` | `{ updated_product_count: usize }` | Wired from the review page. |

All existing commands keep their current signatures. `create_product` / `update_product` accept the new optional `default_unit_id` field; legacy callers that send only `default_unit` text still work.

`ExpiryLotCreate` and `ExpiryLotUpdate` are not modified.

## Frontend wiring

### Types

`src/lib/products.ts` gains:

```ts
export type UnitKind = "integer" | "decimal";

export interface UnitDefinitionResponse {
  id: string;
  key: string;
  display_name: string;
  kind: UnitKind;
  is_preset: boolean;
  archived_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProductResponse {
  // ... existing fields
  default_unit_id: string | null;
  unit_type: UnitKind | null;
}

export interface ProductCreate {
  // ... existing fields
  default_unit_id?: string | null;
}
```

A new `src/lib/unit_definitions.ts` mirrors the catalog commands.

### `ProductForm.svelte`

- Replaces the free-text `defaultUnit` input with a `<datalist>`-backed input bound to a cached unit list.
- Display label is `display_name` (per UX decision). The `key` is sent only when it differs from `display_name` so the backend can resolve by either.
- An inline "+ Create new unit" affordance opens a small sub-form with two fields: `key` (auto-derived from the typed value, lowercased, slugified) and `display_name` (auto-prefilled with the typed value). The kind selector offers two radio buttons (`Integer`, `Decimal`). On submit, the new unit is created via `createUnitDefinition`, then immediately selected.
- Validation:
  - `display_name` cannot be empty (trimmed).
  - `key` must match `^[a-z0-9_-]{1,16}$` and must not collide (case-insensitive) with existing keys.
  - On `DuplicateField { field: "key" }`, the form re-fetches the catalog and surfaces a recoverable error.

### `LotForm.svelte`

- The `quantity` input becomes:

  ```html
  <input
    type="number"
    bind:value={quantity}
    min={productUnitKind === "integer" ? 1 : 0.01}
    step={productUnitKind === "integer" ? 1 : 0.01}
    required
  />
  ```

- The `productUnitKind` prop is threaded from the parent that opens the form (Dashboard passes `detailProduct.product.unit_type ?? "decimal"`; the quick-create path always uses `decimal` until the product is saved).
- Display label for the resolved unit comes from the catalog (via the existing `default_unit` text back-compat field, which now echoes `display_name`).
- The `unit` text input is hidden when the parent has a catalog unit and shows the resolved display name as a read-only chip; it remains editable for legacy lots whose product has `unit_type = null`.

### `DashboardPage.svelte` / `ReportsPage.svelte`

- The quantity cell renders `{format_qty(quantity)} {display_name}`.
- When the lot is bound to a catalog unit, `display_name` comes from the joined product data; otherwise the raw `lot.unit` is used (unchanged behavior for legacy rows).
- The `format_qty` helper in the frontend mirrors the PDF behavior for whole-number values (`4 → "4"`, `4.5 → "4.5"`). The proposal explicitly preserves both layers.

### `ResolveQuantityDialog.svelte`

- Quantity input rules mirror `LotForm`. The display uses `display_name` when available.

### New components

- `src/components/UnitReviewBanner.svelte` — renders only on the Dashboard, between the scan row and the filter row. Shows the unrecognized unit count and a `Review` button.
- `src/components/UnitReviewPage.svelte` — new screen, mounted from the banner. Lists each unrecognized `raw_value` with the row count and three actions: `Map to preset`, `Create custom unit`, `Leave for later`. The first two invoke `apply_unit_review_action` and refresh the banner state.

The Dashboard receives a new optional `onReviewUnits` prop that the parent (`App.svelte` or whichever shell hosts routing) wires to navigate to the review page. The shell-level routing decision is tracked in `tasks.md` (out of design scope here; we lock the page-level contract).

### `CsvImportPage.svelte`

- Preview rows whose status is `UnknownUnit` render a yellow badge with the suggested keys (up to 3).
- No new toolbar toggle in this slice. Warn-and-continue remains the only behavior; the per-row badge is informational.
- The commit (`import_product_csv`) is unchanged: rows with `UnknownUnit` create products with `default_unit_id = None` and `default_unit = raw_value`, and they will surface on the audit banner after the import lands.

## Canonical spec delta

The canonical spec (`openspec/specs/caduxo-expiry-tracker/spec.md`) gains a new requirement under the existing `## Capability: Product catalog` capability:

```markdown
### Requirement: unit catalog and integer/decimal classification

Caduxo shall expose a typed unit catalog seeded with preset units covering mass, volume, and count families. Each catalog unit carries a stable `key`, a localized `display_name`, and a `kind` classifier of exactly `integer` or `decimal`. Products may reference a catalog unit via `default_unit_id`; the `kind` of the chosen unit drives how LotForm's quantity input renders (`step="1"` and `min="1"` for `integer`, `step="0.01"` for `decimal`). Storage of `quantity` remains `REAL` for both kinds in this slice; lot-level quantity enforcement is explicitly deferred.

The catalog supports inline custom unit creation from the ProductForm. Renaming a unit's `display_name` is allowed; archiving a unit is blocked while it is referenced; re-classifying `kind` is not exposed.

#### Scenario: product picks a preset unit
- GIVEN the catalog contains a preset `kg` with `kind = decimal`
- WHEN the user selects `Kilogramo` from the ProductForm unit datalist
- THEN the product is saved with `default_unit_id` referencing `kg`
- AND `unit_type = decimal` is set on the product

#### Scenario: product creates a custom unit inline
- GIVEN the user types `bandejas` in the ProductForm unit field
- AND `bandejas` is not in the catalog
- WHEN the user clicks "+ Create new unit" and confirms with `kind = integer`
- THEN a new `unit_definitions` row is created
- AND the product is saved with `default_unit_id` referencing the new unit

#### Scenario: integer unit drives LotForm quantity input
- GIVEN a product has `unit_type = integer`
- WHEN the user opens the lot entry form for that product
- THEN the `quantity` input renders with `step="1"` and `min="1"`

#### Scenario: unrecognized legacy units are surfaced
- GIVEN a pre-existing product has `default_unit = "kg."` (typo, no match)
- WHEN the user opens the dashboard after upgrade
- THEN a non-blocking banner shows the count of unrecognized units
- AND the existing `default_unit` text on the product is NOT rewritten

#### Scenario: CSV preview flags unknown units
- GIVEN a CSV row contains `default_unit = "litross"` (typo)
- WHEN the user previews the import
- THEN the row's status includes `UnknownUnit` with up to 3 suggested keys
- AND the import still commits (warn-and-continue)
```

The existing `## Capability: Expiry lots > Requirement: lot registration > Required lot fields` keeps the `unit` requirement unchanged; a clarifying note is added that `unit` is a free-text label whose shape is driven by the product's catalog unit when one is selected.

## PRD follow-up

`docs/prd.md` is updated in tasks.md to:

- Line 261 (`Default unit | No | Example: units, kg, g, L, ml, boxes`) → point to the catalog.
- Line 273 (`Unit | Yes | Pre-filled from product default when available`) → clarify that the prefill now resolves through `default_unit_id`.
- Line 530 (`default_unit | TEXT | No | Example: units, box, kg, g, L, ml`) → add `default_unit_id` (FK) and `unit_type` (TEXT) columns.
- Line 563 (`unit | TEXT | Yes | Unit for this lot`) → unchanged.

The PRD is treated as documentation-only; the canonical spec is the source of truth.

## Test strategy

### Backend (Rust) — covered

The canonical spec's `Engineering safety > tests accompany implementation` requirement applies. The following tests are added or updated:

- **Migration V3** (`src-tauri/src/db/migrations.rs`):
  - `v3_schema_applies_on_fresh_db` — applied_count becomes 3.
  - `unit_definitions_seeded_with_presets` — asserts all 14 preset rows exist with the expected `(key, kind, display_name)` triples.
  - `products_default_unit_id_backfilled_for_known_keys` — seeds a V2 product with `default_unit = "kg"` and asserts `default_unit_id` and `unit_type` are populated after V3.
  - `products_with_unknown_default_unit_remain_unlinked` — seeds a V2 product with `default_unit = "foo"` and asserts `default_unit_id IS NULL`, `default_unit = "foo"`.
  - `v3_migration_is_idempotent` — running migrations twice adds no new rows.

- **Unit definitions service** (`src-tauri/src/services/unit_definitions.rs`):
  - `create_custom_unit_succeeds` / `create_custom_unit_rejects_duplicate_key` (case-insensitive).
  - `create_custom_unit_rejects_empty_display_name`.
  - `rename_unit_updates_display_name_and_keeps_key_stable`.
  - `archive_unit_succeeds_when_unreferenced` / `archive_unit_referenced_returns_business_rule_error`.
  - `list_active_units_excludes_archived`.

- **Product service changes** (`src-tauri/src/services/products.rs`):
  - `create_product_with_catalog_unit_id_sets_unit_type`.
  - `create_product_with_known_default_unit_text_resolves_to_catalog` (e.g. typing `kg` populates `default_unit_id`).
  - `create_product_with_unknown_default_unit_keeps_text_only`.
  - `update_product_clears_catalog_link_when_default_unit_text_removed`.

- **Expiry lot service** (`src-tauri/src/services/expiry_lots.rs`):
  - `create_lot_pre_fills_unit_from_catalog_display_name` — replaces the current `"kg"`-text assertion with a `Kilogramo` assertion when the product has `default_unit_id` set.
  - `create_lot_legacy_product_falls_back_to_units_preset` — product with `default_unit_id IS NULL` and `default_unit = NULL` falls back to `Unidades` (the seeded `units` preset).
  - `create_lot_user_unit_override_wins_over_catalog` — preserved from current tests.

- **Audit / banner service** (`src-tauri/src/services/unit_audit.rs`):
  - `unrecognized_units_groups_by_raw_value`.
  - `current_signature_changes_when_new_unknown_unit_appears`.
  - `dismiss_banner_persists_signature`.
  - `banner_is_hidden_when_signature_matches`.
  - `apply_review_action_map_to_preset_assigns_default_unit_id`.
  - `apply_review_action_keep_as_custom_creates_and_assigns_unit`.

- **CSV service** (`src-tauri/src/services/csv_io.rs`):
  - `preview_flags_unknown_unit_with_suggestions`.
  - `preview_does_not_block_unknown_unit` — the row is still importable.
  - `import_unknown_unit_creates_product_with_text_only` — the resulting product appears in the audit.

### Frontend — manual verification gap

The canonical spec's `tests accompany implementation` requirement has an explicit "accepted for MVP with manual verification and tracked for a dedicated post-MVP frontend harness" clause. The frontend harness is not part of this slice. The following flows are exercised manually and recorded as the verification gap:

- ProductForm datalist + inline unit creation happy path.
- ProductForm inline unit creation with a colliding key (DuplicateField error path).
- LotForm quantity input shape with `unit_type = integer` vs `decimal`.
- Banner re-appears only when a new unrecognized value is introduced.
- CSV import preview shows the `UnknownUnit` badge.
- PDF export still renders `{qty} {display_name}` correctly for both kinds.

This list is copied verbatim into `tasks.md` under "Post-MVP frontend harness follow-up" so the slice notes list the exact flows requiring manual verification and the test commands that were run for the Rust side.

### Test commands to run before merge

- `cd src-tauri && cargo test` — backend unit + migration tests.
- `cd src-tauri && cargo fmt -- --check && cargo clippy --all-targets -- -D warnings` — lint gate.
- The frontend lint/build commands are listed in `tasks.md` per the project's existing CI setup.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Catalog FK + duplicate text columns confuse future readers of the schema. | Onboarding cost; potential for stale reads. | README note in the migration file explains the dual representation. The canonical spec carries the rationale. |
| `unit_definitions` seed inserts collide on a database that already has user-created custom units with the same key (lowercase). | Seed `INSERT OR IGNORE` keeps the user's row. The preset never wins. Documented in the migration comment. |
| `resolve_unit` no longer falls back to `"pcs"`. Existing legacy lots whose product had `default_unit = NULL` and `unit = "pcs"` continue to render `"pcs"` because the lot text is independent. | None at the lot level. The catalog `units` preset only affects the *pre-fill* path for new lots; existing lot text is preserved. |
| Back-fill `UPDATE` is case-insensitive; if a user has both `Kg` and `kg` on different products they map to the same preset. | Minor — the kind is the same so display rendering is identical. The audit query groups by raw text, so the user sees both entries and can rename if desired. |
| `key` immutability prevents future users from renaming `kg → kilogram`. | Users rename `display_name` (`Kilogramo → Kilo`) instead. `key` stability is required for CSV exports and FK integrity. |
| Banner signature hash uses SHA-256; collision risk is negligible but acknowledged. | Truncated to 16 hex chars (64 bits); collision probability is astronomically low for realistic unit lists. |
| Frontend harness gap means UI regressions slip past CI. | Manual checklist + screenshots in the PR description. The post-MVP frontend harness is tracked. |
| PDF `format_qty` strips trailing zero (`4.0 → "4"`). For an `integer` unit this is the right shape, but if a lot is ever saved with `2.5 pcs` the PDF will print `2.5` (decimal) even though the unit is `integer`. | Accepted: storage stays `REAL` in this slice, validation is product-level only. Documented as a follow-up. |
| Inline unit creation produces noisy unit names (`caja de pan 1`, `caja de pan 2`). | Inline form slugifies the typed value into `key` and uses the typed value as `display_name`. The form is intentionally simple — users can rename afterwards if we expose the rename affordance later. |
| Migration ordering: a V2 database with thousands of products on a slow device triggers a slow back-fill UPDATE. | `default_unit_id` and `unit_type` are nullable and indexed; the UPDATE runs once inside the migration transaction. Worst case is O(N rows), which matches the V2 schema size. No background job is required. |

## Rollback

The migration is additive (CREATE TABLE + nullable columns + idempotent seed + back-fill UPDATE). Removing the catalog is safe at the schema level: drop `unit_definitions` and the two product columns.

Application rollback steps mirror the proposal:

1. Revert the migration (`DROP TABLE unit_definitions; ALTER TABLE products DROP COLUMN default_unit_id; ALTER TABLE products DROP COLUMN unit_type;`).
2. Revert DTO/service additions (`default_unit_id` and `unit_type` removed from the wire; `default_unit` text becomes the only unit field on products).
3. Restore the `resolve_unit` `"pcs"` literal fallback.
4. Remove `UnitReviewBanner` and `UnitReviewPage` from the frontend.
5. Remove the `UnknownUnit` CSV preview variant.

The canonical spec requirement added in this slice is reverted by deleting the corresponding `### Requirement: unit catalog and integer/decimal classification` block.

## Open assumptions (locked here, not reopened)

These were captured in the proposal and are restated for the design so the implementation has a single source of truth:

1. The alias set in §"Audit / banner query" is acceptable for v1. Extended regional Spanish abbreviations are a follow-up.
2. Banner dismissal is sticky until the signature changes.
3. Inline creation prompts for both `key` and `display_name`; both fields are exposed.
4. CSV import is warn-and-continue by default; no per-row toggle in this slice.
5. No Settings page; rename only via the review page in this slice; re-classification is not exposed.

## Out of scope confirmation

The following are explicitly NOT in this slice and are tracked as follow-ups:

- Lot-level quantity validation against `unit_type`.
- Stock/movement/aggregate math by unit kind.
- Translation of `display_name` (single canonical label per preset; i18n follow-up).
- A dedicated Settings page for catalog management.
- Removing or re-classifying user-created units.
- Adding `INTEGER quantity` storage.
- A frontend harness for the ProductForm/LotForm changes (manual-frontend gap accepted per the canonical spec).
