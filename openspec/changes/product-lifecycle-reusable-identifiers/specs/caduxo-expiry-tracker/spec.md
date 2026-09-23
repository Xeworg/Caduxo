# Delta for Caduxo Expiry Tracker

This delta introduces a third product lifecycle state (`retired`)
distinct from the existing reversible `archived` state. It adds a
lifecycle audit table, partial-uniqueness carve-outs so retired
product SKUs and barcodes can be reused, and the UI, IPC, CSV, and
scanner surface changes needed to make the lifecycle visible to the
operator. The change targets `Capability: Product catalog`.

The proposal artifact is at
`openspec/changes/product-lifecycle-reusable-identifiers/proposal.md`.
The user has confirmed every product default the proposal surfaced
(CSV import preview advisory notice for released identifiers,
catalog `Show retired` toggle, audit-table shape, lot-code scans for
retired products, and independent retired-vs-replacement rows).

## MODIFIED Requirements

### Requirement: mandatory unique SKU

Caduxo shall require every product to have a non-empty SKU unique
within the local database among every product whose `lifecycle` is
not `retired`. A retired product's SKU MAY be reused by a future
product row; the new row starts with `lifecycle = 'active'` and is
independent of the retired row. Uniqueness across `active` and
`archived` rows is preserved.

A product's category membership is expressed as `category_ids:
string[]` (zero or more ids) backed by the `product_categories`
junction table. The legacy single nullable `products.category_id`
FK is not part of the runtime model anymore; the junction is the
canonical source of truth. See the `multi-category product model`
requirement under `Capability: Categories` for the data model and ON
DELETE semantics.

(Previously: SKU uniqueness spanned every product row regardless of
soft-archive state. The new carve-out restricts uniqueness to rows
whose `lifecycle` is not `retired`, so retired products release
their SKU for reuse.)

#### Scenario: duplicate SKU

- Given a product exists with SKU `ABC-001`
- When the user creates or imports another product with SKU `ABC-001`
- Then the app blocks the duplicate or asks the user to review/update the existing product

#### Scenario: product with multiple categories still has unique SKU

- GIVEN a product P exists with SKU `ABC-001` and `category_ids = ["dairy-id", "bakery-id"]`
- WHEN the user creates another product Q with the same SKU `ABC-001` but `category_ids = ["produce-id"]`
- THEN the duplicate-SKU rule rejects Q regardless of Q's category set

#### Scenario: product with zero categories still has unique SKU

- GIVEN a product P exists with SKU `ABC-001` and `category_ids = []`
- WHEN the user creates another product Q with the same SKU `ABC-001` and `category_ids = ["produce-id"]`
- THEN the duplicate-SKU rule rejects Q regardless of Q's category set

#### Scenario: SKU released by a retired product is reusable

- GIVEN a retired product R has `sku = 'ABC-001'`
- WHEN the user creates a new product with `sku = 'ABC-001'`
- THEN the new product is saved with `lifecycle = 'active'`
- AND no UNIQUE-constraint violation is raised
- AND the retired product R remains in the database with its full history

#### Scenario: SKU uniqueness against an archived row is still enforced

- GIVEN an archived product A has `sku = 'ABC-002'`
- WHEN the user creates a new product with `sku = 'ABC-002'`
- THEN the insert is rejected as a duplicate SKU
- AND no new row is written

### Requirement: barcode uniqueness

Caduxo shall prevent the same barcode from being assigned to more
than one product whose `lifecycle` is not `retired`. A retired
product's barcode MAY be attached to a future active or archived
product; uniqueness across `active` and `archived` products is
preserved. The constraint is enforced through a partial UNIQUE
index that excludes retired entries so a retired product's
existing `product_barcodes` rows stay readable while their barcode
value becomes available for reuse. The implementation keeps every
`product_barcodes` row's `lifecycle` value synchronized with the
parent product's `lifecycle` at write time so the partial index's
predicate can be a single-table expression; the design phase
documents the synchronization invariant in detail.

(Previously: barcode uniqueness spanned every product row
regardless of soft-archive state. The new carve-out restricts
uniqueness to rows whose `lifecycle` is not `retired`, so retired
products release their barcode for reuse.)

#### Scenario: barcode attached to a retired product is reusable

- GIVEN a retired product R has barcode `7501234567890`
- WHEN the user attaches `7501234567890` to an active product P
- THEN a new `product_barcodes` row is created with `product_id = P.id`
- AND the retired product R's existing `product_barcodes` row remains in the database unchanged

#### Scenario: barcode uniqueness against an archived product is still enforced

- GIVEN an archived product A has barcode `7501234567891`
- WHEN the user attempts to attach `7501234567891` to any other active or archived product
- THEN the attach is rejected as a duplicate barcode
- AND no `product_barcodes` row is written

#### Scenario: barcode released by a retired product can be re-attached after the retired row is removed from active lookup paths

- GIVEN a retired product R has barcode `7501234567892`
- AND an active product P already exists without that barcode
- WHEN the user invokes `add_product_barcode` for P with value `7501234567892`
- THEN the attach succeeds
- AND a subsequent scanner lookup of `7501234567892` resolves to P (not to R)

## ADDED Requirements

The following requirements extend `Capability: Product catalog` and
land alongside the modified requirements above.

### Requirement: product lifecycle states

The system MUST persist a `lifecycle` value on every `products` row
drawn from `{active, archived, retired}`. The default for new
products is `active`. A row whose `lifecycle = 'retired'` is
terminal: no service-layer path may transition it back to `active`
or `archived`, and no IPC may mutate a retired row other than to
read it. The only allowed lifecycle transitions are
`active → archived`, `archived → active` (unarchive),
`active → retired`, and `archived → retired`. Each transition MUST
write exactly one row to `product_lifecycle_events` in the same
database transaction as the product mutation.

The Rust migration harness refuses checksum drift on existing
migrations (`src-tauri/src/db/migrations.rs` V15 comment). The new
`lifecycle` column, the partial UNIQUE indexes, and the
`product_lifecycle_events` table MUST land as a single fresh
additive migration at the next available index; existing migrations
MUST NOT be edited. The new migration MUST back-fill every existing
row by projecting `is_active = 0` to `lifecycle = 'archived'` and
`is_active = 1` to `lifecycle = 'active'`; no row projects to
`retired` at backfill time.

#### Scenario: new product starts active

- GIVEN the user creates a new product via `create_product`
- WHEN the insert completes
- THEN the row's `lifecycle = 'active'`

#### Scenario: archiving flips lifecycle to archived

- GIVEN a product P has `lifecycle = 'active'`
- WHEN the user invokes the archive IPC for P
- THEN P's `lifecycle` becomes `'archived'`
- AND exactly one `product_lifecycle_events` row is written with `event_type = 'archived'`, `from_state = 'active'`, `to_state = 'archived'`

#### Scenario: unarchiving flips lifecycle back to active

- GIVEN a product P has `lifecycle = 'archived'`
- WHEN the user invokes the unarchive IPC for P
- THEN P's `lifecycle` becomes `'active'`
- AND exactly one `product_lifecycle_events` row is written with `event_type = 'unarchived'`, `from_state = 'archived'`, `to_state = 'active'`

#### Scenario: active product can be retired with a reason

- GIVEN a product P has `lifecycle = 'active'`
- WHEN the user invokes the retire IPC with a non-blank `reason`
- THEN P's `lifecycle` becomes `'retired'`
- AND exactly one `product_lifecycle_events` row is written with `event_type = 'retired'`, `from_state = 'active'`, `to_state = 'retired'`, and the supplied reason

#### Scenario: archived product can be retired with a reason

- GIVEN a product P has `lifecycle = 'archived'`
- WHEN the user invokes the retire IPC with a non-blank `reason`
- THEN P's `lifecycle` becomes `'retired'`
- AND exactly one `product_lifecycle_events` row is written with `event_type = 'retired'`, `from_state = 'archived'`, `to_state = 'retired'`, and the supplied reason

#### Scenario: retired product cannot be reactivated

- GIVEN a product P has `lifecycle = 'retired'`
- WHEN the user invokes any IPC that mutates P (archive, unarchive, retire, update_product, add_barcode, remove_barcode, etc.)
- THEN the service returns a domain error
- AND no `products` row is updated
- AND no `product_lifecycle_events` row is written

#### Scenario: legacy is_active = 0 rows project to lifecycle = 'archived' at backfill

- GIVEN the new migration applies against an existing database that contains rows with `is_active = 0` and rows with `is_active = 1`
- WHEN the migration runs
- THEN every `products` row with `is_active = 0` projects to `lifecycle = 'archived'`
- AND every `products` row with `is_active = 1` projects to `lifecycle = 'active'`
- AND no row projects to `lifecycle = 'retired'` at backfill time
- AND the existing `is_active` column is preserved (read paths MAY continue to consult it during the transition window, but `lifecycle` is the canonical contract)

#### Scenario: backup/restore round-trip preserves the lifecycle column

- GIVEN `export_backup` snapshots the database with `VACUUM INTO` and `REQUIRED_TABLES` lists every table the harness backs up
- WHEN the new migration applies
- THEN `products` and `product_barcodes` and `product_lifecycle_events` round-trip through `VACUUM INTO` without schema drift
- AND `REQUIRED_TABLES` includes `product_lifecycle_events` so the audit table is included in every backup snapshot

### Requirement: lifecycle audit trail

The system MUST persist a dedicated `product_lifecycle_events`
audit table whose columns are:

- `id` — surrogate primary key.
- `product_id` — foreign key to `products.id` (non-null).
- `event_type` — one of `'archived'`, `'unarchived'`, `'retired'`.
- `from_state` — the product's `lifecycle` value immediately before the transition.
- `to_state` — the product's `lifecycle` value immediately after the transition.
- `actor` — nullable string identifying the operator (or `NULL` for system-emitted events).
- `reason` — nullable free-text reason. REQUIRED (non-null, non-blank after trim) when `event_type = 'retired'`; nullable for archive and unarchive.
- `created_at` — wall-clock timestamp of the transition.

Every archive, unarchive, and retire transition MUST write exactly
one row to `product_lifecycle_events` in the same database
transaction as the product mutation. The table is read by the
per-product history pane (see `per-product lifecycle history pane`)
and surfaced only there; it is NOT exported by `export_products_csv`
and NOT surfaced through the scanner or operational catalog
surfaces.

#### Scenario: retire writes a lifecycle event with the supplied reason and actor

- GIVEN a product P has `lifecycle = 'active'`
- WHEN the user invokes the retire IPC with `reason = "Discontinued by supplier"` and `actor = "user-1"`
- THEN `product_lifecycle_events` contains a row with `event_type = 'retired'`, `from_state = 'active'`, `to_state = 'retired'`, `actor = 'user-1'`, `reason = 'Discontinued by supplier'`
- AND the row's `created_at` equals the wall-clock time of the retire call

#### Scenario: archive writes a lifecycle event without a reason

- GIVEN a product P has `lifecycle = 'active'`
- WHEN the user invokes the archive IPC with no reason supplied
- THEN `product_lifecycle_events` contains a row with `event_type = 'archived'`, `from_state = 'active'`, `to_state = 'archived'`, `reason = NULL`
- AND the row's `created_at` equals the wall-clock time of the archive call

#### Scenario: retire with blank reason is rejected

- GIVEN the user submits a retire request with an empty or whitespace-only `reason`
- WHEN the service validates the request
- THEN the request is rejected with a validation error
- AND no `products` row is updated
- AND no `product_lifecycle_events` row is written

#### Scenario: lifecycle event writes are atomic with the product mutation

- GIVEN a product P has `lifecycle = 'active'`
- WHEN the user invokes the retire IPC
- THEN either BOTH the `products.lifecycle = 'retired'` update AND the `product_lifecycle_events` row commit, OR neither commits
- AND a partial commit (one without the other) is impossible

#### Scenario: lifecycle event table is not exported by export_products_csv

- GIVEN `export_products_csv` runs against a database that contains `product_lifecycle_events` rows
- WHEN the CSV export file is written
- THEN the file does not contain a section for `product_lifecycle_events`
- AND the export remains a snapshot of the product catalog only

### Requirement: retired products are excluded from operational paths

The system MUST exclude retired products from operational lookup
paths. The scanner resolver (`resolve_scanner_code`), the dashboard
scan/search surface's product-resolution branch, the dashboard
product-create quick path, and the `add_barcode` guard MUST treat
`lifecycle = 'retired'` rows as not-found for write and selection
paths. Retired products remain readable through explicit history
affordances: the per-product detail view, the
`product_lifecycle_events` history pane, the dashboard scan/search
surface's history mode, and the catalog list when the `Show
retired` toggle is on (see `catalog list Show retired toggle`).

#### Scenario: scanner resolver does not match a barcode owned only by a retired product

- GIVEN a retired product R has barcode `7501234567890`
- AND no active or archived product has barcode `7501234567890`
- WHEN the Scanner input resolves `7501234567890`
- THEN `resolve_scanner_code` returns `Unknown { scanned_value: "7501234567890" }`
- AND no product row is surfaced to the scanner UI
- AND the active-mode dispatch (Sale / Registration / Stock-out) treats the scan as an unmatched scan

#### Scenario: scanner resolver does not match a SKU owned only by a retired product

- GIVEN a retired product R has `sku = 'ABC-001'`
- AND no active or archived product has `sku = 'ABC-001'`
- WHEN the Scanner input resolves `ABC-001`
- THEN `resolve_scanner_code` returns `Unknown { scanned_value: "ABC-001" }`

#### Scenario: add_barcode rejects attaching a new barcode to a retired product

- GIVEN a product P has `lifecycle = 'retired'`
- WHEN the user invokes `add_product_barcode` for P
- THEN the service returns the user message `ProductRetiredForBarcode`
- AND no `product_barcodes` row is written

#### Scenario: dashboard scan/search surface includes retired products with a Retired badge

- GIVEN a retired product R exists with `sku = 'ABC-001'`
- WHEN the user types `ABC-001` into the dashboard scan/search field
- THEN R is surfaced in the result list
- AND a `Retired` badge is rendered alongside R
- AND no scanner side-effect (sale, registration, stock-out) is triggered from the dashboard

### Requirement: lot-code scans for retired products surface the lot with a Retired product badge

When a lot-code scan resolves to a lot whose parent product is
retired, the lot detail view MUST render the lot, its full movement
history, AND a `Retired product` badge on the parent product
header. Lot resolution MUST NOT depend on the parent product's
lifecycle; a lot that was created before the parent was retired
MUST continue to resolve by its batch code.

#### Scenario: lot scan for a retired product opens the lot with a Retired product badge

- GIVEN a lot L has `batch_code = 'SUP-XYZ'` and parent product P with `lifecycle = 'retired'`
- AND no other active or archived lot has `batch_code = 'SUP-XYZ'`
- WHEN the Scanner input resolves `SUP-XYZ`
- THEN the Scanner tab opens the lot detail view for L
- AND the parent product header renders a `Retired product` badge
- AND the Historial panel lists every movement for L, including movements recorded before P was retired

#### Scenario: lot resolution is independent of parent lifecycle

- GIVEN a lot L was created while its parent P had `lifecycle = 'active'`
- WHEN P is later retired
- THEN L's `batch_code` continues to resolve correctly through the Scanner tab and through any lot lookup
- AND the resolution does not consider P's lifecycle

### Requirement: CSV import preview distinguishes released SKU/barcode rows

The CSV import preview MUST distinguish three collision states when
a CSV row's SKU or barcode matches an existing product:

- `DuplicateSku` / `DuplicateBarcode` — the SKU or barcode matches an `active` or `archived` row; the import is blocked.
- `ReleasedSku` / `ReleasedBarcode` — the SKU or barcode matches only `retired` rows; the import passes through with a non-blocking per-row advisory notice that the new product reuses an identifier previously associated with a retired product. The import commits the new product with `lifecycle = 'active'`.

#### Scenario: CSV import preview advisory notice for a released SKU

- GIVEN a retired product R has `sku = 'ABC-001'`
- WHEN the user previews a CSV whose row contains `sku = 'ABC-001'`
- THEN the row's status includes `ReleasedSku`
- AND a non-blocking per-row badge states that the SKU was previously associated with a retired product
- AND the import commits the new product with `lifecycle = 'active'`

#### Scenario: CSV import preview advisory notice for a released barcode

- GIVEN a retired product R has barcode `7501234567890`
- WHEN the user previews a CSV whose row attaches barcode `7501234567890` to a new product
- THEN the row's status includes `ReleasedBarcode`
- AND a non-blocking per-row badge states that the barcode was previously associated with a retired product
- AND the import commits the new product with `lifecycle = 'active'`

#### Scenario: CSV import blocks duplicate SKU against an active or archived row

- GIVEN an active product A has `sku = 'ABC-002'`
- WHEN the user previews a CSV whose row contains `sku = 'ABC-002'`
- THEN the row's status is `DuplicateSku`
- AND the row is NOT committed

#### Scenario: CSV import blocks duplicate barcode against an active or archived row

- GIVEN an active product A has barcode `7501234567891`
- WHEN the user previews a CSV whose row attaches barcode `7501234567891` to a new product
- THEN the row's status is `DuplicateBarcode`
- AND the row is NOT committed

### Requirement: catalog list Show retired toggle

The product catalog list (`ProductCatalogPage.svelte`) MUST expose a
`Show retired` toggle alongside the existing `Hide archived` toggle.
The new toggle MUST be persisted in `localStorage` under the key
`caduxo.products.catalog.showRetired.v1` and MUST default to
`false`. When the toggle is off (the default), retired products are
hidden from the catalog list; when on, retired products are listed
with a distinct `Retired` badge and sort to the bottom of the
default ordering.

#### Scenario: Show retired is off by default on first render

- GIVEN the user opens the catalog list for the first time on a fresh profile (no `caduxo.products.catalog.showRetired.v1` value in `localStorage`)
- WHEN the page mounts
- THEN no retired products appear in the list
- AND the `Show retired` toggle is off

#### Scenario: Show retired toggle persists across reloads

- GIVEN the user enables `Show retired` and reloads the page
- WHEN the page mounts
- THEN retired products are listed with a `Retired` badge
- AND the toggle remains on
- AND the localStorage key `caduxo.products.catalog.showRetired.v1` is `true`

#### Scenario: retired rows sort to the bottom when the toggle is on

- GIVEN the catalog list contains at least one active, one archived, and one retired product
- AND `Show retired` is on
- WHEN the user views the default ordering
- THEN retired products are listed after every active and archived row

### Requirement: per-product lifecycle history pane

The product detail page (`ProductDetailPage.svelte`) MUST render a
lifecycle history section that lists every `product_lifecycle_events`
row for the selected product, newest first. Each entry MUST display
the event type, the from/to states, the timestamp, the actor when
present, and the reason when present. The history pane is read-only;
it MUST NOT expose any mutation affordance for the events it lists.

#### Scenario: lifecycle history pane lists every event for the product

- GIVEN a product P has been archived, unarchived, and retired during its lifetime
- WHEN the user opens the lifecycle history section on P's detail page
- THEN the pane lists three rows, newest first
- AND each row shows the event type, from/to states, timestamp, actor (when present), and reason (when present)

### Requirement: retire confirmation requires a reason

The retire flow MUST require the user to type a non-blank free-text
reason and confirm a second time before the retire IPC is invoked.
The frontend MUST NOT submit the retire request until the reason
input is non-blank AND the second confirmation is acknowledged.

#### Scenario: retire submit is disabled until reason is provided

- GIVEN the user opens the retire confirmation for an active or archived product
- WHEN the reason input is empty or whitespace-only
- THEN the submit button is disabled
- AND no IPC is invoked

#### Scenario: retire submit invokes the retire IPC with the reason

- GIVEN the user types a non-blank reason and clicks the second-confirm button
- WHEN the submit button is activated
- THEN the retire IPC is invoked with the supplied reason
- AND the product's `lifecycle` becomes `'retired'`
- AND a `product_lifecycle_events` row is persisted with that reason

### Requirement: retired and replacement products are independent rows

When a new product claims an identifier (SKU or barcode) previously
released by a retired product, the new product row MUST be
independent of the retired row. The system MUST NOT introduce any
required link, foreign key, or rebinding relation between the
retired row and the replacement row. Both rows continue to coexist
in the database, each with its own history and lifecycle.

#### Scenario: retired product and replacement share identifiers but remain independent

- GIVEN a retired product R has `sku = 'ABC-001'` and barcode `7501234567890`
- AND the user creates a new product P with `sku = 'ABC-001'` and attaches barcode `7501234567890`
- WHEN either row is read
- THEN the row's `id`, `lifecycle`, `created_at`, `updated_at`, and `product_lifecycle_events` history are independent of the other row
- AND no runtime path resolves the replacement through the retired row's id
- AND the catalog list (when `Show retired` is on) shows both rows side by side with their respective `Retired` and active badges
