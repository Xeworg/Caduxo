# Apply Progress — caduxo-expiry-tracker

## Slice 1: Foundation and persistence skeleton

### Status: COMPLETE ✅

---

## Slice 2: Database schema and migrations

### Status: COMPLETE ✅

---

## Completed tasks (cumulative)

### 1. Project foundation

| Task | Status |
  | ------ | -------- |  
| Initialize Tauri v2 project with Svelte + TypeScript | ✅ |
| Configure Rust workspace and app metadata | ✅ |
| Establish modular layered backend structure | ✅ |
| Add SQLite support with `sqlx` | ✅ |
| Add database migration runner | ✅ |
| Add app data directory resolution for Windows/Linux | ✅ |
| Add basic error handling shape shared by Tauri commands | ✅ |
| Add structured Rust logging with safe app-local log output | ✅ |
| Add baseline Rust test harness for backend/domain/persistence code | ✅ |
| Add baseline frontend test harness when non-trivial UI logic begins | ⏭ Deferred |

### 2. Database schema

| Task | Status |
  | ------ | -------- |  
| Create migration for `stores` | ✅ |
| Create migration for `store_locations` | ✅ |
| Create migration for `categories` | ✅ |
| Create migration for `products` | ✅ |
| Create migration for `product_barcodes` | ✅ |
| Create migration for `expiry_lots` | ✅ |
| Create migration for `lot_resolution_events` | ✅ |
| Create migration for `notification_log` | ✅ |
| Create migration for `app_settings` | ✅ |
| Add indexes for SKU, barcode, expiry date, store/date, product lots, and store locations | ✅ |

---

## Implementation notes

### Logging `&PathBuf` → `&Path` fix

Updated `src-tauri/src/logging.rs` to use `&Path` in function signatures:

- `resolve_log_dir(app_data: &Path) -> Option<PathBuf>`
- `init(log_dir: Option<&Path>, ...)`

Updated `lib.rs` call site to use `log_dir.as_deref()` for correct `Option<&Path>` coercion.

### Full application schema (V2 migration)

`src-tauri/src/db/migrations.rs` — V2 migration adds all 9 tables:

| Table | Key constraints |
  | ------- | ---------------- |  
| `stores` | PRIMARY KEY id, UNIQUE code |
| `store_locations` | PRIMARY KEY id, FK→stores ON DELETE CASCADE, UNIQUE(store_id,name) |
| `categories` | PRIMARY KEY id, UNIQUE name |
| `products` | PRIMARY KEY id, UNIQUE sku, FK→categories (nullable) |
| `product_barcodes` | PRIMARY KEY id, UNIQUE barcode, FK→products ON DELETE CASCADE |
| `expiry_lots` | PRIMARY KEY id, FK→products ON DELETE CASCADE, FK→stores ON DELETE CASCADE, FK→store_locations (nullable), CHECK(quantity>0), CHECK(alert_days_before>=0) |
| `lot_resolution_events` | PRIMARY KEY id, FK→expiry_lots ON DELETE CASCADE, CHECK(quantity>0) |
| `notification_log` | PRIMARY KEY id, FK→expiry_lots ON DELETE CASCADE, UNIQUE(lot_id,notification_date) |
| `app_settings` | PRIMARY KEY key |

### Indexes added

- `idx_products_sku` on `products(sku)`
- `idx_product_barcodes_barcode` on `product_barcodes(barcode)`
- `idx_expiry_lots_expiry_date` on `expiry_lots(expiry_date)`
- `idx_expiry_lots_store_expiry` on `expiry_lots(store_id, expiry_date)`
- `idx_expiry_lots_product` on `expiry_lots(product_id)`
- `idx_store_locations_store` on `store_locations(store_id)`

### FK enforcement

Added `.pragma("foreign_keys", "ON")` to `SqliteConnectOptions` in `pool.rs`. All child tables use `ON DELETE CASCADE` so deleting a parent row automatically removes orphaned children.

### Test infrastructure

