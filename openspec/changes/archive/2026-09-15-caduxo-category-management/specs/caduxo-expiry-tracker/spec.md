# Delta for caduxo-expiry-tracker

This delta moves the product-category relation from a single nullable FK (`products.category_id`) to a many-to-many junction table (`product_categories`) and lands a shared multi-category picker primitive for `ProductForm`, `ProductCatalogPage`, and `ReportsPage`. The legacy `products.category_id` column remains in the schema as ignored legacy data; the junction is the runtime source of truth. Both the catalog and reports filters accept a multi-category selection with ANY-of semantics (no duplicate-count risk via SQL-level `EXISTS`). The `Uncategorized` pseudo-row represents products with zero active category relations and is implemented as a sentinel id rendered by the picker (never persisted). Case-only duplicates (`Dairy` vs `dairy`) are resolved in the V4 migration; the service-layer guard prevents new duplicates.

## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: report filters

Reports shall support filters for:

- store/local
- internal location
- category — the category filter accepts a multi-category selection (`category_ids: string[]`) with **ANY-of** semantics. Default `None` or empty list means "no category filter applied" (not an explicit All chip). The selection may include the `Uncategorized` sentinel id (`__uncategorized__`) to include products with zero active category relations. The filter is applied at the SQL level via `EXISTS` (never `JOIN`) so that a product in two selected categories is matched exactly once per lot row. See the `multi-category filter ANY-of semantics` scenario under `Capability: Categories > multi-category product model` for the canonical contract.
- status
- date range — the `dateFrom` and `dateTo` fields use the `Date input` picker (clearable, optional). Empty values are translated to `null` by the existing `dateFrom.trim() || null` / `dateTo.trim() || null` chain in `ReportsPage.buildFilters()`. No `<input type="date">` is rendered for these fields.

(Previously: the requirement listed `category` as a single-category filter. This delta replaces the single-category filter with a multi-category list (`category_ids`), establishes ANY-of semantics, introduces the `Uncategorized` sentinel, and pins the filter to SQL-level `EXISTS` to prevent duplicate-count risk.)

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
