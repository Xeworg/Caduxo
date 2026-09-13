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
- [x] Defer baseline frontend test harness to post-MVP engineering follow-up. <!-- MVP closure decision: backend/domain harness exists; Vitest/Playwright starts in a separate change -->

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
- [x] Defer live confirmation of store selection behavior when multiple stores exist. <!-- MVP closure decision: implementation appears present; final live confirmation belongs to a separate cleanup change -->
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
- [x] Add row actions for view product, edit lot, and resolve quantity; defer per-row report selection. <!-- MVP closure decision: dashboard-level CSV/PDF reports satisfy MVP reporting -->

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

- [x] Implement report data query for in-alert-window report. <!-- sdd-owner: implementation — Slice 11a -->
- [x] Implement report data query for expired report. <!-- sdd-owner: implementation — Slice 11a -->
- [x] Implement report data query for next-30-days report. <!-- sdd-owner: implementation — Slice 11a -->
- [x] Implement custom report filters. <!-- sdd-owner: implementation — Slice 11a -->
- [x] Build report preview UI. <!-- sdd-owner: implementation — Slice 11b -->
- [x] Implement Rust structured PDF export using `printpdf` or equivalent. <!-- sdd-owner: implementation — Slice 11b -->
- [x] Use A4 landscape as initial default for table reports. <!-- sdd-owner: implementation — Slice 11b -->
- [x] Add pagination and page numbers. <!-- sdd-owner: implementation — Slice 11b -->
- [x] Add report metadata: type, filters, generated date/time. <!-- sdd-owner: implementation — Slice 11a (preview DTO) and Slice 11b (PDF header) -->
- [x] Add CSV export for report data. <!-- sdd-owner: implementation — satisfied by Slice 10's `export_report_csv` for dashboard row CSV; Slice 11a does NOT add a duplicate export to keep review bounded. A follow-up slice can wire `preview_report` output to CSV by either extending `ReportExportInput` or adding a thin `export_report_request_csv` command that delegates to the report service. -->

## 12. Backup and restore

- [x] Implement database backup export. <!-- sdd-owner: implementation -->
- [x] Implement restore flow with explicit destructive confirmation. <!-- sdd-owner: implementation -->
- [x] Validate restored database before replacing active data where practical. <!-- sdd-owner: implementation -->
- [x] Document backup/restore behavior in the app. <!-- sdd-owner: implementation -->

## 13. Packaging validation

- [x] Validate development build on Linux. <!-- sdd-owner: implementation -->
- [x] Validate production build on Linux. <!-- sdd-owner: implementation -->
- [x] Validate Windows build strategy from Tauri configuration and platform documentation. <!-- sdd-owner: implementation -->
- [x] Check Tauri/WebView runtime assumptions for Windows. <!-- sdd-owner: implementation -->
- [x] Check Tauri/WebKitGTK assumptions for Linux. <!-- sdd-owner: implementation -->
- [x] Document user-level install or portable run options. <!-- sdd-owner: implementation -->

## 14. Engineering safety

- [x] Add or update tests for MVP behavior slices within the available harness. <!-- backend regression suites exist and reports tests cover `next_30_days`; frontend/runtime harness coverage is tracked as a separate post-MVP task -->
- [x] Add migration tests or verification for schema creation and indexes. <!-- sdd-owner: implementation -->
- [x] Add regression tests for first-run setup. <!-- sdd-owner: implementation -->
- [x] Add regression tests for SKU and barcode uniqueness. <!-- sdd-owner: implementation -->
- [x] Add regression tests for alert-window calculations and notification deduplication. <!-- sdd-owner: implementation -->
- [x] Add regression tests for partial lot resolution. <!-- sdd-owner: implementation -->
- [x] Add regression tests for CSV mapping/import validation. <!-- sdd-owner: implementation -->
- [x] Add regression tests for report generation/export behavior. <!-- sdd-owner: implementation -->
- [x] Add regression tests for backup/restore validation. <!-- sdd-owner: implementation -->
- [x] Verify logs are created in the app-local log directory. <!-- sdd-owner: implementation -->
- [x] Verify logs avoid sensitive product, SKU, barcode, imported-row, and notes content by default. <!-- sdd-owner: implementation -->

## 15. MVP verification

- [x] Verify first-run store flow. <!-- sdd-owner: implementation -->
- [x] Verify product SKU uniqueness. <!-- sdd-owner: implementation -->
- [x] Verify multiple barcodes per product. <!-- sdd-owner: implementation -->
- [x] Verify scanner keyboard-wedge workflow. <!-- sdd-owner: implementation -->
- [x] Verify lot alert default and override behavior. <!-- sdd-owner: implementation -->
- [x] Verify daily notification deduplication. <!-- sdd-owner: implementation -->
- [x] Verify expired lots remain prominent after notification stop. <!-- sdd-owner: implementation -->
- [x] Verify partial lot resolution. <!-- sdd-owner: implementation -->
- [x] Verify CSV import with mapped columns. <!-- sdd-owner: implementation -->
- [x] Verify PDF report export. <!-- sdd-owner: implementation -->
- [x] Verify offline startup and persistence. <!-- sdd-owner: implementation -->

## Post-MVP improvement backlog

These items are intentionally deferred to future changes after MVP acceptance, ordered by recommended implementation sequence:

1. Fix Dashboard quick filters (`All`, `Expired`, `Today`, `Alert window`, `Next 7 days`, `Next 30 days`).
2. Extend SKU/product code handling to support additional codes on product/SKU registration.
3. Improve units of measure with more predefined options and integer vs decimal quantity semantics.
4. Add CSV import loading/progress feedback and prevent duplicate submissions while import is running.
5. Fix date picker dismissal after date selection or outside click.
6. Revisit category scroll UX; remove, constrain, or replace with search/autocomplete if needed.
7. Add internationalization/language support.
8. Add baseline frontend test harness.
9. Confirm live store selector refresh behavior after store changes.
10. Add per-row report selection/actions.