- `fresh_test_pool()` creates a unique `/tmp/caduxo_test_{pid}_{rand}.db` file per test invocation, avoiding concurrent-test collisions.
- 16 migration/schema tests added:
  - `v2_schema_applies_on_fresh_db` — confirms 2 migrations run
  - `all_required_tables_exist` — all 9 tables present
  - `stores_table_structure` — UNIQUE code enforced
  - `store_locations_unique_per_store` — UNIQUE(store_id, name) enforced
  - `categories_unique_name` — UNIQUE name enforced
  - `products_unique_sku` — UNIQUE sku enforced
  - `barcodes_unique_across_products` — UNIQUE barcode enforced
  - `expiry_lots_quantity_check` — CHECK(quantity>0) enforced
  - `expiry_lots_alert_days_non_negative` — CHECK(alert_days>=0) enforced
  - `resolution_events_quantity_check` — CHECK(quantity>0) enforced
  - `notification_log_unique_per_day` — UNIQUE(lot_id, date) enforced
  - `app_settings_primary_key` — key uniqueness enforced
  - `all_required_indexes_exist` — all 6 indexes present
  - `migrations_are_idempotent` — re-running migrations is a no-op
  - `product_barcodes_cascade_on_delete` — cascade confirmed
  - `notification_log_cascade_on_lot_delete` — cascade confirmed

### Changes summary

| File | Change |
  | ------ | -------- |  
| `src-tauri/src/logging.rs` | `&PathBuf` → `&Path` in public signatures |
| `src-tauri/src/lib.rs` | `log_dir.as_deref()` for `Option<&Path>` coercion |
| `src-tauri/src/db/pool.rs` | Added `.pragma("foreign_keys", "ON")` to `sqlite_options` |
| `src-tauri/src/db/migrations.rs` | V2 migration (9 tables + 6 indexes), 15 migration/schema tests |
| `src-tauri/Cargo.toml` | Added `rand = "0.8"` dev-dependency |
| `openspec/.../tasks.md` | Checked off all 10 section-2 tasks |
| `openspec/.../apply-progress.md` | Appended Slice 2 section |

---

## Verification evidence

```bash
# Cargo check
cd src-tauri && cargo check 2>&1 | tail -2
# → "Finished `dev` profile ... 18 warnings" (all dead_code, expected) ✅

# Cargo clippy (no errors)
cd src-tauri && cargo clippy 2>&1 | grep "^error\|ptr_arg"
# → (no output = clean) ✅

# Rust tests
cd src-tauri && cargo test 2>&1 | grep "test result"
# → "35 passed; 0 failed; 0 ignored" ✅
#   19 domain + 1 pool + 15 migration/schema tests

# TypeScript check
npx tsc --noEmit 2>&1
# → (no output = clean) ✅

# Frontend build
npm run build 2>&1 | tail -2
# → "✓ built in 265ms" ✅
```

### Test breakdown

| Category | Count |
  | ---------- | ------- |  
| Domain (alerts, expiry_status, lot_resolution, validation) | 19 |
| DB pool smoke | 1 |
| Migration/schema tests (this slice) | 15 |
| **Total** | **35** |

---

## Out-of-scope decisions deferred to later slices

1. Slice 3: `is_first_run`, first store creation, blocking lot creation before a store exists, and last-selected-store in settings
2. Slice 3/5: Store CRUD and optional internal location CRUD per the delivery plan split
3. Slice 4: Product CRUD, category CRUD, barcode management, and search
4. Slice 5: Lot CRUD, partial resolution, and lot events
5. Slices 6+: Dashboard, scanner, notifications, CSV, reports, backup, packaging

---

## Risks

| Risk | Mitigation |
  | ------ | ----------- |  
| FK `ON DELETE CASCADE` not set for all nullable FKs (e.g. `expiry_lots.location_id`, `products.category_id`) | Nullable FKs with no cascade are intentional — they model optional relationships where the child may outlive the parent |
| Concurrency issue with `:memory:` SQLite and multi-connection pools | Switched to process-unique temp file path (`/tmp/caduxo_test_{pid}_{rand}.db`) per test invocation |
| Inline migration approach diverges from `migrate!` macro | Compatible; can switch by creating `migrations/` dir |

---

