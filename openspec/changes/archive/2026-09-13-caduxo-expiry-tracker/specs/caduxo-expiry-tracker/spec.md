# Spec: Caduxo Expiry Tracker Requirements

## Capability: First-run setup

### Requirement: first store creation

Caduxo shall require the user to create an initial store/local during first run before expiry lots can be registered.

#### Scenario: no store exists

- Given the app has no stores
- When the user opens Caduxo
- Then the app prompts the user to create the first store/local
- And expiry lot creation is unavailable until a store exists

## Capability: Product catalog

### Requirement: mandatory unique SKU

Caduxo shall require every product to have a non-empty SKU unique within the local database.

#### Scenario: duplicate SKU

- Given a product exists with SKU `ABC-001`
- When the user creates or imports another product with SKU `ABC-001`
- Then the app blocks the duplicate or asks the user to review/update the existing product

### Requirement: multiple barcodes per product

Caduxo shall allow a product to have zero, one, or multiple UPC/EAN/barcode values.

#### Scenario: scanner finds product by barcode

- Given a product has barcode `7501234567890`
- When the user scans `7501234567890` in the scan/search field
- Then the app opens that product or starts an expiry lot entry for that product

### Requirement: barcode uniqueness

Caduxo shall prevent the same barcode from being assigned to more than one product.

## Capability: Categories

### Requirement: editable category list

Caduxo shall provide a user-editable category list for product classification.

#### Scenario: category filter

- Given products are assigned to categories
- When the user filters by category
- Then dashboard/report lists show only matching products/lots

## Capability: Expiry lots

### Requirement: lot registration

Caduxo shall allow users to register expiry lots linked to a product.

Required lot fields:

- product
- quantity
- unit
- expiry date
- alert days before expiry

Conditionally required:

- store/local when multiple stores exist

Optional:

- internal location
- batch/lot code
- notes

### Requirement: product default alert days

Caduxo shall suggest 30 alert days when creating a product, but the user must confirm or change the product default.

### Requirement: lot alert override

Caduxo shall pre-fill lot alert days from the product default and allow the user to override the value for that specific lot.

### Requirement: partial resolution

Caduxo shall allow users to resolve part of a lot quantity and keep the remaining quantity active.

#### Scenario: partial quantity resolved

- Given an active lot has quantity 10
- When the user resolves 4 units
- Then the app records 4 units as resolved
- And the active lot remains with quantity 6

## Capability: Stores and locations

### Requirement: multi-store support

Caduxo shall allow users to create multiple stores/locales.

### Requirement: implicit single store

Caduxo shall not force store selection during lot creation when only one store exists.

### Requirement: optional internal locations

Caduxo shall allow users to create optional internal locations inside stores and assign them to lots.

## Capability: Dashboard

### Requirement: operational main screen

Caduxo shall provide a dashboard focused on expiry actions.

It shall include:

- expired lots
- lots expiring today
- lots inside alert window
- next 30 days
- scan/search field
- store filter

### Requirement: expired visibility

Caduxo shall show expired lots prominently until they are resolved or archived.

## Capability: Local notifications

### Requirement: daily alert window notification

Caduxo shall generate local OS notifications once per day for each active lot from `expiry_date - alert_days_before` through `expiry_date`.

### Requirement: no duplicate same-day notifications

Caduxo shall record shown notifications and prevent duplicate notifications for the same lot on the same date.

### Requirement: stop after expiry date

Caduxo shall stop daily OS notifications after the expiry date, while keeping expired lots prominent in the dashboard.

## Capability: CSV import

### Requirement: simple catalog import

Caduxo shall import product catalog data from CSV with canonical fields:

- `upc`
- `sku`
- `description`

### Requirement: column mapping

Caduxo shall allow users to map CSV columns manually when the source file uses different column names.

### Requirement: import preview

Caduxo shall show an import preview with duplicate warnings before committing imported products.

## Capability: Reports

### Requirement: MVP reports

Caduxo shall provide these reports:

- in alert window
- expired
- next 30 days
- custom filtered report

### Requirement: report filters

Reports shall support filters for:

- store/local
- internal location
- category
- status
- date range

### Requirement: PDF export

Caduxo shall export reports to PDF using Rust-side structured PDF generation.

### Requirement: CSV report export

Caduxo shall export report rows to CSV.

## Capability: Data persistence and safety

### Requirement: local persistence

Caduxo shall store all MVP data in a local SQLite database.

### Requirement: offline use

Caduxo shall support normal operation without internet access.

### Requirement: backup and restore

Caduxo shall provide a manual backup/export and restore path.

### Requirement: migrations

Caduxo shall preserve existing local data across database schema migrations.

## Capability: Engineering safety

### Requirement: tests accompany implementation

Caduxo shall add or update automated tests in the same implementation slice as backend, persistence, domain, or covered frontend behavior is introduced or changed, except for documentation-only work, explicitly justified temporary scaffolding, or frontend/runtime behavior that is accepted for MVP with manual verification and tracked for a dedicated post-MVP frontend harness.

#### Scenario: behavior slice is implemented

- Given a slice introduces backend, persistence, domain, or frontend behavior covered by the current automated harness
- When the slice is prepared for review
- Then matching tests are added or updated
- And the slice notes list the test commands that were run
- And any accepted frontend/runtime manual-verification gap is recorded as a post-MVP follow-up

### Requirement: regression coverage for critical workflows

Caduxo shall maintain regression coverage for first-run setup, SKU/barcode uniqueness, lot alert calculations, partial resolution, notification deduplication, CSV import mapping, report generation, and backup/restore validation as those workflows are implemented.

### Requirement: structured local logging

Caduxo shall provide structured local application logging for backend startup, database initialization, migrations, command errors, notification checks, import/export, report generation, and backup/restore operations.

### Requirement: safe log content

Caduxo logs shall avoid sensitive business data such as product names, barcodes, SKU values, full imported file contents, and free-form notes unless a deliberate debug mode is introduced later.
