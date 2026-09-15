# PRD: Caduxo — Product Expiry Tracker

Caduxo is a local-first desktop application focused only on product expiry tracking. It is not a billing, POS, purchasing, or full inventory system. The app prioritizes fast product registration by SKU/UPC, barcode scanner input, offline use, and multi-platform packaging without administrator permissions for normal execution.

## Decision summary

| Area | Recommendation |
|------|----------------|
| App type | Desktop, offline-first |
| Priority platforms | Windows + Linux |
| Business scope | Multiple stores/locations supported |
| UI shell | Tauri v2 |
| Backend | Rust |
| Database | SQLite local file |
| Packaging | Portable executable / user-level installer where possible |
| Sync | Out of scope for v1 |
| Product identity | SKU and UPC/barcode-first |
| Product structure | Product catalog + expiry lots + store/location context |
| Scanner mode | Keyboard-wedge scanner input |
| CSV import | Simple CSV with optional column mapping |
| Reports | Simple printable expiry reports with PDF export |
| POS/billing | Explicitly out of scope |
| Admin rights | Must not be required for normal execution |

## Product goal

Help local businesses track product expiry dates so staff can identify what is expired or close to expiry before losses occur. Caduxo only manages expiry visibility and related product identification; it does not manage sales, payments, accounting, or stock valuation.

## Target users

- Store owners or managers who need expiry control without replacing their POS.
- Staff who receive products and register expiry dates.
- Staff who check shelves, storage rooms, refrigerators, or warehouses.
- Businesses that already use barcode scanners and want scanner-friendly data entry.

## Core problem

Expiry dates are often checked manually, written in notebooks, or lost inside general inventory systems. The user needs a dedicated tool that answers:

1. What expires today?
2. What expires this week?
3. Which products should be used, sold, donated, moved, or discarded first?
4. Which SKU/UPC does this expiring item belong to?
5. Where is each product located?

## Success criteria

- A user can add or find a product quickly by scanning or typing a UPC/SKU.
- The dashboard immediately shows expired and soon-to-expire products.
- The app works offline after installation or extraction.
- The app stores data locally and does not require cloud accounts.
- The app avoids POS, billing, payment, and accounting features.
- Normal use does not require administrator permissions.

## Recommended technical stack

### Primary recommendation: Tauri v2 + Rust + SQLite

Use Tauri with a Rust backend because Tauri is Rust-native. This avoids running a separate Go sidecar process and keeps packaging simpler.

| Layer | Choice | Reason |
|-------|--------|--------|
| Desktop shell | Tauri v2 | Small desktop bundle, native integration, cross-platform path |
| Backend | Rust | First-class Tauri support, single process, strong type safety |
| Data | SQLite | Embedded, local file, no database server |
| DB crate | `rusqlite` or `sqlx` | `rusqlite` is simple; `sqlx` is stronger for typed queries |
| Frontend | Svelte, Solid, or React | Any works; Svelte/Solid keep the UI lightweight |
| Styling | CSS or Tailwind | Tailwind is productive but adds build complexity |

### Important packaging note

Tauri applications depend on the operating system WebView:

- Windows: WebView2 is usually present on modern systems, but not guaranteed.
- macOS: WebKit is system-provided.
- Linux: WebKitGTK may need to be present depending on the distribution.

If the strict requirement is “copy one binary to any machine and run with no runtime dependency surprises,” then a pure Rust desktop UI such as `egui`/`eframe` is safer than Tauri. If the requirement is “modern lightweight desktop app with no admin rights for normal use on target machines,” Tauri + Rust remains the best fit.

## Architecture standard

Caduxo shall use a simple modular layered architecture, not a heavy enterprise/clean-architecture framework. The goal is to keep the app easy to test, debug, and extend while avoiding unnecessary abstraction.

```text
Svelte UI
  ↓
Tauri command API
  ↓
Application services
  ↓
Domain logic + repositories
  ↓
SQLite
```

Architecture rules:

- Svelte owns presentation, forms, navigation, and lightweight UI state.
- Tauri commands are thin adapters that receive DTOs, call services, and return user-safe errors.
- Application services implement use cases and coordinate validation, repositories, transactions, logging, and error handling.
- Domain modules contain pure business rules that are cheap to unit test.
- Repositories contain SQLite access and mapping; they should not contain complex business rules.
- Product, lot, alert, notification, import, report, and backup behavior should be organized by feature module where practical.