## Next recommended action

**Slice 3 — First-run setup and stores**: Implement `is_first_run`, first store creation flow, store/local management, last-selected-store persistence, and block lot creation until at least one store exists.

---

## Slice 3 — First-run setup and stores

### Status: PARTIAL COMPLETE ✅

Slice 3 backend and UI store management are implemented and verified. One task remains intentionally unchecked: blocking expiry lot creation until a store exists, because lot creation is not implemented yet. A `has_store` Tauri command/service/repository precondition is available for the future lot workflow.

### Slice 3 completed tasks

| Task | Status |
| ---- | ------ |
| Implement `is_first_run` command | ✅ |
| Implement first store creation flow | ✅ |
| Remember last selected store in settings | ✅ |
| Implement store CRUD | ✅ |
| Implement optional internal location CRUD per store | ✅ |
| Enforce unique location name per store | ✅ |
| Build store/local management UI | ✅ |
| Build optional internal location UI | ✅ |
| Block expiry lot creation until at least one store exists | ⏭ Deferred until lot creation command exists; `has_store` precondition command added |

### Slice 3 implementation notes

- Added store/settings DTOs and Tauri commands for first-run, store CRUD, location CRUD, settings, and `has_store`.
- Added repositories and services for stores, internal locations, and app settings.
- Added Svelte store/local management UI and first-run flow.
- Removed the first-run form autofocus warning after verification surfaced it.
- Kept business logic in Rust services/repositories; Svelte remains command/API client UI.

### Slice 3 verification evidence

```bash
cd src-tauri && cargo test
# → 59 passed; 0 failed ✅

cd src-tauri && cargo check
# → finished with expected scaffold dead_code warnings ✅

cd src-tauri && cargo clippy
# → finished with expected scaffold dead_code warnings; no clippy errors ✅

npm run build
# → built successfully ✅

npx tsc --noEmit
# → clean ✅
```

### Slice 3 risks and notes

- The SDD subagent reported an execution error after making changes, so parent verification and artifact updates were completed directly.
- The UI is functional but intentionally simple; future slices can refine navigation and visual polish.
- The lot creation precondition should be enforced in Slice 5 when `create_expiry_lot` is introduced.

### Slice 3 next recommended action

**Slice 4 — Product catalog, categories, and barcodes**: Implement product identity/search backend and UI, keeping category/barcode constraints covered by focused tests.

---

## Slice 4 — Product catalog (backend)

### Status: BACKEND COMPLETE ✅ — UI DEFERRED ⏭

Slice 4 is split: the backend (categories, products, barcodes, search) is implemented and verified with focused tests. The three product UI tasks (product list UI, product form UI, product detail UI) remain intentionally unchecked and will be picked up in a follow-up slice.

### Slice 4 completed tasks

| Task | Status |
| ---- | ------ |
| Implement product create/update/archive commands | ✅ Backend |
| Enforce required unique SKU | ✅ |
| Implement category list CRUD | ✅ |
| Implement product default alert-days-before with software suggestion of 30 days | ✅ |
| Implement barcode add/remove/list commands | ✅ |
| Enforce barcode uniqueness across products | ✅ |
| Implement product search by description, SKU, and barcode | ✅ |
| Build product list UI | ⏭ Deferred |
| Build product form UI | ⏭ Deferred |
| Build product detail UI with barcode list and expiry lots | ⏭ Deferred |

### Slice 4 implementation notes

