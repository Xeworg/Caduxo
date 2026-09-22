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

A product's category membership is expressed as `category_ids: string[]` (zero or more ids) backed by the `product_categories` junction table. The legacy single nullable `products.category_id` FK is not part of the runtime model anymore; the junction is the canonical source of truth. See the `multi-category product model` requirement under `Capability: Categories` for the data model and ON DELETE semantics.

(Previously: the requirement stated only that SKU is unique. It did not reference the category model; the legacy `products.category_id` column was implied to be the source of truth for category membership.)

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

### Requirement: multiple barcodes per product

Caduxo shall allow a product to have zero, one, or multiple UPC/EAN/barcode values.

#### Scenario: scanner finds product by barcode

- Given a product has barcode `7501234567890`
- When the user scans `7501234567890` in the scan/search field
- Then the app opens that product or starts an expiry lot entry for that product

### Requirement: barcode uniqueness

Caduxo shall prevent the same barcode from being assigned to more than one product.

### Requirement: UPC attach during product creation

The product creation form MUST expose a UPC/barcode input field that allows the user to attach a barcode to a product without first creating the product and then navigating to the product detail page.

This requirement extends the existing "multiple barcodes per product" capability from after-creation only to also during-creation, while leaving the detail-page Barcodes section as the place to manage more than one barcode per product.

#### Scenario: user attaches UPC during manual product creation

- GIVEN the user is creating a product from the catalog UI in create mode
- WHEN the user enters a UPC value in the create form's Barcodes subsection
- AND submits the form
- THEN the system creates the product via `create_product` first
- AND the system attempts to attach the UPC to the created product via the post-save barcode attach step
- AND the system invokes `onSaved` exactly once once the product exists

#### Scenario: UPC field empty stays the default and quiet path

- GIVEN the user is creating a product
- WHEN the UPC field is empty or trimmed to empty
- THEN the system creates the product
- AND the system MUST NOT invoke any `add_product_barcode` command
- AND no barcode-related UI notice is rendered

#### Scenario: UPC field shape mirrors detail page

- GIVEN the create form is rendered in create mode
- THEN it MUST display a Barcodes subsection containing at minimum:
  - a "Barcode value" text input with placeholder `e.g. 7501234567890`
  - an optional "Type" text input backed by a `<datalist>` offering `EAN13`, `EAN8`, `UPC`, `CODE128`, `CODE39`, `QR`
  - an optional "Set as primary" checkbox (unchecked by default)
- AND the subsection MUST be visually consistent with the Barcodes section on the product detail page
- AND the subsection MUST NOT be rendered in edit mode

### Requirement: non-blocking barcode attach on create

The product creation submit flow MUST treat a barcode attach failure as non-blocking: the new product MUST remain saved and `onSaved` MUST be invoked regardless of the attach outcome, with attach outcomes surfaced via a non-blocking inline notice.

#### Scenario: UPC already attached to another product

- GIVEN the user submitted the create form with a non-empty UPC value
- AND the backend reports the UPC is already attached to a different product
- WHEN the post-save attach step runs
- THEN the system keeps the new product saved
- AND renders an inline notice under the Barcodes subsection stating that the UPC already belongs to another product and was not attached
- AND does not roll back the product creation
- AND does not throw

#### Scenario: UPC already attached to this product

- GIVEN the user submitted the create form with a non-empty UPC value
- AND the backend reports the UPC is already attached to the same product (defensive retry case)
- WHEN the post-save attach step runs
- THEN the system keeps the new product saved
- AND renders an inline notice with the backend message
- AND does not throw

#### Scenario: backend command error during attach

- GIVEN the user submitted the create form with a non-empty UPC value
- WHEN the post-save attach step encounters any other backend error (network, database, schema, validation)
- THEN the system keeps the new product saved
- AND renders the backend message in the inline notice slot
- AND does not roll back the product creation
- AND does not throw

### Requirement: typed barcode attach wrapper for create flow

The frontend MUST provide a wrapper around `add_product_barcode` used during product creation that returns a discriminated result instead of throwing, so the create flow can react to attach outcomes without crashing the product save.

#### Scenario: wrapper result shape on success

- GIVEN the create flow invokes the wrapper with a trimmed non-empty UPC value
- WHEN the backend `add_product_barcode` call succeeds
- THEN the wrapper returns `{ ok: true }`
- AND the wrapper MUST NOT throw

#### Scenario: wrapper result shape on duplicate against another product

- GIVEN the create flow invokes the wrapper
- WHEN the backend rejects the attach because the UPC is already attached to a different product
- THEN the wrapper returns `{ ok: false, kind: "duplicate_other" }`
- AND the wrapper MUST NOT throw
- AND the create flow renders the "already belongs to another product" notice from this result kind

#### Scenario: wrapper result shape on duplicate against the same product

- GIVEN the create flow invokes the wrapper
- WHEN the backend rejects the attach because the UPC is already attached to the same product
- THEN the wrapper returns `{ ok: false, kind: "duplicate_same" }`
- AND the wrapper MUST NOT throw

#### Scenario: wrapper result shape on any other failure

- GIVEN the create flow invokes the wrapper
- WHEN the backend reports any other error (network, database, schema, validation)
- THEN the wrapper returns `{ ok: false, kind: "other", message }` containing the backend message
- AND the wrapper MUST NOT throw

### Requirement: scanner quick-create seeds UPC field

The dashboard scan quick-create path MUST seed the scanned value into the create form's UPC/barcode field, not into the SKU field, when no product matches the scan. UPC and SKU remain independent identifiers; the user types the SKU themselves.

#### Scenario: unmatched scan opens quick-create with UPC pre-fill

- GIVEN the user scans or types a value into the dashboard scan/search field
- AND no product matches that value as a barcode or SKU
- WHEN the quick-create modal opens
- THEN the create form's UPC/barcode field is pre-filled with the scanned value
- AND the SKU field is empty
- AND the user MUST be able to submit the form with the scanned UPC and a typed SKU

#### Scenario: matched scan opens product directly

- GIVEN the user scans or types a value into the dashboard scan/search field
- AND the value matches an existing product by barcode
- WHEN the dashboard handles the scan
- THEN the matched product is opened
- AND no quick-create modal is shown
- AND no UPC attach is attempted

### Requirement: silent barcode helper retired once nothing calls it

The dashboard quick-create path MUST migrate off the silent `addProductBarcodeIfNew` helper to the new typed wrapper, and the silent helper MUST be removed once no production caller remains.

#### Scenario: dashboard quick-create uses typed wrapper

- GIVEN the dashboard opens the quick-create modal after a non-matching scan
- WHEN the create form completes a successful product save with a non-empty UPC
- THEN the form's submit flow uses the typed wrapper to attach the UPC
- AND the create form MUST NOT invoke the silent `addProductBarcodeIfNew` helper

#### Scenario: silent helper removal once obsolete

- GIVEN no production caller invokes the silent `addProductBarcodeIfNew` helper after the migration
- WHEN this change is archived
- THEN the silent helper is removed from `src/lib/products.ts`
- AND no automated test references the silent helper

### Requirement: unit catalog and integer/decimal classification

Caduxo shall expose a typed unit catalog seeded with preset units covering mass, volume, and count families. Each catalog unit SHALL carry a stable `key`, a localized `display_name`, and a `kind` classifier of exactly `integer` or `decimal`. Products may reference a catalog unit via `default_unit_id`; the `kind` of the chosen unit SHALL drive how LotForm's quantity input renders (`step="1"` and `min="1"` for `integer`, `step="0.01"` for `decimal`). Storage of `quantity` SHALL remain `REAL` for both kinds in this slice; lot-level quantity enforcement against `unit_type` is explicitly deferred.

The catalog SHALL support inline custom unit creation from the ProductForm. Renaming a unit's `display_name` SHALL be allowed; archiving a unit SHALL be blocked while any product still references it via `default_unit_id`; re-classifying a unit's `kind` SHALL NOT be exposed in this slice.

Catalog units created inline from the ProductForm SHALL be persisted in the same catalog table and become selectable from any product form from then on. The UI SHALL display the full `display_name` label (not the `key` symbol) in ProductForm, LotForm, the dashboard, reports, the resolve-quantity dialog, the PDF export, and the CSV import preview.

#### Scenario: product picks a preset unit

- GIVEN the catalog contains a preset `kg` with `kind = decimal` and `display_name = "Kilogramo"`
- WHEN the user selects `Kilogramo` from the ProductForm unit datalist
- THEN the product is saved with `default_unit_id` referencing `kg`
- AND `unit_type = decimal` is set on the product
- AND the existing `default_unit` text column on `products` echoes `Kilogramo`

#### Scenario: product creates a custom unit inline

- GIVEN the user types `bandejas` in the ProductForm unit field
- AND `bandejas` is not in the catalog (neither as a preset key nor as an existing custom unit)
- WHEN the user clicks "+ Create new unit" and confirms with `kind = integer`
- THEN a new `unit_definitions` row is created with `key = "bandejas"` and `display_name = "bandejas"`
- AND the product is saved with `default_unit_id` referencing the new unit
- AND `unit_type = integer` is set on the product
- AND the new unit becomes selectable from the ProductForm unit datalist of any other product afterwards

#### Scenario: integer unit drives LotForm quantity input

- GIVEN a product has `unit_type = integer`
- WHEN the user opens the lot entry form for that product
- THEN the `quantity` input renders with `step="1"` and `min="1"`
- AND the resolved unit label displayed alongside the input is the catalog unit's `display_name`

#### Scenario: decimal unit drives LotForm quantity input

- GIVEN a product has `unit_type = decimal`
- WHEN the user opens the lot entry form for that product
- THEN the `quantity` input renders with `step="0.01"` and `min="0.01"`
- AND the resolved unit label displayed alongside the input is the catalog unit's `display_name`

#### Scenario: unrecognized legacy units are surfaced without rewriting

- GIVEN a pre-existing product has `default_unit = "kg."` (typo) and `default_unit_id IS NULL` after the catalog migration
- WHEN the user opens the dashboard after upgrade
- THEN a non-blocking banner shows the count of unrecognized units with a "Review" action
- AND the existing `default_unit` text on the product is NOT rewritten
- AND the product's `default_unit_id` remains NULL
- AND the user can navigate to a review page that lists each unrecognized `raw_value` with options to map it to a catalog preset, create a custom unit from it, or leave it for later

#### Scenario: banner dismissal persists until a new unrecognized value appears

- GIVEN the dashboard banner was dismissed after the user reviewed all current unrecognized units
- AND the dismissed signature was persisted to `app_settings.unit_audit.dismissed_signature`
- WHEN the user opens the dashboard on a subsequent launch with no new unrecognized values introduced
- THEN the banner is hidden because the current signature matches the persisted signature
- AND if a new unrecognized unit is introduced (for example via a new product or a CSV import), the banner reappears because the signature changes

#### Scenario: CSV preview flags unknown units without blocking the import

- GIVEN a CSV row contains `default_unit = "litross"` (typo)
- WHEN the user previews the import
- THEN the row's status includes `UnknownUnit` with the raw value and up to 3 suggested catalog keys
- AND the row renders a non-blocking per-row badge in the preview
- AND the import still commits (warn-and-continue): the resulting product is saved with `default_unit = "litross"`, `default_unit_id = NULL`, and surfaces in the audit banner after the import lands

## Capability: Categories

### Requirement: editable category list

Caduxo shall provide a user-editable category list for product classification.

A product may belong to one, many, or zero categories from this list. A product with zero active category relations is valid and surfaces as `Uncategorized` wherever categories are listed or filtered. Archive (`is_active = 0`) hides a category from pickers while preserving junction rows and history; archive is preferred over hard delete.

(Previously: the requirement stated only that products are assigned to categories and that the user filters by category. The requirement did not specify that products may belong to zero or many categories, did not mention the `Uncategorized` representation, and did not state the archive-over-delete policy.)

#### Scenario: category filter

- Given products are assigned to categories
- When the user filters by category
- Then dashboard/report lists show only matching products/lots

#### Scenario: archived categories are hidden from pickers but preserved in history

- GIVEN a category `Dairy` is archived (`is_active = 0`)
- WHEN the user opens `ProductForm`, `ProductCatalogPage`, or `ReportsPage`
- THEN the picker no longer surfaces `Dairy`
- AND existing junction rows referencing `Dairy` are preserved
- AND products with only archived category relations appear under `Uncategorized` in filters

#### Scenario: products with no active category appear as Uncategorized

- GIVEN a product P has zero rows in `product_categories` whose category has `is_active = 1`
- WHEN the user views P's detail page or the dashboard detail modal for one of P's lots
- THEN a muted `Uncategorized` badge or label is rendered
- AND when the user filters reports or catalog by the `Uncategorized` pseudo-row
- THEN P is included in the result

### Requirement: multi-category product model

Products may belong to one, many, or zero categories from the editable category list. The relation is stored in the `product_categories` junction table whose composite primary key `(product_id, category_id)` enforces many-to-many uniqueness. `product_id` carries `ON DELETE CASCADE` so deleting a product removes its junction rows; `category_id` carries `ON DELETE RESTRICT` so deleting a category that is still referenced is rejected by SQLite (archive-first is the supported path). The junction is the canonical runtime source of truth for category membership. The legacy `products.category_id` column remains in the schema but is not read or written by any runtime path after V4 applies.

The service-layer case-fold guard MUST reject `create_category` and `update_category` (rename path) when the lower-cased proposed name collides with an existing category name. The guard MUST translate the SQLite `UNIQUE` violation on `categories.name` to `DuplicateField { field: "name" }` on the conflict path so the wire shape stays consistent.

The runtime MUST NOT fan out to two category-read paths: per-product reads use a single junction SELECT; cross-product reads use the batched helper `list_product_category_ids_by_product_ids(ids) -> HashMap<String, Vec<String>>`; catalog and report filters apply the filter at the SQL level via `EXISTS` (never `JOIN`).

#### Scenario: product with three categories persists three junction rows

- GIVEN the database contains three active categories `Dairy`, `Bakery`, `Produce`
- WHEN the user creates a product with `category_ids = ["dairy-id", "bakery-id", "produce-id"]`
- THEN `product_categories` contains three rows for the new product id, one per category id
- AND `get_product` returns `category_ids = ["dairy-id", "bakery-id", "produce-id"]`
- AND `ProductDetailResponse.categories` lists the three resolved `CategoryResponse` values

#### Scenario: update replaces the full category set

- GIVEN a product has two junction rows for `Dairy` and `Bakery`
- WHEN the user updates the product with `category_ids = ["produce-id"]`
- THEN the product's `product_categories` rows are exactly `[("product-id", "produce-id")]`
- AND no rows for `Dairy` or `Bakery` remain for that product

#### Scenario: deleting a category with junction rows is rejected

- GIVEN a category has at least one row in `product_categories`
- WHEN a caller attempts to delete the category (hard delete)
- THEN SQLite rejects the operation due to `ON DELETE RESTRICT`
- AND the junction rows are preserved

#### Scenario: deleting a product cascades its junction rows

- GIVEN a product has at least one row in `product_categories`
- WHEN the product is deleted
- THEN every `product_categories` row referencing the product is removed (`ON DELETE CASCADE`)
- AND the corresponding categories remain untouched

#### Scenario: case-fold guard rejects new case-only duplicates

- GIVEN the database contains a category named `Dairy` with id `dairy-id`
- WHEN the user submits `create_category({ name: "dairy" })`
- THEN the service-layer guard returns `DuplicateField { field: "name", value: "dairy" }`
- AND no new category row is inserted
- WHEN the user submits `create_category({ name: "DAIRY" })`
- THEN the same `DuplicateField` result is returned

#### Scenario: renaming to a case-only duplicate is rejected

- GIVEN the database contains a category named `Dairy` with id `dairy-id`
- AND another category named `Bakery` with id `bakery-id`
- WHEN the user submits `update_category({ id: "bakery-id", name: "dairy" })`
- THEN the service-layer guard returns `DuplicateField { field: "name", value: "dairy" }`
- AND `Bakery` retains its original name and is_active state

#### Scenario: renaming to the category's own current name succeeds

- GIVEN a category has `name = "Dairy"` and `id = "dairy-id"`
- WHEN the user submits `update_category({ id: "dairy-id", name: "Dairy" })`
- THEN the operation succeeds and the row is unchanged

#### Scenario: legacy products.category_id column is ignored at runtime

- GIVEN V4 has applied and the junction is the canonical source of truth
- WHEN any runtime command reads or writes a product's category membership
- THEN the path is the junction (`product_categories`) and the `INSERT INTO products` SQL does not reference `category_id`
- AND the legacy `products.category_id` column is not selected by any runtime query
- AND the legacy column remains in the schema with its existing values from before V4

#### Scenario: multi-category filter ANY-of returns any-of lots without double count

- GIVEN product P1 is in categories `Dairy` and `Bakery`
- AND product P2 is in category `Produce` only
- AND product P3 has no category relations
- AND each product has at least one active lot
- WHEN the user filters reports by `category_ids = ["dairy-id", "bakery-id"]`
- THEN the result includes lots for P1 and P2
- AND P1's lots appear exactly once each (no duplicate count for being in both categories)
- AND P3's lots are not in the result
- AND the SQL uses `EXISTS` against `product_categories` and never `JOIN`s the junction

#### Scenario: single-category filter selection preserves single-category behavior

- GIVEN the user has the reports page open with `category_ids = ["dairy-id"]`
- WHEN the user triggers a report run
- THEN the result includes lots whose product has a junction row referencing `dairy-id`
- AND the row count matches the legacy single-`category_id` filter for the same id

