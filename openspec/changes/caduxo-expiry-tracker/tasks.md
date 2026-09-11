# Tasks: Caduxo Expiry Tracker

## 1. Project foundation

- [ ] Initialize Tauri v2 project with Svelte + TypeScript.
- [ ] Configure Rust workspace and app metadata.
- [ ] Add SQLite support with `sqlx`.
- [ ] Add database migration runner.
- [ ] Add app data directory resolution for Windows/Linux.
- [ ] Add basic error handling shape shared by Tauri commands.

## 2. Database schema

- [ ] Create migration for `stores`.
- [ ] Create migration for `store_locations`.
- [ ] Create migration for `categories`.
- [ ] Create migration for `products`.
- [ ] Create migration for `product_barcodes`.
- [ ] Create migration for `expiry_lots`.
- [ ] Create migration for `lot_resolution_events`.
- [ ] Create migration for `notification_log`.
- [ ] Create migration for `app_settings`.
- [ ] Add indexes for SKU, barcode, expiry date, store/date, product lots, and store locations.

## 3. First-run setup

- [ ] Implement `is_first_run` command.
- [ ] Implement first store creation flow.
- [ ] Block expiry lot creation until at least one store exists.
- [ ] Remember last selected store in settings.

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

- [ ] Implement store CRUD.
- [ ] Implement optional internal location CRUD per store.
- [ ] Enforce unique location name per store.
- [ ] Build store/local management UI.
- [ ] Build optional internal location UI.

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

## 14. MVP verification

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
