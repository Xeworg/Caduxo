# Tasks: caduxo-measurement-unit-options

Change: `caduxo-measurement-unit-options`
Source of truth: `openspec/changes/caduxo-measurement-unit-options/proposal.md`, `openspec/changes/caduxo-measurement-unit-options/design.md`, `openspec/specs/caduxo-expiry-tracker/spec.md`.

This slice is intentionally narrow: typed `unit_definitions` SQLite table seeded with preset units, inline custom unit creation from `ProductForm`, integer/decimal classifier flowing into `LotForm` input rules and display labels, a banner + review page for unrecognized legacy units, and CSV preview surfacing `UnknownUnit`. `quantity` storage stays `REAL` and the PDF `format_qty` semantics are preserved.

`openspec/config.yaml` declares `strictTdd: false` and there is no Svelte component harness today, so the order below uses RED→GREEN for the Rust backend (where `cargo test` is the harness) and records manual frontend verification explicitly as a tracked gap per the canonical spec's `Engineering safety > tests accompany implementation` requirement.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1500–2000 (Backend Rust ~700, Frontend Svelte/TS ~600, Docs ~200; well under 3000-line session budget). |
| 3000-line budget risk | Low (≈50–65% of the 3000-line session budget; additive migration, no cross-cutting rewrites, no generated artifacts). |
| Chained PRs recommended | No (single coherent slice fits in a single PR; backend+frontend+docs coupling is tight; splitting would fragment one logical change). |
| Suggested split | Single PR. |
| Delivery strategy | auto-chain (per session preflight; chain strategy is `pending` because no risk currently justifies slicing). |
| Chain strategy | pending (deferred unless a future review surfaces >3000-line risk or >400-line canonical threshold concerns; user preflight allows deferring chain strategy). |

```text
Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low
```

## Scope summary

- **Backend (Rust):** V3 migration (`unit_definitions` table, idempotent seed, `products.default_unit_id` + `products.unit_type` columns, case-insensitive back-fill `UPDATE`); new `unit_definitions` repository + service + DTOs; product service unit-resolution step; lot service replaces the hardcoded `"pcs"` fallback with catalog lookup; CSV service adds an `UnknownUnit` preview status; new `unit_audit` service with banner signature/dismissal/review actions; seven new Tauri commands wired into the command registry.
- **Frontend (Svelte/TS):** types in `src/lib/products.ts` and new `src/lib/unit_definitions.ts`; `ProductForm` datalist + inline unit creation; `LotForm` quantity input rules; display-only updates to `DashboardPage`, `ReportsPage`, `ResolveQuantityDialog`; new `UnitReviewBanner` + `UnitReviewPage`; `CsvImportPage` per-row `UnknownUnit` badge.
- **Docs:** new requirement under `## Capability: Product catalog` in `openspec/specs/caduxo-expiry-tracker/spec.md`; clarifying note under `## Capability: Expiry lots > Required lot fields`; PRD follow-up on `docs/prd.md` lines 261/273/530 (line 563 unchanged).
- **Migration safety:** additive migration only; existing `products.default_unit` text is preserved; the back-fill `UPDATE` is case-insensitive and never overwrites unrecognized values.

## Affected files (read-only inventory from design.md)

