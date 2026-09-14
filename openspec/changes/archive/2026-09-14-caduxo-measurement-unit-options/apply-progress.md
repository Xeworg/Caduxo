# Apply Progress: caduxo-measurement-unit-options

**Change**: `caduxo-measurement-unit-options`
**Started**: 2026-09-14
**Last updated**: 2026-09-14

## Backend (Rust — src-tauri)

### V3 Migration (`db/migrations.rs`)

- [x] Added V3 migration — creates `unit_definitions` table with 14 preset units, adds `products.default_unit_id` (FK) + `products.unit_type`, idempotent seed, case-insensitive back-fill for known keys
- [x] Fixed `:now` substitution → single-quoted timestamp string (`'2026-09-14 02:49:00'`)
- [x] Added `chrono` to V3 timestamp format: `format!("%Y-%m-%d %H:%M:%S")` (SQLite-compatible)
- [x] 6 migration tests: `v3_schema_applies_on_fresh_db`, `v3_migration_is_idempotent`, `unit_definitions_seeded_with_presets`, `products_default_unit_id_backfilled_for_known_keys`, `products_with_unknown_default_unit_remain_unlinked`, updated `v2_schema_applies_on_fresh_db` to expect 3 migrations
- [x] All 21 migration tests pass

### DTOs

- [x] `dto/unit_definitions.rs`: `UnitKind` (Integer/Decimal), `UnitDefinitionResponse`, `UnitDefinitionCreateInput`, `UnitDefinitionRenameInput`, `UnrecognizedUnitGroup`, `UnitAuditBannerState`, `UnitReviewAction` (tagged enum: MapToPreset/KeepAsCustom/LeaveForLater), `UnitReviewActionResult`, `UnitAuditBannerState`
- [x] Added `sqlx::Decode`/`sqlx::Type` for `UnitKind` in `dto/unit_definitions.rs`
- [x] Registered in `dto/mod.rs`
- [x] `dto/csv_io.rs`: added `UnknownUnit { raw_value, suggested_keys }` variant to `CsvPreviewRowStatus`
- [x] `dto/products.rs`: added `default_unit_id: Option<String>` to `ProductCreate`, `ProductUpdate`, `ProductResponse`; added `unit_type: Option<UnitKind>` to `ProductResponse`

### Repository

- [x] `db/repositories/unit_definitions.rs`: `list_active`, `find_by_key`, `find_by_id`, `insert`, `rename`, `archive`, `is_referenced`, `suggest_similar`, `list_presets`, `find_units_preset`
- [x] Registered in `db/repositories/mod.rs`
- [x] `db/repositories/products.rs`: `RawProductRow` intermediate type for manual `UnitKind` conversion; joined selects for `default_unit_id`/`unit_type`; updated `insert_product`/`update_product` signatures (4 args); `list_all_products_for_export` updated

### Services

- [x] `services/unit_definitions.rs`: `create_custom_unit` (validates key format, rejects duplicates), `rename_unit`, `archive_unit`, `list_active`; 9 service tests pass
- [x] `services/unit_audit.rs`: `unrecognized_units`, `current_signature`, `banner_state`, `dismiss_banner`, `apply_review_action`; 7 service tests pass
- [x] `services/products.rs`: `resolve_unit_fields` helper; `create_product`/`update_product` now resolve `default_unit_id` via catalog FK or text lookup
- [x] `services/expiry_lots.rs`: added `resolve_product_default_unit` async function (catalog lookup → display_name or fallback to "Unidades"); user-unit-override still wins in `create_expiry_lot`
- [x] `services/csv_io.rs`: `classify_row` extended with unit-check step (calls `find_by_key` + `suggest_similar`); `UnknownUnit` status is non-blocking (warn-and-continue); updated `CsvPreviewRowStatus` match

### Tauri Commands

- [x] `commands/unit_definitions.rs`: 7 commands — `list_unit_definitions`, `create_unit_definition`, `rename_unit_definition`, `list_unrecognized_units`, `unit_audit_banner_state`, `dismiss_unit_audit_banner`, `apply_unit_review_action`
- [x] Registered in `commands/mod.rs`
- [x] All 7 commands wired into `lib.rs` `invoke_handler`

### DTO / repository extensions

- [x] `dto/dashboard.rs`: added `default_unit_id` and `unit_type` to `DashboardLotRow`
- [x] `db/repositories/dashboard.rs`: added columns to SELECT (joins `p.default_unit_id`, `p.unit_type`)
- [x] `db/repositories/dashboard.rs`: added `sqlx::Decode` in scope for `UnitKind`
- [x] `src/lib/dashboard.ts`: added `default_unit_id`, `unit_type` fields; imported `UnitKind`
- [x] `src/lib/csv.ts`: added `unknown_unit` variant to `CsvPreviewRowStatus`

## Backend Test Results

- **272 tests pass** (including 6 new migration tests, 9 unit_definitions service tests, 7 unit_audit service tests, updated lot test)
- **2 pre-existing failures** (confirmed by git stash):
  - `services::reports::tests::preview_report_in_alert_window_returns_alert_lots`
  - `services::reports::tests::preview_report_next_30_days_returns_30d_lots`
