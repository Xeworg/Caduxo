# Tasks: Caduxo Expiry Tracker

## 1. Project foundation

- [x] Initialize Tauri v2 project with Svelte + TypeScript. <!-- sdd-owner: implementation -->
- [x] Configure Rust workspace and app metadata. <!-- sdd-owner: implementation -->
- [x] Establish modular layered backend structure: commands, services, domain, repositories, DTOs, shared errors, and app state. <!-- sdd-owner: implementation -->
- [x] Add SQLite support with `sqlx`. <!-- sdd-owner: implementation -->
- [x] Add database migration runner. <!-- sdd-owner: implementation -->
- [x] Add app data directory resolution for Windows/Linux. <!-- sdd-owner: implementation -->
- [x] Add basic error handling shape shared by Tauri commands. <!-- sdd-owner: implementation -->
- [x] Add structured Rust logging with safe app-local log output. <!-- sdd-owner: implementation -->
- [x] Add baseline Rust test harness for backend/domain/persistence code. <!-- sdd-owner: implementation -->
- [ ] Add baseline frontend test harness when non-trivial UI logic begins. <!-- sdd-owner: implementation -->

## 2. Database schema

- [x] Create migration for `stores`.
- [x] Create migration for `store_locations`.
- [x] Create migration for `categories`.
- [x] Create migration for `products`.
- [x] Create migration for `product_barcodes`.
- [x] Create migration for `expiry_lots`.
- [x] Create migration for `lot_resolution_events`.
- [x] Create migration for `notification_log`.
- [x] Create migration for `app_settings`.
- [x] Add indexes for SKU, barcode, expiry date, store/date, product lots, and store locations.

## 3. First-run setup

- [x] Implement `is_first_run` command. <!-- sdd-owner: implementation -->
- [x] Implement first store creation flow. <!-- sdd-owner: implementation -->
- [x] Block expiry lot creation until at least one store exists. <!-- sdd-owner: implementation -->
- [x] Remember last selected store in settings. <!-- sdd-owner: implementation -->

## 4. Product catalog

- [x] Implement product create/update/archive commands. <!-- sdd-owner: implementation -->
- [x] Enforce required unique SKU. <!-- sdd-owner: implementation -->
- [x] Implement category list CRUD. <!-- sdd-owner: implementation -->
- [x] Implement product default alert-days-before with software suggestion of 30 days. <!-- sdd-owner: implementation -->
- [x] Implement barcode add/remove/list commands. <!-- sdd-owner: implementation -->
- [x] Enforce barcode uniqueness across products. <!-- sdd-owner: implementation -->
- [x] Implement product search by description, SKU, and barcode. <!-- sdd-owner: implementation -->
- [x] Build product list UI. <!-- sdd-owner: implementation -->
- [x] Build product form UI. <!-- sdd-owner: implementation -->
- [x] Build product detail UI with barcode list and expiry lots. <!-- sdd-owner: implementation -->

## 5. Stores and internal locations

- [x] Implement store CRUD. <!-- sdd-owner: implementation -->
- [x] Implement optional internal location CRUD per store. <!-- sdd-owner: implementation -->
- [x] Enforce unique location name per store. <!-- sdd-owner: implementation -->
- [x] Build store/local management UI. <!-- sdd-owner: implementation -->
- [x] Build optional internal location UI. <!-- sdd-owner: implementation -->

## 6. Expiry lots

- [x] Implement expiry lot create/update/archive commands. <!-- sdd-owner: implementation -->
- [x] Pre-fill lot unit from product default when available. <!-- sdd-owner: implementation -->
- [x] Pre-fill lot alert days from product default. <!-- sdd-owner: implementation -->
- [x] Allow per-lot alert-days-before override. <!-- sdd-owner: implementation -->
- [ ] Require store selection only when multiple stores exist. <!-- UI concern -->
- [x] Support optional internal location and batch code. <!-- sdd-owner: implementation -->
- [x] Implement partial quantity resolution. <!-- sdd-owner: implementation -->
- [x] Record partial resolutions in `lot_resolution_events`. <!-- sdd-owner: implementation -->
- [x] Build lot form UI. <!-- sdd-owner: implementation -->
- [x] Build resolve quantity UI. <!-- sdd-owner: implementation -->

## 7. Dashboard

- [x] Implement dashboard query command. <!-- sdd-owner: implementation -->
- [x] Compute expired, today, alert-window, and next-30-days groups. <!-- sdd-owner: implementation -->
- [x] Build urgency cards. <!-- sdd-owner: implementation -->
- [x] Build prominent expired section. <!-- sdd-owner: implementation -->
- [x] Build urgent lot table sorted by urgency. <!-- sdd-owner: implementation -->
- [x] Add store/local filter. <!-- sdd-owner: implementation -->
- [x] Add quick filters: expired, today, next 7 days, next 30 days, alert window. <!-- sdd-owner: implementation -->
- [ ] Add row actions: view product, edit lot, resolve quantity, report selection. <!-- report selection deferred — reports do not exist yet; view product, edit lot, resolve quantity implemented -->

## 8. Scanner/search workflow

- [x] Build always-visible scan/search input.
- [x] On Enter, search exact barcode first.
- [x] If no barcode match, search exact SKU.
- [x] If match exists, open product or lot entry flow.
- [x] If no match exists, open quick product creation with scanned value pre-filled.
- [x] Support manual typed SKU/UPC input.

## 9. Local notifications