## v1 scope

### Product catalog management

- Categories are managed through a user-editable list.
- Create product by typing or scanning UPC/barcode.
- Create product by internal SKU when UPC is unavailable.
- Edit product.
- Delete product.
- Mark product as consumed, sold, discarded, donated, or archived.
- Support quantity and unit.
- Support category.
- Support storage location.
- Support optional batch/lot code.
- Support SKU.
- Support UPC/barcode.
- Support scanner input as keyboard wedge mode: most scanners type the code and press Enter.
- Products can be created manually.
- Products can be imported from a simple compatible CSV file with only UPC, SKU, and description. Other product details can be completed later inside the app.

### Expiry lot tracking

- Store one or more expiry lots per product.
- Each lot stores expiry date, quantity, optional batch code, optional location, and alert-days-before value.
- Show status:
  - Expired
  - Expires today
  - Expires soon
  - Safe
- Configurable alert days per expiry lot; user chooses how many days before expiry to be alerted when registering the lot.
- Automatic alert calculation based on current date and each lot's alert-days setting.
- Once a lot enters its alert window, the app should alert once per day until the expiry date or until the lot is resolved/archived.
- Product-level default alert-days value to speed up lot entry.
- When creating a lot, the app pre-fills the product default alert-days value, but the user can override it for that specific lot.
- Software global default alert-days value is 30 days.
- When registering a product, the app suggests 30 days, but the user must confirm or define the product default alert-days value.
- That product default is then used to pre-fill new expiry lots for that product, while still allowing per-lot override.
- Sort by earliest expiry first.

### Dashboard

The main screen should be an operational dashboard focused on what the user needs to act on today.

- Summary cards:
  - Expired lots.
  - Lots expiring today.
  - Lots inside their alert window.
  - Total active lots.
- Urgent list sorted by expiry date.
- Scan/search field always visible for SKU/UPC lookup.
- Store/local filter: all stores or one selected store.
- Quick filters:
  - Expired.
  - Today.
  - Next 7 days.
  - Alert window.
- Quick actions from each row:
  - View product.
  - Edit lot.
  - Mark resolved.
  - Print/report selection.

### Calendar tab

A **Calendar** tab appears in the main navigation between Products and Reports. It opens at the current month with today highlighted and selected, and shows a per-day calendar grid. Each day that has one or more expiring lots displays a dot badge under the day number. Clicking a day reveals a detail panel listing all lots expiring on that date, with columns for product, quantity, unit, store, location, days remaining, and status. Clicking a lot row opens the lot detail/edit overlay. The Calendar tab sources its data from the same dashboard lot list and performs no additional backend calls during month navigation in v1.

### SKU/UPC and scanner workflow

- User can place the cursor in a scan/search field and scan a UPC.
- If the UPC exists, the app opens the product or starts a new expiry entry for that product.
- If the UPC does not exist, the app opens a quick-create product form with the scanned code pre-filled.
- User can type a SKU manually when the product has no UPC.
- SKU is required and must be unique inside the local database.
- A product can have multiple UPC/barcode values.
- UPC values should be unique across products, but the app should allow correction if a code was registered incorrectly.

### Stores and locations

- On first run, the app asks the user to create their first store/local before registering expiry lots.
- User can create additional stores/locales inside the app after opening the program.
- If multiple stores/locales exist, the user selects the store/local when creating an expiry lot.
- Store/local is required only when the user has multiple stores/locales; with one store it can be implicit.
- Each lot can also have an optional internal location, such as shelf, aisle, fridge, warehouse, or display area.
- Dashboard can show all stores or filter by one store.

### Search and filters

- Search by product name.
- Search by SKU.
- Search by UPC/barcode.
- Filter by category.
- Filter by store/local.
- Filter by internal location.
- Filter by expiry status.
- Filter archived items separately.

### Simple reports

Reports are not advanced analytics in v1. They are practical lists that the user can view, print, or export.

- Expired products report.
- Products expiring today report.
- Products expiring in the next 30 days report.
- Products inside their configured alert window report.
- Report filters:
  - Store/local.
  - Date range.
  - Category.
  - Internal location.
  - Status.