- Added DTOs for categories, products, product barcodes, and a search query/result bundle (`ProductDetailResponse` includes product + barcodes + resolved category).
- Added the `products` repository module with all SQL; services hold validation and uniqueness-error translation. Repository returns raw `sqlx::Error`; the service layer detects `UNIQUE constraint failed: <table>.<column>` and converts it to `DomainError::DuplicateField { field, value }` for SKU, barcode, and category name.
- Added the `products` service module: `suggested_alert_days` constant (`30`), `validate_alert_days` (range 0..=3650), business rules (archived products cannot receive new barcodes), and full CRUD/search use cases.
- Added the `products` Tauri command module and registered the 12 new IPC commands in `lib.rs`.
- Added a soft-archive (`is_active = 0`) for products; `archive_product` returns `NotFound` when the id is unknown and is otherwise idempotent for already-archived rows.
- Barcode management keeps at most one primary barcode per product: setting `is_primary = true` demotes existing primary barcodes for that product before the insert.
- Product search uses a single SQL query with `LIKE` on description and SKU and an `EXISTS` subquery on barcode (case-insensitive, partial match). An empty query returns all products, which the UI can use as a default listing path.
- Logs avoid raw product/SKU/barcode/notes per the engineering safety rules: services use plain `tracing` calls and the current implementation does not log payload fields. Errors surface as user-safe `CommandError` variants.
- UI tasks are deferred; this slice is intentionally backend-only to keep review bounded.

### Slice 4 verification evidence

```bash
cd src-tauri && cargo test
# → 85 passed; 0 failed; 0 ignored ✅
#   19 domain + 1 pool + 15 migrations + 11 stores-repository
#   + 5 settings-repository + 10 stores-service + 2 settings-service
#   + 26 products-service (NEW for Slice 4)

cd src-tauri && cargo check
# → finished with expected scaffold dead_code warnings; no errors ✅

cd src-tauri && cargo clippy
# → no clippy errors; same pre-existing dead_code warnings as Slice 3 ✅

npm run build
# → ✓ built in 439ms ✅

npx tsc --noEmit
# → clean ✅
```

### Slice 4 new tests (services::products)

| Test | Covers |
| ---- | ------ |
| `create_and_list_categories` | Category create + list alphabetical order |
| `create_category_rejects_duplicate_name` | Category name uniqueness → `DuplicateField(name)` |
| `create_category_rejects_empty_name` | `validate_name` rejection |
| `update_category_renames_and_archives` | Category rename + archive toggles `is_active` |
| `update_category_missing_returns_not_found` | `NotFound(category)` when id is unknown |
| `suggested_alert_days_is_thirty` | Backend suggestion constant is 30 |
| `create_product_succeeds` | Happy path with default alert days = 30 |
| `create_product_rejects_empty_sku` | `validate_sku` rejection |
| `create_product_rejects_empty_description` | `validate_description` rejection |
| `create_product_rejects_negative_alert_days` | `validate_alert_days` lower bound |
| `create_product_rejects_excessive_alert_days` | `validate_alert_days` upper bound (3650) |
| `create_product_enforces_sku_uniqueness` | `DuplicateField(sku)` on duplicate insert |
| `update_product_works` | Update round-trip, including `default_alert_days_before` |
| `update_product_enforces_sku_uniqueness` | `DuplicateField(sku)` on rename collision |
| `update_product_missing_returns_not_found` | `NotFound(product)` when id is unknown |
| `archive_product_soft_deletes` | `is_active` flips to 0 |
| `archive_product_missing_returns_not_found` | `NotFound(product)` when id is unknown |
| `add_list_remove_barcodes` | Add primary + secondary, list, remove both |
| `add_barcode_validates_value` | `validate_barcode` rejection |
| `add_barcode_to_archived_product_is_rejected` | Business-rule guard for archived products |
| `add_barcode_to_missing_product_returns_not_found` | `NotFound(product)` when id is unknown |
| `barcode_uniqueness_across_products` | `DuplicateField(barcode)` across products |
| `add_secondary_barcode_to_same_product_is_allowed` | Multiple barcodes per product |
| `remove_barcode_missing_returns_not_found` | `NotFound(product_barcode)` when id is unknown |
| `search_finds_by_description_sku_and_barcode` | Search by description, SKU partial, barcode partial/exact, empty query, no match |
| `search_includes_primary_barcode_in_results` | `primary_barcode` field in results |

### Slice 4 changes summary