- [x] Add notification permission/request flow. <!-- sdd-owner: implementation -->
- [x] Implement due notification query using alert window rules. <!-- sdd-owner: implementation -->
- [x] Use `notification_log` to prevent duplicate same-day notifications. <!-- sdd-owner: implementation -->
- [x] Trigger notification check on app startup. <!-- sdd-owner: implementation -->
- [x] Trigger periodic notification check while app is open. <!-- sdd-owner: implementation -->
- [x] Stop OS notifications after expiry date. <!-- sdd-owner: implementation -->
- [x] Keep expired lots prominent on dashboard after notification period ends. <!-- satisfied by existing DashboardPage: row-expired CSS (#fff5f5 bg), expired urgency card, expired preset filter — expires lots are shown regardless of notification state -->

## 10. CSV import/export

- [x] Implement CSV file selection flow. <!-- sdd-owner: implementation -->
- [x] Detect CSV headers. <!-- sdd-owner: implementation -->
- [x] Build column mapping UI for SKU, description, UPC/barcode. <!-- sdd-owner: implementation -->
- [x] Validate required mapped fields. <!-- sdd-owner: implementation -->
- [x] Implement import preview with invalid rows and duplicate warnings. <!-- sdd-owner: implementation -->
- [x] Implement conflict strategies: skip, update, review. <!-- sdd-owner: implementation -->
- [x] Import products and barcodes. <!-- sdd-owner: implementation -->
- [x] Export products CSV. <!-- sdd-owner: implementation -->
- [x] Export report rows CSV. <!-- sdd-owner: implementation -->

### 10a. CSV foundation, exports, and backend preview (Slice 10a)

- [x] Add `csv` Rust crate and `tauri-plugin-dialog` dependency. <!-- sdd-owner: implementation -->
- [x] Register `tauri-plugin-dialog` and add `dialog:default` capability. <!-- sdd-owner: implementation -->
- [x] Add backend DTO/service/command module for CSV I/O. <!-- sdd-owner: implementation -->
- [x] Implement `preview_product_csv` backend preview/classification (no import commit). <!-- sdd-owner: implementation -->
- [x] Implement header detection for canonical and aliased columns. <!-- sdd-owner: implementation -->
- [x] Classify duplicate SKU/barcode against existing data in preview. <!-- sdd-owner: implementation -->
- [x] Return rows and status counts from preview. <!-- sdd-owner: implementation -->
- [x] Implement `export_products_csv` writing canonical products CSV. <!-- sdd-owner: implementation -->
- [x] Implement `export_report_csv` reusing dashboard filters/rows. <!-- sdd-owner: implementation -->
- [x] Add TypeScript wrapper `src/lib/csv.ts` with command wrappers and dialog helpers. <!-- sdd-owner: implementation -->
- [x] Add ProductCatalogPage export button. <!-- sdd-owner: implementation -->
- [x] Add DashboardPage export button. <!-- sdd-owner: implementation -->
- [x] Add backend tests for preview validation/classification and exports. <!-- sdd-owner: implementation -->

### 10b. CSV import commit, conflict strategies, mapping modal (Slice 10b)

- [x] Build column mapping UI for SKU, description, UPC/barcode. <!-- sdd-owner: implementation -->
- [x] Implement import preview UI surfacing invalid rows and duplicate warnings. <!-- sdd-owner: implementation -->
- [x] Implement conflict strategies: skip, update, review. <!-- sdd-owner: implementation -->
- [x] Import products and barcodes (commit). <!-- sdd-owner: implementation -->

## 11. Reports and PDF

- [ ] Implement report data query for in-alert-window report.
- [ ] Implement report data query for expired report.
- [ ] Implement report data query for next-30-days report.
- [ ] Implement custom report filters.
- [ ] Build report preview UI.
- [ ] Implement Rust structured PDF export using `printpdf` or equivalent.
- [ ] Use A4 landscape as initial default for table reports.
- [ ] Add pagination and page numbers.
- [ ] Add report metadata: type, filters, generated date/time.
- [ ] Add CSV export for report data.

## 12. Backup and restore

- [ ] Implement database backup export.
- [ ] Implement restore flow with explicit destructive confirmation.
- [ ] Validate restored database before replacing active data where practical.
- [ ] Document backup/restore behavior in the app.

## 13. Packaging validation

- [ ] Validate development build on Linux.
- [ ] Validate production build on Linux.
- [ ] Validate Windows build strategy.
- [ ] Check Tauri/WebView runtime assumptions for Windows.
- [ ] Check Tauri/WebKitGTK assumptions for Linux.
- [ ] Document user-level install or portable run options.

## 14. Engineering safety

- [ ] Add or update tests in each implementation slice that changes behavior.
- [ ] Add migration tests or verification for schema creation and indexes.
- [ ] Add regression tests for first-run setup.
- [ ] Add regression tests for SKU and barcode uniqueness.
- [x] Add regression tests for alert-window calculations and notification deduplication. <!-- sdd-owner: implementation -->
- [x] Add regression tests for partial lot resolution. <!-- sdd-owner: implementation -->
- [ ] Add regression tests for CSV mapping/import validation.
- [ ] Add regression tests for report generation/export behavior.
- [ ] Add regression tests for backup/restore validation.
- [ ] Verify logs are created in the app-local log directory.
- [ ] Verify logs avoid sensitive product, SKU, barcode, imported-row, and notes content by default.

## 15. MVP verification

- [ ] Verify first-run store flow.
- [ ] Verify product SKU uniqueness.
- [ ] Verify multiple barcodes per product.
- [ ] Verify scanner keyboard-wedge workflow.
- [x] Verify lot alert default and override behavior. <!-- sdd-owner: implementation -->
- [x] Verify daily notification deduplication. <!-- sdd-owner: implementation -->
- [ ] Verify expired lots remain prominent after notification stop.
- [ ] Verify partial lot resolution.
- [ ] Verify CSV import with mapped columns.
- [ ] Verify PDF report export.
- [ ] Verify offline startup and persistence.
