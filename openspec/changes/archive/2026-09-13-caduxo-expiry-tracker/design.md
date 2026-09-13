# Design: Caduxo Expiry Tracker

## Architecture decision

Caduxo will use a local desktop app with a simple modular layered architecture. This is intentionally lighter than full Clean Architecture: enough separation to test and evolve safely, without adding unnecessary framework ceremony.

```text
Tauri v2 desktop shell
├── Frontend UI: Svelte + TypeScript
│   └── presentation, forms, navigation, lightweight UI state
├── Rust command layer: thin Tauri command adapters
│   └── DTO input/output, user-safe error conversion
├── Application services: feature use cases
│   └── products, stores, lots, dashboard, notifications, import, reports, backup
├── Domain modules: pure rules and calculations
│   └── alert windows, expiry status, partial resolution, validation, report filters
├── Repositories: SQLite persistence boundaries
│   └── SQL queries, transactions, database-to-DTO mapping
└── SQLite database: local file with migrations
```

Dependency direction should stay one-way: commands call services; services call domain functions and repositories; repositories talk to SQLite. Domain logic must not depend on Tauri, SQLite, or Svelte.

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
| Logging | Rust `tracing` + `tracing-subscriber` and app-local log file layer | Structured diagnostics without leaking business data |
| Tests | Rust unit/integration tests plus TypeScript/Vitest where configured | Keep implementation slices safe and regression-resistant |

## Module boundaries

```text
src-tauri/src/
├── main.rs
├── state.rs
├── error.rs
├── logging.rs
├── db/
│   ├── mod.rs
│   ├── migrations.rs
│   ├── pool.rs
│   └── repositories/
│       ├── products.rs
│       ├── stores.rs
│       ├── lots.rs
│       ├── settings.rs
│       └── notification_log.rs
├── commands/
│   ├── products.rs
│   ├── stores.rs
│   ├── lots.rs
│   ├── dashboard.rs
│   ├── reports.rs
│   ├── import.rs
│   └── settings.rs
├── services/
│   ├── products.rs
│   ├── stores.rs
│   ├── lots.rs
│   ├── dashboard.rs
│   ├── notifications.rs
│   ├── import.rs
│   ├── reports.rs
│   └── backup.rs
├── domain/
│   ├── alerts.rs
│   ├── expiry_status.rs
│   ├── lot_resolution.rs
│   ├── validation.rs
│   └── report_filters.rs
├── dto/
│   ├── products.rs
│   ├── stores.rs
│   ├── lots.rs
│   └── reports.rs
└── pdf/
    └── report_pdf.rs
```

### Layer responsibilities

- `commands/`: thin Tauri adapters only. No SQL and no complex business decisions.
- `services/`: use-case orchestration, transactions, logging points, and business error selection.
- `domain/`: pure functions and value-level rules; primary target for fast unit tests.
- `db/repositories/`: persistence and SQL mapping; primary target for temporary-SQLite tests.
- `dto/`: command input/output structs that define the frontend/backend boundary.
- `error.rs`: shared internal and command-safe error shape.
- `state.rs`: app-wide dependencies such as database pool and services.

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

## Engineering safety design

### Testing approach

Testing is part of each implementation slice. The project should prefer small, focused tests near the behavior under development instead of a large test pass at the end.

- Rust domain logic: unit tests in the owning module.
- Rust command/persistence behavior: integration-style tests using temporary SQLite databases where practical.
- Migrations: tests or verification helpers that apply migrations to a fresh temporary database and assert required tables, constraints, and indexes.
- Frontend state/validation: Vitest/component tests where practical once the UI test harness exists.
- End-to-end/manual verification: reserved for desktop shell behavior that cannot be cheaply automated early.

Each delivery slice should record test evidence in its review notes: added tests, commands run, and any deferred coverage with rationale.

### Logging approach

The foundation slice should install a Rust-side structured logger before feature workflows are implemented. Recommended baseline:

- Use `tracing` instrumentation in Rust services and Tauri commands.
- Initialize logging at app startup before database initialization.
- Write logs under the app-specific local data directory, for example `logs/caduxo.log`, with console logging enabled in development.
- Use event fields for technical identifiers and error categories, but avoid raw SKU, barcode, product description, imported row contents, or free-form notes by default.
- Convert internal errors into user-safe UI messages while preserving detailed diagnostics in logs.

Initial log coverage should include startup, app data path resolution, database open/migration, command failures, notification check summary, import/export start/end, report generation, and backup/restore start/end.

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