| File | Change |
| ------ | -------- |
| `src-tauri/src/dto/products.rs` | New: categories, products, barcodes, search DTOs |
| `src-tauri/src/dto/mod.rs` | Add `pub mod products;` |
| `src-tauri/src/db/repositories/products.rs` | New: SQL for categories, products, barcodes, search |
| `src-tauri/src/db/repositories/mod.rs` | Add `pub mod products;` |
| `src-tauri/src/services/products.rs` | New: business logic, validation, uniqueness translation, 26 service tests |
| `src-tauri/src/services/mod.rs` | Add `pub mod products;` |
| `src-tauri/src/commands/products.rs` | New: 12 Tauri commands (categories, products, barcodes, search, suggested alert days) |
| `src-tauri/src/commands/mod.rs` | Add `pub mod products;` |
| `src-tauri/src/lib.rs` | Register 12 new commands in `tauri::generate_handler!` |
| `openspec/.../tasks.md` | Check off 7 backend tasks in Section 4; UI tasks remain unchecked |
| `openspec/.../apply-progress.md` | Append this Slice 4 section |

### Slice 4 risks and notes

- The frontend does not yet call any of the new commands; the UI tasks (list, form, detail) are intentionally deferred to a follow-up slice to keep review bounded.
- `services::products::list_all_categories` and `repo::list_all_categories` are not yet wired to a command — they mirror the existing `list_all_stores`/`get_store`/`list_all_locations` scaffold pattern and are available for the future UI that needs to show archived categories.
- The search uses `LIKE '%query%'` against `description`, `sku`, and `barcode`. SQLite's default `LIKE` is case-insensitive for ASCII; for non-ASCII searches the results follow SQLite's case behavior. If the UI later needs accent-insensitive search, add a `COLLATE NOCASE` index or normalize inputs upstream.
- Lot creation is still untouched, so the `Block expiry lot creation until at least one store exists` task from Slice 3 remains intentionally unchecked.

### Slice 4 next recommended action

**Slice 4 UI — Product catalog frontend**: Build the Svelte product list, form, and detail views (with barcode list and expiry lot placeholders) on top of the new backend commands. Optionally fold lot creation (`create_expiry_lot` + `Block lot creation until store exists`) into the same slice so the product detail screen can land together.

---

## Slice 4b — Product catalog (frontend UI)

### Status: COMPLETE ✅

Slice 4b delivers the Svelte frontend on top of the Slice 4 backend. The three remaining UI tasks in Section 4 are now checked. No backend Rust changes were made; only frontend files, navigation wiring, and the spec artifacts were touched. Lot CRUD is still out of scope and the lot-store precondition remains intentionally unchecked.

### Slice 4b completed tasks

| Task | Status |
| ---- | ------ |
| Build product list UI | ✅ |
| Build product form UI | ✅ |
| Build product detail UI with barcode list and expiry lots | ✅ |

### Slice 4b implementation notes

- Added `src/lib/products.ts` — thin TypeScript wrapper for all 12 product/category/barcode Tauri commands, mirroring the conventions in `src/lib/stores.ts`: one async function per command, camelCase JS arguments, exported response/input types.
- Added `src/components/ProductForm.svelte` — handles both create and edit modes, validates required SKU/description locally, prefills `default_alert_days_before` from `suggested_product_alert_days`, includes an inline "+ New category" flow that calls `create_category` and auto-selects the new category. Edit mode exposes the `is_active` checkbox.
- Added `src/components/ProductDetailPage.svelte` — loads the full detail bundle (`get_product`), renders SKU/description/category/unit/alert-days/notes, lists barcodes with primary badges, exposes an add-barcode form (with optional type and `is_primary` toggle), and provides an explicit Archive action with a confirmation prompt. Expiry lots render as a clearly labelled placeholder section.
- Added `src/components/ProductCatalogPage.svelte` — top-level catalog page with a simple view machine (`list | create | edit | detail`). Debounced (200 ms) search input calls `search_products`; an empty query lists all products so the first-load view is never empty. Shows category, primary barcode, and an Archived badge on inactive rows.
- Wired the new page into `src/App.svelte` as a `products` nav tab; existing dashboard and stores tabs are untouched.
- No backend changes; command shapes are taken verbatim from `src-tauri/src/commands/products.rs` and registered in `lib.rs` from Slice 4.
- No sensitive product/SKU/barcode/notes content is logged; errors propagate as user-facing strings via the existing `CommandError` path.