#### Scenario: empty category filter selection applies no category filter

- GIVEN the user has the reports page open with `category_ids = []`
- WHEN the user triggers a report run
- THEN every active lot matching the other filters is returned
- AND no `EXISTS` clause against `product_categories` is added to the SQL

#### Scenario: category_filter_sentinel_uncategorized

- GIVEN a product P has zero rows in `product_categories` whose category has `is_active = 1`
- AND P has at least one active lot
- WHEN the user selects the `Uncategorized` pseudo-row (sentinel id `__uncategorized__`) in the catalog or reports filter
- THEN the filter selection is `["__uncategorized__"]`
- AND the SQL applies `NOT EXISTS (... product_categories JOIN categories ON is_active = 1)`
- AND P's lots are included in the result

#### Scenario: real id plus Uncategorized sentinel composes correctly

- GIVEN product P1 is in category `Dairy` only
- AND product P2 has zero active category relations
- WHEN the user filters reports by `category_ids = ["dairy-id", "__uncategorized__"]`
- THEN the result includes lots for P1 and lots for P2
- AND the SQL composes `EXISTS (... category_id IN ('dairy-id')) OR NOT EXISTS (... is_active = 1)` rather than emitting two separate row sets

#### Scenario: category search excludes archived categories by default

- GIVEN the database contains `Dairy` (`is_active = 1`) and `Legacy-Brand` (`is_active = 0`)
- WHEN the picker calls `list_categories_search({ query: "" })`
- THEN `Dairy` is in the page items and `Legacy-Brand` is not
- AND the response `total` equals the count of active categories only

#### Scenario: category search prefers prefix then falls back to substring

- GIVEN the database contains `Dairy`, `Bakery`, `Dairy-Free`, and `Produce`
- WHEN the picker calls `list_categories_search({ query: "Dai" })`
- THEN the page items list `Dairy` and `Dairy-Free` (prefix matches) ordered by name asc
- AND no substring-only matches (`Bakery`, `Produce`) are included
- WHEN the picker calls `list_categories_search({ query: "airy" })`
- THEN no prefix matches exist
- AND the page items list `Dairy` and `Dairy-Free` (substring matches) ordered by name asc

#### Scenario: category search has_more flag is computed from total

- GIVEN the database contains 7 active categories whose name starts with `D`
- WHEN the picker calls `list_categories_search({ query: "D", limit: 5 })`
- THEN the response contains 5 items
- AND `total = 7`
- AND `has_more = true`

#### Scenario: category picker chip row and remove affordance

- GIVEN the picker is mounted on `ProductForm` with current selection `["dairy-id", "bakery-id"]`
- WHEN the picker renders
- THEN the chip row above the input contains two chips labelled `Dairy` and `Bakery`
- AND each chip has an `aria-label="Remove <name>"` `×` button
- WHEN the user clicks the `×` on the `Dairy` chip
- THEN `Dairy` is removed from the selection
- AND the chip row updates without refetching categories from the backend

#### Scenario: category picker Clear all emits empty selection

- GIVEN the picker selection contains at least one chip
- WHEN the user activates the `Clear all` link
- THEN the picker emits `value = []`
- AND every chip is removed from the chip row

#### Scenario: category picker inline create requires explicit confirmation

- GIVEN the picker is open and the typed query `abc` matches no result in the popover
- WHEN the picker renders
- THEN a `Create "abc"` row is rendered at the bottom of the popover
- WHEN the user clicks `Create "abc"`
- THEN the picker calls `create_category({ name: "abc" })`
- AND on success the new id is appended to the selection and the new chip appears
- AND on `DuplicateField { field: "name" }` the picker renders an inline error and keeps the popover open
- AND there is no silent arbitrary-text creation path

#### Scenario: category picker keyboard ergonomics

- GIVEN the picker trigger is focused
- WHEN the user presses `Tab` then `ArrowDown` repeatedly
- THEN `aria-activedescendant` moves through the result list and clamps at the ends
- WHEN the user presses `Enter`
- THEN the active result is toggled into `value` (chip added or removed)
- WHEN the user presses `Esc`
- THEN the popover closes without committing a typed query (the input keeps the typed text)
- AND focus returns to the trigger
- GIVEN the input is focused and empty
- WHEN the user presses `Backspace`
- THEN the last chip is removed from `value`

#### Scenario: category picker Uncategorized pseudo-row is selectable and rendered distinctly

- GIVEN the picker is mounted with `includeUncategorized={true}` (catalog and reports filters)
- WHEN the popover is open
- THEN an `Uncategorized` pseudo-row is rendered at the top of the result list with a distinct style
- AND toggling the pseudo-row pushes or pops `__uncategorized__` into `value`
- AND the pseudo-row is never persisted as a real category id (it is rendered by the picker, not by the backend)
- WHEN the picker is mounted with `includeUncategorized={false}` (`ProductForm`)
- THEN the `Uncategorized` pseudo-row is not rendered

## Capability: Expiry lots

### Requirement: lot registration

(Previously: the requirement listed required, conditionally required, and optional fields. It specified the `unit` resolution and the `expiry date` picker shape. It did not mention initial-movement emission, batch-code auto-generation, settings-aware location validation, or the sentinel location for unassigned stock.)

The lot registration contract is extended as follows:

- On lot creation, the system MUST emit an `entry:initial` movement in the same database transaction as the `INSERT INTO expiry_lots` (per the new `initial entry on lot creation` requirement).
- The lot's `quantity` field SHALL remain the canonical total. Per-location balances are derived from the ledger (per the new `derived per-location balance` requirement).
- When the user leaves `batch_code` blank, the system MUST auto-generate a `PREFIX-YYYYMMDD-NNN` code where:
  - `PREFIX` is the sanitized alphanumeric SKU short prefix (3–6 chars, uppercased), padded with `X` if the source has fewer than three alphanumeric characters, or the literal `LOT` if the product has no SKU.
  - `YYYYMMDD` is the local creation date.
  - `NNN` is a per-day per-prefix counter starting at `001`. If `PREFIX-YYYYMMDD-NNN` already exists, the system SHALL try `NNN+1`, `NNN+2`, etc.; if `NNN > 999`, the system SHALL fall back to `PREFIX-YYYYMMDD-NNN-M` where `M = 2, 3, ...` until a free slot is found.
- When the user enters a non-blank `batch_code`, the system MUST preserve it verbatim. The auto-generator MUST NOT be invoked for non-blank input. This invariant SHALL be enforced at the service layer and SHALL also be enforced by the LotForm frontend guard.
- The lot registration form MUST respect the `require_initial_location_on_lot_create` setting:
  - When ON (default) and the user submits without a location, the form rejects the submission with an inline error and no lot or movement row is written.
  - When OFF and the user submits without a location, the lot is created against the per-store sentinel `Sin ubicación` (per the new `sentinel location for unassigned stock` requirement).
- The remaining fields and constraints from the prior version of this requirement are preserved:

  Required lot fields:

  - product
  - quantity
  - unit — the resolved free-text label displayed on the lot. When the product references a catalog unit via `default_unit_id`, the value SHALL be the catalog unit's `display_name`. When no catalog unit is set, the value SHALL be the legacy `products.default_unit` text, or, if that is empty, the seeded `units` preset's `display_name` (`Unidades`). The lot form SHALL render the `unit` field as a read-only chip when the product has a catalog unit, and SHALL keep it editable for legacy lots whose product has `unit_type = null`.
  - expiry date
  - alert days before expiry

  Conditionally required:

  - store/local when multiple stores exist

  Optional:

  - internal location
  - batch/lot code
  - notes

  The `expiry date` field uses the `Date input` picker with `clearable={false}`. The host `required` JS guard (`if (!expiryDate) errorMsg = "Expiry date is required";`) remains the source of truth for blocking empty submits. No `<input type="date">` is rendered for this field.

#### Scenario: lot creation with auto-generated batch

- GIVEN a product P has `sku = "YOG-001"` and the user creates a lot with `batch_code = ""` on `2026-10-15`
- WHEN the lot creation submit succeeds
- THEN the returned lot has `batch_code = "YOG001-20261015-001"` (or `...-002` if `...-001` already exists for that prefix and date)
- AND the LotForm displays a confirmation chip with the generated code
- AND an `entry:initial` movement is written in the same transaction

#### Scenario: manual batch is preserved verbatim

- GIVEN the user creates a lot with `batch_code = "SUPPLIER-XYZ-2026"`
- WHEN the lot creation submit succeeds
- THEN the returned lot has `batch_code = "SUPPLIER-XYZ-2026"`
- AND the auto-generator is not invoked
- AND an `entry:initial` movement is written in the same transaction

#### Scenario: blank batch with the required-location toggle off uses the sentinel

- GIVEN the `require_initial_location_on_lot_create` setting is off
- WHEN the user submits a lot creation form with no location selected and `batch_code = ""`
- THEN the lot is created against the per-store sentinel `Sin ubicación`
- AND the initial entry's `destination_location_id` is the sentinel id
- AND no validation error is raised

#### Scenario: blank batch with the required-location toggle on is rejected

- GIVEN the `require_initial_location_on_lot_create` setting is on
- WHEN the user submits a lot creation form with no location selected
- THEN the form rejects the submission with the message `Selecciona una ubicación`
- AND no `expiry_lots` row is created
- AND no `lot_movements` row is created

### Requirement: product default alert days

Caduxo shall suggest 30 alert days when creating a product, but the user must confirm or change the product default.

### Requirement: lot alert override

Caduxo shall pre-fill lot alert days from the product default and allow the user to override the value for that specific lot.

### Requirement: partial resolution

(Previously: the requirement was a one-paragraph statement with a single scenario. It did not specify the data model or the reason vocabulary. The audit trail was a `lot_resolution_events` row with a free-text `resolution` and `notes`.)

The `partial resolution` concept is now realized as an exit movement drawn from the v1 reason vocabulary. The user-facing behavior is preserved (resolve part of a lot, keep the rest active), but the data model and reason contract change.

- Resolving part of a lot quantity SHALL emit a single `exit:*` movement with the user's selected reason, source location, and positive `quantity`.
- The reason MUST be drawn from the v1 vocabulary (`Venta`, `Merma`, `Vencido`, `Dañado`, `Consumo interno`, `Devolución a proveedor`, `Ajuste de inventario`, `Otro`).
- The `notes` field SHALL be REQUIRED when the reason is `Ajuste de inventario` or `Otro`; OPTIONAL otherwise.
- Pre-existing `lot_resolution_events` rows SHALL be migrated into `lot_movements` rows during the v5 migration (per the new `legacy resolution events migrate into the unified ledger` requirement) and the legacy table becomes read-only.

#### Scenario: partial exit with reason preserves the remaining quantity

- GIVEN an active lot has `quantity = 10` and the user selects reason `Venta`
- WHEN the user resolves 4 units via `Registrar salida` with reason `Venta`
- THEN the lot total becomes `6`
- AND the lot remains active
- AND the Historial panel lists the `exit:sale` movement with `quantity = 4`

#### Scenario: legacy resolution data is migrated into the ledger

- GIVEN a pre-V5 `lot_resolution_events` row with `resolution = 'consumed'`, `quantity = 2`, and `notes = 'staff lunch'`
- WHEN the v5 migration applies
- THEN a new `exit:internal_consumption` movement is inserted with `quantity = 2` and `notes = 'staff lunch'`
- AND the legacy row remains in place (read-only)

## Capability: Lot movements

### Requirement: append-only movement ledger

The system MUST record every change in a lot's quantity or per-location distribution as an immutable row in a `lot_movements` table. The ledger is the canonical source of truth for a lot's movement history; the lot's total quantity is a denormalized column maintained inside the same transaction as every ledger write, and per-location balances are derived on demand.

- `expiry_lots.quantity` SHALL remain the denormalized canonical total. It MUST equal `SUM(quantity WHERE destination_location_id IS NOT NULL) − SUM(quantity WHERE source_location_id IS NOT NULL)` for the affected lot after every ledger write.
- The system MUST NOT expose any runtime path that `UPDATE`s or `DELETE`s an existing `lot_movements` row. Corrections SHALL be made by inserting a new compensating movement row; never by editing or removing a prior row.
- No `lot_movements` row SHALL carry any monetary or financial field (no price, no payment method, no customer, no tax, no invoice, no margin, no cost). "Sale" remains a stock-out reason only.

#### Scenario: ledger write and lot-total update are atomic

- GIVEN a lot has `quantity = 10`, with 5 at `Bodega` and 5 at `Exhibición`
- WHEN the user records a `Registrar salida` of 2 units with reason `Venta` from `Bodega`
- THEN a new `exit:sale` row is inserted into `lot_movements`
- AND `expiry_lots.quantity` is updated to `8` in the same transaction
- AND the per-location balance for `Bodega` becomes `3`
- AND the per-location balance for `Exhibición` remains `5`

#### Scenario: correction is a compensating movement, never an edit

- GIVEN a `Venta` of 2 units was recorded against the wrong lot by mistake
- WHEN the user records a compensating `Venta` of 2 units against the correct lot's same source location
- THEN a new `exit:sale` row is inserted (no `UPDATE` or `DELETE` is issued against the prior row)
- AND the original row remains in the ledger unchanged

### Requirement: movement kind vocabulary

The system MUST use a fixed movement kind vocabulary. Each kind binds to a fixed source/destination nullability pattern, and the binding MUST be enforced both at the service layer and by a database `CHECK` constraint on `lot_movements`.

| Kind | `source_location_id` | `destination_location_id` | Free-text note |
| --- | --- | --- | --- |
| `entry:initial` | `NULL` | set | not applicable (system-emitted) |
| `transfer` | set | set (≠ source) | optional |
| `exit:sale` | set | `NULL` | optional |
| `exit:waste` | set | `NULL` | optional |
| `exit:expired` | set | `NULL` | optional |
| `exit:damaged` | set | `NULL` | optional |
| `exit:internal_consumption` | set | `NULL` | optional |
| `exit:return_to_supplier` | set | `NULL` | optional |
| `exit:inventory_adjustment` | set | `NULL` | REQUIRED |
| `exit:other` | set | `NULL` | REQUIRED |
| `inventory_adjustment` | follows direction | follows direction | REQUIRED |

The `inventory_adjustment` kind is the unified count-correction movement emitted by the `Ajustar conteo` flow. It carries a directional sign so a single kind encodes both an increase (physical count exceeds the prior system balance) and a decrease (physical count falls below it). The source/destination nullability follows the sign: an increase sets `destination_location_id` only (entry-shaped), and a decrease sets `source_location_id` only (exit-shaped). The magnitude stored in `quantity` is always the absolute difference and SHALL be `> 0` for any persisted row.

#### Scenario: vocabulary is exhaustive and the kind/source/destination contract is enforced

- GIVEN any persisted `lot_movements` row
- WHEN the row is read
- THEN its `movement_kind` is one of the eleven values listed in the table
- AND its source/destination nullability matches the table
- AND rows of kind `exit:other`, `exit:inventory_adjustment`, or `inventory_adjustment` carry a non-blank `notes` value
- AND rows of any other `exit:*` kind carry either a null or a non-blank `notes` value (optional)

### Requirement: initial entry on lot creation

When a lot is created, the system MUST insert a single `entry:initial` movement in the same database transaction as the `INSERT INTO expiry_lots`.

- The initial movement's `quantity` SHALL equal the lot's `quantity` at creation.
- The initial movement's `destination_location_id` SHALL equal the lot's `location_id` after location resolution (the chosen location, or the per-store sentinel `Sin ubicación` when the `require_initial_location_on_lot_create` setting is off and the user left the picker empty).
- The initial movement's `actor` SHALL be the literal string `system`.

#### Scenario: lot creation writes both lot and initial movement atomically

- GIVEN a product P and a chosen location L exist
- WHEN the user submits the lot creation form with `quantity = 12` and `location_id = L`
- THEN the `expiry_lots` row is inserted with `quantity = 12` and `location_id = L`
- AND a corresponding `entry:initial` movement is inserted in the same transaction
- AND the lot detail's Historial tab shows the initial entry as the oldest row

### Requirement: transfers within and across stores

The system MUST allow moving stock from one internal location to another internal location. The destination location MAY belong to the same store as the source location or to a different store.

- A transfer is one user action that emits a single `transfer` row with `source_location_id`, `destination_location_id` (both non-null and distinct), and a positive `quantity`.
- The service MUST reject a transfer whose `quantity` exceeds the current source balance for the lot at the source location.
- The service MUST reject a transfer whose source or destination location is inactive (`is_active = 0`).
- The service MUST allow the destination store to differ from the source store. The lot's `expiry_lots.store_id` anchor remains the original store; per-location balances MAY include locations of other stores via the ledger.