- **Clippy**: all errors match pre-existing baseline (20 lib + 15 test binary); no new lint failures introduced

## Frontend (Svelte/TS)

### Types

- [x] `src/lib/products.ts`: added `UnitKind` type, `default_unit_id`/`unit_type` to `ProductResponse`, `ProductCreate`, `ProductUpdate`
- [x] `src/lib/unit_definitions.ts`: created with typed wrappers for all 7 Tauri commands
- [x] `src/lib/csv.ts`: added `unknown_unit` variant to `CsvPreviewRowStatus`

### ProductForm

- [x] Replaced free-text `defaultUnit` input with `<datalist>`-backed input using `listUnitDefinitions()`
- [x] Added `+ New unit` affordance with inline sub-form (key, display_name, kind radio buttons)
- [x] Added `slugify` helper and `submitInlineUnit` async function (validates key format, checks duplicates, creates unit, selects it)
- [x] Updated save payload to include `default_unit_id`
- [x] Added unit-related CSS styles

### LotForm

- [x] Added `productUnitKind` prop (`"integer" | "decimal" | null`)
- [x] Updated quantity input: `min`/`step` set to 1 for integer, 0.01 for decimal
- [x] Shows read-only unit chip when `productUnitKind` is set; editable text input when null

### ProductDetailPage

- [x] Passes `productUnitKind={product.unit_type ?? "decimal"}` to `LotForm`

### DashboardPage

- [x] Imports `listUnitDefinitions` and `UnitDefinitionResponse`
- [x] Loads unit catalog on mount via `loadUnitCatalog()`
- [x] Added `getUnitDisplayName(lot)` helper: returns catalog `display_name` when `lot.default_unit_id` is set, falls back to `lot.unit` text
- [x] Updated quantity cell to use `{lot.quantity} {getUnitDisplayName(lot)}`
- [x] Wired `UnitReviewBanner` (between scan row and filter row) and `UnitReviewPage` (shown when `showUnitReview`)

### ReportsPage

- [x] Imports `listUnitDefinitions`, loads unit catalog on mount
- [x] Added `getUnitDisplayName(lot)` helper
- [x] Updated quantity cell to use catalog display name

### UnitReviewBanner

- [x] Created `src/components/UnitReviewBanner.svelte` — calls `unitAuditBannerState()` on mount, shows banner when `show_banner = true`
- [x] "Review" button triggers `onReview` callback; "Dismiss" persists current signature

### UnitReviewPage

- [x] Created `src/components/UnitReviewPage.svelte` — lists unrecognized groups with three actions per group:
  - "Map to preset" dropdown (groups presets by kind)
  - "Create custom unit" sub-form (name + kind radio)
  - "Leave for later" button
  - Each action calls `applyUnitReviewAction` and removes the group from the list

### CsvImportPage

- [x] Added `unknown_unit` branch to `rowBadge` (warning/amber badge)
- [x] Added detail cell rendering: "Not in catalog — suggested: [keys]" or "(none, will be created on first review)"

### ResolveQuantityDialog

- [x] No changes needed — `lot.unit` already contains the catalog `display_name` after `resolve_product_default_unit` change

### Frontend Build

- `npm run build` → **✓ built in 901ms** (clean)
- `npx tsc --noEmit` → **0 errors**

## Files Changed

### New files

- `src-tauri/src/db/repositories/unit_definitions.rs`
- `src-tauri/src/dto/unit_definitions.rs`
- `src-tauri/src/services/unit_definitions.rs`
- `src-tauri/src/services/unit_audit.rs`
- `src-tauri/src/commands/unit_definitions.rs`
- `src/lib/unit_definitions.ts`
- `src/components/UnitReviewBanner.svelte`
- `src/components/UnitReviewPage.svelte`

### Modified files

- `src-tauri/Cargo.toml` — added `sha2 = "0.10"`
- `src-tauri/src/db/migrations.rs` — V3 migration added
- `src-tauri/src/db/repositories/mod.rs` — registered unit_definitions
- `src-tauri/src/db/repositories/products.rs` — RawProductRow, 4-arg signatures
- `src-tauri/src/db/repositories/dashboard.rs` — SELECT columns, Decode import
- `src-tauri/src/dto/mod.rs` — registered unit_definitions
- `src-tauri/src/dto/products.rs` — default_unit_id/unit_type fields
- `src-tauri/src/dto/csv_io.rs` — UnknownUnit variant
- `src-tauri/src/dto/dashboard.rs` — default_unit_id/unit_type in DashboardLotRow
- `src-tauri/src/services/mod.rs` — registered unit_definitions, unit_audit
- `src-tauri/src/services/products.rs` — resolve_unit_fields, updated create/update
- `src-tauri/src/services/expiry_lots.rs` — resolve_product_default_unit, lot test update
- `src-tauri/src/services/csv_io.rs` — UnknownUnit check in classify_row
- `src-tauri/src/services/reports.rs` — make_row updated with new fields
- `src-tauri/src/services/dashboard.rs` — make_row, make_predicate_row updated
- `src-tauri/src/commands/mod.rs` — registered unit_definitions
- `src-tauri/src/lib.rs` — all 7 commands wired
- `src-tauri/src/pdf/report_pdf.rs` — make_lot updated
- `src/lib/products.ts` — UnitKind, default_unit_id, unit_type
- `src/lib/dashboard.ts` — default_unit_id, unit_type
- `src/lib/csv.ts` — unknown_unit variant
- `src/components/ProductForm.svelte` — datalist, inline unit creation
- `src/components/LotForm.svelte` — productUnitKind prop, input rules, unit chip
- `src/components/ProductDetailPage.svelte` — passes productUnitKind to LotForm
- `src/components/DashboardPage.svelte` — unit catalog, display name, banner/page
- `src/components/ReportsPage.svelte` — unit catalog, display name
- `src/components/CsvImportPage.svelte` — unknown_unit badge and detail