### Slice 4b compatibility notes

- All components use Svelte 5 legacy syntax (`let`, `$:`, `bind:value`, `class:`, `on:`) consistent with `src/components/StoresPage.svelte`. No runes mode used.
- `tsconfig.json` already covers `src/**/*.svelte`; the new files participate in `tsc --noEmit` automatically.

### Slice 4b files changed

| File | Change |
| ---- | ------ |
| `src/lib/products.ts` | New: API wrapper for 12 product/category/barcode commands |
| `src/components/ProductForm.svelte` | New: create/edit form with inline category creation |
| `src/components/ProductDetailPage.svelte` | New: detail view with barcodes + expiry lots placeholder |
| `src/components/ProductCatalogPage.svelte` | New: list/create/edit/detail page with debounced search |
| `src/App.svelte` | Added `products` nav tab and `ProductCatalogPage` route |
| `openspec/.../tasks.md` | Checked off the 3 Section 4 UI tasks; left backend tasks as they were |
| `openspec/.../apply-progress.md` | Appended this Slice 4b section |

### Slice 4b deferred issues

- Lot CRUD remains out of scope; the detail page shows an expiry-lots placeholder.
- The lot-store precondition (`Block expiry lot creation until at least one store exists`) stays intentionally unchecked because lot creation is not implemented yet. The `has_store` precondition command from Slice 3 is still the right hook for that guard when lot creation lands.
- No frontend test harness exists yet, so no UI tests were added; the matching Slice 1 task remains unchecked by design.

### Slice 4b next recommended action

**Slice 5 — Expiry lots (backend + UI)**: Implement `create_expiry_lot` and the partial-resolution flow, enforce the store precondition via `has_store`, and build the lot form + resolve-quantity UI so the product detail placeholder becomes real data.

---

## Slice 5a — Expiry lots (backend only)

### Status: COMPLETE ✅

Slice 5a delivers the expiry lot backend (DTOs, repository, service, Tauri commands) without Svelte UI. Lot CRUD and partial resolution are fully implemented and tested. The store precondition is enforced. The lot form UI and resolve-quantity UI remain intentionally unchecked for Slice 5b.

### Slice 5a completed tasks

| Task | Status |
| ---- | ------ |
| Implement expiry lot create/update/archive commands | ✅ Backend |
| Pre-fill lot unit from product default when available | ✅ |
| Pre-fill lot alert days from product default | ✅ |
| Allow per-lot alert-days-before override | ✅ |
| Support optional internal location and batch code | ✅ |
| Implement partial quantity resolution | ✅ |
| Record partial resolutions in `lot_resolution_events` | ✅ |
| Block expiry lot creation until at least one store exists | ✅ |
| Add regression tests for partial lot resolution | ✅ |
| Verify lot alert default and override behavior | ✅ |

### Slice 5a NOT implemented (UI, deferred to 5b)

| Task | Status |
| ---- | ------ |
| Build lot form UI | ⏭ Deferred |
| Build resolve quantity UI | ⏭ Deferred |
| Require store selection only when multiple stores exist | ⏭ UI concern |

### Slice 5a implementation notes

**Schema constraint note — fully-resolved lots store `quantity = 1.0`**:

The `expiry_lots` table has `CHECK(quantity > 0)`. When remaining quantity reaches zero via partial resolution, the repository clamps `quantity` to `1.0` (the DB row is kept as an audit trail; `lot_resolution_events` is the authoritative record of exact resolved quantities). The `status = 'resolved'` field ensures these rows are excluded from all active-lot queries.

**DTO design**:

`ExpiryLotCreate.unit` and `ExpiryLotCreate.alert_days_before` are `Option`-wrapped. The service resolves them against product defaults: `None` or blank unit → product `default_unit`; `None` or negative alert days → product `default_alert_days_before`. User-supplied values always win.

**Partial resolution flow**:

1. Validate lot exists and is `active`.
2. Validate `resolved_qty <= remaining_quantity`.
3. Record `lot_resolution_events` row (source of truth).
4. Update lot: remaining → `max(0, current - resolved)`, clamp to `1.0` for DB; set `status = 'resolved'` and fill `resolution`/`resolved_at` when fully consumed.