- Report output:
  - Printable view.
  - PDF export if Tauri/platform support is practical.
  - CSV export as fallback.

Report columns:

| Column | Notes |
|--------|-------|
| SKU | Required product identifier |
| UPC/barcode | Show primary or matched barcode when available |
| Description | Product name |
| Store/local | Where the lot belongs |
| Location | Shelf, aisle, fridge, warehouse, etc. |
| Quantity | Remaining quantity in the lot |
| Unit | Unit of measure |
| Expiry date | Main report date |
| Days remaining | Negative number when expired |
| Alert days before | Configured alert threshold |
| Batch code | Optional |

### Data ownership and import

- Store data locally.
- Export CSV.
- Import a simple product catalog CSV with UPC, SKU, and description.
- Imported products can be completed or corrected later inside the app.
- Importing expiry lots is out of scope for the first MVP unless explicitly added later.
- Manual backup by copying the database file or exporting a backup file.

## Out of scope for v1

- Cloud sync.
- Multi-user collaboration.
- Mobile app.
- Advanced scanner driver integration. Basic keyboard-wedge scanner input is in scope.
- POS, billing, payment, invoicing, accounting, or cash register features.
- Automatic notifications running in the background after app close.
- Role-based permissions.

## Functional requirements

### Product fields

| Field | Required | Notes |
|-------|----------|-------|
| Description | Yes | Human-readable product name/description |
| SKU | Yes | Internal product identifier, unique locally |
| UPC / barcode | No | Scanner-friendly identifier; multiple values allowed |
| Category | No | User-defined or predefined |
| Default unit | No | Chosen from the unit catalog — see capability spec |
| Default alert days before | No | Used to pre-fill new expiry lots |
| Notes | No | Free text |

### Expiry lot fields

| Field | Required | Notes |
|-------|----------|-------|
| Product | Yes | Linked by SKU/UPC search or manual selection |
| Store/local | Yes | Selected when creating the lot |
| Internal location | No | Optional shelf, fridge, warehouse, etc. |
| Quantity | Yes | Number greater than zero |
| Unit | Yes | Pre-filled from product default when available; resolves through `default_unit_id` |
| Expiry date | Yes | Main tracking field |
| Alert days before | Yes | Pre-filled from product/global default, editable per lot |
| Batch / lot code | No | Useful for repeated products |
| Notes | No | Free text |
| Status | Yes | Active, resolved, archived |

### Expiry status rules

| Status | Rule |
|--------|------|
| Expired | Expiry date is before today |
| Expires today | Expiry date is today |
| Expires soon | Expiry date is after today and within alert window |
| Safe | Expiry date is after alert window |

## PDF report strategy

Caduxo reports are simple operational documents, not complex visual layouts. For MVP, PDF export should be generated directly by the Rust backend rather than depending on Chromium, wkhtmltopdf, or platform-specific WebView APIs.

Recommended approach:

- Generate structured PDFs with a Rust PDF library such as `printpdf`.
- Use a predictable report layout: title, filters, generation date, and table.
- Prioritize readability and print quality over advanced styling.
- Keep the report preview printable from the UI as a fallback.
- Avoid external PDF binaries because they complicate portable Windows/Linux packaging.

Out of scope for MVP:

- Pixel-perfect HTML-to-PDF rendering.
- Bundled Chromium for PDF generation.
- Platform-specific native WebView PDF APIs.

## Notification strategy

Caduxo v1 keeps notifications local and simple.

- The app shows alerts inside the dashboard.
- The app can show operating-system notifications for the current user while the app is running.
- Notifications are calculated locally from expiry date and alert-days-before.
- Notification frequency: once per day for each active lot from the alert start date through the expiry date.
- Alert start date formula: `expiry_date - alert_days_before`.
- Notifications stop when the lot is marked as resolved or archived.
- After the expiry date, daily system notifications stop, but expired lots must remain highly visible in the dashboard.
- No WhatsApp, Telegram, email, cloud service, or remote notification channel is required for v1.

## Engineering safety standards

Caduxo shall treat automated tests and structured logging as project standards, not optional cleanup. Every implementation slice should include tests for the behavior it adds or changes, unless a task is explicitly documentation-only or a short-lived scaffold that cannot be meaningfully tested yet. When a test is deferred, the slice notes must explain why and where coverage will be added.