- `src-tauri/src/db/migrations.rs` — append V3 migration; extend `MIGRATIONS` array; new migration tests.
- `src-tauri/src/db/repositories/{mod.rs,unit_definitions.rs}` (new) — catalog CRUD + `suggest_similar`.
- `src-tauri/src/db/repositories/products.rs` — joined select for `default_unit_id` + `unit_type`; new insert/update fields.
- `src-tauri/src/dto/{mod.rs,unit_definitions.rs}` (new) — `UnitKind`, `UnitDefinitionResponse`, `UnitDefinitionCreateInput`, `UnitDefinitionRenameInput`.
- `src-tauri/src/dto/products.rs` — add `default_unit_id`, `unit_type` to `ProductCreate`/`ProductUpdate`/`ProductResponse`.
- `src-tauri/src/dto/csv_io.rs` — add `UnknownUnit { raw_value, suggested_keys }` variant.
- `src-tauri/src/services/{mod.rs,unit_definitions.rs,unit_audit.rs}` (new), `products.rs` (extend), `expiry_lots.rs` (replace `resolve_unit`), `csv_io.rs` (extend `classify_row`).
- `src-tauri/src/commands/{mod.rs,unit_definitions.rs}` (new) — seven new Tauri commands.
- `src/lib/products.ts` — add `UnitKind`, `UnitDefinitionResponse`, `default_unit_id`/`unit_type`.
- `src/lib/unit_definitions.ts` (new) — typed wrappers for the seven commands.
- `src/components/{ProductForm.svelte,LotForm.svelte,DashboardPage.svelte,ReportsPage.svelte,ResolveQuantityDialog.svelte,CsvImportPage.svelte}` — UI changes.
- `src/components/{UnitReviewBanner.svelte,UnitReviewPage.svelte}` (new) — dashboard banner + review page.
- `openspec/specs/caduxo-expiry-tracker/spec.md` — new requirement + clarifying note.
- `docs/prd.md` — follow-up edits.

---

## Implementation work

Each checkbox below ends with exactly one ownership marker. Implementation tasks use `sdd-owner: implementation`; parent-owned lifecycle gates are grouped in a single section after implementation work.

### Section A — Data layer foundation (Backend)

Strict TDD ordering: RED test → GREEN impl → refactor pass. Every RED test must fail on its own before the matching impl lands. Backend harness is `cargo test` (`src-tauri/src/services/*/mod tests`, `src-tauri/src/db/migrations.rs::tests`).

#### A.1 V3 migration shell + RED tests

- [x] Add a `v3_schema_applies_on_fresh_db` test in `src-tauri/src/db/migrations.rs::tests` that runs `run_migrations(&fresh_test_pool())` on a fresh DB and asserts `count_applied_migrations(&pool) == 3`. The test must fail before the V3 entry is appended. <!-- sdd-owner: implementation -->
- [x] Add a `v3_migration_is_idempotent` test that runs `run_migrations` twice on a fresh DB and asserts no new `_sqlx_migrations` rows are added on the second pass. The test must fail before the V3 entry is appended. <!-- sdd-owner: implementation -->

#### A.2 V3 migration GREEN

- [x] Append the V3 entry to `MIGRATIONS` in `src-tauri/src/db/migrations.rs` per design.md §"Migration V3": CREATE TABLE `unit_definitions` (id, key UNIQUE, display_name, kind CHECK `('integer','decimal')`, is_preset, archived_at, created_at, updated_at) with `idx_unit_definitions_kind_active` partial index on `kind WHERE archived_at IS NULL`; ALTER TABLE `products` ADD COLUMN `default_unit_id TEXT REFERENCES unit_definitions(id)` and `unit_type TEXT`; idempotent preset seed (14 rows: `ud-units`, `ud-pcs`, `ud-cajas`, `ud-box`, `ud-boxes`, `ud-bottles`, `ud-bags`, `ud-packs`, `ud-kg`, `ud-g`, `ud-mg`, `ud-tn`, `ud-l`, `ud-ml`); case-insensitive back-fill `UPDATE` for `default_unit_id` + `unit_type` on existing rows whose `default_unit` matches a preset key. <!-- sdd-owner: implementation -->
- [x] Confirm `v3_schema_applies_on_fresh_db` and `v3_migration_is_idempotent` pass after the V3 entry is in place. <!-- sdd-owner: implementation -->

#### A.3 Seed + back-fill behavior RED/GREEN