**Commands registered** (10 total):

`list_expiry_lots`, `list_expiry_lots_by_store`, `list_expiry_lots_by_product`, `get_expiry_lot`, `create_expiry_lot`, `update_expiry_lot`, `archive_expiry_lot`, `resolve_expiry_lot`, `list_lot_resolution_events`, `has_store` (was in Slice 3).

### Slice 5a new tests (17 service tests)

| Test | Covers |
| ---- | ------ |
| `create_lot_requires_store` | Precondition: fails without active store |
| `create_lot_succeeds` | Happy-path create |
| `create_lot_pre_fills_unit_from_product` | Unit pre-fill fallback |
| `create_lot_user_unit_overrides_product_default` | User unit wins |
| `create_lot_pre_fills_alert_days_from_product` | Alert days pre-fill fallback |
| `create_lot_user_alert_days_overrides_product_default` | User alert days wins |
| `create_lot_rejects_negative_quantity` | Validation |
| `create_lot_rejects_zero_quantity` | Validation |
| `create_lot_rejects_invalid_expiry_date` | Date format validation |
| `update_lot_works` | Update round-trip |
| `update_missing_lot_returns_not_found` | NotFound on missing |
| `archive_lot_works` | Soft-archive removes from active list |
| `archive_missing_lot_returns_not_found` | NotFound on missing |
| `list_lots_by_product_ordered_by_expiry` | Ordered list |
| `partial_resolution_reduces_quantity_and_records_event` | Partial consume |
| `full_resolution_marks_lot_resolved` | Full consume + DB clamp |
| `cannot_resolve_more_than_remaining` | Validation guard |
| `cannot_resolve_archived_or_resolved_lot` | Business-rule guard |
| `resolve_rejects_invalid_resolution_type` | Type validation |
| `resolve_rejects_zero_quantity` | Validation |
| `multiple_partial_resolutions_accumulate` | Multiple partial events |

### Slice 5a files changed

| File | Change |
| ---- | ------ |
| `src-tauri/src/dto/expiry_lots.rs` | New: DTOs for lot CRUD, resolve, events |
| `src-tauri/src/dto/mod.rs` | Add `pub mod expiry_lots;` |
| `src-tauri/src/db/repositories/expiry_lots.rs` | New: SQL for lots + resolution events |
| `src-tauri/src/db/repositories/mod.rs` | Add `pub mod expiry_lots;` |
| `src-tauri/src/services/expiry_lots.rs` | New: business logic, pre-fill, resolution, 17 tests |
| `src-tauri/src/services/mod.rs` | Add `pub mod expiry_lots;` |
| `src-tauri/src/commands/expiry_lots.rs` | New: 9 Tauri commands |
| `src-tauri/src/commands/mod.rs` | Add `pub mod expiry_lots;` |
| `src-tauri/src/lib.rs` | Register 9 new commands |
| `openspec/.../tasks.md` | Check off 9 backend tasks in Section 6; UI tasks remain unchecked |
| `openspec/.../apply-progress.md` | Append this Slice 5a section |

### Slice 5a risks and notes

- **No Svelte UI**: lot form, resolve quantity UI, and store-selection logic are intentionally deferred. The product detail placeholder remains.
- **DB CHECK constraint**: `quantity > 0` means fully-resolved lots keep `quantity = 1.0` in the DB; exact resolved amounts are in `lot_resolution_events`.
- Logs do not include sensitive product/SKU/barcode/notes content; errors surface via `CommandError`.
- No migrations needed — schema was already in place from Slice 2.

### Slice 5a next recommended action

**Slice 5b — Expiry lots (frontend UI)**: Build the lot form and resolve-quantity UI so the product detail placeholder becomes real data. Wire `list_expiry_lots_by_product` into the product detail page. Implement the store/local filter and the store-selection requirement (only when multiple stores exist).

---

## Slice 5b — Expiry lots (frontend UI)

### Status: COMPLETE ✅