### Testing standard

- Backend/domain behavior must be covered with Rust tests close to the code under test.
- Database migrations and persistence rules must have focused tests or reproducible verification scripts.
- Frontend behavior that contains non-trivial state, validation, or user flow logic must have component or unit tests where practical.
- Critical workflows require regression tests as they are implemented: first-run setup, SKU/barcode uniqueness, lot alert calculations, partial resolution, notification deduplication, CSV import mapping, report generation, and backup/restore validation.
- Pull requests/slices should report which tests were added and which commands were run.

### Logging standard

Caduxo shall include a structured application logger early in the foundation work. The logger must help diagnose local desktop failures without exposing sensitive business data.

- Use Rust-side structured logging for backend startup, database initialization, migrations, command errors, notification checks, import/export operations, report generation, and backup/restore operations.
- Log levels should distinguish developer diagnostics from user-relevant warnings/errors.
- Logs should avoid storing product names, barcodes, SKU values, full file contents, or personal/business-sensitive notes unless explicitly required for a specific debug mode.
- Logs should be written to an app-specific local data/log directory and be safe for normal offline use.
- UI-visible errors should remain user-friendly; detailed technical information belongs in logs.

## Non-functional requirements

- Offline by default.
- Fast startup on ordinary laptops.
- Local data remains readable across app updates.
- No administrator permissions for normal execution.
- No external database server.
- No telemetry in v1.
- Local system notifications should be supported for the current user on their own computer when the app is running and permission is available.
- External notification channels such as WhatsApp, Telegram, email, or cloud messaging are out of scope for v1.
- Clear backup/export path.

## CSV import draft

The first import format should be intentionally simple so a user can prepare it from Excel, LibreOffice, or a POS export.

### Simple product catalog CSV

```csv
upc,sku,description
7501234567890,LECHE-001,Leche entera 1L
7509876543210,ARROZ-001,Arroz blanco 2kg
```

Rules:

- `sku` is required.
- `description` is required and becomes the product name.
- `upc` is optional when a product has no barcode, but recommended.
- If the SKU already exists, the importer should offer to skip, update, or review.
- The importer supports manual column mapping when column names are different.
- Extra fields are ignored in the first MVP unless explicitly mapped to supported fields.

## First user flow

1. User opens Caduxo.
2. Dashboard shows urgent products.
3. User clicks “Add product”.
4. User scans UPC or types SKU.
5. If the SKU/UPC exists, the app opens a fast expiry-lot entry form.
6. If the product does not exist, the app opens a quick product form with the code pre-filled.
7. User selects store/local, enters quantity, unit, expiry date, optional internal location/batch, and confirms or changes the pre-filled alert-days-before value.
8. Product appears automatically in alert lists when it reaches its configured alert window.
9. User marks the expiry lot as resolved when action is taken.

## Acceptance criteria for MVP

- [ ] User can add a product with description and mandatory SKU.
- [ ] User can add expiry lots with quantity, unit, expiry date, store/local, and alert-days-before.
- [ ] User can scan or type UPC to find/create a product.
- [ ] A product can have multiple UPC/barcode values.
- [ ] SKU is mandatory and unique.
- [ ] User can edit and delete a product.
- [ ] User can view expired products.
- [ ] User can view products expiring soon.
- [ ] User can use the dashboard to filter urgent lots by store/local and date urgency.
- [ ] User can generate a simple printable expiry report.
- [ ] User can export expiry reports to PDF.
- [ ] Software suggests 30 default alert days when creating a product.
- [ ] User must confirm or define product-level default alert-days-before.
- [ ] User can set or override alert-days-before when registering each expiry lot.
- [ ] On first run, user is asked to create the first store/local.
- [ ] User can assign expiry lots to a store/local.
- [ ] Alert lists update automatically based on current date and each lot's alert-days-before value.
- [ ] User can search and filter products.
- [ ] User can export products and expiry lots to CSV.
- [ ] User can import a simple product CSV with UPC, SKU, and description.
- [ ] User can map CSV columns manually when the file uses different column names.
- [ ] App runs offline.
- [ ] App data persists after closing and reopening.
- [ ] User can create a manual backup/export and restore from backup.
- [ ] Database migrations preserve existing local data across app updates.