- [x] Add a `unit_definitions_seeded_with_presets` test asserting all 14 preset rows exist with the exact `(key, kind, display_name)` triples from design.md. <!-- sdd-owner: implementation -->
- [x] Add a `products_default_unit_id_backfilled_for_known_keys` test that seeds a V2 product with `default_unit = "kg"`, runs V3, and asserts `default_unit_id == Some("ud-kg".to_string())` and `unit_type == Some("decimal".to_string())`. <!-- sdd-owner: implementation -->
- [x] Add a `products_with_unknown_default_unit_remain_unlinked` test that seeds a V2 product with `default_unit = "foo"`, runs V3, and asserts `default_unit_id IS NULL`, `unit_type IS NULL`, `default_unit == "foo"`. <!-- sdd-owner: implementation -->

### Section B — Unit definitions catalog (Backend)

#### B.1 DTO + repository RED/GREEN

- [x] Add the `UnitKind` enum and `UnitDefinitionResponse` / `UnitDefinitionCreateInput` / `UnitDefinitionRenameInput` DTOs in `src-tauri/src/dto/unit_definitions.rs` (new file) per design.md §"`unit_definitions` module". `UnitKind` must `#[serde(rename_all = "lowercase")]` so the JSON wire shape is `integer` / `decimal`. <!-- sdd-owner: implementation -->
- [x] Register `pub mod unit_definitions;` in `src-tauri/src/dto/mod.rs`. <!-- sdd-owner: implementation -->
- [x] Create `src-tauri/src/db/repositories/unit_definitions.rs` with `list_active`, `find_by_key` (case-insensitive), `insert`, `rename`, `archive`, `is_referenced`, and `suggest_similar` (case-insensitive prefix match, then contains match; returns up to 3). <!-- sdd-owner: implementation -->
- [x] Register `pub mod unit_definitions;` in `src-tauri/src/db/repositories/mod.rs`. <!-- sdd-owner: implementation -->

#### B.2 `unit_definitions` service RED/GREEN

- [x] Add RED tests in `src-tauri/src/services/unit_definitions.rs::tests` (new module, registered in `src-tauri/src/services/mod.rs`): `create_custom_unit_succeeds`, `create_custom_unit_rejects_duplicate_key` (case-insensitive, returns `DomainError::DuplicateField { field: "key" }`), `create_custom_unit_rejects_empty_display_name`, `rename_unit_updates_display_name_and_keeps_key_stable`, `archive_unit_succeeds_when_unreferenced`, `archive_unit_referenced_returns_business_rule_error`, `list_active_units_excludes_archived`. <!-- sdd-owner: implementation -->
- [x] GREEN: implement the seven service functions per design.md §"`unit_definitions` service". Archive returns `DomainError::BusinessRule { message: "Unit is still referenced by N product(s)" }` when `is_referenced` is true. <!-- sdd-owner: implementation -->

### Section C — Product catalog linkage (Backend)

#### C.1 DTO + repository extensions

- [x] Extend `ProductCreate`, `ProductUpdate`, and `ProductResponse` in `src-tauri/src/dto/products.rs` with `default_unit_id: Option<String>` and `unit_type: Option<UnitKind>` (reusing `UnitKind` from `dto::unit_definitions`). `default_unit: Option<String>` remains as a back-compat echo per design.md §"Product DTO additions". <!-- sdd-owner: implementation -->
- [x] Update `src-tauri/src/db/repositories/products.rs`: `insert_product` writes `default_unit_id` + `unit_type`; `update_product` writes the same; `get_product` and the search queries `SELECT` the joined catalog columns so `unit_type` is populated without an extra round-trip. <!-- sdd-owner: implementation -->

#### C.2 Product service unit-resolution RED/GREEN

- [x] Add RED tests in `src-tauri/src/services/products.rs::tests`: `create_product_with_catalog_unit_id_sets_unit_type`, `create_product_with_known_default_unit_text_resolves_to_catalog`, `create_product_with_unknown_default_unit_keeps_text_only`, `update_product_clears_catalog_link_when_default_unit_text_removed`. <!-- sdd-owner: implementation -->
- [x] GREEN: extend `create_product` and `update_product` in `src-tauri/src/services/products.rs` with the three-step resolver per design.md §"Product service changes" (id path → text path via `find_by_key` → no path leaves both columns null). <!-- sdd-owner: implementation -->

