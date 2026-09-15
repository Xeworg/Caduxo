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

Caduxo shall allow users to register expiry lots linked to a product.

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

The Calendar tab MUST reuse the same `CalendarMonth.svelte` primitive used by the date picker popover so the day grid, year picker, visual styling, and keyboard surface are implemented once. The Calendar tab MUST source its expiration data from the existing `list_dashboard_lots` Tauri command (no new backend command in this slice); the day-bucket map MUST be computed client-side from the returned active lots. Month navigation MUST NOT trigger a refetch in this slice.

Clicking a day MUST set the selected date and reveal a day-detail panel that lists every active lot whose `expiry_date` equals the selected date, with columns for product, quantity, unit, store, location, days remaining, and status. Clicking a lot row MUST open the existing lot edit flow used elsewhere in the app. Clicking a day with no expirations MUST still select it and show a `No expirations on YYYY-MM-DD` placeholder.

The Calendar tab MUST inherit the full keyboard surface of the calendar primitive (Tab, Esc, Enter, arrow keys, PageUp/Down, Shift+PageUp/Shift+PageDown) with a tab order of prev-month chevron → month label → year chip → day grid → next-month chevron → day-detail rows.

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
