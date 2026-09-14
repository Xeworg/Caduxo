# Delta for Caduxo Expiry Tracker

## ADDED Requirements

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

## MODIFIED Requirements

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

(Previously: the `unit` field was listed as a free-text label with no note about catalog-driven resolution or about how the value falls back when no catalog unit is selected.)