## Suggested implementation phases

### Phase 1 — Foundation

- Initialize Tauri v2 project.
- Choose frontend framework.
- Add SQLite storage.
- Define product schema and migrations.

### Phase 2 — MVP inventory

- Product catalog CRUD with mandatory SKU and multiple UPC/barcodes.
- Store/local CRUD created by the user inside the app.
- Expiry lot CRUD linked to products and optionally selected stores.
- Scanner-friendly scan/search input.
- Product default alert-days-before and editable per-lot alert setting.
- Automatic alert dashboard sections.
- Simple printable reports for expired and soon-to-expire lots.
- Search and filters.

### Phase 3 — Data safety

- CSV export.
- CSV import.
- Backup/restore documentation.

### Phase 4 — Packaging

- Portable build strategy.
- User-level installer strategy.
- Test on target OS.

## Edge cases and validation

- SKU cannot be empty and must be unique.
- Barcode cannot be assigned to more than one product.
- Expiry date cannot be empty.
- Alert days before must be zero or greater.
- Quantity must be greater than zero.
- If no store exists on first run, the app must ask the user to create the first store/local.
- Deleting products with active lots should be blocked or converted to deactivation/archive.
- Partial lot resolution is supported: user can resolve part of the quantity and keep the remaining quantity active.
- Deleting stores with active lots should be blocked or require moving/archiving lots first.

## MVP report set

Recommended first reports:

1. In alert window.
2. Expired.
3. Next 30 days.
4. Custom report with filters for store/local, internal location, category, status, and date range.

## Open questions

1. Is “no administrator permissions” required only for running the app, or also for first installation?
2. Will the user manage a few dozen products, hundreds, or thousands?
3. Should categories and locations be free text, predefined lists, or both?
4. Confirm final PDF library after implementation spike: `printpdf` is the preferred starting point for structured reports.
5. Should each store/local have independent alert settings, or should alert rules stay product/lot-based only?

## Database structure draft

Caduxo uses a local SQLite database. The model separates catalog data from expiry lots so one product can have many barcodes, stores, locations, and expiry records.

### Entity overview

```text
stores 1 ── * store_locations
stores 1 ── * expiry_lots
store_locations 1 ── * expiry_lots
products 1 ── * product_barcodes
products 1 ── * expiry_lots
```

### `stores`

Stores, branches, or business locations created by the user.

| Column | Type | Required | Notes |
|--------|------|----------|-------|
| id | TEXT | Yes | UUID primary key |
| name | TEXT | Yes | Store/local name |
| code | TEXT | No | Optional short code, unique when present |
| notes | TEXT | No | Free text |
| is_active | INTEGER | Yes | 1 active, 0 inactive |
| created_at | TEXT | Yes | ISO datetime |
| updated_at | TEXT | Yes | ISO datetime |

### `store_locations`

Optional internal locations inside a store. Users can ignore this feature.

| Column | Type | Required | Notes |
|--------|------|----------|-------|
| id | TEXT | Yes | UUID primary key |
| store_id | TEXT | Yes | References `stores.id` |
| name | TEXT | Yes | Example: shelf, fridge, warehouse, aisle |
| notes | TEXT | No | Free text |
| is_active | INTEGER | Yes | 1 active, 0 inactive |
| created_at | TEXT | Yes | ISO datetime |
| updated_at | TEXT | Yes | ISO datetime |

Unique rule: `store_id + name` should be unique.

### `products`

Product catalog. SKU is mandatory because one product can have multiple UPC/barcodes.

| Column | Type | Required | Notes |
|--------|------|----------|-------|
| id | TEXT | Yes | UUID primary key |
| sku | TEXT | Yes | Unique internal identifier |
| description | TEXT | Yes | Product name/description |
| category | TEXT | No | Optional category |
| default_unit | TEXT | No | Example: units, box, kg, g, L, ml | (legacy echo; prefer `default_unit_id` when set) |
| default_unit_id | TEXT | No | FK to `unit_definitions.id`; preferred link |
| unit_type | TEXT | No | `integer` or `decimal`; derived from the linked catalog unit |
| default_alert_days_before | INTEGER | No | Default used when creating expiry lots |
| notes | TEXT | No | Free text |
| is_active | INTEGER | Yes | 1 active, 0 inactive |
| created_at | TEXT | Yes | ISO datetime |
| updated_at | TEXT | Yes | ISO datetime |