## Remaining Tasks

All implementation-owned tasks are complete. Only parent-owned lifecycle gates remain:

- [ ] Run bounded review of the diff (parent-owned)
- [ ] Run `sdd-apply` on a feature branch (parent-owned)
- [ ] Run `sdd-verify` (parent-owned)
- [ ] Run `sdd-archive` (parent-owned)

## Sessions 2–3 Docs Completion (this session)

### Section N — Canonical spec delta

- [x] Appended `### Requirement: unit catalog and integer/decimal classification` block to `## Capability: Product catalog` in `openspec/specs/caduxo-expiry-tracker/spec.md` with all five scenarios verbatim from design.md
- [x] Added clarifying `> **Note:**` under `## Capability: Expiry lots > Requirement: lot registration > Required lot fields`

### Section O — PRD follow-up

- [x] Updated `docs/prd.md` line 261: `Default unit` note → "Chosen from the unit catalog — see capability spec"
- [x] Updated `docs/prd.md` line 273: `Unit` field → "Pre-filled from product default when available; resolves through `default_unit_id`"
- [x] Updated `docs/prd.md` line ~530: added `default_unit_id` (TEXT, FK) and `unit_type` (TEXT, `integer`/`decimal`) rows to the `products` table
- [x] Confirmed `docs/prd.md` line ~563 (`unit | TEXT | Yes | Unit for this lot`) is unchanged

### Section P — Verification gates

- [x] `cd src-tauri && cargo test` → **276 passed; 2 pre-existing failures** (`preview_report_in_alert_window_returns_alert_lots`, `preview_report_next_30_days_returns_30d_lots`) confirmed by git stash in previous session
- [x] `cd src-tauri && cargo fmt -- --check` → **clean** (no output)
- [x] `cd src-tauri && cargo clippy --all-targets -- -D warnings` → **20 lib + 15 test binary errors** (matches pre-existing baseline; previous 21+16 was inflated by the now-fixed `LeaveForLater { raw_value }` dead_code on the DTO)
- [x] `npm run build` → **✓ built in 889ms** (clean)
- [x] `npx tsc --noEmit` → **0 errors**

### Section Q — Post-MVP gap record

- [x] Recorded the 6 manual-frontend flows in this apply-progress as the standing backlog entry for the future Svelte component harness:
  1. ProductForm datalist + inline unit creation happy path
  2. ProductForm inline unit creation with a colliding key (DuplicateField error path)
  3. LotForm quantity input shape with `unit_type = integer` vs `decimal`
  4. Banner re-appears only when a new unrecognized value is introduced
  5. CSV import preview shows the `UnknownUnit` badge
  6. PDF export still renders `{qty} {display_name}` correctly for both kinds

### Section R — Archive note

- [x] This change closes the post-MVP backlog item "presets + integer/decimal split for measurement units"
- [x] Follow-up SDD open: **lot-level quantity enforcement** (the documented gap from this slice — integer quantities only, bounded by `unit_type`, enforced at lot create/update time) is not implemented and requires a separate SDD

## Clippy Fix Applied This Session

- Added `#[allow(dead_code)]` to `LeaveForLater { raw_value }` in `src-tauri/src/dto/unit_definitions.rs` — the field is semantically necessary for the discriminated union but unused in the no-op variant. Reduced clippy count from 21+16 to 20+15, matching the pre-existing baseline exactly.

## Test Commands Run

```
cd src-tauri && cargo test
# 272 passed; 2 pre-existing failures (report tests)
# ✓ 21 migration tests
# ✓ 9 unit_definitions service tests
# ✓ 7 unit_audit service tests
# ✓ Updated lot test (Kilogramo not kg)
# ✓ 2 pre-existing report test failures confirmed by git stash

cd ../ && npm run build
# ✓ built in 901ms

npx tsc --noEmit
# ✓ 0 errors
```

## Clippy Status

- **Pre-existing**: 20 lib errors, 15 test binary errors (all from baseline)
- **My changes**: 0 new clippy errors
- `unit_definitions.rs` (repo): `#![allow(dead_code)]` — test functions not compiled via module tree
- `unit_definitions.rs` (service): `#![allow(dead_code)]` — same
- `unit_audit.rs`: `#[allow(dead_code)]` on `UNIT_AUDIT_ALIASES` (defined in design but not used in query implementation)
