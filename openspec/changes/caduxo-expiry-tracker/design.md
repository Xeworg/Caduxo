# Design: Caduxo Expiry Tracker

## Architecture decision

Caduxo will use a local desktop architecture:

```text
Tauri v2 desktop shell
├── Frontend UI: Svelte + TypeScript
├── Rust command layer: Tauri commands
├── Domain services: products, lots, reports, notifications, import
└── SQLite database: local file with migrations
```

## Technology choices

| Area | Choice | Reason |
|------|--------|--------|
| Desktop | Tauri v2 | Small Windows/Linux desktop app with Rust backend |
| Backend | Rust | Native Tauri integration and safe local data handling |
| Frontend | Svelte + TypeScript | Lightweight UI, low boilerplate, good desktop fit |
| Database | SQLite | Embedded local storage without server dependency |
| DB access | `sqlx` with SQLite | Migrations and typed async query support |
| IDs | UUID stored as TEXT | Portable and simple to inspect |
| Dates | ISO strings | Easy SQLite querying and export |
| PDF | Rust structured generation, starting with `printpdf` | Avoid external binaries and platform-specific PDF APIs |
| CSV | Rust `csv` crate | Reliable import/export parsing |
| Notifications | Tauri notification plugin/API | Local OS notifications while app is running |

## Module boundaries

```text
src-tauri/src/
├── main.rs
├── db/
│   ├── mod.rs
│   ├── migrations.rs
│   └── pool.rs
├── commands/
│   ├── products.rs
│   ├── stores.rs
│   ├── lots.rs
│   ├── reports.rs
│   ├── import.rs
│   └── settings.rs
├── domain/
│   ├── alerts.rs
│   ├── reports.rs
│   ├── csv_import.rs
│   └── validation.rs
└── pdf/
    └── report_pdf.rs
```

```text
src/
├── routes or views/
│   ├── Dashboard
│   ├── Products
│   ├── ProductDetail
│   ├── LotForm
│   ├── Stores
│   ├── Categories
│   ├── ImportCsv
│   ├── Reports
│   └── Settings
├── components/
│   ├── ScanSearchBox
│   ├── UrgencyCards
│   ├── LotTable
│   ├── ReportPreview
│   └── ColumnMapper
└── api/
    └── tauriCommands.ts
```

## Database schema

### `stores`

