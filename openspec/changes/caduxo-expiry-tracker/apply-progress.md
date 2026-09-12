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