### Section D — Lot service refactor (Backend)

- [x] Add RED test `create_lot_pre_fills_unit_from_catalog_display_name` in `src-tauri/src/services/expiry_lots.rs::tests`: a product with `default_unit_id = Some("ud-kg")` and a lot create with `unit = None` yields `lot.unit == "Kilogramo"` (catalog display name), not `"kg"` text. <!-- sdd-owner: implementation -->
- [x] Add RED test `create_lot_legacy_product_falls_back_to_units_preset`: a product with `default_unit_id = None` and `default_unit = None` falls back to `Unidades` (the seeded `units` preset) on lot create. <!-- sdd-owner: implementation -->
- [x] GREEN: replace the `resolve_unit` helper in `src-tauri/src/services/expiry_lots.rs` (currently at line 64 with the hardcoded `"pcs"` fallback) with `resolve_unit_from_catalog` per design.md §"Lot service changes". The hardcoded `"pcs"` literal disappears; `fallback` is the seeded `units` preset when the product has no catalog unit. Existing `create_lot_user_unit_override_wins_over_catalog` test must still pass without modification. <!-- sdd-owner: implementation -->

### Section E — CSV import awareness (Backend)

- [x] Add `UnknownUnit { raw_value: String, suggested_keys: Vec<String> }` variant to `CsvPreviewRowStatus` in `src-tauri/src/dto/csv_io.rs` using the existing `#[serde(tag = "kind", rename_all = "snake_case")]` shape. `CsvPreviewRow` gains no new fields. <!-- sdd-owner: implementation -->
- [x] Add RED tests in `src-tauri/src/services/csv_io.rs::tests`: `preview_flags_unknown_unit_with_suggestions` (a row with `default_unit = "litro"` returns `UnknownUnit { raw_value, suggested_keys: ["L"] }` via `suggest_similar` prefix match), `preview_does_not_block_unknown_unit`, `import_unknown_unit_creates_product_with_text_only`. <!-- sdd-owner: implementation -->
- [x] GREEN: extend `classify_row` in `src-tauri/src/services/csv_io.rs` with the unit-match step per design.md §"CSV service changes". Rows with `UnknownUnit` remain importable; the resulting product appears in the audit banner. <!-- sdd-owner: implementation -->

### Section F — Audit / banner service (Backend)

- [x] Create `src-tauri/src/services/unit_audit.rs` (new module, registered in `src-tauri/src/services/mod.rs`) per design.md §"Audit / banner service" with the public surface: `unrecognized_units`, `current_signature` (SHA-256 of the sorted raw-value list, truncated to 16 hex chars), `is_banner_dismissed`, `dismiss_banner` (writes `unit_audit.dismissed_signature` to `app_settings`), `review_action` accepting `UnitReviewAction::MapToPreset { raw_value, preset_id }`, `UnitReviewAction::KeepAsCustom { raw_value }`, and `UnitReviewAction::LeaveForLater { raw_value }`. <!-- sdd-owner: implementation -->
- [x] Define `UNIT_AUDIT_ALIASES: &[(&str, &str)]` (alias → canonical kind) at module top per design.md §"Audit / banner query": integer set (`units`, `pcs`, `cajas`, `box`, `boxes`, `bottles`, `bags`, `packs`, `u`, `pza`, `pzas`, `und`); decimal set (`kg`, `g`, `mg`, `tn`, `l`, `ml`, `lb`, `oz`). Aliases are audit-only and never used to silently back-fill `default_unit_id`. <!-- sdd-owner: implementation -->
- [x] Add RED tests in `src-tauri/src/services/unit_audit.rs::tests`: `unrecognized_units_groups_by_raw_value`, `current_signature_changes_when_new_unknown_unit_appears`, `banner_is_hidden_when_signature_matches`, `dismiss_banner_persists_signature`, `apply_review_action_map_to_preset_assigns_default_unit_id`, `apply_review_action_keep_as_custom_creates_and_assigns_unit`. <!-- sdd-owner: implementation -->