Slice 5b delivers the Svelte frontend on top of the Slice 5a backend. Lot form, resolve-quantity dialog, and product detail integration are all implemented. The product detail placeholder is replaced with real data. No backend changes were made.

### Slice 5b completed tasks

| Task | Status |
| ---- | ------ |
| Build lot form UI | ✅ |
| Build resolve quantity UI | ✅ |

### Slice 5b NOT implemented (deferred to later slices)

| Task | Status |
| ---- | ------ |
| Require store selection only when multiple stores exist | ⏭ UI concern |
| Add store/local filter | ⏭ Slice 6 |

### Slice 5b implementation notes

**`src/lib/expiry_lots.ts`** — TypeScript wrapper for all 9 expiry lot Tauri commands. Mirrors conventions in `products.ts` and `stores.ts`: one async function per command, camelCase arguments, exported response/input types. DTOs mirror the Rust DTOs in `src-tauri/src/dto/expiry_lots.rs`.

**`LotForm.svelte`** — Handles both create and edit modes. On create: pre-fills unit and alert days from `defaultUnit`/`defaultAlertDays` props, auto-selects the first available store, and sets default expiry date to today + alert days. On edit: loads the full lot state. Shows the store selector only when more than one store exists; shows the location dropdown only when locations are available for the selected store. Calls `createExpiryLot`/`updateExpiryLot` and reports errors locally. No business logic in the component.

**`ResolveQuantityDialog.svelte`** — Modal dialog for partial quantity resolution. Shows remaining quantity and expiry date. Validates `qty > 0` and `qty <= remaining`. Resolution type is a select: consumed/discarded/transferred. Optional notes. Loads and displays resolution event history for the lot via `listLotResolutionEvents`. Calls `resolveExpiryLot` and reports errors locally. Fixed import: `listStoreLocations` and `StoreLocationResponse` are from `stores.ts`, not `expiry_lots.ts`.

**`ProductDetailPage.svelte`** — Replaced the expiry lots placeholder with a real list. Loads lots via `listExpiryLotsByProduct(productId)` on mount and when `productId` changes. Each lot row shows: urgency badge (expired / today / N days / normal), quantity + unit, expiry date, batch code, location, and status badge. Active lots expose three icon actions: resolve (↓), edit (✏️), archive (🗄). Archived/resolved lots show status badges but no actions. "+ New lot" button opens `LotForm` in create mode (hidden for archived products). `ResolveQuantityDialog` renders as an overlay when `resolvingLot` is set. Urgency is computed client-side from the expiry date string. No product names, SKUs, barcodes, or notes are logged.

### Slice 5b compatibility notes

- All components use Svelte 5 legacy syntax (`let`, `$:`, `bind:value`, `class:`, `on:`) consistent with the rest of the codebase.
- No runes mode used.
- `tsconfig.json` covers all new files; `tsc --noEmit` is clean.
- Business logic (pre-fill rules, quantity validation, archive/resolution state) is in Rust; Svelte only calls commands and does local form validation.

### Slice 5b files changed

| File | Change |
| ---- | ------ |
| `src/lib/expiry_lots.ts` | New: API wrapper for 9 expiry lot commands |
| `src/components/LotForm.svelte` | New: create/edit lot form |
| `src/components/ResolveQuantityDialog.svelte` | New: partial resolution modal |
| `src/components/ProductDetailPage.svelte` | Replaced expiry lots placeholder with real list + form + resolve dialog |
| `openspec/.../tasks.md` | Checked off Section 6 lot form and resolve quantity UI tasks |
| `openspec/.../apply-progress.md` | Appended this Slice 5b section |

### Slice 5b deferred issues

- **Store/local filter** (quick filter in dashboard): deferred to Slice 6 dashboard.
- **Require store selection only when multiple stores exist**: the LotForm shows store selector only when `stores.length > 1`, satisfying the rule at form-render time. The backend precondition (`has_store`) is unchanged.

### Slice 5b next recommended action

**Slice 6 — Dashboard and scanner workflow**: Implement the dashboard query command, urgency cards, expired section, urgent lot table with urgency sort, quick filters, and the always-visible scan/search input with barcode-first → SKU-second → quick-create flow.