### `product_barcodes`

Multiple UPC/EAN/barcodes per product.

| Column | Type | Required | Notes |
|--------|------|----------|-------|
| id | TEXT | Yes | UUID primary key |
| product_id | TEXT | Yes | References `products.id` |
| barcode | TEXT | Yes | UPC/EAN/barcode value |
| barcode_type | TEXT | No | UPC, EAN, other |
| is_primary | INTEGER | Yes | 1 primary, 0 secondary |
| created_at | TEXT | Yes | ISO datetime |

Unique rule: `barcode` should be unique across the database.

### `expiry_lots`

Expiry records. This is the main operational table.

| Column | Type | Required | Notes |
|--------|------|----------|-------|
| id | TEXT | Yes | UUID primary key |
| product_id | TEXT | Yes | References `products.id` |
| store_id | TEXT | Yes | References `stores.id` |
| location_id | TEXT | No | References `store_locations.id`; optional |
| quantity | REAL | Yes | Remaining quantity |
| unit | TEXT | Yes | Unit for this lot |
| expiry_date | TEXT | Yes | ISO date `YYYY-MM-DD` |
| alert_days_before | INTEGER | Yes | Copied from product default, editable per lot |
| batch_code | TEXT | No | Optional lot/batch code |
| status | TEXT | Yes | active, resolved, archived |
| resolution | TEXT | No | consumed, sold, discarded, donated, other |
| resolved_at | TEXT | No | ISO datetime when resolved |
| notes | TEXT | No | Free text |
| created_at | TEXT | Yes | ISO datetime |
| updated_at | TEXT | Yes | ISO datetime |

Derived values, not stored unless needed later:

- `days_remaining = expiry_date - today`
- `alert_start_date = expiry_date - alert_days_before`
- `is_expired = expiry_date < today`
- `is_in_alert_window = today >= alert_start_date`
- `should_notify_today = status = active AND today >= alert_start_date AND today <= expiry_date`

### `app_settings`

Simple key-value settings.

| Column | Type | Required | Notes |
|--------|------|----------|-------|
| key | TEXT | Yes | Primary key |
| value | TEXT | Yes | Stored as text/JSON depending on setting |
| updated_at | TEXT | Yes | ISO datetime |

Suggested settings:

| Key | Default | Notes |
|-----|---------|-------|
| `global_default_alert_days_before` | `30` | Suggested when creating products; user confirms/changes product default |
| `notifications_enabled` | `true` | Local OS notifications |
| `daily_notification_time` | `09:00` | Preferred time for daily local alerts |
| `last_selected_store_id` | empty | Convenience for data entry |

### `notification_log`

Tracks which daily alerts have already been shown so the same lot is not notified repeatedly on the same day.

| Column | Type | Required | Notes |
|--------|------|----------|-------|
| id | TEXT | Yes | UUID primary key |
| expiry_lot_id | TEXT | Yes | References `expiry_lots.id` |
| notification_date | TEXT | Yes | ISO date `YYYY-MM-DD` |
| shown_at | TEXT | Yes | ISO datetime |

Unique rule: `expiry_lot_id + notification_date` should be unique.

### Suggested indexes

| Table | Index | Purpose |
|-------|-------|---------|
| `products` | `sku` unique | Fast SKU search |
| `product_barcodes` | `barcode` unique | Fast scanner lookup |
| `expiry_lots` | `expiry_date` | Dashboard and reports |
| `expiry_lots` | `store_id, expiry_date` | Store-filtered reports |
| `expiry_lots` | `product_id` | Product detail screen |
| `store_locations` | `store_id, name` unique | Avoid duplicate internal locations |

### Report query basis

Reports should read from `expiry_lots` joined with `products`, `stores`, optional `store_locations`, and optional primary barcode.

Common report filters:

- Expired: `expiry_date < today`.
- Next 30 days: `expiry_date >= today AND expiry_date <= today + 30 days`.
- In alert window: `today >= expiry_date - alert_days_before AND status = active`.
- Store: `store_id = selected_store_id`.
- Location: `location_id = selected_location_id`.