### Section G — Tauri commands (Backend)

- [x] Create `src-tauri/src/commands/unit_definitions.rs` (new module, registered in `src-tauri/src/commands/mod.rs`) with seven commands per design.md §"Backend API surface": `list_unit_definitions`, `create_unit_definition`, `rename_unit_definition`, `list_unrecognized_units`, `unit_audit_banner_state`, `dismiss_unit_audit_banner`, `apply_unit_review_action`. <!-- sdd-owner: implementation -->
- [x] Wire all seven commands into the command registry (`register_commands!` or the project's equivalent pattern in `src-tauri/src/main.rs` / `commands/mod.rs`). Confirm `tauri::generate_handler!` compiles cleanly. <!-- sdd-owner: implementation -->
- [x] Ensure all existing commands keep their current signatures. `create_product` / `update_product` accept the new optional `default_unit_id` field; legacy callers that send only `default_unit` text still work. `ExpiryLotCreate` and `ExpiryLotUpdate` are not modified. <!-- sdd-owner: implementation -->

### Section H — Frontend types (Frontend)

- [x] Update `src/lib/products.ts` per design.md §"Types": add `export type UnitKind = "integer" | "decimal"`, add `export interface UnitDefinitionResponse`, add `default_unit_id: string | null` and `unit_type: UnitKind | null` to `ProductResponse`, add `default_unit_id?: string | null` to `ProductCreate`. <!-- sdd-owner: implementation -->
- [x] Create `src/lib/unit_definitions.ts` with typed wrappers for the seven new Tauri commands: `listUnitDefinitions`, `createUnitDefinition`, `renameUnitDefinition`, `listUnrecognizedUnits`, `unitAuditBannerState`, `dismissUnitAuditBanner`, `applyUnitReviewAction`. Each wrapper mirrors the Rust DTO shape (camelCase wire → snake_case via existing `invoke` pattern). <!-- sdd-owner: implementation -->
- [x] Run `npm run check` (or the project's TypeScript check command per existing CI) and confirm types compile cleanly. <!-- sdd-owner: implementation -->

### Section I — ProductForm datalist + inline unit creation (Frontend)

No frontend test harness exists. These tasks are implemented against existing types and verified manually in Section M.

- [x] Replace the free-text `defaultUnit` input in `src/components/ProductForm.svelte` (lines 328–334 per explore.md) with a `<datalist>`-backed input bound to a cached unit list. Display label is `display_name` per design.md §"`ProductForm.svelte`". <!-- sdd-owner: implementation -->
- [x] Add an inline "+ Create new unit" affordance that opens a small sub-form with two fields: `key` (auto-derived from the typed value, lowercased, slugified) and `display_name` (auto-prefilled with the typed value), plus a kind selector with two radio buttons (`Integer`, `Decimal`). On submit, call `createUnitDefinition`, then immediately select the new unit. <!-- sdd-owner: implementation -->
- [x] Add validation per design.md §"`ProductForm.svelte`": `display_name` cannot be empty (trimmed); `key` must match `^[a-z0-9_-]{1,16}$` and must not collide (case-insensitive) with existing keys; on `DuplicateField { field: "key" }`, refresh the catalog and surface a recoverable error message. <!-- sdd-owner: implementation -->

### Section J — LotForm quantity input rules (Frontend)

- [x] Update `src/components/LotForm.svelte` quantity input per design.md §"`LotForm.svelte`": `min={productUnitKind === "integer" ? 1 : 0.01}`, `step={productUnitKind === "integer" ? 1 : 0.01}`, `required`. <!-- sdd-owner: implementation -->
- [x] Thread `productUnitKind: "integer" | "decimal"` as a prop on `LotForm`. Dashboard passes `detailProduct.product.unit_type ?? "decimal"`; the quick-create path always uses `"decimal"` until the product is saved. <!-- sdd-owner: implementation -->
- [x] Hide the `unit` text input when the parent has a catalog unit and show the resolved display name as a read-only chip; keep the input editable for legacy lots whose product has `unit_type = null`. <!-- sdd-owner: implementation -->

### Section K — Display updates (Frontend)

- [x] Update `src/components/DashboardPage.svelte`, `src/components/ReportsPage.svelte`, and `src/components/ResolveQuantityDialog.svelte` per design.md §"`DashboardPage.svelte` / `ReportsPage.svelte`" and §"`ResolveQuantityDialog.svelte`": quantity cell renders `{format_qty(quantity)} {display_name}`. When the lot is bound to a catalog unit, `display_name` comes from joined product data; otherwise the raw `lot.unit` is used (unchanged behavior for legacy rows). <!-- sdd-owner: implementation -->
- [x] Confirm the existing `format_qty` test in `src-tauri/src/pdf/report_pdf.rs` (`format_qty_strips_trailing_zero`) still passes and that `src/components/ReportsPage.svelte`'s `formatQty` (line 203) mirrors the same shape for whole-number values. Storage stays `REAL`; no regression in PDF text rendering for `integer` units. <!-- sdd-owner: implementation -->

### Section L — UnitReviewBanner + UnitReviewPage (Frontend)

- [x] Create `src/components/UnitReviewBanner.svelte` per design.md §"New components": renders only on the Dashboard, between the scan row and the filter row. Shows the unrecognized unit count and a `Review` button. The banner query result is hashed and compared against `app_settings.unit_audit.dismissed_signature`; if the signature matches, the banner is hidden. The dismissal action writes the current signature. <!-- sdd-owner: implementation -->
- [x] Create `src/components/UnitReviewPage.svelte` per design.md §"New components": lists each unrecognized `raw_value` with row count and three actions (`Map to preset`, `Create custom unit`, `Leave for later`). The first two invoke `applyUnitReviewAction` and refresh the banner state via `unitAuditBannerState`. <!-- sdd-owner: implementation -->
- [x] Add a `unit_audit_banner_state` query call on Dashboard mount; expose an `onReviewUnits` prop on the dashboard that the parent shell (`App.svelte` or the project's routing shell) wires to navigate to the new review page. Confirm shell-level routing handles the new page mount/unmount cleanly. <!-- sdd-owner: implementation -->

### Section M — CSV import badge (Frontend)

- [x] Update `src/components/CsvImportPage.svelte` to render an `UnknownUnit` badge per row with the suggested keys (up to 3) when `status.kind === "unknown_unit"`. No new toolbar toggle in this slice; warn-and-continue remains the default behavior. The commit (`import_product_csv`) is unchanged: rows with `UnknownUnit` create products with `default_unit_id = None` and `default_unit = raw_value`, and they surface in the audit banner after the import lands. <!-- sdd-owner: implementation -->

### Section N — Canonical spec delta (Docs)

- [x] Append the `### Requirement: unit catalog and integer/decimal classification` block to `## Capability: Product catalog` in `openspec/specs/caduxo-expiry-tracker/spec.md` per design.md §"Canonical spec delta". The block must include the full requirement text and all five scenarios (`product picks a preset unit`, `product creates a custom unit inline`, `integer unit drives LotForm quantity input`, `unrecognized legacy units are surfaced`, `CSV preview flags unknown units`). <!-- sdd-owner: implementation -->
- [x] Add a clarifying note under `## Capability: Expiry lots > Requirement: lot registration > Required lot fields` that `unit` is a free-text label whose shape is driven by the product's catalog unit when one is selected. <!-- sdd-owner: implementation -->

### Section O — PRD follow-up (Docs)

- [x] Update `docs/prd.md` line 261 (`Default unit | No | Example: units, kg, g, L, ml, boxes`) to point at the catalog (e.g. "chosen from the unit catalog — see capability spec"). <!-- sdd-owner: implementation -->
- [x] Update `docs/prd.md` line 273 (`Unit | Yes | Pre-filled from product default when available`) to clarify the prefill now resolves through `default_unit_id`. <!-- sdd-owner: implementation -->
- [x] Update `docs/prd.md` line 530 (schema doc) to add `default_unit_id` (FK to `unit_definitions`) and `unit_type` (TEXT, `integer` | `decimal`) columns alongside the existing `default_unit` (TEXT) row. <!-- sdd-owner: implementation -->
- [x] Confirm `docs/prd.md` line 563 (`unit | TEXT | Yes | Unit for this lot`) remains unchanged. <!-- sdd-owner: implementation -->

### Section P — Verification gates (Backend Rust)

- [x] Run `cd src-tauri && cargo test` and confirm all new tests in Sections A–G pass plus the existing `format_qty_strips_trailing_zero` and the existing lot/product/CSV tests still pass with no regressions. <!-- sdd-owner: implementation -->
- [x] Run `cd src-tauri && cargo fmt -- --check && cargo clippy --all-targets -- -D warnings` and confirm a clean lint pass. <!-- sdd-owner: implementation -->
- [x] Run the project's existing frontend lint/build command (`npm run check` or equivalent per existing CI) and confirm a clean pass for the type changes in `src/lib/products.ts` and `src/lib/unit_definitions.ts`. <!-- sdd-owner: implementation -->

### Section Q — Post-MVP frontend harness follow-up (Recorded gap)

The following list is recorded verbatim from design.md §"Frontend — manual verification gap". Per the canonical spec's `Engineering safety > tests accompany implementation` requirement, this is the project's accepted post-MVP posture for this slice.

- [x] Add a post-MVP backlog entry (e.g. to `openspec/changes/archive/<date>-caduxo-measurement-unit-options/tasks.md` at archive time, or to the project's standing post-MVP backlog) recording that the six manual-frontend flows below need a dedicated Svelte component harness. <!-- sdd-owner: implementation -->

Manual-frontend flows requiring the future harness:

- ProductForm datalist + inline unit creation happy path.
- ProductForm inline unit creation with a colliding key (DuplicateField error path).
- LotForm quantity input shape with `unit_type = integer` vs `decimal`.
- Banner re-appears only when a new unrecognized value is introduced.
- CSV import preview shows the `UnknownUnit` badge.
- PDF export still renders `{qty} {display_name}` correctly for both kinds.

### Section R — Documentation note for archive

- [x] When `openspec/changes/caduxo-measurement-unit-options/status.md` is created during the apply phase, include a note that this change closes the post-MVP backlog item "presets + integer/decimal split for measurement units" and that any lot-level quantity enforcement (the documented gap from this slice) remains an open follow-up SDD. <!-- sdd-owner: implementation -->

---

## Lifecycle gates (parent-owned)

These are parent-owned lifecycle notes, not implementation checkboxes. They must not block native `apply` completion.

- Bounded review: native review inspect stopped with `rdd_disabled`; no lineage was started. Follow ordinary repository policy unless review mode is re-enabled.
- Apply materialization: `sdd-apply` implemented the change and captured evidence in `openspec/changes/caduxo-measurement-unit-options/apply-progress.md`.
- Verify next: run `sdd-verify` after apply; confirm the five new scenarios (`product picks a preset unit`, `product creates a custom unit inline`, `integer unit drives LotForm quantity input`, `unrecognized legacy units are surfaced`, `CSV preview flags unknown units`) against diff + manual smoke evidence.
- Archive later: run `sdd-archive` once verify passes; ensure the archive report records the post-MVP manual-frontend gap and that existing canonical requirements (`mandatory unique SKU`, `multiple barcodes per product`, `barcode uniqueness`, `lot registration`) remain intact.