```sql
CREATE TABLE stores (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  code TEXT UNIQUE,
  notes TEXT,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### `store_locations`

```sql
CREATE TABLE store_locations (
  id TEXT PRIMARY KEY,
  store_id TEXT NOT NULL REFERENCES stores(id),
  name TEXT NOT NULL,
  notes TEXT,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(store_id, name)
);
```

### `categories`

```sql
CREATE TABLE categories (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### `products`

```sql
CREATE TABLE products (
  id TEXT PRIMARY KEY,
  sku TEXT NOT NULL UNIQUE,
  description TEXT NOT NULL,
  category_id TEXT REFERENCES categories(id),
  default_unit TEXT,
  default_alert_days_before INTEGER NOT NULL DEFAULT 30,
  notes TEXT,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### `product_barcodes`

```sql
CREATE TABLE product_barcodes (
  id TEXT PRIMARY KEY,
  product_id TEXT NOT NULL REFERENCES products(id),
  barcode TEXT NOT NULL UNIQUE,
  barcode_type TEXT,
  is_primary INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);
```

### `expiry_lots`

```sql
CREATE TABLE expiry_lots (
  id TEXT PRIMARY KEY,
  product_id TEXT NOT NULL REFERENCES products(id),
  store_id TEXT NOT NULL REFERENCES stores(id),
  location_id TEXT REFERENCES store_locations(id),
  quantity REAL NOT NULL CHECK(quantity > 0),
  unit TEXT NOT NULL,
  expiry_date TEXT NOT NULL,
  alert_days_before INTEGER NOT NULL CHECK(alert_days_before >= 0),
  batch_code TEXT,
  status TEXT NOT NULL DEFAULT 'active',
  resolution TEXT,
  resolved_at TEXT,
  notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### `lot_resolution_events`

Partial resolution should be auditable. When a user resolves part of a lot, record the event and reduce the active lot quantity.

```sql
CREATE TABLE lot_resolution_events (
  id TEXT PRIMARY KEY,
  expiry_lot_id TEXT NOT NULL REFERENCES expiry_lots(id),
  quantity REAL NOT NULL CHECK(quantity > 0),
  resolution TEXT NOT NULL,
  notes TEXT,
  created_at TEXT NOT NULL
);
```

### `notification_log`

```sql
CREATE TABLE notification_log (
  id TEXT PRIMARY KEY,
  expiry_lot_id TEXT NOT NULL REFERENCES expiry_lots(id),
  notification_date TEXT NOT NULL,
  shown_at TEXT NOT NULL,
  UNIQUE(expiry_lot_id, notification_date)
);
```

### `app_settings`

```sql
CREATE TABLE app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

## Core commands

### Setup and settings

- `is_first_run() -> bool`
- `create_store(input) -> Store`
- `list_stores() -> Vec<Store>`
- `get_settings() -> Settings`
- `update_settings(input) -> Settings`

### Product catalog

- `create_product(input) -> Product`
- `update_product(input) -> Product`
- `archive_product(id) -> void`
- `get_product(id) -> ProductDetail`
- `search_products(query) -> Vec<ProductSearchResult>`
- `add_product_barcode(product_id, barcode) -> Barcode`
- `remove_product_barcode(id) -> void`

### Expiry lots

- `create_expiry_lot(input) -> ExpiryLot`
- `update_expiry_lot(input) -> ExpiryLot`
- `archive_expiry_lot(id) -> void`
- `resolve_lot_quantity(input) -> ExpiryLot`
- `list_dashboard_lots(filters) -> DashboardLots`

### Import/export

- `preview_product_csv(file_path, mapping) -> ImportPreview`
- `import_product_csv(file_path, mapping, conflict_strategy) -> ImportResult`
- `export_products_csv(file_path) -> ExportResult`
- `export_report_csv(report_request, file_path) -> ExportResult`

### Reports and PDF

- `preview_report(request) -> ReportData`
- `export_report_pdf(request, file_path) -> ExportResult`

### Notifications

- `check_due_notifications(today) -> Vec<NotificationCandidate>`
- `mark_notification_shown(lot_id, date) -> void`

## Alert algorithm

For each active lot:

```text
alert_start_date = expiry_date - alert_days_before
should_notify_today = today >= alert_start_date AND today <= expiry_date
```

Rules:

- Notify at most once per lot per day.
- Use `notification_log` to prevent duplicates.
- Do not send OS notifications after expiry date.
- Expired lots remain visually prominent in dashboard until resolved/archived.
- Notification checks run on app startup and while the app remains open.

## Dashboard design

Main screen sections:

1. Global scan/search box.
2. Store filter.
3. Urgency cards:
   - expired
   - today
   - in alert window
   - next 30 days
4. Main lot table sorted by urgency:
   - expired first
   - today
   - alert window
   - next 30 days
5. Row actions:
   - view product
   - edit lot
   - resolve quantity
   - include in report

Expired items should use the strongest visual treatment in the application.

## CSV import design

Flow:

1. User selects CSV file.
2. App detects headers.
3. User maps columns to:
   - SKU
   - description
   - UPC/barcode
4. App validates rows.
5. App previews:
   - new products
   - duplicate SKUs
   - duplicate barcodes
   - invalid rows
6. User chooses conflict strategy:
   - skip duplicates
   - update existing product description/barcodes
   - review manually
7. App imports valid rows.

## PDF report design

Use a fixed report template:

- title
- report type
- selected filters
- generation date/time
- table rows
- page numbers

Initial page format:

- A4 landscape for wider tables.
- A4 portrait may be used for short reports.

Required report columns:

- SKU
- primary/matched barcode
- description
- store/local
- internal location
- quantity
- unit
- expiry date
- days remaining
- alert days before
- batch code

## Backup and restore

MVP should provide:

- Export database backup file.
- Restore database backup file with confirmation.
- CSV export for products/lots/report data.

Restore is destructive and must require explicit user confirmation.

## Design risks

| Risk | Response |
|------|----------|
| `printpdf` table layout becomes time-consuming | Keep table layout fixed; reduce columns if needed |
| `sqlx` compile-time checks can complicate setup | Use runtime checked queries if offline compile setup slows development |
| Local notifications differ between Windows/Linux | Treat dashboard as source of truth and notification as convenience |
| Store selection logic creates hidden state | Show selected store clearly on dashboard and forms |

## Design decisions deferred

- Exact visual theme.
- Exact PDF typography.
- Whether categories can be merged/deleted after use.
- Whether backup should include generated PDFs or only database data.
