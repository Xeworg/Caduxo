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
- [ ] Block expiry lot creation until at least one store exists. <!-- deferred until lot creation command exists; `has_store` precondition command added -->
- [x] Remember last selected store in settings. <!-- sdd-owner: implementation -->

## 4. Product catalog

- [ ] Implement product create/update/archive commands.
- [ ] Enforce required unique SKU.
- [ ] Implement category list CRUD.
- [ ] Implement product default alert-days-before with software suggestion of 30 days.
- [ ] Implement barcode add/remove/list commands.
- [ ] Enforce barcode uniqueness across products.
- [ ] Implement product search by description, SKU, and barcode.
- [ ] Build product list UI.
- [ ] Build product form UI.
- [ ] Build product detail UI with barcode list and expiry lots.

## 5. Stores and internal locations

- [x] Implement store CRUD. <!-- sdd-owner: implementation -->
- [x] Implement optional internal location CRUD per store. <!-- sdd-owner: implementation -->
- [x] Enforce unique location name per store. <!-- sdd-owner: implementation -->
- [x] Build store/local management UI. <!-- sdd-owner: implementation -->
- [x] Build optional internal location UI. <!-- sdd-owner: implementation -->

## 6. Expiry lots

- [ ] Implement expiry lot create/update/archive commands.
- [ ] Pre-fill lot unit from product default when available.
- [ ] Pre-fill lot alert days from product default.
- [ ] Allow per-lot alert-days-before override.
- [ ] Require store selection only when multiple stores exist.
- [ ] Support optional internal location and batch code.
- [ ] Implement partial quantity resolution.
- [ ] Record partial resolutions in `lot_resolution_events`.
- [ ] Build lot form UI.
- [ ] Build resolve quantity UI.

## 7. Dashboard

- [ ] Implement dashboard query command.
- [ ] Compute expired, today, alert-window, and next-30-days groups.
- [ ] Build urgency cards.
- [ ] Build prominent expired section.
- [ ] Build urgent lot table sorted by urgency.
- [ ] Add store/local filter.
- [ ] Add quick filters: expired, today, next 7 days, next 30 days, alert window.
- [ ] Add row actions: view product, edit lot, resolve quantity, report selection.

## 8. Scanner/search workflow

- [ ] Build always-visible scan/search input.
- [ ] On Enter, search exact barcode first.
- [ ] If no barcode match, search exact SKU.
- [ ] If match exists, open product or lot entry flow.
- [ ] If no match exists, open quick product creation with scanned value pre-filled.
- [ ] Support manual typed SKU/UPC input.

## 9. Local notifications

- [ ] Add notification permission/request flow.
- [ ] Implement due notification query using alert window rules.
- [ ] Use `notification_log` to prevent duplicate same-day notifications.
- [ ] Trigger notification check on app startup.
- [ ] Trigger periodic notification check while app is open.
- [ ] Stop OS notifications after expiry date.
- [ ] Keep expired lots prominent on dashboard after notification period ends.

## 10. CSV import/export

- [ ] Implement CSV file selection flow.
- [ ] Detect CSV headers.
- [ ] Build column mapping UI for SKU, description, UPC/barcode.
- [ ] Validate required mapped fields.
- [ ] Implement import preview with invalid rows and duplicate warnings.
- [ ] Implement conflict strategies: skip, update, review.
- [ ] Import products and barcodes.
- [ ] Export products CSV.
- [ ] Export report rows CSV.

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
- [ ] Add regression tests for alert-window calculations and notification deduplication.
- [ ] Add regression tests for partial lot resolution.
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
- [ ] Verify lot alert default and override behavior.
- [ ] Verify daily notification deduplication.
- [ ] Verify expired lots remain prominent after notification stop.
- [ ] Verify partial lot resolution.
- [ ] Verify CSV import with mapped columns.
- [ ] Verify PDF report export.
- [ ] Verify offline startup and persistence.