(Previously: cross-store transfers were excluded from v1; the proposal's "Cross-store transfers are out of scope for v1" non-goal is reversed in this delta.)

#### Scenario: same-store transfer preserves the lot total

- GIVEN a lot has 10 units at `Bodega-A` and 0 units at `Exhibición-A`, both in store A
- WHEN the user records a Mover stock of 3 units from `Bodega-A` to `Exhibición-A`
- THEN a single `transfer` row is inserted
- AND the lot total remains `10`
- AND `Bodega-A`'s per-location balance becomes `7`
- AND `Exhibición-A`'s per-location balance becomes `3`

#### Scenario: cross-store transfer moves stock across stores

- GIVEN a lot L was created in store A with `store_id = A`
- AND a location `Bodega-B` exists in store B
- WHEN the user records a Mover stock of 5 units from `Bodega-A` to `Bodega-B`
- THEN the transfer is accepted
- AND a single `transfer` row is inserted
- AND the lot total remains unchanged
- AND `Bodega-A`'s balance decreases by `5`
- AND `Bodega-B`'s balance increases by `5`
- AND `expiry_lots.store_id` remains `A`

#### Scenario: transfer rejected when source balance is insufficient

- GIVEN a lot has only 3 units at `Bodega`
- WHEN the user records a Mover stock of 5 units from `Bodega`
- THEN the insert is rejected with a validation error
- AND no movement row is written
- AND the per-location balances are unchanged

### Requirement: exit movements with reason vocabulary

The system MUST accept stock-out movements drawn from the v1 reason vocabulary. Each `Registrar salida` submission emits a single `exit:*` movement with the selected reason, a chosen source location, and a positive `quantity`.

- The reason MUST be one of: `Venta`, `Merma`, `Vencido`, `Dañado`, `Consumo interno`, `Devolución a proveedor`, `Ajuste de inventario`, `Otro`. Each maps to one of the eight `exit:*` kinds in the vocabulary table.
- The system MUST reject the insert when the reason is `Otro` or `Ajuste de inventario` (i.e., `exit:other` or `exit:inventory_adjustment`) and the supplied `notes` is blank.
- The system MUST reject the insert when the chosen source location's per-location balance for the lot is less than `quantity`.

#### Scenario: exit with optional note succeeds

- GIVEN an active lot has 10 units at `Bodega`
- WHEN the user records a `Registrar salida` of 4 units with reason `Venta` from `Bodega` and an empty `notes` field
- THEN a single `exit:sale` row is inserted with `quantity = 4`
- AND the lot total becomes `6`
- AND the lot remains active
- AND the Historial panel lists the `exit:sale` movement

#### Scenario: exit with reason Otro requires a non-blank note

- GIVEN an active lot has 10 units at `Bodega`
- WHEN the user records a `Registrar salida` of 4 units with reason `Otro` and a blank `notes` field
- THEN the service returns a validation error
- AND no movement row is written
- AND the lot total and per-location balance are unchanged

#### Scenario: exit with reason Ajuste de inventario requires a non-blank note

- GIVEN an active lot has 10 units at `Bodega`
- WHEN the user records a `Registrar salida` of 4 units with reason `Ajuste de inventario` and a blank `notes` field
- THEN the service returns a validation error
- AND no movement row is written

### Requirement: count adjustment with directional sign

The system MUST provide an `Ajustar conteo` flow that records a single count-correction movement of kind `inventory_adjustment` with a directional sign and a required justification note.

- The kind is `inventory_adjustment` (a single kind, distinct from the eight `exit:*` kinds). The movement carries a `direction` of `increase` or `decrease` and a `quantity` equal to the absolute difference between the physical count and the prior system balance for the lot at the chosen location. `quantity` SHALL be `> 0`; a zero-delta submission is a no-op and writes no row.
- When `direction = increase`, the row is entry-shaped: `source_location_id IS NULL`, `destination_location_id = chosen_location`.
- When `direction = decrease`, the row is exit-shaped: `source_location_id = chosen_location`, `destination_location_id IS NULL`.
- The `notes` field is REQUIRED. The service MUST reject the insert when `notes` is blank.
- The `quantity` magnitude MUST equal the absolute difference between the physical count and the prior system balance for the lot at the chosen location.

#### Scenario: Ajustar conteo records a single increase movement

- GIVEN a lot has 7 units at `Bodega` and the user performs a physical count that finds 9 units at `Bodega`
- WHEN the user submits Ajustar conteo with physical count `9`, location `Bodega`, and a non-blank justification
- THEN a single `inventory_adjustment` row is inserted with `direction = increase`, `destination_location_id = Bodega`, `quantity = 2`, and the supplied `notes`
- AND `expiry_lots.quantity` becomes `9`
- AND the per-location balance for `Bodega` becomes `9`
- AND no separate `entry:*` kind is emitted

#### Scenario: Ajustar conteo records a single decrease movement

- GIVEN a lot has 7 units at `Bodega` and the user performs a physical count that finds 5 units at `Bodega`
- WHEN the user submits Ajustar conteo with physical count `5`, location `Bodega`, and a non-blank justification
- THEN a single `inventory_adjustment` row is inserted with `direction = decrease`, `source_location_id = Bodega`, `quantity = 2`, and the supplied `notes`
- AND `expiry_lots.quantity` becomes `5`

#### Scenario: Ajustar conteo with zero delta is a no-op

- GIVEN a lot has 7 units at `Bodega` and the user performs a physical count that finds 7 units at `Bodega`
- WHEN the user submits Ajustar conteo with physical count `7`, location `Bodega`, and any note
- THEN no `lot_movements` row is inserted
- AND `expiry_lots.quantity` and the per-location balances are unchanged

#### Scenario: Ajustar conteo with blank note is rejected

- GIVEN the user submits Ajustar conteo with a non-zero delta and a blank justification
- WHEN the service validates the insert
- THEN the insert is rejected with a validation error
- AND no movement row is written

### Requirement: reactivation of a resolved lot via a compensating increase

A lot whose `status = 'resolved'` and `quantity = 0` MAY be reactivated by a single `inventory_adjustment` movement with `direction = increase` (the count-correction flow). The service SHALL flip `status` back to `active`, clear `resolution` and `resolved_at`, and increase `expiry_lots.quantity` by the magnitude in the same transaction.

#### Scenario: resolved lot is reactivated by a compensating count increase

- GIVEN a lot is fully resolved with `quantity = 0`, `status = 'resolved'`, and `resolution = 'exit:expired'`
- WHEN the user submits Ajustar conteo with physical count `3` at `Bodega` and a non-blank justification
- THEN a single `inventory_adjustment` row is inserted with `direction = increase`, `destination_location_id = Bodega`, `quantity = 3`, and the supplied `notes`
- AND `expiry_lots.quantity` becomes `3`
- AND `expiry_lots.status` becomes `'active'`
- AND `expiry_lots.resolution` becomes `NULL`
- AND `expiry_lots.resolved_at` becomes `NULL`

### Requirement: derived per-location balance

The system MUST compute a lot's per-location balance from the ledger on demand. The balance for a `(lot_id, location_id)` pair SHALL be `SUM(quantity WHERE destination_location_id = location_id) − SUM(quantity WHERE source_location_id = location_id)`; a zero or negative balance is treated as fully depleted for that location.

- The system MUST NOT persist a per-location balance as a primary column. The only persisted state is the ledger; balances are derived at read time.
- The derivation MUST be computed against a consistent snapshot so concurrent inserts cannot leak half-applied state to a reader.

#### Scenario: per-location balance reflects the full ledger

- GIVEN a lot has the following movements: entry `10` at `Bodega`, transfer `4` from `Bodega` to `Exhibición`, exit:sale `1` from `Exhibición`
- WHEN the lot detail requests per-location balances
- THEN `Bodega` reports `6`
- AND `Exhibición` reports `3`
- AND the lot total reports `9`

### Requirement: unified movement history on the per-lot detail

The system MUST surface a chronological movement history on each lot's detail view, rendered as the `Historial` tab inside the existing lot detail surface.

- The Historial tab MUST list every movement for the lot (including the initial entry, all exits and transfers, all `Ajustar conteo` rows, and any migrated legacy events), newest first.
- Each row MUST display the timestamp, the kind label in Spanish, the reason label when applicable, the source and/or destination location when applicable, the `quantity` as a positive magnitude (with an explicit `+` / `−` sign for `inventory_adjustment` rows), and the `notes` when present.
- The Historial tab MUST also render the lot's current total remaining quantity and a per-location breakdown.
- The dashboard MUST NOT introduce a separate "recent activity" or movement widget for active lots in v1. Movements remain visible only via the per-lot detail.

#### Scenario: Historial panel shows initial and subsequent movements

- GIVEN a lot has an `entry:initial` of `10` at `Bodega`, a `transfer` of `4` from `Bodega` to `Exhibición`, and an `exit:sale` of `2` from `Exhibición`
- WHEN the user opens the Historial tab for that lot
- THEN the panel lists three rows, newest first
- AND the panel header shows lot total `8`
- AND the per-location breakdown shows `Bodega 6, Exhibición 2`
- AND no entry is duplicated or missing

#### Scenario: dashboard does not show a movement widget in v1

- GIVEN the dashboard renders with active lots
- WHEN the dashboard list is shown
- THEN no per-lot recent-movement widget is rendered for any lot
- AND the dashboard continues to show one row per active lot, as before

### Requirement: legacy resolution events migrate into the unified ledger

The system MUST back-fill pre-existing `lot_resolution_events` rows as `lot_movements` rows during the v5 schema migration. Each migrated row is converted to a single exit movement drawn from the v1 vocabulary and appears in the same Historial panel as native ledger movements.

- If the original `resolution` text matches a v1 vocabulary term (case-insensitive), the new movement uses that term.
- If the original `resolution` text does not match any v1 vocabulary term, the new movement uses `exit:other` and the original text is preserved in the new `notes` field (with any original `notes` appended).
- The legacy `lot_resolution_events` table SHALL remain in place for one release as a read-only anchor; no runtime path SHALL read or write it after migration.

#### Scenario: known legacy resolution maps to v1 vocabulary

- GIVEN a `lot_resolution_events` row with `resolution = 'sold'`, `quantity = 2`, and `notes = 'staff lunch'`
- WHEN the v5 migration applies
- THEN a new `exit:sale` movement is inserted with `quantity = 2`, `notes = 'staff lunch'`, and the original `created_at`
- AND the original row in `lot_resolution_events` is left untouched

#### Scenario: unknown legacy resolution maps to Otro with note

- GIVEN a `lot_resolution_events` row with `resolution = 'donated'` and `quantity = 1`
- WHEN the v5 migration applies
- THEN a new `exit:other` movement is inserted with `notes = 'donated'`
- AND the original row in `lot_resolution_events` is left untouched
- AND the Historial panel renders this row under reason `Otro` with the note visible

### Requirement: backfill of initial entries for pre-existing lots

The system MUST insert one `entry:initial` movement for every pre-existing lot during the v5 schema migration, into the lot's current `location_id`.

- Lots with `location_id IS NULL` at migration time SHALL be repointed to the per-store sentinel `Sin ubicación` location before the initial entry is inserted.
- The backfill is idempotent: re-running the migration adds zero additional rows.

#### Scenario: pre-existing lot receives an initial entry on migration

- GIVEN a database with one lot L having `quantity = 8` and `location_id = 'bodega-id'`
- WHEN the v5 migration applies
- THEN exactly one `entry:initial` row exists for L with `destination_location_id = 'bodega-id'` and `quantity = 8`
- AND re-running the migration does not insert a second row

#### Scenario: pre-existing lot with NULL location points to the sentinel

- GIVEN a database with one lot L having `quantity = 5` and `location_id IS NULL` in store S
- WHEN the v5 migration applies
- THEN a sentinel `Sin ubicación` location exists for store S
- AND L's `location_id` is repointed to the sentinel
- AND exactly one `entry:initial` row exists for L with `destination_location_id = sentinel.id` and `quantity = 5`

### Requirement: sentinel location for unassigned stock

The system MUST guarantee that every store has at most one sentinel `Sin ubicación` location. The sentinel is a regular active location used as the destination for the initial entry when a lot is created without a chosen location and the `require_initial_location_on_lot_create` setting is off.

- The sentinel is created lazily during migration and during lot creation.
- The sentinel's id SHALL be deterministic for the store so re-runs are idempotent.
- The sentinel SHALL be treated as a regular active location everywhere (filters, joins, dashboard) and is not hidden from pickers in v1; the Spanish label `Sin ubicación` makes the intent obvious.

#### Scenario: lot creation without location uses the sentinel when the setting is off

- GIVEN a store S exists with a sentinel `Sin ubicación`
- AND the `require_initial_location_on_lot_create` setting is off
- WHEN the user creates a lot in store S with `quantity = 6` and leaves the location picker empty
- THEN the lot's `location_id` is set to the sentinel id
- AND the initial entry's `destination_location_id` is the sentinel id

### Requirement: actor placeholder is `system`

The system MUST store the actor of every movement as the literal string `system` until users or login exist. No runtime path SHALL emit any other actor value in v1.

#### Scenario: actor is recorded as `system`

- GIVEN any user action creates a movement via `Registrar salida`, `Mover stock`, or `Ajustar conteo`
- WHEN the new `lot_movements` row is persisted
- THEN the `actor` column contains the string `system`

## Capability: Date input

### Requirement: Custom date picker

The system MUST provide an in-house Svelte custom date picker (no native `<input type="date">`) for every expiry date and report date-range field, and the picker MUST be the single source of truth for date input across the app.

The picker MUST bind ISO `YYYY-MM-DD` strings in and out, MUST reject any year outside the inclusive range `[1900-01-01, 2100-12-31]`, and MUST support both a typed-text manual entry path and a calendar popover path that share the same bound value. The picker MUST render no `<input type="date">` element in the DOM.

The picker MUST expose a `clearable` prop (default `true`); when `clearable={false}` the picker MUST NOT render any clear affordance and MUST NOT silently commit an empty value. The host (`LotForm` for the required expiry date) is responsible for the `required` JS guard; the picker preserves the bound value on empty input when `clearable={false}` and surfaces a visual invalid state instead.

The picker MUST support basic accessible keyboard ergonomics: Tab enters and leaves the trigger without trapping focus inside the popover; Escape closes the popover without committing a calendar selection and without rewriting typed text; Enter on a focused day cell emits that day's ISO date and closes the popover. Arrow keys MUST move the focused day by ±1 (Left/Right) or ±7 (Up/Down) days; PageUp/PageDown MUST move by ±1 month; Shift+PageUp/Shift+PageDown MUST move by ±1 year; every move MUST be clamped to the configured year range.

Manual `YYYY-MM-DD` text input MUST visually validate on blur (red border + helper text for invalid shape, invalid calendar date, or out-of-range year); the bound value MUST NOT update while the text is invalid and the typed text MUST remain visible in the input element (no silent rewrite).

#### Scenario: picker is in-house Svelte, not native

- GIVEN any date field in the app (LotForm expiry date, Reports `dateFrom`, Reports `dateTo`)
- WHEN `grep -R 'type="date"' src/` is executed after this slice lands
- THEN the command returns zero matches
- AND each field is rendered by the in-house `DatePicker.svelte` Svelte component composed around `CalendarMonth.svelte`

#### Scenario: ISO YYYY-MM-DD in/out with year range enforcement

- GIVEN the user opens the picker on any date field
- WHEN the user selects a day in any month of any year between 1900 and 2100 inclusive (via the calendar popover or manual entry)
- THEN the picker emits the selected day as an ISO `YYYY-MM-DD` string
- AND the bound value is updated to that string
- AND the text input displays the selected ISO date

#### Scenario: selection closes the popover and commits

- GIVEN the picker popover is open and the user is focused on a day cell
- WHEN the user clicks or presses Enter on a day cell
- THEN the popover closes immediately
- AND the selected `YYYY-MM-DD` is emitted and committed to the host
- AND the text input displays the selected ISO date

#### Scenario: manual YYYY-MM-DD text input validates without silent rewrite

- GIVEN the picker text input is focused
- WHEN the user types any of `2026-13-40`, `2026-02-30`, `1899-12-31`, or `2101-01-01`
- AND blurs the input
- THEN the input displays a visual invalid state (red border + helper text)
- AND the bound value is NOT updated
- AND the typed text remains visible in the input element (no silent rewrite)

#### Scenario: clearable=false hides the clear icon and prevents silent empty commit

- GIVEN the picker is mounted with `clearable={false}` (LotForm expiry date)
- WHEN the picker is rendered
- THEN no `×` clear icon is present in the DOM
- AND the user cannot clear the bound value through the picker
- WHEN the user deletes the typed text down to empty and blurs
- THEN the picker shows a visual invalid state
- AND the bound value is NOT silently committed to `""`

#### Scenario: clearable=true exposes the clear icon

- GIVEN the picker is mounted with the default `clearable={true}` (Reports `dateFrom`, `dateTo`)
- AND the bound value is non-empty
- WHEN the picker is rendered
- THEN a `×` clear icon is visible in the trigger
- AND clicking `×` clears the bound value to `""` and closes the popover

#### Scenario: basic accessible keyboard ergonomics

- GIVEN the picker trigger is focused
- WHEN the user presses Tab
- THEN focus moves through the trigger and the calendar-icon button in source order
- WHEN the user presses Tab again
- THEN focus leaves the picker (no focus trap inside the popover)
- GIVEN the popover is open
- WHEN the user presses Escape
- THEN the popover closes without committing a calendar selection
- AND any typed text remains visible (still subject to blur-validation rules)
- GIVEN a day cell is focused inside the popover
- WHEN the user presses Enter
- THEN that day's ISO date is emitted and the popover closes
- AND ArrowLeft/ArrowRight move the focused day by ±1 day
- AND ArrowUp/ArrowDown move the focused day by ±7 days
- AND PageUp/PageDown move the focused day by ±1 month
- AND Shift+PageUp/Shift+PageDown move the focused day by ±1 year
- AND every move is clamped to `[1900-01-01, 2100-12-31]`

#### Scenario: Today shortcut inside popover

- GIVEN the picker popover is open
- AND today is within `[minDate, maxDate]`
- WHEN the user clicks the `Today` button in the popover footer
- THEN the bound value is set to today's ISO `YYYY-MM-DD`
- AND the popover closes

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
- category filter — accepts a multi-category selection (`category_ids: string[]`) with **ANY-of** semantics. Default `None` or empty list means "no category filter applied" (not an explicit All chip). The selection may include the `Uncategorized` sentinel id (`__uncategorized__`) to include products with zero active category relations. The filter is applied at the SQL level via `EXISTS` (never `JOIN`).

The dashboard SHALL also surface two additional quick-filter buttons in its filter bar: "Next 7 days" and "All". Each named section in the list above MUST correspond to the row set defined for the matching quick-filter button in the `Dashboard quick-filter buttons return promised rows` requirement; activating the matching button MUST return exactly that section's rows.

(Previously: the dashboard listed four sections by name without specifying the exact row set per section or the existence of the "Next 7 days" and "All" buttons. The category filter was not part of the dashboard surface at all.)

#### Scenario: dashboard sections align with the matching filter buttons

- GIVEN the dashboard is loaded with no preset active
- WHEN the user activates each of the six filter buttons in turn
- THEN the row set displayed for "Expired" matches the expired-lots section
- AND the row set displayed for "Today" matches the lots-expiring-today section
- AND the row set displayed for "Alert window" matches the lots-inside-alert-window section
- AND the row set displayed for "Next 30 days" matches the next-30-days section
- AND the row set displayed for "Next 7 days" is the seven-day subset of "Next 30 days"
- AND the row set displayed for "All" is the unfiltered active-lot set

#### Scenario: dashboard category filter applies ANY-of at SQL level

- GIVEN the dashboard is loaded with `category_ids = ["dairy-id", "bakery-id"]`
- WHEN the dashboard requests `list_dashboard_lots`
- THEN the SQL includes an `EXISTS` subquery against `product_categories` matching any of the selected ids
- AND no `JOIN` against `product_categories` is added
- AND products in two selected categories appear exactly once per lot row

#### Scenario: dashboard category filter Uncategorized sentinel

- GIVEN a product P has zero rows in `product_categories` whose category has `is_active = 1`
- AND P has at least one active lot
- WHEN the dashboard loads with `category_ids = ["__uncategorized__"]`
- THEN P's lots are included in the result
- AND the SQL uses `NOT EXISTS (... product_categories JOIN categories ON is_active = 1)`

#### Scenario: dashboard category filter empty selection is no filter

- GIVEN the dashboard is loaded with `category_ids = []`
- WHEN the dashboard requests `list_dashboard_lots`
- THEN no `EXISTS` clause against `product_categories` is added to the SQL
- AND every active lot matching the other filters is returned

### Requirement: expired visibility

Caduxo shall show expired lots prominently until they are resolved or archived.

### Requirement: Dashboard quick-filter buttons return promised rows

The Dashboard quick-filter bar SHALL provide six buttons in the fixed order `All`, `Expired`, `Today`, `Alert window`, `Next 7 days`, `Next 30 days`. For each button, the dashboard's `lots` array MUST contain exactly the active lots whose enriched row satisfies the corresponding predicate below. The predicate is computed from the row's `days_remaining` (an `i64` day count where `< 0` means expired, `0` means today, `> 0` means future) and `alert_days_before` (an `i32`). The repository continues to enforce `WHERE el.status = 'active'`, so resolved and archived lots are not in scope regardless of preset.

| Button | Predicate on the enriched row |
|---|---|
| `All` | `true` (no additional filter; the row is already active) |
| `Expired` | `row.days_remaining < 0` |
| `Today` | `row.days_remaining == 0` |
| `Alert window` | `row.days_remaining >= 0 && row.alert_days_before > 0 && row.days_remaining <= row.alert_days_before as i64` |
| `Next 7 days` | `row.days_remaining >= 0 && row.days_remaining <= 7` |
| `Next 30 days` | `row.days_remaining >= 0 && row.days_remaining <= 30` |

The predicates are intentionally independent ranges. A row may satisfy zero, one, or several predicates. Each button is its own filter.

#### Scenario: All returns every active lot

- GIVEN active lots with `days_remaining` in `{-2, 0, 5, 31}`
- WHEN the user activates the "All" button
- THEN the dashboard lists every active lot
- AND the row set is identical to the unfiltered repository result for the same store/location filters

#### Scenario: Expired returns only lots past their expiry date

- GIVEN active lots with `days_remaining` in `{-2, 0, 5}`
- WHEN the user activates the "Expired" button
- THEN the dashboard lists only the lot with `days_remaining = -2`
- AND lots with `days_remaining >= 0` are NOT listed

#### Scenario: Today returns only lots expiring today

- GIVEN active lots with `days_remaining` in `{-1, 0, 1}`
- WHEN the user activates the "Today" button
- THEN the dashboard lists only the lot with `days_remaining = 0`

#### Scenario: Alert window includes lots inside their configured window including today

- GIVEN active lots with `(days_remaining, alert_days_before)` of `(0, 14)`, `(10, 30)`, `(25, 30)`, and `(-1, 14)`
- WHEN the user activates the "Alert window" button
- THEN the dashboard lists the lots with `days_remaining` in `{0, 10, 25}`
- AND the expired lot with `days_remaining = -1` is NOT listed
- AND a lot with `days_remaining = 0` and `alert_days_before = 14` IS listed

#### Scenario: Alert window excludes lots with no configured alert window

- GIVEN an active lot with `days_remaining = 5` and `alert_days_before = 0`
- WHEN the user activates the "Alert window" button
- THEN the lot is NOT listed
- AND a lot with `days_remaining = 5` and `alert_days_before = 6` IS listed

#### Scenario: Next 7 days returns lots within seven calendar days inclusive

- GIVEN active lots with `days_remaining` in `{0, 5, 7, 8, 25}`
- WHEN the user activates the "Next 7 days" button
- THEN the dashboard lists only the lots with `days_remaining` in `{0, 5, 7}`
- AND a lot with `days_remaining = 25` is NOT listed even when its `alert_days_before` is `30`

#### Scenario: Next 30 days includes alert-window lots that expire within 30 days

- GIVEN an active lot with `days_remaining = 20` and `alert_days_before = 14` (which classifies as the `alert_window` urgency bucket)
- WHEN the user activates the "Next 30 days" button
- THEN the lot IS listed
- AND a lot with `days_remaining = 31` is NOT listed

#### Scenario: Negative days_remaining falls out of every range except Expired and All

- GIVEN an active lot with `days_remaining = -3`
- WHEN the user activates any of "Today", "Alert window", "Next 7 days", or "Next 30 days"
- THEN the lot is NOT listed under that filter
- AND the lot IS listed under "Expired" and under "All"

### Requirement: Dashboard preset matcher uses per-row day counts, not bucket strings

The Dashboard preset matcher MUST evaluate the row's `days_remaining` and `alert_days_before` fields. The matcher MUST NOT rely on the row's `urgency` bucket string as a filter key. Changing the classifier's bucket boundaries MUST NOT change the rows returned by any quick-filter button.

#### Scenario: matcher signature uses the enriched row, not the urgency string

- GIVEN the matcher implementation
- WHEN it is invoked during `get_dashboard` for each enriched row
- THEN it receives the enriched `DashboardLotRow` and an `Option<DashboardPreset>`
- AND it returns `true` when the row satisfies the predicate for that preset per the requirement above
- AND the result does not depend on the value of `row.urgency`

### Requirement: Dashboard urgency cards remain bucket counts

The four urgency cards on the Dashboard MUST continue to display counts derived from the row's `Urgency` classification (`Expired` / `Today` / `AlertWindow` / `Next30Days`). The card counts are independent of the quick-filter row sets and MAY differ from them.

#### Scenario: card counts are derived from the urgency bucket

- GIVEN an active lot with `days_remaining = 10` and `alert_days_before = 30` (which classifies as the `next_30_days` bucket because `alert_days_before >= 30`)
- WHEN the dashboard loads with no filter active
- THEN the "Next 30 days" card count includes this lot
- AND the "Alert window" card count does NOT include this lot
- AND activating the "Alert window" button still lists this lot because `days_remaining` is in `[0, alert_days_before]`

#### Scenario: card counts may diverge from filter row counts by design

- GIVEN active lots whose `alert_days_before >= 30` classify them as `next_30_days` even when they expire within their own window
- WHEN the dashboard loads
- THEN the "Next 30 days" card count includes those lots
- AND the "Alert window" filter row set also includes those lots
- AND the two numbers are allowed to differ; this divergence is accepted scope and is not a defect

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
- category — the category filter accepts a multi-category selection (`category_ids: string[]`) with **ANY-of** semantics. Default `None` or empty list means "no category filter applied" (not an explicit All chip). The selection may include the `Uncategorized` sentinel id (`__uncategorized__`) to include products with zero active category relations. The filter is applied at the SQL level via `EXISTS` (never `JOIN`) so that a product in two selected categories is matched exactly once per lot row. See the `multi-category filter ANY-of semantics` scenario under `Capability: Categories > multi-category product model` for the canonical contract.
- status
- date range — the `dateFrom` and `dateTo` fields use the `Date input` picker (clearable, optional). Empty values are translated to `null` by the existing `dateFrom.trim() || null` / `dateTo.trim() || null` chain in `ReportsPage.buildFilters()`. No `<input type="date">` is rendered for these fields.

(Previously: the requirement listed `category` as a single-category filter. This delta replaces the single-category filter with a multi-category list (`category_ids`), establishes ANY-of semantics, introduces the `Uncategorized` sentinel, and pins the filter to SQL-level `EXISTS` to prevent duplicate-count risk.)

### Requirement: PDF export

Caduxo shall export reports to PDF using Rust-side structured PDF generation.

### Requirement: CSV report export

Caduxo shall export report rows to CSV.

## Capability: Calendar

### Requirement: Calendar tab

The system MUST provide a top-level main-navigation Calendar tab (added between Products and Reports) that opens the current month with today highlighted and selected, displays per-day lot expirations as dot badges, and lists the lots and products expiring on a selected day in a day-detail panel.

The Calendar tab MUST reuse the same `CalendarMonth.svelte` primitive used by the date picker popover so the day grid, year picker, month picker, visual styling, and keyboard surface are implemented once. The Calendar tab MUST source its expiration data from the existing `list_dashboard_lots` Tauri command (no new backend command in this slice); the day-bucket map MUST be computed client-side from the returned active lots. Month navigation MUST NOT trigger a refetch in this slice.

Clicking a day MUST set the selected date and reveal a day-detail panel that lists every active lot whose `expiry_date` equals the selected date, with columns for product, quantity, unit, store, location, days remaining, and status. Clicking a lot row MUST open the existing lot edit flow used elsewhere in the app. Clicking a day with no expirations MUST still select it and show a `No expirations on YYYY-MM-DD` placeholder.

The Calendar tab MUST inherit the full keyboard surface of the calendar primitive (Tab, Esc, Enter, arrow keys, PageUp/Down, Shift+PageUp/Shift+PageDown) with a tab order of prev-month chevron → month label → year chip → day grid → next-month chevron → day-detail rows.

The month label inside the header MUST act as a **trigger for the month picker** (click or `Enter`/`Space` opens the picker), not as a button that advances the view by one month. Sequential month navigation remains available through the prev/next-month chevrons and the `PageUp` / `PageDown` keyboard shortcuts. The month label's accessible name MUST use the `ariaOpenMonthPicker` i18n key ("Open month picker" / "Abrir selector de mes").

(Previously: the requirement governed the calendar's data source, the keyboard surface, the day-detail panel columns, the lot row edit flow, and the inherited keyboard surface from `CalendarMonth.svelte`. It did not yet reference a month picker and used `ariaCycleMonth` for the month label's accessible name.)

#### Scenario: opens at current month with today highlighted and selected

- GIVEN the user clicks the Calendar tab in main navigation
- WHEN the page renders
- THEN the calendar grid displays the current month (`viewYear` and `viewMonth` derived from `new Date()`)
- AND today is visually highlighted (distinct from the selected highlight)
- AND today is the selected date (`selectedDate = today`)

#### Scenario: per-day expirations visible as dot badges

- GIVEN active lots exist in the local database with various `expiry_date` values
- WHEN the Calendar tab renders
- THEN each day cell with at least one active lot displays a small dot under the day number
- AND the dot corresponds to the count of active lots whose `expiry_date` equals that day
- AND the count is also shown in the day-detail panel for the selected date

#### Scenario: day-detail panel lists lots and products

- GIVEN a day is selected on the Calendar tab
- WHEN the day-detail panel renders
- THEN the panel lists every active lot whose `expiry_date` equals the selected date
- AND each row shows product, quantity, unit, store, location, days remaining, and status

#### Scenario: empty day shows the placeholder message

- GIVEN a day is selected on the Calendar tab
- AND no active lot has an `expiry_date` equal to that day
- WHEN the day-detail panel renders
- THEN the panel shows `No expirations on YYYY-MM-DD` (or an equivalent English placeholder that names the date)

#### Scenario: lot row opens the existing lot edit flow

- GIVEN the day-detail panel shows one or more lot rows
- WHEN the user clicks a lot row
- THEN the existing lot edit overlay opens for that lot (the same pattern used by `DashboardPage`)
- AND no new edit-flow primitive is introduced

#### Scenario: data source is the existing list_dashboard_lots command

- GIVEN the user opens the Calendar tab
- WHEN the page mounts
- THEN the page calls `list_dashboard_lots({ store_id: null, location_id: null, preset: null, urgency: null })` exactly once
- AND the day-bucket map is computed client-side from the returned active lots
- AND month navigation does NOT trigger a refetch in this slice

#### Scenario: keyboard surface inherited from the calendar primitive

- GIVEN the Calendar tab is rendered
- WHEN the user navigates with Tab, Esc, Enter, arrow keys, and PageUp/Down
- THEN focus traverses prev-month chevron → month label → year chip → day grid → next-month chevron → day-detail rows
- AND arrow keys / PageUp / PageDown / Shift+PageUp / Shift+PageDown move the focused day with the same clamping rules as the picker
- AND Enter on a focused day cell emits the ISO date and updates `selectedDate`
- AND `Enter` / `Space` on the month label opens the month picker overlay without changing `viewMonth`

#### Scenario: clicking the month label opens a month picker grid

- GIVEN the Calendar tab is rendered
- AND the month picker overlay is not currently open
- WHEN the user clicks the month label in the calendar header
- THEN a month picker overlay opens anchored under the header
- AND the overlay lists all 12 months in a 3-column × 4-row grid
- AND the chip for the currently viewed month is rendered with the selected visual treatment
- AND the year shown in the overlay's header matches `viewYear`
- AND no `monthChange` event has been dispatched yet

## Capability: Data persistence and safety

### Requirement: local persistence

Caduxo shall store all MVP data in a local SQLite database.

### Requirement: offline use

Caduxo shall support normal operation without internet access.

### Requirement: backup and restore

Caduxo shall provide a manual backup/export and restore path.

### Requirement: migrations

Caduxo shall preserve existing local data across database schema migrations.

The V4 migration introduces the `product_categories` junction table whose composite primary key `(product_id, category_id)` enforces many-to-many uniqueness, plus the supporting indexes `idx_product_categories_category` and `idx_product_categories_product`. V4 back-fills the junction from the legacy `products.category_id` column in an idempotent insert (`INSERT … WHERE NOT EXISTS`) so re-running V4 is a no-op. V4 also runs a case-fold deduplication step: for groups of `categories` rows whose names differ only in case, the **oldest** row (by `created_at ASC, id ASC`) is canonical, younger duplicates are archived (`is_active = 0`), and all junction rows (plus any `products.category_id` rows referencing a duplicate) are reassigned to the canonical id. No hard delete occurs in V4. The legacy `products.category_id` column remains in the schema as ignored data; the junction is the runtime source of truth.

(Previously: the requirement stated only that data is preserved across migrations. It did not describe V4's junction table, the idempotent back-fill from the legacy FK, or the case-fold deduplication step.)

#### Scenario: V4 applies on a fresh database

- GIVEN a fresh database with no prior migrations
- WHEN the migrator runs V4
- THEN the `product_categories` table exists with the composite primary key `(product_id, category_id)`
- AND `idx_product_categories_category` and `idx_product_categories_product` exist
- AND no junction rows are inserted (the legacy column is NULL for every product on a fresh DB)

#### Scenario: V4 back-fill from legacy column is idempotent

- GIVEN a V3-era database with `products.category_id` set on several rows
- WHEN the migrator runs V4 once and then runs again on the same database
- THEN the junction row count after the second run equals the count after the first run
- AND no duplicate `(product_id, category_id)` pairs exist (composite PK enforces this)

#### Scenario: V4 case-fold dedup keeps the oldest category canonical

- GIVEN the database contains two categories `Dairy` (older) and `dairy` (younger) with case-only duplicate names
- AND a junction row references the younger `dairy` id
- AND a `products.category_id` row also references the younger `dairy` id
- WHEN the migrator runs V4
- THEN the older `Dairy` row remains active and is the canonical id
- AND the younger `dairy` row is archived (`is_active = 0`)
- AND the junction row is reassigned to the canonical `Dairy` id
- AND the legacy `products.category_id` row is reassigned to the canonical `Dairy` id
- AND the migration logs `duplicates_found`, `remapped_junction`, `remapped_legacy`, and `archived_duplicate` counts

#### Scenario: pre-V4 backup restored onto a V4 build picks up the junction

- GIVEN a backup file from a V3-era database with `products.category_id` rows
- WHEN the user restores the backup on a V4 build
- THEN the migrator runs V4 on the restored pool
- AND the junction is populated from the legacy column
- AND products appear in the picker with their assigned categories without a manual re-migration step

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

---

<!-- archived-change: caduxo-daisyui-redesign -->

<!-- Archived manually on 2026-09-20 after native sdd-archive selection gate blocked execution. Source: openspec/changes/archive/2026-09-20-caduxo-daisyui-redesign/. -->

### Capability: Design system foundation

#### Requirement: Tailwind v4 + DaisyUI dependency stack wired into Vite

The project MUST depend on `tailwindcss` (v4), `@tailwindcss/vite`, and
`daisyui`. `@tailwindcss/vite` MUST be registered alongside
`@sveltejs/vite-plugin-svelte` in `vite.config.ts`. Vite dev and
`npm run build` MUST both succeed with the plugin chain present, Svelte
HMR MUST continue to work, and no console errors MAY be introduced by
the plugin order.

#### Scenario: vite dev and build both pass with the new plugin chain

- GIVEN the dependencies are installed and `vite.config.ts` registers
  `@tailwindcss/vite` before `svelte()`
- WHEN `npm run build` and `npm run dev` are executed
- THEN both commands exit successfully
- AND the Svelte HMR overlay reports no plugin-order errors

#### Requirement: project stylesheet imports Tailwind and the DaisyUI plugin

A new `src/app.css` MUST exist and MUST contain `@import "tailwindcss";`
followed by `@plugin "daisyui";` plus the project's custom theme blocks
and any global motion / accessibility utilities. `src/main.ts` MUST
import `src/app.css` and MUST NOT import the legacy `./style.css`.

#### Scenario: src/app.css is the active stylesheet entry

- GIVEN the project after this slice
- WHEN `src/main.ts` is read
- THEN it imports `./app.css`
- AND `grep -R 'style.css' src/main.ts` returns zero matches

#### Scenario: Tailwind preflight does not regress the unmigrated UI

- GIVEN the Tailwind + DaisyUI plugin is registered and no component
  has been migrated yet
- WHEN the app is launched in a Tauri / dev browser shell and the main
  screens (Dashboard, Stores, Products, Product detail, Calendar,
  Reports, CSV Import, Backup/Restore, Configuration, Unit Review) are
  visually inspected
- THEN preflight MAY shift default heading, list, button, input, or
  table styling
- AND no widespread breakage (illegible text, broken layouts,
  unusable controls) is present
- AND if widespread breakage appears, the foundation phase pauses
  before any page migration

#### Requirement: components reference DaisyUI theme tokens, not inline colour literals

Every visual primitive and migrated component MUST source its colour
values from DaisyUI theme tokens (`primary`, `secondary`, `accent`,
`neutral`, `base-100`, `base-200`, `base-300`, `info`, `success`,
`warning`, `error`, plus the corresponding `-content` tokens). No
migrated component MAY inline a hex / rgb colour that is not theme-
derived. This requirement applies to migrated surfaces; vestigial code
in archived legacy folders is out of scope.

#### Scenario: no hex / rgb colour literals remain in migrated surfaces

- GIVEN the migration has finished for a given component
- WHEN the component source is grepped for hex literals matching
  `#[0-9a-fA-F]{3,8}\b` and rgb / rgba literals
- THEN the grep returns zero matches inside that component file
- AND every colour reference resolves through a DaisyUI token or a
  Tailwind utility backed by a token

#### Requirement: legacy src/style.css retired after migration

The legacy `src/style.css` MUST be retired once the migration
confirms no production component references its classes. Until that
confirmation is in place, the file MAY remain; once the migration is
complete, the file MUST be deleted and `src/main.ts` MUST NOT import
it.

#### Scenario: legacy stylesheet is removed after migration

- GIVEN every migrated component no longer references any class
  declared in `src/style.css`
- WHEN `git grep -E '\.(shell|hero|eyebrow|cards)\b' src/` is executed
- THEN the command returns zero matches
- AND `src/style.css` is removed from the repository

### Capability: Theme support

#### Requirement: custom `caduxo-light` brand theme

A custom DaisyUI theme named `caduxo-light` MUST exist. The theme's
`primary` MUST be derived from the existing Caduxo brand blue
`#2563eb` (or its close semantic equivalent), the `base-100` family
MUST be derived from the existing base background `#f6f8fb`, and the
semantic `info` / `success` / `warning` / `error` tokens MUST be
derived from the existing red / amber / green / blue palette Caduxo
already uses for urgency and feedback. `caduxo-light` MUST be the
default theme when no other choice is in effect.

#### Scenario: caduxo-light resolves to brand-aligned tokens

- GIVEN the DaisyUI plugin is registered with the `caduxo-light` block
- WHEN a migrated component renders a `btn-primary`
- THEN the rendered primary colour family derives from `#2563eb`
- AND when the component renders a surface (`bg-base-100`), the
  background derives from `#f6f8fb`

#### Requirement: shipped theme set at v1 GA

The v1 GA build MUST bundle exactly two themes: `caduxo-light` (custom
brand) and DaisyUI built-in `dark`. No other DaisyUI built-in theme
MUST be enabled by default in v1. Accent themes are deferred to a
post-v1 follow-up change.

#### Scenario: only the curated theme set is bundled

- GIVEN the project builds for production
- WHEN the DaisyUI plugin configuration is read
- THEN the `themes` list contains exactly `["caduxo-light", "dark"]`
- AND no `cupcake`, `corporate`, `business`, `emerald`, `night`,
  `dim`, `nord`, or other DaisyUI built-in is enabled

#### Requirement: theme switcher on Configuration page

The Configuration page MUST expose a theme switcher adjacent to the
existing language selector. The switcher MUST display the available
themes by name in the active locale, MUST indicate which theme is
currently applied, and MUST allow the user to pick a different theme.
Switching a theme MUST update the rendered surface optimistically and
MUST persist the choice to `app_settings.theme` via the existing
`updateSettings` IPC command.

#### Scenario: switcher lists the curated themes and shows the active one

- GIVEN the active theme is `caduxo-light`
- WHEN the user opens the Configuration page
- THEN the theme switcher lists `caduxo-light` and `dark`
- AND `caduxo-light` is marked as the currently active selection
- AND the labels are rendered in the active locale

#### Scenario: switching a theme applies it optimistically and persists it

- GIVEN the user opens the Configuration page
- WHEN the user selects `dark` from the theme switcher
- THEN the rendered UI re-themes to `dark` without a page reload
- AND a subsequent `app_settings.theme` read returns `dark`
- AND a relaunch of the application restores `dark`

#### Scenario: failed theme persistence rolls back the optimistic switch

- GIVEN the user opens the Configuration page with theme `caduxo-light`
- WHEN the user selects `dark` and `updateSettings` fails on the
  backend
- THEN the rendered UI returns to `caduxo-light`
- AND `app_settings.theme` is unchanged on the next read
- AND the user sees an inline error in the switcher describing the
  failure

#### Requirement: theme persistence via app_settings.theme

The active theme MUST be persisted in the existing `app_settings`
table using the same read / write pattern already used for the
`language` row. No new Tauri command surface outside the existing
`settings` command group is introduced. The persisted value MUST be
exactly one of the bundled theme names (`caduxo-light` or `dark`); any
other value MUST be rejected.

#### Scenario: theme is read from app_settings on launch

- GIVEN `app_settings.theme` exists with value `dark`
- WHEN the application launches
- THEN the active theme is `dark` without consulting the OS preference

#### Scenario: an unsupported stored theme value falls back to caduxo-light

- GIVEN `app_settings.theme` exists with value `synthwave` (not in
  the v1 curated set)
- WHEN the application launches
- THEN the active theme is `caduxo-light`
- AND no error is surfaced to the user

#### Requirement: OS-aware theme defaulting via prefers-color-scheme

When no `app_settings.theme` row exists, the bootstrap MUST read the
host's `prefers-color-scheme` and select `dark` when the value is
`dark` and `caduxo-light` otherwise. The OS preference MUST be re-read on
every app start; no `localStorage` cache may shadow the persisted
setting.

#### Scenario: no stored theme, OS prefers dark

- GIVEN `app_settings.theme` does not exist
- AND the host reports `prefers-color-scheme: dark`
- WHEN the application launches
- THEN the active theme is `dark`

#### Scenario: no stored theme, OS prefers light or no preference

- GIVEN `app_settings.theme` does not exist
- AND the host reports `prefers-color-scheme: light` or the media
  query is unavailable
- WHEN the application launches
- THEN the active theme is `caduxo-light`

#### Scenario: stored theme wins over OS preference

- GIVEN `app_settings.theme` exists with value `caduxo-light`
- AND the host reports `prefers-color-scheme: dark`
- WHEN the application launches
- THEN the active theme is `caduxo-light` and the OS preference is
  not consulted

#### Requirement: theme precedence is manual > persisted > OS > caduxo-light

When multiple sources of truth for the active theme are present, the
precedence MUST be: an in-session manual override (if any) beats a
persisted `app_settings.theme` row; the persisted row beats the host
`prefers-color-scheme`; the OS preference beats the literal
`caduxo-light` fallback. The Configuration page's theme switcher MUST
show which source currently determines the active theme so the user
can tell why the theme they see is the theme they see.

#### Scenario: switcher shows the source of the active theme

- GIVEN `app_settings.theme` does not exist
- AND the host reports `prefers-color-scheme: dark`
- WHEN the user opens the Configuration page
- THEN the switcher marks `dark` as active
- AND the switcher labels the source as the OS preference

#### Requirement: readable contrast and unbroken layouts in every shipped theme

Every main surface (Dashboard, Stores, Products, Product detail,
Calendar, Reports, CSV Import, Backup/Restore, Configuration, Unit
Review, plus every modal dialog) MUST remain visually legible and
unbroken in both `caduxo-light` and `dark`. Legibility means
WCAG-AA-readable text / background contrast on body text, headings,
inputs, badges, alerts, and primary button labels. Layout integrity
means no overflow, no clipped content, no overlap, no broken modal
backdrop, and no hidden focus ring in either theme.

#### Scenario: dark theme passes a manual contrast and layout pass on the main surfaces

- GIVEN the active theme is `dark`
- WHEN the main surfaces are manually inspected
- THEN no body / heading / input / badge text falls below WCAG-AA
  contrast against its background
- AND no layout is broken (no overflow, no clipping, no overlap)
- AND every focus ring remains visible

#### Scenario: caduxo-light theme passes the same pass

- GIVEN the active theme is `caduxo-light`
- WHEN the main surfaces are manually inspected
- THEN no body / heading / input / badge text falls below WCAG-AA
  contrast against its background
- AND no layout is broken
- AND every focus ring remains visible

### Capability: Shared UI primitives

The primitives below live under `src/components/ui/`. They are the only
canonical Svelte components for their respective surfaces in v1; ad-hoc
local classes that duplicate this responsibility MUST be retired as
their owning surfaces migrate.

#### Requirement: shared Button.svelte wraps DaisyUI button variants

A `src/components/ui/Button.svelte` primitive MUST exist and MUST
expose variants for `primary`, `secondary`, `ghost`, `outline`,
`danger`, `warning`, `success`, `link`, and `icon`. The primitive
MUST support a `disabled` state, a `loading` state, optional leading
or trailing icon slots, an accessible label for icon-only usage, and
consistent focus / hover / active / disabled treatment across all
variants.

#### Scenario: every variant renders a DaisyUI-themed button

- GIVEN a screen uses `<Button variant="primary">`, `<Button
  variant="danger">`, and `<Button variant="icon" aria-label="Close">`
- WHEN the screen is rendered
- THEN each instance renders the corresponding DaisyUI `btn-*` class
- AND the icon-only button carries the `aria-label`

#### Scenario: loading state shows a DaisyUI spinner and disables interaction

- GIVEN a submit button is in `loading` state
- WHEN the screen renders
- THEN the button contains the DaisyUI `loading loading-spinner`
- AND the button cannot be re-activated until `loading` clears

#### Requirement: shared Card.svelte with optional header/footer slots

A `src/components/ui/Card.svelte` primitive MUST exist with consistent
padding, border, shadow, and optional header / footer slots. The
primitive MUST be the only canonical card surface for urgency cards,
report type cards, settings sections, backup / import result
summaries, and product / store detail panels in v1.

#### Scenario: every migrated card surface uses the Card primitive

- GIVEN the migration has finished for a given screen that renders a
  card-like surface
- WHEN the screen source is grepped for the legacy `.urgency-card`,
  `.settings-section`, or one-off `.card` rules
- THEN the grep returns zero matches in that screen
- AND the screen composes the Card primitive instead

#### Requirement: shared Modal.svelte built on native dialog

A `src/components/ui/Modal.svelte` primitive MUST exist and MUST be
backed by the native `<dialog>` element driven by `dialog.showModal()`
to obtain native focus trap, native Escape handling, and a themed
backdrop. The primitive MUST support size variants (`small`, `default`,
`wide`), an optional close button, click-outside-to-close when allowed,
and focus restoration to the element that opened it.

#### Scenario: modal opens with native focus trap and Escape closes it

- GIVEN a user activates the trigger for any migrated modal
- WHEN the modal opens
- THEN focus is trapped inside the modal
- AND pressing `Escape` closes the modal
- AND focus returns to the trigger element after the modal closes

#### Scenario: click-outside closes the modal when allowed

- GIVEN a modal is open and click-outside-to-close is allowed
- WHEN the user clicks on the backdrop
- THEN the modal closes
- AND focus returns to the trigger element

#### Requirement: shared Table.svelte with sticky / zebra / alignment helpers

A `src/components/ui/Table.svelte` primitive MUST exist and MUST
expose `zebra`, `sticky header`, `dense`, `default` sizing, a numeric
alignment helper, an `empty-state` slot, and a `loading-state` slot.
The primitive MUST be the canonical table wrapper for every table in
the migrated app.

#### Scenario: shared Table covers all migrated table surfaces

- GIVEN the migration has finished for the dashboard lot table,
  LotMovementsPanel, ReportsPage, CsvImportPage preview, StoresPage,
  and BackupRestorePage info lists
- WHEN those surfaces are inspected
- THEN each table composes the Table primitive
- AND no legacy `lot-table`, `reports-table`, `reports-empty`,
  `lot-picker`, `info-list`, `checks-list`, or `confirm-box` local
  class remains as the table wrapper

#### Requirement: shared Badge.svelte with urgency and semantic variants

A `src/components/ui/Badge.svelte` primitive MUST exist and MUST
expose urgency variants (`expired`, `today`, `alert`, `soon`,
`normal`) and semantic variants (`success`, `warning`, `error`,
`info`, `neutral`). The primitive MAY optionally render a leading
status dot.

#### Scenario: urgency badge uses semantic DaisyUI classes

- GIVEN a lot has urgency `expired`
- WHEN the dashboard renders the lot's badge
- THEN the badge renders with the semantic `badge-error` (or the
  equivalent urgency token) and the visual treatment matches every
  other urgency badge

#### Requirement: shared Alert.svelte with semantic feedback variants

A `src/components/ui/Alert.svelte` primitive MUST exist and MUST
expose `success`, `error`, `warning`, and `info` variants. Each
variant MUST render a consistent icon, text slot, and optional
action slot.

#### Scenario: every migrated feedback surface uses Alert

- GIVEN the migration has finished for UnitReviewBanner and the
  dashboard error banner
- WHEN those surfaces are inspected
- THEN each surface composes the Alert primitive
- AND no legacy `.unit-banner`, `.banner-icon`, `.banner-text`,
  `.warning-banner`, or `.error-banner` local rule remains as the
  banner surface

#### Requirement: shared EmptyState.svelte

A `src/components/ui/EmptyState.svelte` primitive MUST exist and MUST
expose a title, body text, an optional icon, and an optional action
slot. The primitive MUST be the canonical empty surface for every
migrated page.

#### Scenario: every migrated empty surface uses EmptyState

- GIVEN the migration has finished for LotMovementsPanel, Dashboard
  lot table, StoresPage store list, ReportsPage, CalendarPage
  day-detail, ProductsPage, and CSV import preview
- WHEN those surfaces render their empty state
- THEN each composes the EmptyState primitive
- AND no legacy `.empty-state` / `.empty-hint` local rule remains as
  the empty surface

#### Requirement: shared LoadingState.svelte

A `src/components/ui/LoadingState.svelte` primitive MUST exist and
MUST expose a `skeleton` row option, a `text` fallback, and a
reduced-motion-safe shimmer (or a static skeleton when reduced motion
is requested).

#### Scenario: LoadingState respects prefers-reduced-motion

- GIVEN the host reports `prefers-reduced-motion: reduce`
- WHEN any LoadingState is rendered
- THEN the rendered skeleton is static (no shimmer animation)
- AND the same loading flow reaches the same end state as a
  non-reduced-motion user

#### Requirement: shared Tabs.svelte

A `src/components/ui/Tabs.svelte` primitive MUST exist and MUST
support `tabs-bordered` or `tabs-lifted` style, an active state, and
keyboard navigation. The primitive MUST be the canonical tab strip
for the product detail page, the dashboard / calendar detail panels,
and any other tab surface in v1.

#### Scenario: tabs keyboard surface is preserved across migrated tabs

- GIVEN the user has focus inside the migrated Tabs primitive
- WHEN the user presses `ArrowLeft` / `ArrowRight`, `Home`, `End`,
  `Enter`, or `Space`
- THEN the active tab updates to the focused or activated tab
- AND the corresponding panel becomes visible

#### Requirement: shared Select.svelte and Input.svelte themed form primitives

A `src/components/ui/Select.svelte` primitive and a
`src/components/ui/Input.svelte` primitive MUST exist. The Select
primitive MUST render a DaisyUI-themed select surface (no OS-styled
chrome) with a consistent caret, focus ring, and theme-aware hover /
active state. The Input primitive MUST render a DaisyUI `input
input-bordered` with an `input-error` invalid state, paired with
`label`, `label-text`, and `label-text-alt` for label, required
marker, and helper text.

#### Scenario: locale selector on Configuration page renders the DaisyUI select

- GIVEN the Configuration page renders the locale selector
- WHEN the page is inspected
- THEN the selector composes the Select primitive
- AND no `<select>` element renders with the host operating system's
  native dropdown chrome inside the app's visual chrome

#### Requirement: shared Toggle.svelte replaces the custom toggle

A `src/components/ui/Toggle.svelte` primitive MUST exist and MUST
render the DaisyUI `toggle toggle-primary` surface. The primitive
MUST replace the bespoke `toggle-wrap` / `toggle-track` /
`toggle-thumb` toggle pattern in `ConfigurationPage.svelte` and any
other migrated location.

#### Scenario: Configuration page toggle renders the DaisyUI toggle

- GIVEN the Configuration page renders the settings toggles
- WHEN the page source is grepped for `.toggle-wrap`, `.toggle-track`,
  and `.toggle-thumb`
- THEN the grep returns zero matches
- AND the rendered toggle carries the DaisyUI `toggle` class

#### Requirement: shared Tooltip.svelte keyboard-reachable

A `src/components/ui/Tooltip.svelte` primitive MUST exist and MUST
be activated by both hover and keyboard focus. The primitive MUST
be used wherever the existing app relies on the browser's native
`title=` attribute for icon-only buttons or otherwise essential
information, and it MUST render via DaisyUI `tooltip` classes.

#### Scenario: icon-only button exposes a keyboard-reachable tooltip

- GIVEN an icon-only button (for example the UnitReviewBanner
  dismiss button or a calendar navigation arrow) is rendered through
  the migrated surface
- WHEN the button receives keyboard focus
- THEN the tooltip text becomes visible
- AND the tooltip text is rendered in the active locale

### Capability: Native control replacement

#### Requirement: Configuration page locale select uses the DaisyUI Select primitive

The Configuration page's locale `<select>` MUST be replaced by the
`Select.svelte` primitive. The replacement MUST preserve all
existing locale choices, persist the selection through the existing
`app_settings.language` flow, default to the host's detected locale
when no row is on file, and behave identically on the existing
`hint reappears if persisted value is cleared` and `selector update
rolls back on IPC failure` scenarios carried over from the
internationalisation capability.

#### Scenario: locale selector no longer renders OS-styled chrome

- GIVEN the Configuration page renders the locale selector after
  migration
- WHEN the page is visually inspected
- THEN the selector shows a DaisyUI-themed caret and surface
- AND no operating-system dropdown chrome (Windows / macOS / GTK)
  appears inside the app's visual chrome

#### Requirement: ProductForm datalist inputs are wrapped with the Input primitive

The `<datalist>` autocomplete inputs in `ProductForm.svelte` (barcode
type and unit definition lists) MUST be wrapped by the `Input.svelte`
primitive for visual styling while preserving the underlying
`<datalist>` element for native keyboard and screen-reader
announcements. The wrapper MUST NOT replace the `<datalist>` markup.

#### Scenario: datalist inputs keep autocomplete semantics

- GIVEN the user opens `ProductForm.svelte` in create mode
- WHEN the barcode type and unit definition fields render
- THEN each field is wrapped by the Input primitive
- AND each underlying `<datalist>` element remains present with the
  same `id` and option list
- AND typing in the field continues to surface autocomplete
  suggestions

#### Requirement: text inputs themed via DaisyUI input across all forms

Every bare `<input type="text">` in the migrated components
(`ProductForm`, `LotForm`, `StoresPage`, `BackupRestorePage`,
`ConfigurationPage`, `ResolveQuantityDialog`, etc.) MUST render via
the `Input.svelte` primitive. The primitive MUST render `input
input-bordered`, MUST apply `input-error` when the host marks the
field invalid, MUST pair with `label` / `label-text` / `label-text-alt`
for label, required marker, and helper text, and MUST keep all
existing validation behaviour intact.

#### Scenario: every migrated text input goes through the Input primitive

- GIVEN the migration has finished for `ProductForm`, `LotForm`,
  `StoresPage`, `BackupRestorePage`, `ConfigurationPage`, and
  `ResolveQuantityDialog`
- WHEN those source files are grepped for `<input type="text"` and
  the legacy `.form-group` / `.input` / `.field-label` /
  `.small-label` local classes
- THEN the grep returns zero matches for `<input type="text"` outside
  the Input primitive
- AND the legacy local classes no longer act as the input surface on
  those files

#### Requirement: checkboxes and radios themed via DaisyUI

Plain `<input type="checkbox">` controls in `ProductForm`, `LotForm`,
`ReportsPage`, and any other migrated form MUST render through
DaisyUI `checkbox checkbox-primary checkbox-sm`. Plain `<input
type="radio">` controls in `UnitReviewPage` (kind integer / decimal)
and any other migrated location MUST render through DaisyUI `radio
radio-primary radio-sm`. The native `<input>` element MUST remain in
the DOM for form semantics; only the visual surface changes.

#### Scenario: checkbox and radio controls keep form semantics

- GIVEN the user opens `ProductForm.svelte` after migration
- WHEN the "set as primary barcode" checkbox is rendered
- THEN the visible surface is the DaisyUI-themed checkbox
- AND the underlying `<input type="checkbox">` element remains in the
  DOM
- AND toggling the visible surface toggles the underlying input

#### Requirement: details / summary panels in UnitReviewPage use DaisyUI dropdown

The `<details>` / `<summary>` "Map to preset" and "Keep as custom"
panels in `UnitReviewPage.svelte` MUST be replaced by DaisyUI
`dropdown` / `dropdown-content` surfaces. The replacement MUST
preserve the underlying toggle behaviour and MUST use proper ARIA
roles instead of the `details` / `summary` semantics.

#### Scenario: UnitReviewPage dropdown panels keep their toggle behaviour

- GIVEN the user opens `UnitReviewPage.svelte`
- WHEN the user activates the "Map to preset" or "Keep as custom"
  panel header
- THEN the corresponding content panel expands or collapses
- AND no `<details>` or `<summary>` element remains in the DOM

#### Requirement: DatePicker popover restyled with DaisyUI dropdown / popover classes

The popover surface of `DatePicker.svelte` MUST be restyled using
DaisyUI `dropdown` / `dropdown-content` and the `popover` class. The
restyle MUST NOT change any of the keyboard ergonomics, ISO bind
contract, year-range enforcement, validation rules, or
`clearable={true | false}` semantics carried over from the canonical
`Custom date picker` requirement.

#### Scenario: DatePicker visual surface uses DaisyUI classes while behaviour is preserved

- GIVEN `DatePicker.svelte` after migration
- WHEN the popover is rendered
- THEN the popover root carries DaisyUI `dropdown` / `dropdown-content`
  classes (and any project theme overrides)
- AND every scenario in the canonical `Custom date picker`
  requirement continues to pass without modification

#### Requirement: CategoryPicker popover restyled with DaisyUI dropdown / popover classes

The popover surface of `inputs/CategoryPicker.svelte` MUST be
restyled using DaisyUI `dropdown` / `dropdown-content` and the
`popover` class. The restyle MUST preserve every keyboard contract
(arrow keys, Enter, Escape, Backspace) and every chip / pseudo-row
semantic carried over from the canonical `category picker keyboard
ergonomics` and `category picker Uncategorized pseudo-row is
selectable and rendered distinctly` requirements.

#### Scenario: CategoryPicker keyboard contract is preserved

- GIVEN `CategoryPicker.svelte` after migration
- WHEN the user navigates the popover with `ArrowDown`, `Enter`,
  `Escape`, and `Backspace`
- THEN `aria-activedescendant` moves through the result list and
  clamps at the ends, `Enter` toggles the active result, `Escape`
  closes the popover without committing typed text, and `Backspace`
  on an empty input removes the last chip
- AND the `Uncategorized` pseudo-row continues to be selectable with
  a distinct style

#### Requirement: modal dialogs migrate to native dialog class modal

Every modal surface — `MoveStockModal`, `RegisterExitModal`,
`AdjustCountModal`, `ArchiveLotDialog`, `ResolveQuantityDialog`, plus
the inline product / lot / quick-create overlays inside
`DashboardPage` — MUST be migrated to the native `<dialog
class="modal">` surface driven by `dialog.showModal()`. The
`Modal.svelte` primitive is the only canonical shell for these
surfaces in v1.

#### Scenario: every migrated modal opens, closes, traps focus, and restores focus

- GIVEN the user activates the trigger for any migrated modal
- WHEN the modal opens
- THEN focus is trapped inside the modal
- AND pressing `Escape` closes the modal
- AND focus returns to the trigger element after close
- AND no legacy `.modal-overlay` / `.modal-box` / `.modal-box-wide` /
  `.modal-header` / `.modal-body` / `.modal-footer` / `.modal-close`
  / `.modal-loading` local class remains as the shell on the migrated
  files

#### Scenario: existing Escape / cancel semantics are preserved

- GIVEN any migrated modal that previously cancelled in-progress edits
  on `Escape`
- WHEN the user presses `Escape` while the modal is open
- THEN the in-progress edits are discarded exactly as they were
  before the redesign

#### Requirement: app shell navigation uses DaisyUI navbar

The top navigation in `App.svelte` MUST render through DaisyUI
`navbar` with `navbar-start`, `navbar-center`, and `navbar-end`
slots. Narrow-viewport collapse MUST go through DaisyUI `dropdown`
so the navigation remains reachable on small widths.

#### Scenario: app shell renders the DaisyUI navbar

- GIVEN the user launches the application on a wide viewport
- WHEN the app shell renders
- THEN the brand sits in `navbar-start`, the tab buttons sit in
  `navbar-center`, and any overflow / utility actions (including the
  Configuration tab) sit in `navbar-end`
- AND no `.nav` / `.nav-btn` / `.nav-brand` local class remains as
  the navigation surface

#### Scenario: navbar collapses to a dropdown on narrow viewports

- GIVEN the user launches the application at a viewport width of
  720px or less
- WHEN the app shell renders
- THEN the tab buttons collapse behind a dropdown trigger
- AND every tab remains reachable via keyboard
- AND the active tab is still obvious

#### Requirement: no native OS-styled control surfaces remain inside the app chrome

After v1 lands, every main screen MUST show zero OS-styled controls
inside the app's visual chrome. The check is verifiable by a manual
screenshot pass on every main surface in both `caduxo-light` and
`dark`: the locale selector, every form input, every checkbox / radio
/ toggle, the date and category pickers, every modal, every list /
picker surface, and every tooltip must render through Caduxo's
visual vocabulary.

#### Scenario: screenshot pass finds zero OS-styled controls

- GIVEN v1 is shipped
- WHEN every main surface is screenshotted in `caduxo-light` and
  `dark` and the screenshots are reviewed for operating-system
  chrome
- THEN no Windows / macOS / GTK dropdown chrome, native checkbox or
  radio surface, native input border, or native tooltip appears
  inside the app's visual chrome

### Capability: Visual surface redesign

#### Requirement: dashboard redesign through shared primitives

The Dashboard MUST be migrated to use the shared primitives
(`Card`, `Badge`, `Button`, `Table`, `EmptyState`, `LoadingState`,
`Alert`, `Modal`) for every visible surface. The migration MUST
preserve every existing canonical scenario under `Capability:
Dashboard` (sections align with the matching filter buttons,
category filter ANY-of semantics, urgency-card bucket counts,
quick-filter row predicates, no separate movement widget in v1,
etc.) and MUST preserve scan / search, urgency filters, row actions,
and the existing modal triggers.

#### Scenario: dashboard preserves every canonical Dashboard scenario

- GIVEN the Dashboard is migrated
- WHEN every scenario under the canonical `Capability: Dashboard`
  is exercised after migration
- THEN each scenario continues to pass without modification

#### Scenario: dashboard surfaces render through shared primitives

- GIVEN the migration has finished for `DashboardPage.svelte`
- WHEN the file is grepped for the legacy `.urgency-card`,
  `.urgency-badge`, `.status-*`, `.lot-table`, `.error-banner`,
  `.tab-btn`, `.detail-tabs`, `.tab-content`, `.loading`,
  `.loading-row`, `.scan-spinner` local classes
- THEN the grep returns zero matches as the active surface class
  on that file

#### Requirement: forms redesign through shared primitives

`ProductForm.svelte`, `LotForm.svelte`, `ConfigurationPage.svelte`,
`ReportsPage.svelte` filters, `BackupRestorePage.svelte`, and
`CsvImportPage.svelte` MUST migrate to the shared `Input`, `Select`,
`Toggle`, `Button`, `Alert`, and field-label primitives. The
migration MUST preserve every existing canonical validation
behaviour, every existing canonical i18n flow, every existing
canonical locale selector behaviour, and every existing canonical
required-marker semantics.

#### Scenario: every migrated form preserves its canonical contract

- GIVEN the form migrations are complete
- WHEN each migrated form is exercised against its canonical
  scenarios (e.g., `product picks a preset unit`, `product creates a
  custom unit inline`, `lot creation with auto-generated batch`,
  `locale selector update rolls back on IPC failure`)
- THEN each scenario continues to pass without modification

#### Requirement: data tables redesign through shared primitives

The dashboard lot table, `LotMovementsPanel`, `ReportsPage`,
`CsvImportPage` preview, `StoresPage`, `BackupRestorePage` info
lists, `ConfigurationPage` info lists, and `CalendarPage` day-detail
panel MUST migrate to the shared `Table.svelte` primitive. The
migration MUST preserve every existing canonical data contract: no
column is lost, no sort / filter behaviour is altered, the CSV
import preview remains readable, the calendar day-detail panel
continues to render one row per active lot with the same columns.

#### Scenario: every migrated table preserves its canonical contract

- GIVEN the table migrations are complete
- WHEN each migrated table is exercised against its canonical
  scenarios
- THEN each scenario continues to pass without modification

#### Requirement: empty and loading states use shared primitives

Every migrated empty and loading surface MUST render through
`EmptyState.svelte` and `LoadingState.svelte` instead of the
existing scattered `<p class="loading">`, `<p class="loading-row">`,
`<span class="loading-msg">`, `<p class="empty-hint">` patterns.

#### Scenario: every migrated empty and loading surface uses the shared primitives

- GIVEN the migration is complete
- WHEN every migrated surface is grepped for the legacy
  `.loading`, `.loading-row`, `.loading-msg`, `.empty-state`, and
  `.empty-hint` local rules as the active empty / loading surface
- THEN the grep returns zero matches on those files

#### Requirement: responsive pass at 1024 / 720 / 480 px

The migrated UI MUST remain usable and unbroken at viewport widths
of 1024, 720, and 480 pixels. The pass MUST cover the navbar
collapse (see the navbar requirement above), wide-table horizontal
scroll wrappers, modal fit on narrow screens, and the calendar and
forms on narrow widths.

#### Scenario: responsive pass has no horizontal page overflow

- GIVEN the user resizes the viewport to 1024, 720, and 480 pixels in
  turn
- WHEN each main surface is rendered
- THEN no horizontal page overflow appears (intentional table
  wrappers excepted)
- AND every primary action remains reachable
- AND every modal fits within the viewport
- AND the navbar remains usable

### Capability: Motion and effects

#### Requirement: motion token inventory with named durations and easings

A shared motion-token inventory MUST exist and MUST be the single
source of truth for every animated surface in v1. The inventory MUST
define at minimum the durations and easings used by: default hover /
focus transitions, modal fade + scale, dropdown / popover slide-in,
card hover lift, button press feedback, skeleton shimmer, urgency
pulse, and backdrop blur. Every component animation MUST source its
duration and easing from this inventory rather than from a hand-
written literal.

#### Scenario: every animated surface resolves through the motion-token inventory

- GIVEN the motion-token inventory is defined
- WHEN the codebase is grepped for `transition: ... 0\.[0-9]+s` or
  `animation: ... 0\.[0-9]+s` literals on migrated files
- THEN the grep returns zero matches as the source of an animation
  on migrated files
- AND every animated surface resolves its duration and easing
  through a token

#### Requirement: prefers-reduced-motion guard on every animated surface

Every animated surface in v1 MUST be wrapped in
`@media (prefers-reduced-motion: no-preference) { ... }` (or its
Tailwind equivalent: the `motion-safe:` variant). When the host
reports `prefers-reduced-motion: reduce`, the surface MUST still
work — it MUST just not animate. The guard applies to the default
hover / focus transitions, modal show / hide, dropdown / popover
slide-in, card hover lift, button press feedback, skeleton shimmer,
urgency pulse, and backdrop blur.

#### Scenario: reduced-motion users reach the same end state without animation

- GIVEN the host reports `prefers-reduced-motion: reduce`
- WHEN the user opens every modal, every dropdown / popover, every
  skeleton-loading surface, and every hover / focus surface
- THEN no animation runs on that surface
- AND the same end state is reached as for a non-reduced-motion user

#### Requirement: modal fade and scale motion

Modal show / hide MUST render a fade + scale transition (~150 ms)
sourced from the motion-token inventory. The transition MUST be
disabled by the `prefers-reduced-motion` guard.

#### Scenario: modal show / hide is animated when motion is allowed

- GIVEN the host reports `prefers-reduced-motion: no-preference`
- WHEN the user opens or closes any migrated modal
- THEN a fade + scale transition runs once
- AND its duration and easing come from the motion-token inventory

#### Requirement: dropdown and popover slide-in motion

DaisyUI `dropdown` / `dropdown-content` open / close MUST render a
subtle slide-in (~150 ms) sourced from the motion-token inventory.
The transition MUST be disabled by the `prefers-reduced-motion`
guard.

#### Scenario: dropdown / popover slide-in is animated when motion is allowed

- GIVEN the host reports `prefers-reduced-motion: no-preference`
- WHEN the user opens any migrated dropdown / popover
- THEN a slide-in transition runs once
- AND its duration and easing come from the motion-token inventory

#### Requirement: card hover lift motion

Urgency cards and report type cards MUST render a card hover lift
(`hover:-translate-y-0.5 hover:shadow-lg`) sourced from the
motion-token inventory. The translation MUST be disabled by the
`prefers-reduced-motion` guard; the shadow MAY remain.

#### Scenario: card hover lift runs only when motion is allowed

- GIVEN the user hovers an urgency or report-type card
- WHEN the host reports `prefers-reduced-motion: no-preference`
- THEN the card lifts and the shadow increases
- WHEN the host reports `prefers-reduced-motion: reduce`
- THEN the card does not translate but the hover affordance remains
  visually distinct

#### Requirement: button press feedback motion

Buttons MUST render an `active:translate-y-px` or `active:scale-95`
press feedback sourced from the motion-token inventory. The
translation MUST be disabled by the `prefers-reduced-motion` guard.

#### Scenario: button press feedback runs only when motion is allowed

- GIVEN the user activates a primary button
- WHEN the host reports `prefers-reduced-motion: no-preference`
- THEN a tactile press feedback runs once
- WHEN the host reports `prefers-reduced-motion: reduce`
- THEN the press feedback does not run

#### Requirement: skeleton shimmer during initial load with reduced-motion fallback

Tables on the Dashboard, Reports, Stores, and Products screens MUST
render a DaisyUI `skeleton` shimmer during the initial load. The
shimmer MUST be disabled by the `prefers-reduced-motion` guard; the
skeleton MUST remain as a static grey block under reduced motion.

#### Scenario: skeleton shimmer falls back to static on reduced motion

- GIVEN the host reports `prefers-reduced-motion: reduce`
- WHEN the Dashboard, Reports, Stores, or Products page is loading
- THEN the table rows render as a static skeleton (no shimmer
  animation)
- AND the same data lands once loading completes

#### Requirement: backdrop blur on modal backdrops

Modal backdrops MUST support a subtle `backdrop-blur-sm` on capable
platforms. The effect MUST be disabled by the `prefers-reduced-motion`
guard.

#### Scenario: backdrop blur is applied when motion is allowed

- GIVEN the user opens any migrated modal
- WHEN the host reports `prefers-reduced-motion: no-preference`
- AND the host platform supports `backdrop-filter`
- THEN the modal backdrop renders with a subtle blur
- WHEN the host reports `prefers-reduced-motion: reduce`
- THEN the backdrop blur is not applied

#### Requirement: urgency pulse is allowed only on the expired variant

A slow pulse (1.5–2 s period, opacity-only, no scale or translation)
MUST be approved for the `expired` urgency badge and the leading
status dot inside the `.urgency-card-expired` surface. The pulse
MUST NOT be applied to `today`, `alert`, `soon`, or `normal`. The
pulse MUST be disabled by the `prefers-reduced-motion` guard, MUST
NOT be hover-triggered, and MUST be visually subtle.

#### Scenario: only the expired variant pulses, and only when motion is allowed

- GIVEN the dashboard renders urgency badges for an expired lot, a
  today lot, and a soon lot
- WHEN the host reports `prefers-reduced-motion: no-preference`
- THEN the expired badge (and only the expired badge) renders a
  pulse animation
- AND no other urgency badge pulses
- WHEN the host reports `prefers-reduced-motion: reduce`
- THEN no urgency badge pulses
- AND every badge remains visually distinct through colour, icon,
  and text

### Capability: i18n preservation in visual layer

#### Requirement: every new visible string flows through typesafe-i18n

Every new visible string introduced by this change (theme switcher
labels, empty-state titles and bodies, loading-state text, alert
copy, motion copy, tooltip text, configuration section titles
introduced by the redesign) MUST be added to both
`src/i18n/en/index.ts` and `src/i18n/es/index.ts` and MUST be
regenerated by `npm run i18n:generate`. The predev and prebuild
hooks MUST continue to enforce the regeneration gate. No new
visible string MAY be hard-coded in a Svelte template or component
file.

#### Scenario: a new visible string lands in EN and ES before it ships

- GIVEN a new visible string is introduced (for example the empty
  state of the migrated CSV import preview)
- WHEN the implementation is prepared for review
- THEN the string exists under a key in `src/i18n/en/index.ts`
- AND the same key exists under the equivalent Spanish text in
  `src/i18n/es/index.ts`
- AND `npm run i18n:generate` regenerates the catalogue without
  diagnostics
- AND `npm run check` passes
- AND no hard-coded copy of the new string exists in the migrated
  Svelte file

#### Scenario: the canonical internationalisation scenarios still pass

- GIVEN the redesign is in flight
- WHEN the canonical scenarios under `Capability: Internationalisation`
  are exercised (missing key fails the build, locale switch
  re-renders bound text, no hard-coded user-visible strings, EN
  fallback, persisted value wins, closest-supported mapping, etc.)
- THEN each scenario continues to pass without modification

#### Requirement: theme labels and motion copy are localised

The theme switcher MUST show theme names in the active locale.
Motion copy (for example any motion-related alert copy or any
loading-state wording tied to a motion affordance) MUST flow through
the translation tree and MUST be regenerated by `npm run
i18n:generate`.

#### Scenario: theme names render in the active locale

- GIVEN the active locale is `es`
- WHEN the user opens the theme switcher
- THEN each theme label is rendered in Spanish
- WHEN the active locale is `en`
- THEN each theme label is rendered in English

#### Requirement: empty / loading / error copy is localised in EN and ES

Every empty, loading, and error copy introduced by the redesign
MUST be present in both `src/i18n/en/index.ts` and
`src/i18n/es/index.ts`. Spanish strings MUST have enough room in the
layout that they do not force awkward wrapping at common desktop
widths.

#### Scenario: Spanish empty / loading / error copy fits without awkward wrapping

- GIVEN the active locale is `es`
- WHEN the user renders the migrated empty, loading, and error
  surfaces on a 1280 px viewport
- THEN no surface forces the Spanish copy to wrap mid-word or to
  overflow its container

### Capability: Accessibility preservation in visual layer

#### Requirement: visible focus rings preserved or improved

Every migrated interactive surface MUST render a visible focus ring
sourced from DaisyUI's default outline ring (or an equivalent
project token). The focus ring MUST remain visible in both
`caduxo-light` and `dark`.

#### Scenario: every migrated interactive surface shows a visible focus ring

- GIVEN the user tabs through every migrated interactive surface in
  `caduxo-light` and `dark`
- WHEN each surface receives keyboard focus
- THEN a visible focus ring renders against the surface background

#### Requirement: keyboard reachability for all primitives

Every migrated primitive MUST be reachable by keyboard in source
order. No migrated primitive MAY rely on hover alone for activation.

#### Scenario: every migrated primitive is reachable by keyboard

- GIVEN the user navigates with `Tab` and `Shift+Tab` from the app
  shell into every migrated page
- WHEN the user reaches the last interactive element on the page
- THEN every interactive surface has been focusable along the way
- AND no surface has required pointer-only activation

#### Requirement: Escape and click-outside behaviour for modals and popovers

Every migrated modal and popover MUST close on `Escape`. Every
migrated modal and popover that previously closed on click-outside
MUST continue to close on click-outside. The migrated
`CategoryPicker` popover MUST close on `Escape` without committing
the typed query (the input keeps the typed text).

#### Scenario: modal and popover Escape closes without committing

- GIVEN any migrated modal or popover is open
- WHEN the user presses `Escape`
- THEN the surface closes
- AND any in-progress edit that should be cancelled is cancelled
- AND any typed text in the popover input remains visible

#### Requirement: dialog focus restoration to trigger element

Every migrated modal MUST restore focus to the element that opened
it after the modal closes. The focus restoration MUST be a no-op if
the trigger element is no longer in the DOM at the time of close.

#### Scenario: focus returns to the trigger after a modal closes

- GIVEN the user opens a migrated modal from a specific trigger
  button
- AND the modal closes via `Escape`, the close button, or
  click-outside
- WHEN focus is observed
- THEN it is on the same trigger button

#### Requirement: WCAG-readable contrast in every shipped theme

Body text, headings, inputs, badges, alerts, and primary button
labels MUST meet WCAG-AA contrast in both `caduxo-light` and `dark`
on every main surface. See the `readable contrast and unbroken
layouts in every shipped theme` requirement for the manual check
gating.

#### Scenario: contrast pass covers body, heading, input, badge, alert, and button surfaces

- GIVEN the active theme is `caduxo-light` or `dark`
- WHEN the manual contrast pass on the main surfaces is performed
- THEN no body / heading / input / badge / alert / button label
  falls below WCAG-AA contrast

#### Requirement: status communicated through multiple channels

Status, urgency, and feedback MUST be communicated through colour,
icon, and text together. Colour MUST NOT be the only signal. This
applies to urgency badges, alerts, loading vs loaded vs empty vs
error states, and motion affordances.

#### Scenario: no status uses colour alone

- GIVEN the user views an urgency badge, alert, loading / loaded /
  empty / error state, or motion affordance
- WHEN the surface is rendered with a desaturated palette (for
  example the browser's high-contrast or a colour-blind simulator)
- THEN the user can still tell the status apart from other statuses
  through icon, text, or both

#### Requirement: DatePicker and CategoryPicker keyboard contracts are preserved

The keyboard contracts for `DatePicker.svelte` (Tab, Escape, Enter,
arrow keys, PageUp / PageDown, Shift+PageUp / Shift+PageDown, Today
shortcut) and `inputs/CategoryPicker.svelte` (arrow keys, Enter,
Escape, Backspace, `aria-activedescendant`, `Uncategorized`
pseudo-row) MUST be preserved exactly across the visual migration.
Restyling the visual surface MUST NOT introduce a focus trap,
remove a keyboard shortcut, or change any of the ARIA semantics.

#### Scenario: picker keyboard contracts pass after migration

- GIVEN the visual migration of `DatePicker.svelte` and
  `inputs/CategoryPicker.svelte` is complete
- WHEN every canonical scenario under `Capability: Date input > Custom
  date picker` and every CategoryPicker keyboard scenario under
  `Capability: Categories > editable category list` is exercised
- THEN each scenario continues to pass without modification

### Capability: Chained PR delivery

#### Requirement: one phase per chained PR by default

The implementation is delivered as chained PRs. Each of the 13
phases in `docs/daisyui-redesign-plan.md` becomes a work unit on
the `feat/daisyui-redesign` branch and is delivered as a single
chained PR by default. When a phase's forecast crosses the 400-line
review budget, the tasks phase splits that phase further (by page,
by primitive, or by primitive-batch) and delivers it as a chain of
smaller PRs that all reference the same plan doc. No chained PR
mixes backend changes with visual changes, and no chained PR mixes
two unrelated surfaces.

#### Scenario: each chained PR has its own review

- GIVEN the implementation branch is `feat/daisyui-redesign`
- WHEN each chained PR is opened
- THEN the PR description names the phase and references the plan
  doc
- AND the PR diff is reviewable in isolation
- AND no PR mixes two unrelated surfaces

#### Requirement: 400-line review budget enforced per PR

Each chained PR MUST stay within the 400-line review budget unless
an explicit `size: exception` is approved by the user via
`ask-on-risk`. The forecast is performed by the tasks phase; the
budget is enforced at apply time.

#### Scenario: an oversized phase is split before apply

- GIVEN a phase's tasks forecast exceeds 400 changed lines
- WHEN the tasks phase declares the phase
- THEN the phase is split by page or by primitive into smaller
  chained PRs
- OR the user is asked explicitly via `ask-on-risk` before any
  oversized PR is opened

#### Requirement: foundation-first delivery order

The first chained PR MUST be the foundation: dependencies + theme
config + design tokens + shared primitives, with no page migration
yet. The foundation phase MUST verify `npm run check`, `npm run
build`, and a manual launch in the Tauri / dev browser shell before
any subsequent chained PR may begin.

#### Scenario: foundation phase verifies build and manual launch

- GIVEN the foundation PR is opened
- WHEN the PR is reviewed
- THEN `npm run check` passes
- AND `npm run build` passes
- AND a manual Tauri / dev browser launch shows no widespread
  preflight breakage
- AND the diff is limited to dependencies, theme config, design
  tokens, and shared primitives

## MODIFIED Requirements

### Capability: Date input

#### Requirement: Custom date picker

The system MUST provide an in-house Svelte custom date picker (no
native `<input type="date">`) for every expiry date and report
date-range field, and the picker MUST be the single source of truth
for date input across the app.

The picker MUST bind ISO `YYYY-MM-DD` strings in and out, MUST
reject any year outside the inclusive range `[1900-01-01,
2100-12-31]`, and MUST support both a typed-text manual entry path
and a calendar popover path that share the same bound value. The
picker MUST render no `<input type="date">` element in the DOM.

The picker MUST expose a `clearable` prop (default `true`); when
`clearable={false}` the picker MUST NOT render any clear affordance
and MUST NOT silently commit an empty value. The host (`LotForm`
for the required expiry date) is responsible for the `required` JS
guard; the picker preserves the bound value on empty input when
`clearable={false}` and surfaces a visual invalid state instead.

The picker MUST support basic accessible keyboard ergonomics: Tab
enters and leaves the trigger without trapping focus inside the
popover; Escape closes the popover without committing a calendar
selection and without rewriting typed text; Enter on a focused day
cell emits that day's ISO date and closes the popover. Arrow keys
MUST move the focused day by ±1 (Left/Right) or ±7 (Up/Down) days;
PageUp/PageDown MUST move by ±1 month; Shift+PageUp/Shift+PageDown
MUST move by ±1 year; every move MUST be clamped to the configured
year range.

Manual `YYYY-MM-DD` text input MUST visually validate on blur (red
border + helper text for invalid shape, invalid calendar date, or
out-of-range year); the bound value MUST NOT update while the text
is invalid and the typed text MUST remain visible in the input
element (no silent rewrite).

The picker popover surface MUST render through DaisyUI `dropdown`
/ `dropdown-content` and the `popover` class as defined in the
`DatePicker popover restyled with DaisyUI dropdown / popover
classes` requirement. The visual restyle MUST NOT introduce a focus
trap, MUST NOT remove a keyboard shortcut, and MUST NOT change any
ARIA semantics.

(Previously: the requirement governed only the in-house Svelte
behaviour, the ISO bind contract, the year-range enforcement, the
`clearable` semantics, and the keyboard ergonomics. It did not
constrain the visual surface; the redesign pins the popover surface
to DaisyUI `dropdown` / `dropdown-content` and `popover` while
preserving every existing keyboard, validation, and ISO contract.)

#### Scenario: picker is in-house Svelte, not native

- GIVEN any date field in the app (LotForm expiry date, Reports
  `dateFrom`, Reports `dateTo`)
- WHEN `grep -R 'type="date"' src/` is executed after this slice
  lands
- THEN the command returns zero matches
- AND each field is rendered by the in-house `DatePicker.svelte`
  Svelte component composed around `CalendarMonth.svelte`

#### Scenario: ISO YYYY-MM-DD in/out with year range enforcement

- GIVEN the user opens the picker on any date field
- WHEN the user selects a day in any month of any year between
  1900 and 2100 inclusive (via the calendar popover or manual
  entry)
- THEN the picker emits the selected day as an ISO `YYYY-MM-DD`
  string
- AND the bound value is updated to that string
- AND the text input displays the selected ISO date

#### Scenario: selection closes the popover and commits

- GIVEN the picker popover is open and the user is focused on a day
  cell
- WHEN the user clicks or presses Enter on a day cell
- THEN the popover closes immediately
- AND the selected `YYYY-MM-DD` is emitted and committed to the host
- AND the text input displays the selected ISO date

#### Scenario: manual YYYY-MM-DD text input validates without silent rewrite

- GIVEN the picker text input is focused
- WHEN the user types any of `2026-13-40`, `2026-02-30`,
  `1899-12-31`, or `2101-01-01`
- AND blurs the input
- THEN the input displays a visual invalid state (red border +
  helper text)
- AND the bound value is NOT updated
- AND the typed text remains visible in the input element (no
  silent rewrite)

#### Scenario: clearable=false hides the clear icon and prevents silent empty commit

- GIVEN the picker is mounted with `clearable={false}` (LotForm
  expiry date)
- WHEN the picker is rendered
- THEN no `×` clear icon is present in the DOM
- AND the user cannot clear the bound value through the picker
- WHEN the user deletes the typed text down to empty and blurs
- THEN the picker shows a visual invalid state
- AND the bound value is NOT silently committed to `""`

#### Scenario: clearable=true exposes the clear icon

- GIVEN the picker is mounted with the default `clearable={true}`
  (Reports `dateFrom`, `dateTo`)
- AND the bound value is non-empty
- WHEN the picker is rendered
- THEN a `×` clear icon is visible in the trigger
- AND clicking `×` clears the bound value to `""` and closes the
  popover

#### Scenario: basic accessible keyboard ergonomics

- GIVEN the picker trigger is focused
- WHEN the user presses Tab
- THEN focus moves through the trigger and the calendar-icon button
  in source order
- WHEN the user presses Tab again
- THEN focus leaves the picker (no focus trap inside the popover)
- GIVEN the popover is open
- WHEN the user presses Escape
- THEN the popover closes without committing a calendar selection
- AND any typed text remains visible (still subject to
  blur-validation rules)
- GIVEN a day cell is focused inside the popover
- WHEN the user presses Enter
- THEN that day's ISO date is emitted and the popover closes
- AND ArrowLeft/ArrowRight move the focused day by ±1 day
- AND ArrowUp/ArrowDown move the focused day by ±7 days
- AND PageUp/PageDown move the focused day by ±1 month
- AND Shift+PageUp/Shift+PageDown move the focused day by ±1 year
- AND every move is clamped to `[1900-01-01, 2100-12-31]`

#### Scenario: Today shortcut inside popover

- GIVEN the picker popover is open
- AND today is within `[minDate, maxDate]`
- WHEN the user clicks the `Today` button in the popover footer
- THEN the bound value is set to today's ISO `YYYY-MM-DD`
- AND the popover closes

#### Scenario: popover surface carries DaisyUI dropdown classes after migration

- GIVEN the visual migration of `DatePicker.svelte` has landed
- WHEN the picker popover is rendered
- THEN the popover root carries DaisyUI `dropdown` /
  `dropdown-content` classes (and any project theme overrides)
- AND every scenario above continues to pass without modification

### Capability: Calendar

#### Requirement: Calendar tab

The system MUST provide a top-level main-navigation Calendar tab
(added between Products and Reports) that opens the current month
with today highlighted and selected, displays per-day lot
expirations as dot badges, and lists the lots and products expiring
on a selected day in a day-detail panel.

The Calendar tab MUST reuse the same `CalendarMonth.svelte`
primitive used by the date picker popover so the day grid, year
picker, month picker, visual styling, and keyboard surface are
implemented once.
The Calendar tab MUST source its expiration data from the existing
`list_dashboard_lots` Tauri command (no new backend command in this
slice); the day-bucket map MUST be computed client-side from the
returned active lots. Month navigation MUST NOT trigger a refetch
in this slice.

Clicking a day MUST set the selected date and reveal a day-detail
panel that lists every active lot whose `expiry_date` equals the
selected date, with columns for product, quantity, unit, store,
location, days remaining, and status. Clicking a lot row MUST open
the existing lot edit flow used elsewhere in the app. Clicking a
day with no expirations MUST still select it and show a `No
expirations on YYYY-MM-DD` placeholder.

The Calendar tab MUST inherit the full keyboard surface of the
calendar primitive (Tab, Esc, Enter, arrow keys, PageUp/Down,
Shift+PageUp/Shift+PageDown) with a tab order of prev-month
chevron → month label → year chip → day grid → next-month chevron
→ day-detail rows.

The calendar grid, day cells, today ring, selection ring, badge
dots, and day-detail panel MUST render through the shared DaisyUI
themed primitives as defined in the visual surface redesign and
native control replacement capabilities. The visual restyle MUST
NOT change any domain logic carried by `CalendarMonth.svelte`
(year picker, decade navigation, badge dots, out-of-range guards)
and MUST NOT remove any keyboard shortcut.

(Previously: the requirement governed the calendar's data source,
the keyboard surface, the day-detail panel columns, the lot row
edit flow, and the inherited keyboard surface from
`CalendarMonth.svelte`. It did not constrain the visual surface;
the redesign pins the day cells, today ring, selection ring, badge
dots, and day-detail panel to the shared DaisyUI themed primitives
while preserving every existing keyboard, data, and domain logic
contract. It did not yet reference a month picker and used
`ariaCycleMonth` for the month label's accessible name.)

#### Scenario: opens at current month with today highlighted and selected

- GIVEN the user clicks the Calendar tab in main navigation
- WHEN the page renders
- THEN the calendar grid displays the current month (`viewYear`
  and `viewMonth` derived from `new Date()`)
- AND today is visually highlighted (distinct from the selected
  highlight)
- AND today is the selected date (`selectedDate = today`)

#### Scenario: per-day expirations visible as dot badges

- GIVEN active lots exist in the local database with various
  `expiry_date` values
- WHEN the Calendar tab renders
- THEN each day cell with at least one active lot displays a small
  dot under the day number
- AND the dot corresponds to the count of active lots whose
  `expiry_date` equals that day
- AND the count is also shown in the day-detail panel for the
  selected date

#### Scenario: day-detail panel lists lots and products

- GIVEN a day is selected on the Calendar tab
- WHEN the day-detail panel renders
- THEN the panel lists every active lot whose `expiry_date` equals
  the selected date
- AND each row shows product, quantity, unit, store, location, days
  remaining, and status

#### Scenario: empty day shows the placeholder message

- GIVEN a day is selected on the Calendar tab
- AND no active lot has an `expiry_date` equal to that day
- WHEN the day-detail panel renders
- THEN the panel shows `No expirations on YYYY-MM-DD` (or an
  equivalent English placeholder that names the date)

#### Scenario: lot row opens the existing lot edit flow

- GIVEN the day-detail panel shows one or more lot rows
- WHEN the user clicks a lot row
- THEN the existing lot edit overlay opens for that lot (the same
  pattern used by `DashboardPage`)
- AND no new edit-flow primitive is introduced

#### Scenario: data source is the existing list_dashboard_lots command

- GIVEN the user opens the Calendar tab
- WHEN the page mounts
- THEN the page calls `list_dashboard_lots({ store_id: null,
  location_id: null, preset: null, urgency: null })` exactly once
- AND the day-bucket map is computed client-side from the returned
  active lots
- AND month navigation does NOT trigger a refetch in this slice

#### Scenario: keyboard surface inherited from the calendar primitive

- GIVEN the Calendar tab is rendered
- WHEN the user navigates with Tab, Esc, Enter, arrow keys, and
  PageUp/Down
- THEN focus traverses prev-month chevron → month label → year
  chip → day grid → next-month chevron → day-detail rows
- AND arrow keys / PageUp / PageDown / Shift+PageUp /
  Shift+PageDown move the focused day with the same clamping rules
  as the picker
- AND Enter on a focused day cell emits the ISO date and updates
  `selectedDate`
- AND `Enter` / `Space` on the month label opens the month picker
  overlay without changing `viewMonth`

#### Scenario: calendar surfaces render through the shared primitives after migration

- GIVEN the visual migration of `CalendarPage.svelte` and
  `CalendarMonth.svelte` has landed
- WHEN the Calendar tab is rendered
- THEN the day grid, day cells, today ring, selection ring, badge
  dots, and day-detail panel render through the shared DaisyUI
  themed primitives
- AND every scenario above continues to pass without modification
- AND the year picker, decade navigation, badge dot count, and
  out-of-range guards carried by `CalendarMonth.svelte` continue to
  work exactly as before

#### Scenario: clicking the month label opens a month picker grid

- GIVEN the Calendar tab is rendered
- AND the month picker overlay is not currently open
- WHEN the user clicks the month label in the calendar header
- THEN a month picker overlay opens anchored under the header
- AND the overlay lists all 12 months in a 3-column × 4-row grid
- AND the chip for the currently viewed month is rendered with the
  selected visual treatment
- AND the year shown in the overlay's header matches `viewYear`
- AND no `monthChange` event has been dispatched yet

### Requirement: Calendar month picker

`CalendarMonth.svelte` MUST expose a month picker overlay that mirrors the existing year picker pattern, so users can jump to any month in one click instead of clicking the next-month chevron repeatedly.

The month picker MUST be triggered by the month label button in the calendar header (click, `Enter`, or `Space`) and MUST render anchored under the same header as the year picker. While the month picker is open, the underlying day grid, weekday header, badge dots, today highlight, and selection ring MUST remain rendered in their previous state (they are not unmounted behind the overlay).

The month picker header MUST show the current `viewYear` as a read-only label with a previous-year chevron and a next-year chevron on either side. The two chevrons MUST adjust a local `pickerYear` value used only inside the overlay and MUST NOT dispatch `monthChange` or `viewYearChange`; clicking a month chip is what commits a year change.

The month picker body MUST render a 3-column × 4-row grid of 12 month chips, one chip per calendar month (January through December), labeled with the localized month name from the existing `MONTH_NAMES` array.

A month chip for a month `m` in `pickerYear` MUST be considered **in-range** when there exists at least one day in that `(pickerYear, m)` pair whose ISO date falls within `[minDate, maxDate]`, and **out-of-range** otherwise. Out-of-range chips MUST render with the `month-disabled` visual treatment and MUST be non-interactive (no click, no keyboard activation). The month picker MUST NOT collapse, group, or hide a row that contains disabled chips; the row layout is preserved and only individual chips are dimmed.

The chip for the month equal to the currently viewed `viewMonth` (in `viewYear`) MUST render with the `month-selected` visual treatment. No other month in the grid is highlighted as selected, including the current calendar month if it differs from `viewMonth`.

Clicking an enabled month chip MUST dispatch a single `monthChange` event with `{ year: pickerYear, month: m }` and MUST close the month picker overlay. The picker MUST NOT stay open for a follow-up selection; the click count is one.

The year chip button (the existing trigger that opens the year picker) MUST remain clickable while the month picker is open and MUST continue to open the year picker as before. Opening the year picker while the month picker is open MUST close the month picker. Opening the month picker while the year picker is open MUST close the year picker. The two overlays MUST NEVER be open at the same time.

The keyboard surface inside the month picker MUST be:
- `Enter` or `Space` on a focused, enabled month chip selects it and closes the overlay.
- `←` / `→` / `↑` / `↓` move focus through the 3-column × 4-row grid, wrapping according to the same row/column conventions as the day grid.
- `Esc` closes the overlay without changing `viewMonth` or `viewYear`.
- `Tab` returns focus to the day grid (the day grid remains in the tab order behind the overlay).

When no overlay is open, the existing keyboard surface MUST behave exactly as before: `PageUp` / `PageDown` move by one month, `Shift+PageUp` / `Shift+PageDown` move by one year, `←` / `→` / `↑` / `↓` move by one day, `Enter` selects the focused day.

The month picker MUST NOT introduce a new backend Tauri command, MUST NOT change the dispatch surface for `monthChange` or `viewYearChange` (only the trigger that dispatches them changes), and MUST NOT move the popover anchor logic in `DatePicker.svelte` (the overlay sits inside the `CalendarMonth.svelte` header).

The month picker MUST apply identically in both consumers of `CalendarMonth.svelte`: the Calendar tab (`CalendarPage.svelte`) and the form-field popover (`DatePicker.svelte`). Behavior, keyboard surface, visual classes, and dispatch semantics MUST NOT differ between the two contexts.

The previous `cycleMonth()` behavior — clicking the month label to advance the view by one month with year rollover at December — MUST be retired completely. The label's only job is to open the picker. Sequential month navigation remains available through the prev/next-month chevrons and `PageUp` / `PageDown`.

The i18n surface MUST add the following keys under `calendar.*` in both `src/i18n/en/index.ts` and `src/i18n/es/index.ts`, and `src/i18n/i18n-types.ts` MUST be regenerated so the keys remain type-checked:
- `ariaOpenMonthPicker` ("Open month picker" / "Abrir selector de mes") — replaces the previous `ariaCycleMonth` key, which MUST be removed.
- `ariaCloseMonthPicker` ("Close month picker" / "Cerrar selector de mes").
- `ariaPreviousYear` ("Previous year" / "Año anterior").
- `ariaNextYear` ("Next year" / "Año siguiente").
- `ariaMonth` ("{month} {year}") — accessible name for each month chip, reusing `MONTH_NAMES`.

`DatePicker.svelte` consumers MUST continue to receive `monthChange` on selection and MUST NOT need any new prop to enable the month picker.

#### Scenario: month label opens the month picker overlay

- GIVEN any instance of `CalendarMonth.svelte` (Calendar tab or `DatePicker` popover)
- AND the month picker overlay is not currently open
- WHEN the user clicks the month label
- THEN the month picker overlay becomes visible
- AND the overlay header shows `viewYear`
- AND the overlay body renders 12 month chips arranged in a 3-column × 4-row grid
- AND the chip for the current `viewMonth` carries the `month-selected` visual treatment
- AND `viewMonth`, `viewYear`, and `selectedDate` are unchanged

#### Scenario: clicking a month chip selects and closes

- GIVEN the month picker overlay is open
- WHEN the user clicks the chip for month `m` in the currently displayed `pickerYear`
- THEN a single `monthChange` event is dispatched with `{ year: pickerYear, month: m }`
- AND the month picker overlay closes
- AND the calendar header label and the day grid both reflect the new `(year, month)`
- AND the day-detail panel (Calendar tab) or popover preview (`DatePicker`) is updated by the consumer's existing `monthChange` handler

#### Scenario: Esc closes without changing the view

- GIVEN the month picker overlay is open
- WHEN the user presses `Esc`
- THEN the overlay closes
- AND no `monthChange` event is dispatched
- AND `viewMonth`, `viewYear`, and `selectedDate` are unchanged

#### Scenario: prev/next-year chevrons update only the overlay

- GIVEN the month picker overlay is open
- AND `pickerYear` is currently `Y`
- WHEN the user clicks the previous-year chevron
- THEN `pickerYear` becomes `Y - 1`
- AND no `monthChange` or `viewYearChange` event is dispatched
- AND the calendar header and day grid still show year `Y`
- AND the next click on a month chip dispatches `monthChange { year: Y - 1, month }`

#### Scenario: month picker and year picker are mutually exclusive

- GIVEN the month picker overlay is open
- WHEN the user clicks the year chip
- THEN the month picker overlay closes
- AND the year picker overlay opens anchored at the same header

- GIVEN the year picker overlay is open
- WHEN the user clicks the month label
- THEN the year picker overlay closes
- AND the month picker overlay opens anchored at the same header

- GIVEN neither overlay is open
- WHEN the user opens either overlay
- THEN only that overlay is rendered
- AND the other overlay is not rendered at the same time

#### Scenario: year chip stays clickable while month picker is open

- GIVEN the month picker overlay is open
- WHEN the user clicks the year chip button
- THEN the year picker overlay opens
- AND the month picker overlay closes
- AND the consumer's existing `viewYearChange` behavior continues to apply once a year chip is selected

#### Scenario: out-of-range months render as disabled chips

- GIVEN the calendar is configured with a `minDate` and `maxDate` tighter than the default range (for example, restricted to a single calendar year)
- WHEN the month picker overlay opens
- THEN the chips for months outside the effective `[minDate, maxDate]` range carry the `month-disabled` visual treatment
- AND clicking a disabled chip does NOT dispatch `monthChange` and does NOT close the overlay
- AND enabled chips in the same row remain clickable and laid out in the same 3-column grid (no row collapse or grouping)

#### Scenario: only the viewed month is highlighted

- GIVEN `viewMonth` is `M` and `viewYear` is `Y`
- WHEN the month picker overlay opens
- THEN exactly one chip carries the `month-selected` visual treatment — the chip for month `M` in year `Y`
- AND no other chip is rendered as `selected`, including the chip for the current calendar month if `M` is not the current calendar month

#### Scenario: keyboard opens the month picker and navigates the grid

- GIVEN focus is on the month label in the calendar header
- WHEN the user presses `Enter` or `Space`
- THEN the month picker overlay opens
- AND focus moves to the chip for the currently viewed month

- GIVEN the month picker overlay is open and focus is on a chip
- WHEN the user presses `←` / `→` / `↑` / `↓`
- THEN focus moves through the 3-column × 4-row grid accordingly
- AND when focus moves to a disabled chip, the disabled chip is not activated

- GIVEN the month picker overlay is open
- WHEN the user presses `Enter` or `Space` on a focused, enabled chip
- THEN the chip is selected, `monthChange` is dispatched, and the overlay closes

#### Scenario: sequential chevron and PageUp/Down navigation is unchanged

- GIVEN the month picker overlay is closed
- WHEN the user clicks the prev-month chevron, the next-month chevron, presses `PageUp`, or presses `PageDown`
- THEN `monthChange` is dispatched with the new `(year, month)` exactly as before this change
- AND the month label is NOT activated as a side effect

#### Scenario: cycleMonth behavior is retired

- GIVEN any instance of `CalendarMonth.svelte`
- WHEN the user clicks, `Enter`s, or `Space`s on the month label
- THEN the result is opening the month picker overlay
- AND `monthChange` is NOT dispatched as a side effect of opening the picker
- AND no hidden gesture (for example `Shift+click`) re-enables the retired cycle-by-one behavior

#### Scenario: i18n surface carries the renamed and new keys after migration

- GIVEN the i18n migration of `src/i18n/en/index.ts` and `src/i18n/es/index.ts` has landed
- AND `src/i18n/i18n-types.ts` has been regenerated
- WHEN the calendar primitive is rendered
- THEN the month label's accessible name resolves to `$LL.calendar.ariaOpenMonthPicker()` ("Open month picker" / "Abrir selector de mes")
- AND `$LL.calendar.ariaCloseMonthPicker()`, `$LL.calendar.ariaPreviousYear()`, and `$LL.calendar.ariaNextYear()` resolve to their expected English and Spanish strings
- AND each month chip's accessible name resolves to `$LL.calendar.ariaMonth({ month, year })` using the localized month name
- AND `$LL.calendar.ariaCycleMonth` is no longer referenced from `CalendarMonth.svelte`

#### Scenario: shared behavior between Calendar tab and DatePicker

- GIVEN both `CalendarPage.svelte` (Calendar tab) and `DatePicker.svelte` (form-field popover) render an instance of `CalendarMonth.svelte`
- WHEN the user triggers the month picker in either context
- THEN the overlay, keyboard surface, visual classes, disabled rules, mutual-exclusion rule, and `monthChange` dispatch are identical in both contexts
- AND `DatePicker.svelte` does NOT introduce a new prop to enable the picker
- AND the popover positioning logic in `DatePicker.svelte` is not modified
