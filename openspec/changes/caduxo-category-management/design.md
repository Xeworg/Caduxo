# Design — caduxo-category-management

> **Slice scope.** Real multi-category support for products, including the
> database migration, the picker primitive, the filter semantics on
> `ProductCatalogPage` and `ReportsPage`, and the case-fold guard at the
> category-write layer. A single PR within the user-typed **3,000-line**
> session review budget. No chained-PR split. No `size:exception` needed.
>
> **Naming.** Change directory is `caduxo-category-management`. The slice
> lands the four named outcomes (multi-category data model, multi-category
> picker, multi-category filter, case-fold guard) plus the two named
> guardrails (CSV compatibility, restore-from-backup compatibility) on
> a single shared source-of-truth: `product_categories`.

---

## 1. Executive summary

The slice moves the product-category relation from a single nullable FK
(`products.category_id`) to a junction table (`product_categories`). The
junction is the canonical source of truth for runtime reads and writes.
The legacy `products.category_id` column stays in the schema as **ignored
legacy data** — no runtime path writes to it or reads from it; it is
preserved verbatim so that pre-V4 backups restore cleanly and the V4
back-fill has a single, deterministic input source. The runtime never
fans out to two parallel category-read paths: the picker, the report
filter, the catalog filter, the detail bundle, the dashboard modal, and
the CSV export all read from the same junction helper or the same
SQL-level EXISTS filter.

Filter semantics is **ANY-of** with **EXISTS** (never JOIN), so a
product that is in two selected categories is still listed exactly once.
`Uncategorized` is a sentinel id emitted by the picker; the backend
translates it into a `NOT EXISTS (… rows in product_categories whose
category is_active = 1)` clause so it composes naturally with the
ANY-of filter without breaking out of it.

The picker is a reusable Svelte primitive (`CategoryPicker.svelte`)
backed by a new server-side search command (`list_categories_search`)
so the picker scales to 200+ categories without rendering the full list.
It is the only new front-end primitive; existing surface pages adopt it.

The case-fold guard on `create_category` / `update_category` prevents
new case-only duplicates (`Dairy` vs `dairy`). Pre-existing duplicates,
if any, are resolved in the **same V4 migration** by archiving
non-canonical rows and remapping their junction rows to the oldest
canonical id.

CSV import remains single-category per row; single-column CSV export
continues to work via a comma-joined category-names column in the export
file. Restore-from-backup works because the migrator runs after each
restore: pre-V4 backups are brought forward by V4's idempotent back-fill.

**LOC estimate: ~1,400–2,400 LOC**, well under the 3,000-line session
budget and below the chained-PR trigger. The slice touches one Rust
migrator entry, four DTOs, two repositories, two services, one new
command, and one new frontend primitive; the bulk of the LOC is the
picker primitive and the new tests.

---

## 2. Architectural overview

### 2.1 Source-of-truth split

```text
┌────────────────────────────────────┐   CANONICAL RUNTIME  ┌──────────────────────┐
│ Frontend picker / filters          │ ───────────────────► │ Backend commands     │
│ - binds a `string[]` of ids        │   Tauri IPC          │ - list_categories_   │
│ - includes UNCATEGORIZED_SENTINEL  │                      │   search             │
└────────────────────────────────────┘                      │ - create/update_*    │
           │                                                │ - get_product        │
           │                                                │ - list_dashboard_lots│
           │                                                └──────────────────────┘
           ▼                                                          │
┌────────────────────────────────────┐                                 ▼
│ Rust services                       │   ┌──────────────────────────────────────────┐
│ - validate, deduplicate, trans late  │ ◄─┤ SQLite (caduxo.db)                       │
│ - filter SQL fragments                │   │                                          │
└────────────────────────────────────┘   │  categories      (id, name, is_active)   │
           │                              │       ▲                                  │
           ▼                              │       │ ON DELETE RESTRICT                 │
┌────────────────────────────────────┐   │  product_categories (junction, SoT)       │
│ Rust repositories                   │   │       │ ON DELETE CASCADE / RESTRICT      │
│ - junction reads via batched helper │   │       ▼                                  │
│ - EXISTS filter for catalog/reports │   │  products         (…, NO category_id)     │
└────────────────────────────────────┘   │                                          │
                                        │  (legacy products.category_id column:      │
                                        │   schema-present, runtime-ignored.)        │
                                        └──────────────────────────────────────────┘
```

The runtime never reads or writes `products.category_id`. The junction
is the only path used by commands, repositories, and services.

### 2.2 Legacy `products.category_id` lifecycle (split-brain prevention)

| Lifecycle stage | `products.category_id` | `product_categories` junction | Runtime reads from `category_id`? | Runtime writes to `category_id`? |
|---|---|---|---|---|
| Pre-V4 (V1–V3) | single FK, source of truth | absent | yes (every consumer) | yes (every write) |
| V4 migration applies | preserved verbatim | populated from `products.category_id` (idempotent back-fill) | no — only the migration script reads it; runtime repos stop selecting it | no — migration does not write to it; create/update_product omit it from the SQL |
| V4 onwards (steady state) | exists in schema, untouched | canonical source of truth | no | no |
| Restore from pre-V4 backup | restored verbatim by VACUUM INTO | V4 re-applies; back-fill runs | no | no |
| Restore from V4-onwards backup | restored verbatim (always `NULL`) | restored verbatim (canonical) | no | no |

The "runtime reads from `category_id`?" column is **always `no` after
V4**. There is no period in which two sources are read together. This is
the explicit guard against accidental split-brain.

The migration, when back-filling, does **not** rewrite
`products.category_id` — it leaves the legacy column exactly as it found
it. This makes `git revert` of the slice safe: V4 inserts into the
junction but never destructively overwrites the legacy column, so
reverting the application code restores the single-FK runtime semantics
without data loss. The proposal names this property in the rollback
section; the design enforces it with a code-level invariant
(comment in `migrations.rs` and `repositories/products.rs`).

### 2.3 Single re-usable primitives (no parallel category-read paths)

| Concern | Owned by | Notes |
|---|---|---|
| Junction table, indexes, back-fill, case-fold dedup | `db/migrations.rs` (`V4`) | Single migration; idempotent. |
| Junction reads per product | `list_product_category_ids_by_product_ids` in `db/repositories/products.rs` | Batched; replaces per-row `get_product` reads. |
| Junction reads for the report / catalog filter | `db/repositories/dashboard.rs::list_dashboard_lots` (`EXISTS`) | SQL-level; no double count. |
| Category search | `list_categories_search` Tauri command | Prefix first; substring fallback; active only. |
| Picker primitive | `src/components/inputs/CategoryPicker.svelte` | Used by `ProductForm`, `ProductCatalogPage`, `ReportsPage`. |
| `Uncategorized` sentinel | `src/lib/categories.ts` + `CategoryPicker.svelte` | One sentinel id; never persisted. |
| Restore-from-backup | existing `services/backup_restore.rs` + automatic migrator run on restored pool | Re-uses V4's idempotent back-fill. |
| Case-fold guard at the category-write layer | `services/products.rs::{create_category,update_category}` | Re-uses SQLite UNIQUE violation translation. |

The picker popover does not duplicate a category-read primitive. The
filters do not duplicate a junction-read primitive. Both compose the
batch helper or the SQL-level filter. A future change to category
naming, archive policy, or picker UX lands in one place.

---

## 3. Data model — V4 migration

### 3.1 Migration entry

`V4 = "add_product_categories_v4"` in
`src-tauri/src/db/migrations.rs::MIGRATIONS`. Follows the existing
`MigrationType::Simple` + `no_tx: false` pattern from V3.

### 3.2 DDL

```sql
-- ============================================================
-- V4: product_categories
--   - Many-to-many relation between products and categories.
--   - product_id ON DELETE CASCADE: deleting a product removes junction rows.
--   - category_id ON DELETE RESTRICT: deleting a category with junction rows is
--     rejected (matches today's single-FK semantics; archive-first preferred).
--   - Composite PRIMARY KEY enforces many-to-many uniqueness.
--   - Both indexes drive the filter and detail-bundle read paths.
-- ============================================================
CREATE TABLE product_categories (
    product_id  TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    created_at  TEXT NOT NULL,
    PRIMARY KEY (product_id, category_id)
);

CREATE INDEX IF NOT EXISTS idx_product_categories_category
    ON product_categories(category_id);

-- Redundant safety index; the PK already covers product-leading lookups, but
-- V3 added the same redundant index for unit_definitions and the pattern is
-- preserved here for symmetry with that prior slice.
CREATE INDEX IF NOT EXISTS idx_product_categories_product
    ON product_categories(product_id);
```

No `ALTER TABLE products DROP COLUMN category_id`. The column is left
intact and ignored by the runtime (see §2.2). This avoids the
`DROP COLUMN`-availability check, removes a migration failure path, and
makes restore-from-pre-V4-backup trivially compatible (the column is
already there).

### 3.3 Idempotent back-fill from the legacy FK

```sql
-- Back-fill: copy `products.category_id` into the junction without
-- duplicating pairs (re-runs of V4 are no-ops).
INSERT INTO product_categories (product_id, category_id, created_at)
SELECT p.id, p.category_id, p.created_at
FROM products p
WHERE p.category_id IS NOT NULL
  AND NOT EXISTS (
    SELECT 1 FROM product_categories pc
    WHERE pc.product_id = p.id AND pc.category_id = p.category_id
  );
```

### 3.4 Case-fold deduplication

```sql
-- Case-fold dedup: for groups of categories that differ only in case, keep
-- the OLDEST row as canonical (its id is the survivor), archive the rest, and
-- reassign junction rows (and the legacy `products.category_id` column) to
-- the canonical id. No hard delete.

-- 3.4.1 Find all groups with `lower(name)` collisions.
--      A "group" means ≥ 2 categories whose lower(name) is identical.
WITH dup_groups AS (
    SELECT lower(name) AS name_key
    FROM categories
    GROUP BY lower(name)
    HAVING COUNT(*) > 1
),
-- 3.4.2 For each group, choose the canonical id (oldest by created_at, tie-break by id).
canonical_per_group AS (
    SELECT c.id        AS canonical_id,
           lower(c.name) AS name_key
    FROM categories c
    WHERE c.id IN (
        SELECT id FROM categories
        WHERE lower(name) = c.lower_name
        ORDER BY created_at ASC, id ASC
        LIMIT 1
    )
    -- The CTE above is illustrative; the real implementation uses a window
    -- function so each group picks exactly one canonical id. See note below.
),
-- (Real form using ROW_NUMBER() avoids the inner LIMIT-1 trick:
--  SELECT id, lower(name) AS name_key
--  FROM (SELECT id, lower(name),
--               ROW_NUMBER() OVER (PARTITION BY lower(name) ORDER BY created_at ASC, id ASC) AS rn
--        FROM categories)
--  WHERE rn = 1;
)
-- 3.4.3 Reassign junction rows from non-canonical ids to the canonical id.
UPDATE product_categories
SET category_id = (
    SELECT canonical_id FROM canonical_per_group cp
    WHERE cp.name_key = (
        SELECT lower(name) FROM categories WHERE id = product_categories.category_id
    )
)
WHERE category_id IN (
    SELECT id FROM categories WHERE lower(name) IN (SELECT name_key FROM dup_groups)
      AND id NOT IN (SELECT canonical_id FROM canonical_per_group)
);

-- 3.4.4 Reassign legacy `products.category_id` to canonical id (safety net
--        for restore / migration paths that still read the column).
UPDATE products
SET category_id = (
    SELECT canonical_id FROM canonical_per_group cp
    WHERE cp.name_key = (SELECT lower(name) FROM categories WHERE id = products.category_id)
)
WHERE category_id IN (
    SELECT id FROM categories WHERE lower(name) IN (SELECT name_key FROM dup_groups)
      AND id NOT IN (SELECT canonical_id FROM canonical_per_group)
);

-- 3.4.5 Archive (soft-delete) the non-canonical duplicates.
UPDATE categories
SET is_active = 0, updated_at = ?
WHERE lower(name) IN (SELECT name_key FROM dup_groups)
  AND id NOT IN (SELECT canonical_id FROM canonical_per_group);
```

Notes:

- The actual V4 SQL uses `ROW_NUMBER() OVER (PARTITION BY lower(name)
  ORDER BY created_at ASC, id ASC)` to pick one canonical id per group,
  avoiding the `IN (… LIMIT 1)` semantics inside an UPDATE.
- The migration logs counts: `INFO duplicates_found=N,
  remapped_junction=M, remapped_legacy=L, archived_duplicate=D`.
- The service-layer guard at `create_category` / `update_category`
  prevents new case-only duplicates from being inserted (see §6.4).
- The case-fold fix-up reads `products.category_id` only inside V4
  itself. After V4, the runtime does not select or update this column.

### 3.5 Migration tests (V4 test surface)

| Test | Asserts |
|---|---|
| `v4_applies_on_fresh_db` | `applied_count == 4` after V4 runs on a fresh DB. |
| `v4_migration_is_idempotent` | Re-running migrations on a V4'd DB is a no-op. |
| `v4_backfill_copies_legacy_category_id_into_junction` | Seed a V3-only DB with two products pointing to categories. Run V4. Assert both junction rows exist. |
| `v4_backfill_is_idempotent` | Run V4 twice on a V3-only DB; assert junction row count is unchanged. |
| `v4_backfill_skips_null_category_id` | Seed products with `category_id = NULL`. Run V4. Assert no orphan junction rows. |
| `v4_case_fold_dedup_keeps_oldest` | Seed `Dairy` (older) and `dairy` (younger); seed a junction row referencing `dairy`. Run V4. Assert: `dairy` is archived, the junction row now references `Dairy.id`. |
| `v4_case_fold_dedup_remaps_legacy_column` | Same fixture as above, but the legacy `products.category_id = dairy.id` row also gets remapped to `Dairy.id`. |
| `v4_legacy_column_is_left_intact_for_non_duplicate_categories` | A product whose `category_id` points to a non-duplicate category is not touched in the legacy column beyond what the back-fill assigned. |

These tests are added to `db::migrations::tests` and follow the
V2-only-migrator pattern in `products_default_unit_id_backfilled_for_known_keys`.

---

## 4. DTO contract

### 4.1 Rust DTOs

`src-tauri/src/dto/products.rs`:

```rust
/// Input for creating a product. SKU unique within the local database.
/// `category_ids` replaces the legacy `category_id: Option<String>`.
/// Empty `vec![]` or `None` both mean "unassigned".
#[derive(Debug, Deserialize)]
pub struct ProductCreate {
    pub sku: String,
    pub description: String,
    pub category_ids: Option<Vec<String>>,
    pub default_unit: Option<String>,
    pub default_unit_id: Option<String>,
    pub default_alert_days_before: i32,
    pub notes: Option<String>,
}

/// Input for updating an existing product. `category_ids` is the full set
/// on each update; the service layer performs a delete-all + insert-new
/// inside a transaction.
#[derive(Debug, Deserialize)]
pub struct ProductUpdate {
    pub id: String,
    pub sku: String,
    pub description: String,
    pub category_ids: Option<Vec<String>>,
    pub default_unit: Option<String>,
    pub default_unit_id: Option<String>,
    pub default_alert_days_before: i32,
    pub notes: Option<String>,
    pub is_active: bool,
}

/// Response shape for a product. `category_ids` is always present and
/// may be empty. The legacy `category_id` field is dropped (renamed).
#[derive(Debug, Serialize, FromRow)]
pub struct ProductResponse {
    pub id: String,
    pub sku: String,
    pub description: String,
    pub category_ids: Vec<String>, // NEW
    pub default_unit: Option<String>,
    pub default_unit_id: Option<String>,
    pub unit_type: Option<UnitKind>,
    pub default_alert_days_before: i32,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Detail bundle returned by `get_product`: product, barcodes, and the
/// resolved categories (one or zero-or-more; empty list = unassigned).
#[derive(Debug, Serialize)]
pub struct ProductDetailResponse {
    pub product: ProductResponse,
    pub barcodes: Vec<ProductBarcodeResponse>,
    pub categories: Vec<CategoryResponse>, // replaces `category`
}

/// Search hit shape — minimal fields for scan/search results and listing.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ProductSearchResult {
    pub id: String,
    pub sku: String,
    pub description: String,
    pub category_ids: Vec<String>, // replaces `category_id`
    pub primary_barcode: Option<String>,
    pub is_active: bool,
}
```

Note: `ProductResponse` is no longer derivable from `sqlx::FromRow`
because `category_ids` is a `Vec<String>` that requires a join. The
materializing strategy moves to a `RawProductRow` that resolves
`category_ids` from the junction helper (see §5.2 and §5.3).

`src-tauri/src/dto/reports.rs`:

```rust
/// Filters supported by reports. `category_ids` replaces `category_id`:
/// when `Some(non_empty)`, the report keeps lots whose product has at
/// least one junction row in the selected ids (ANY-of). When the
/// selection contains `UNCATEGORIZED_SENTINEL`, lots whose product has
/// zero junction rows to active categories are also included.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ReportFilters {
    pub store_id: Option<String>,
    pub location_id: Option<String>,
    pub category_ids: Option<Vec<String>>, // REPLACES `category_id`
    pub urgency: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}
```

`src-tauri/src/dto/dashboard.rs`:

```rust
/// Optional filters for the dashboard lot query.
#[derive(Debug, Default, Deserialize)]
pub struct DashboardFilters {
    pub store_id: Option<String>,
    pub location_id: Option<String>,
    pub preset: Option<DashboardPreset>,
    pub urgency: Option<String>,
    pub category_ids: Option<Vec<String>>, // NEW
}
```

`src-tauri/src/dto/categories.rs` (extended; new file or co-located):

```rust
/// Page of category search results. The picker asks for active-only
/// categories matching the typed query.
#[derive(Debug, Serialize)]
pub struct CategorySearchPage {
    pub items: Vec<CategoryResponse>,
    pub total: usize,
    pub has_more: bool,
}

/// Input for the category search command.
#[derive(Debug, Deserialize)]
pub struct CategorySearchInput {
    pub query: String,
    pub limit: Option<usize>,
}
```

`src-tauri/src/dto/categories.rs` may sit next to `dto/products.rs` if a
top-level `dto/categories.rs` already exists, or be added to
`dto/products.rs`. The design keeps the new types in
`dto/products.rs` to avoid a new file unless the codebase convention
favors a split — the proposal does not commit either way and the design
follows whichever convention exists when the slice applies. (As of
this writing, all category DTOs sit in `dto/products.rs`.)

### 4.2 Sentinel id

```rust
// src-tauri/src/dto/categories.rs (or top of dto/products.rs)
pub const UNCATEGORIZED_SENTINEL: &str = "__uncategorized__";
```

The sentinel is a constant string. It is **not** a UUID, **not** a
category id, and never appears in `list_categories_search` results.
It is only emitted by the frontend picker when the user explicitly
selects the `Uncategorized` pseudo-row.

### 4.3 Frontend (TypeScript) DTOs

`src/lib/products.ts`, `src/lib/dashboard.ts`, `src/lib/reports.ts` mirror
the new Rust DTOs. The legacy `category_id` field is removed (no
deprecation cycle; the app is pre-1.0 with no external consumers).

`src/lib/categories.ts` (new):

```ts
export const UNCATEGORIZED_SENTINEL = "__uncategorized__";

export interface CategorySearchInput {
  query: string;
  limit?: number;
}

export interface CategorySearchPage {
  items: CategoryResponse[];
  total: number;
  has_more: boolean;
}

export async function listCategoriesSearch(
  input: CategorySearchInput,
): Promise<CategorySearchPage> {
  return invoke<CategorySearchPage>("list_categories_search", { input });
}
```

The existing `listCategories()` wrapper stays for callers that need the
full active list (e.g. on mount of the picker for an empty query — it
hits the same SQL with `query = ""` and `limit = large`).

---

## 5. Repository contract

### 5.1 `src-tauri/src/db/repositories/products.rs`

#### `insert_product`

The legacy INSERT includes `category_id`. V4 rewrites it to:

```rust
pub async fn insert_product(
    pool: &SqlitePool,
    input: &ProductCreate,
    default_unit_id: Option<String>,
    unit_type: Option<String>,
) -> Result<ProductResponse, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Begin transaction so a junction-write failure rolls back the insert.
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO products (
            id, sku, description, default_unit,
            default_unit_id, unit_type,
            default_alert_days_before, notes, is_active,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, $9, $10)
        "#,
    )
    .bind(&id)
    .bind(&input.sku)
    .bind(&input.description)
    .bind(&input.default_unit)
    .bind(default_unit_id)
    .bind(unit_type)
    .bind(input.default_alert_days_before)
    .bind(&input.notes)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    let category_ids = input.category_ids.clone().unwrap_or_default();
    write_junction(&mut tx, &id, &category_ids).await?;

    tx.commit().await?;

    get_product(pool, &id).await?.ok_or(sqlx::Error::RowNotFound)
}
```

Notes:

- `INSERT INTO products` no longer references `category_id` at all.
- `write_junction` is a private helper that performs the
  `INSERT` per category id, skipping duplicates (composite PK protects
  integrity either way) and never reaching the legacy column.
- The transaction guarantees that a junction-write failure rolls back
  the `INSERT INTO products`.

#### `update_product`

```rust
pub async fn update_product(
    pool: &SqlitePool,
    input: &ProductUpdate,
    default_unit_id: Option<String>,
    unit_type: Option<String>,
) -> Result<Option<ProductResponse>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    let affected = sqlx::query(
        r#"
        UPDATE products
        SET sku = $1, description = $2, default_unit = $3,
            default_unit_id = $4, unit_type = $5,
            default_alert_days_before = $6, notes = $7, is_active = $8,
            updated_at = $9
        WHERE id = $10
        "#,
    )
    .bind(&input.sku)
    .bind(&input.description)
    .bind(&input.default_unit)
    .bind(default_unit_id)
    .bind(unit_type)
    .bind(input.default_alert_days_before)
    .bind(&input.notes)
    .bind(i32::from(input.is_active))
    .bind(&now)
    .bind(&input.id)
    .execute(&mut *tx)
    .await?;

    if affected.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(None);
    }

    // Full set semantics: delete-all + insert-new in the same transaction.
    sqlx::query("DELETE FROM product_categories WHERE product_id = $1")
        .bind(&input.id)
        .execute(&mut *tx)
        .await?;
    let category_ids = input.category_ids.clone().unwrap_or_default();
    write_junction(&mut tx, &input.id, &category_ids).await?;

    tx.commit().await?;
    get_product(pool, &input.id).await
}
```

`update_product` returns `Ok(None)` when the UPDATE matches zero rows
(no product with that id). It does not raise on `DELETE FROM
product_categories …` returning zero rows: a product with no
prior junctions is a valid steady state, and `write_junction` clears
the set on update even if the previous set was empty.

#### `get_product`, `search_products`, `find_by_*_exact`

Each read populates `category_ids` from the junction. For per-product
reads, the implementation uses a single junction query:

```rust
let junction: Vec<(String,)> = sqlx::query_as(
    "SELECT category_id FROM product_categories WHERE product_id = $1",
).bind(product_id).fetch_all(pool).await?;
let category_ids = junction.into_iter().map(|(c,)| c).collect();
```

For search hits (`search_products`, `find_by_*_exact`) the runtime
**does not** resolve category ids per row — the wire shape is the list
of ids, not the resolved names. Resolution to `CategoryResponse`
happens only on `get_product` (which returns `ProductDetailResponse`)
and in the dashboard detail modal (via the batched helper, see §5.3).

This avoids the per-row name join and keeps `ProductSearchResult` a
cheap, narrowly-shaped return.

#### `list_all_products_for_export`

```rust
pub async fn list_all_products_for_export(
    pool: &SqlitePool,
) -> Result<Vec<ProductResponse>, sqlx::Error> {
    // 1. SELECT every product row (no junction join).
    // 2. SELECT junction (product_id, category_name) for every product that
    //    has at least one category, in a single query.
    // 3. For each product, compute `category_ids` (list of ids) and the
    //    backend-export-only `category_names_csv` (comma-joined names).
    //
    // Wire shape: `ProductResponse` carries `category_ids: Vec<String>`.
    // The CSV exporter concatenates names from a separate field that does
    // NOT enter the wire (kept off `ProductResponse` so IPC stays narrow).
}
```

The CSV export uses a single transaction to fetch all products plus a
single batched read of junction rows. It maps through `category_ids`
and a paired `category_names_csv` field that the CSV layer
concatenates into a `category` column. This preserves back-compat:
existing CSV consumers that parse the **first name** from the
`category` column still parse correctly.

#### `list_product_category_ids_by_product_ids`

New helper. The single source of truth for batched junction reads.

```rust
pub async fn list_product_category_ids_by_product_ids(
    pool: &SqlitePool,
    product_ids: &[String],
) -> Result<HashMap<String, Vec<String>>, sqlx::Error> {
    if product_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT pc.product_id, pc.category_id
        FROM product_categories pc
        WHERE pc.product_id = ANY($1)
        ORDER BY pc.product_id, pc.category_id
        "#,
    )
    .bind(product_ids)
    .fetch_all(pool)
    .await?;

    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (product_id, category_id) in rows {
        out.entry(product_id).or_default().push(category_id);
    }
    Ok(out)
}
```

Used by:

- `services::products::get_product` — to resolve the
  `categories: Vec<CategoryResponse>` field of `ProductDetailResponse`
  when the product has more than one category. (For products with ≤ 1
  active category the existing per-product `get_category` would still
  work, but using the batched path keeps the implementation uniform
  and lets `get_product` work without a separate round-trip per
  category.)
- Future batch consumers (the catalog page that fetches many products
  and wants their category ids in one read).

The helper is the **only** batched junction read primitive — no
parallel implementations live in other repositories.

### 5.2 `src-tauri/src/db/repositories/dashboard.rs`

`list_dashboard_lots` accepts a new field on `DashboardFilters`:

```rust
pub async fn list_dashboard_lots(
    pool: &SqlitePool,
    filters: &DashboardFilters,
) -> Result<Vec<DashboardLotRow>, sqlx::Error> {
    // Build an optional EXISTS clause for category_ids, defaulting to no
    // filter when the selection is empty.
    let has_category_filter = filters
        .category_ids
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    // Categories EXCLUDING the sentinel (real ids only).
    let real_category_ids: Vec<String> = filters
        .category_ids
        .as_ref()
        .map(|v| {
            v.iter()
                .filter(|id| *id != UNCATEGORIZED_SENTINEL)
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    let include_uncategorized = filters
        .category_ids
        .as_ref()
        .map(|v| v.iter().any(|id| id == UNCATEGORIZED_SENTINEL))
        .unwrap_or(false);

    let filter_sql = if has_category_filter {
        // Compose:
        //   EXISTS (any-of real categories) OR <uncategorized inclusion>
        // The sentinel path doesn't double-count because EXISTS / NOT EXISTS
        // don't join — they just gate each lot row.
        // The any-of clause itself uses EXISTS, not JOIN, so a product in
        // two selected categories is still listed exactly once per lot.
        match (real_category_ids.is_empty(), include_uncategorized) {
            (false, false) => {
                // ANY-of real categories only.
                r#"
                AND EXISTS (
                    SELECT 1 FROM product_categories pc
                    WHERE pc.product_id = el.product_id
                      AND pc.category_id = ANY($5)
                )
                "#.to_string()
            }
            (false, true) => {
                // ANY-of real categories OR uncategorized.
                r#"
                AND (
                    EXISTS (
                        SELECT 1 FROM product_categories pc
                        WHERE pc.product_id = el.product_id
                          AND pc.category_id = ANY($5)
                    )
                    OR NOT EXISTS (
                        SELECT 1 FROM product_categories pc
                        JOIN categories c ON c.id = pc.category_id
                        WHERE pc.product_id = el.product_id
                          AND c.is_active = 1
                    )
                )
                "#.to_string()
            }
            (true, true) => {
                // Uncategorized-only (no real ids selected).
                r#"
                AND NOT EXISTS (
                    SELECT 1 FROM product_categories pc
                    JOIN categories c ON c.id = pc.category_id
                    WHERE pc.product_id = el.product_id
                      AND c.is_active = 1
                )
                "#.to_string()
            }
            (true, false) => unreachable!(
                "empty real_category_ids without sentinel means no filter"
            ),
        }
    } else {
        // No category filter; the WHERE clause omits the category predicate.
        String::new()
    };

    let rows = sqlx::query_as::<_, DashboardLotRow>(&format!(
        r#"
        SELECT
            el.id             AS lot_id,
            el.product_id,
            p.sku,
            p.description,
            el.store_id,
            s.name            AS store_name,
            el.location_id,
            sl.name           AS location_name,
            el.quantity,
            el.unit,
            el.expiry_date,
            el.alert_days_before,
            el.batch_code,
            el.status,
            ''                AS urgency,
            0                 AS days_remaining,
            p.default_unit_id,
            p.unit_type
        FROM expiry_lots AS el
        JOIN products    AS p  ON p.id = el.product_id
        JOIN stores      AS s  ON s.id = el.store_id
        LEFT JOIN store_locations AS sl ON sl.id = el.location_id
        WHERE el.status = 'active'
          AND (? IS NULL OR el.store_id = ?)
          AND (? IS NULL OR el.location_id = ?)
          {filter_sql}
        ORDER BY el.expiry_date ASC
        "#,
    ))
    .bind(&filters.store_id)
    .bind(&filters.store_id)
    .bind(&filters.location_id)
    .bind(&filters.location_id)
    .bind(&real_category_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
```

Critical invariants:

1. **EXISTS, not JOIN.** A product in two selected categories is
   matched exactly once per lot row.
2. **Empty `category_ids` means no filter** (preserves the existing
   call sites that pass `None` or `Some(vec![])`).
3. **The sentinel is split out** from real ids at the SQL boundary so
   the runtime never leaks the synthetic id into a junction lookup.
4. **Uncategorized** uses `NOT EXISTS` against `product_categories
   JOIN categories` so a product with only archived junction rows is
   treated as uncategorized.

### 5.3 Batched helper as the only junction read primitive

(See §5.1 `list_product_category_ids_by_product_ids`.) No other
module selects from `product_categories` for a list of products. Per-row
selects only happen for the single-product `get_product` path (one
product per call) and the single-product edit / archive paths.

---

## 6. Service contract

### 6.1 `services::products`

#### `create_product` / `update_product`

Validate `category_ids`:

- `None` or `Some(vec![])` → unassigned (valid).
- Each id must exist (`SELECT id FROM categories WHERE id = ?` for
  each candidate). Missing ids → `DomainError::Validation { message:
  "unknown category id(s)" }` listing the offending ids.
- Archived (`is_active = 0`) ids are rejected by default
  (`DomainError::Validation { message: "category archived" }`).
  Future callers may pass `include_archived: true` (not exposed in
  this slice; pre-1.0 contract).
- De-duplicate the input (`HashSet`-style pass) before writing so a
  UI bug that produces duplicates does not surface a unique-violation
  error from the junction PK.

Then call the repository `insert_product` / `update_product`. No
special-case logic in the service; the transaction is the repository's
responsibility.

#### `get_product`

```rust
pub async fn get_product(
    pool: &DbPool,
    id: String,
) -> Result<ProductDetailResponse, AppError> {
    let product = repo::get_product(pool, &id).await?
        .ok_or(DomainError::NotFound { resource: "product", id: id.clone() })?;

    let barcodes = repo::list_barcodes(pool, &id).await?;

    // Resolve `categories: Vec<CategoryResponse>` from the junction.
    let categories: Vec<CategoryResponse> = if product.category_ids.is_empty() {
        Vec::new()
    } else {
        let cats = sqlx::query_as::<_, CategoryResponse>(
            "SELECT id, name, is_active, created_at, updated_at
             FROM categories WHERE id = ANY($1)",
        )
        .bind(&product.category_ids)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)?;
        cats
    };

    Ok(ProductDetailResponse { product, barcodes, categories })
}
```

When the product has a small number of categories (the realistic case),
the resolution is one in-clause SELECT. It is not N+1 in any
plausible product cardinality. The batched helper (§5.1) is reserved
for callers that resolve many products at once.

#### `create_category` / `update_category` (case-fold guard)

```rust
pub async fn create_category(
    pool: &DbPool,
    input: CategoryCreate,
) -> Result<CategoryResponse, AppError> {
    validate_name(&input.name).map_err(|m| DomainError::Validation { message: m })?;
    let normalized = input.name.trim().to_lowercase();

    // Case-fold guard: refuse names that would collide case-insensitively
    // with an existing category. Translate the SQLite UNIQUE violation to
    // `DuplicateField { field: "name" }` on the conflict path so the wire
    // shape stays consistent.
    if let Some(existing) = repo::find_category_by_name_ci(pool, &normalized).await? {
        if existing.name.to_lowercase() == normalized {
            return Err(DomainError::DuplicateField {
                field: "name",
                value: input.name.clone(),
            }.into());
        }
    }

    repo::insert_category(pool, &input).await
        .map_err(|e| category_unique_error(e, &input.name))
}

pub async fn update_category(
    pool: &DbPool,
    input: CategoryUpdate,
) -> Result<CategoryResponse, AppError> {
    validate_name(&input.name).map_err(|m| DomainError::Validation { message: m })?;
    let normalized = input.name.trim().to_lowercase();

    // Case-fold guard: refuse rename to a name that collides with another
    // category's name (case-insensitive). The category may rename to its
    // own current name; `find_by_id` returns Some in that path.
    if let Some(existing) = repo::find_category_by_name_ci(pool, &normalized).await? {
        if existing.id != input.id && existing.name.to_lowercase() == normalized {
            return Err(DomainError::DuplicateField {
                field: "name",
                value: input.name.clone(),
            }.into());
        }
    }

    let row = repo::update_category(pool, &input).await
        .map_err(|e| category_unique_error(e, &input.name))?
        .ok_or(DomainError::NotFound { resource: "category", id: input.id })?;
    Ok(row)
}
```

`find_category_by_name_ci` is a new repository helper that performs a
case-insensitive exact match against `categories.name`. It sits next
to the existing `find_by_key` for units; its query mirrors
`SELECT id, name, is_active, created_at, updated_at FROM categories WHERE
lower(name) = lower($1) LIMIT 1`.

A race between the guard check and the INSERT (a concurrent writer
inserts the same case-fold collision) is caught by the SQLite UNIQUE
constraint and translated by `category_unique_error` to
`DuplicateField`. The pre-check is a fast-path UX optimization; the
UNIQUE constraint is the canonical safety net.

### 6.2 `services::reports`

The `filter_by_category` function (§3.4 of `services/reports.rs`) is
**removed**. The category filter moves to SQL via
`DashboardFilters.category_ids`. `to_dashboard_filters` forwards
`category_ids` through.

```rust
fn to_dashboard_filters(request: &ReportRequest) -> DashboardFilters {
    let filters = request.filters.clone().unwrap_or_default();
    let preset = preset_for_type(request.kind);
    let urgency = if preset.is_none() {
        filters.urgency.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).map(str::to_string)
    } else {
        None
    };
    DashboardFilters {
        store_id: filters.store_id,
        location_id: filters.location_id,
        preset,
        urgency,
        category_ids: filters.category_ids, // NEW pass-through
    }
}
```

`preview_report` no longer calls `filter_by_category`. The end-of-pipeline
post-filter now only carries `filter_by_date_range`. Removing the N+1
post-filter is one of the slice's user-visible benefits.

### 6.3 `services::dashboard` (untouched)

`services::dashboard::get_dashboard` is unchanged structurally: it
delegates to `repo::list_dashboard_lots` with whatever
`DashboardFilters` it was given. The repo transparently applies the
new `EXISTS` predicate, so the filter works without service-layer
changes.

### 6.4 `services::csv_io` (unchanged)

`resolve_category_name` keeps its case-insensitive name match against
`list_all_categories` (semantics it already has). CSV import treats
each row's `category` column as a single category name (no
semicolon-separated, no `category_2`). The DTOs for products in
`services::csv_io` are updated to pass `category_ids` to
`create_product` / `update_product` — the service signature changes
shape but the CSV wire format does not.

### 6.5 Restore / backup (unchanged in code)

`services::backup_restore.rs` uses `VACUUM INTO` to copy the live
database file. The post-restore code re-opens the pool via
`open_pool`, which calls `run_migrations`. After restore, V4 (or any
later migration) applies to the restored DB if not already applied.
This is the mechanism that makes pre-V4 backups automatically inherit
the V4 shape: the migrator applies V4's idempotent back-fill on the
restored pool.

No new restore adapter code is needed. The migrator is the adapter.
This keeps the slice's LOC envelope under the budget.

---

## 7. Command layer

### 7.1 `commands/products.rs`

Existing commands update their signatures; no new command beside
`list_categories_search` is added.

```rust
#[tauri::command]
pub async fn list_categories(
    state: State<'_, AppState>,
) -> Result<Vec<CategoryResponse>, CommandError> { /* unchanged */ }

#[tauri::command]
pub async fn create_category(
    state: State<'_, AppState>,
    input: CategoryCreate,
) -> Result<CategoryResponse, CommandError> { /* unchanged */ }

#[tauri::command]
pub async fn update_category(
    state: State<'_, AppState>,
    input: CategoryUpdate,
) -> Result<CategoryResponse, CommandError> { /* unchanged */ }
```

### 7.2 New `list_categories_search`

```rust
/// Searches active categories with case-insensitive prefix match first
/// and substring fallback when the prefix match returns zero rows.
/// Returns a page with up to `limit` items and a has_more flag.
#[tauri::command]
pub async fn list_categories_search(
    state: State<'_, AppState>,
    input: CategorySearchInput,
) -> Result<CategorySearchPage, CommandError> {
    let pool = state.pool().await;
    service::categories::search(&pool, input).await.map_err(AppError::into)
}
```

### 7.3 `src-tauri/src/lib.rs`

Adds:

```rust
commands::products::list_categories_search,
```

The other commands in `products.rs`, `dashboard.rs`, `reports.rs` are
unchanged in the registry; their Tauri handler signatures already use
DTO types that change in §4.1 and serde parses the new shape
automatically.

---

## 8. `services::categories::search` (new service module)

Lives next to `services::products`. The search semantics:

```rust
pub async fn search(
    pool: &DbPool,
    input: CategorySearchInput,
) -> Result<CategorySearchPage, AppError> {
    let raw = input.query.trim();
    let limit = input.limit.unwrap_or(50).clamp(1, 500);

    // Empty query → list all active categories, ordered by name asc, capped.
    if raw.is_empty() {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM categories WHERE is_active = 1",
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::from)?;
        let items = sqlx::query_as::<_, CategoryResponse>(
            "SELECT id, name, is_active, created_at, updated_at
             FROM categories WHERE is_active = 1 ORDER BY name ASC LIMIT $1",
        )
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)?;
        return Ok(CategorySearchPage {
            has_more: (total.0 as usize) > items.len(),
            total: total.0 as usize,
            items,
        });
    }

    // Non-empty query: try prefix first.
    let needle = raw.to_lowercase();
    let prefix = format!("{needle}%");

    let prefix_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM categories
         WHERE is_active = 1 AND lower(name) LIKE $1",
    )
    .bind(&prefix)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)?;

    if prefix_count.0 > 0 {
        let items = sqlx::query_as::<_, CategoryResponse>(
            "SELECT id, name, is_active, created_at, updated_at
             FROM categories
             WHERE is_active = 1 AND lower(name) LIKE $1
             ORDER BY name ASC LIMIT $2",
        )
        .bind(&prefix)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)?;
        Ok(CategorySearchPage {
            has_more: (prefix_count.0 as usize) > items.len(),
            total: prefix_count.0 as usize,
            items,
        })
    } else {
        // Fallback: substring match excluding perfect-prefix matches
        // (which is the empty case here).
        let substring = format!("%{needle}%");
        let substring_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM categories
             WHERE is_active = 1 AND lower(name) LIKE $1",
        )
        .bind(&substring)
        .fetch_one(pool)
        .await
        .map_err(AppError::from)?;
        let items = sqlx::query_as::<_, CategoryResponse>(
            "SELECT id, name, is_active, created_at, updated_at
             FROM categories
             WHERE is_active = 1 AND lower(name) LIKE $1
             ORDER BY name ASC LIMIT $2",
        )
        .bind(&substring)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)?;
        Ok(CategorySearchPage {
            has_more: (substring_count.0 as usize) > items.len(),
            total: substring_count.0 as usize,
            items,
        })
    }
}
```

Notes:

- `limit` defaults to 50 and is clamped to `[1, 500]`.
- The query is matched against `lower(name)`. SQLite is case-insensitive
  for ASCII by default; the explicit `lower()` is the safety net for
  non-ASCII.
- `total` is the count for the active rule (prefix or substring), not
  a union. This matches the proposal's "prefix match first; substring
  fallback when no prefix results" semantics.
- `has_more` is true if the uncapped count exceeds the returned page.

---

## 9. Frontend — `CategoryPicker.svelte`

### 9.1 Component contract

```svelte
<CategoryPicker
  bind:value={categoryIds}             // string[] (two-way)
  {categories}                         // CategoryResponse[] (initial list, optional)
  {placeholder}                        // string (default "Search categories…")
  {includeUncategorized}               // boolean (default false)
  on:create={onCategoryCreated}        // dispatched on inline-create success
/>
```

### 9.2 Props and state

| Prop | Type | Default | Notes |
|---|---|---|---|
| `value` | `string[]` | `[]` (parent must declare `bind:value`) | Two-way bound. The component never mutates unrelated ids in the array. |
| `categories` | `CategoryResponse[]` | `[]` | Optional initial list. When the picker opens with no typed query and no `categories` provided, it calls `listCategoriesSearch({ query: "", limit })` to populate the popover. |
| `placeholder` | `string` | `"Search categories…"` | Trigger input placeholder. |
| `includeUncategorized` | `boolean` | `false` | When `true`, renders the `Uncategorized` pseudo-row in the result list and as a special chip. `ProductCatalogPage` uses `true`. `ProductForm` uses `false` (every product receives at least one real category or none intentionally). |

`UNCATEGORIZED_SENTINEL` is exported from `src/lib/categories.ts` and
imported by the picker.

### 9.3 A11y attributes

- Trigger: `role="combobox"`, `aria-haspopup="listbox"`,
  `aria-expanded={open}`, `aria-controls={listboxId}`,
  `aria-activedescendant={activeId || undefined}`, `autocomplete="off"`.
- Result list: `role="listbox"`, `id={listboxId}`.
- Each result row: `role="option"`, `id={rowId(row)}`,
  `aria-selected={isSelected(row)}`.
- Chips row: `role="list"` (optional, for screen readers); each chip
  with `aria-label="Remove {category.name}"` on the `×` button.
- "Clear all" link: `aria-label="Clear all selected categories"`.

### 9.4 Keyboard ergonomics

| Key | Behavior |
|---|---|
| `Tab` | Enter / leave the trigger without trapping focus inside the popover. |
| `Esc` (popover open) | Close without committing a typed query (the input keeps typed text). |
| `Esc` (popover closed) | No-op. |
| `ArrowDown` / `ArrowUp` | Move `activeId` through the result list. Clamps at top / bottom. |
| `Enter` | Select the active result (toggles it in `value`). |
| `Backspace` (input empty, focus in input) | Remove the last chip from `value`. |
| `Click outside` (popover open) | Close without committing. |
| `Click ×` (chip) | Remove that chip from `value`. |

### 9.5 Popover positioning

Mirrors the existing `caduxo-custom-date-picker` popover pattern:

```svelte
<div class="popover" use:popoverPositioning={triggerEl}>
  …
</div>
```

`popoverPositioning` is an `action` that reads
`triggerEl.getBoundingClientRect()`, computes an absolute-positioned
placement relative to the trigger (`top: rect.bottom + 4px; left:
rect.left`), and flips up when there isn't `>= 200px` of space below.

### 9.6 Inline create row

When the popover is open, the input's typed query does not match
**any** of the names in the result list (case-insensitive comparison
against both `name` and `id`), the bottom of the popover renders:

```html
<button class="create-row" type="button"
  on:click={handleCreate}
  disabled={creatingCategory}>
  Create "{trimmedQuery}"
</button>
```

`handleCreate`:

1. Trims and validates the typed query locally (non-empty after trim).
2. Calls `createCategory({ name: trimmedQuery })`.
3. On success: appends the new id to `value`, dispatches `create`
   with the new `CategoryResponse`, clears the input, keeps the
   popover open.
4. On `DuplicateField { field: "name" }`: renders a small inline error
   below the popover ("A category with a similar name already exists.
   Pick it from the list instead.") and does not close.
5. While the call is in flight, the button shows a spinner and is
   disabled.

There is **no** silent create path. The user must click the row.

### 9.7 `Uncategorized` pseudo-row

When `includeUncategorized={true}` and the popover is open, a
dedicated row appears at the top of the result list:

```html
<div role="option" class="pseudo-row" tabindex="-1"
  on:click={togglePseudo}>
  <span class="pseudo-tag">Uncategorized</span>
  <span class="pseudo-desc">Products with no active category.</span>
</div>
```

Selecting it pushes/pops `UNCATEGORIZED_SENTINEL` into `value`. The
chip is rendered with a distinct style (e.g., dashed border, italic
label) so users see it is a pseudo-state.

### 9.8 Behavior matrix

| Selection in `value` | Server filter behavior |
|---|---|
| `[]` (empty) | No filter. |
| `["c1"]` (real id) | ANY-of: `EXISTS ... category_id IN ('c1')`. |
| `["c1", "c2"]` | ANY-of. |
| `[UNCATEGORIZED_SENTINEL]` | `NOT EXISTS ...` to active categories. |
| `["c1", UNCATEGORIZED_SENTINEL]` | ANY-of real `OR NOT EXISTS ...` (see §5.2 SQL fragment). |

### 9.9 Reuse boundary

`CategoryPicker.svelte` is the **only** new front-end primitive. It is
consumed by:

- `ProductForm.svelte` (with `includeUncategorized={false}`)
- `ProductCatalogPage.svelte` (with `includeUncategorized={true}`)
- `ReportsPage.svelte` (with `includeUncategorized={true}`)

`ProductDetailPage.svelte` and `DashboardPage.svelte` do **not** use
the picker (read-only render). They compose the existing `.category-badge`
styles into a chip layout.

---

## 10. Frontend host adoptions

### 10.1 `src/components/ProductForm.svelte`

- State: `let categoryIds: string[] = [];`
- Init (edit mode): `categoryIds = initial.category_ids ?? [];`
- Submit payload: `category_ids: categoryIds`.
- Remove `let categoryId: string = "";`, the empty `<select
  bind:value={categoryId}>` (~line 326), and the
  `<ul class="category-pills">` (~line 330).
- Drop the inline-category `<div class="inline-category">` block
  (~lines 133–149) entirely; the picker owns inline-create.
- Wire a new `onCategoryCreated` prop listener so the parent (catalog
  page) gets a refresh callback when an inline create happens.
- The `categories` prop is no longer rendered directly by the form —
  it's passed to the picker as its initial list. The form's existing
  `localCategories` mirror remains for the `onCategoryCreated`
  callback (so the parent list updates without a refetch).

### 10.2 `src/components/ProductDetailPage.svelte`

Single badge → multiple badges or a comma-joined label.

```svelte
{#if detail.categories.length === 0}
  <span class="category-badge category-badge--empty">Uncategorized</span>
{:else}
  <span class="category-badges">
    {#each detail.categories as cat (cat.id)}
      <span class="category-badge">{cat.name}</span>
    {/each}
  </span>
{/if}
```

If the `detail.categories` array contains a single category, the
visual is identical to today's single badge. Empty state is a muted
`Uncategorized` badge to match the filter UX.

### 10.3 `src/components/DashboardPage.svelte`

Detail modal (~line 606):

```svelte
{#if detailProduct && detailProduct.categories.length === 0}
  <dd>Uncategorized</dd>
{:else if detailProduct}
  <dd>{detailProduct.categories.map(c => c.name).join(", ")}</dd>
{/if}
```

When the user opens a detail modal, the page already calls
`get_product` which returns `categories: Vec<CategoryResponse>`. No
extra round-trip is needed.

### 10.4 `src/components/ProductCatalogPage.svelte`

Add a Category filter at the top of the page:

```svelte
<CategoryPicker
  bind:value={categoryIds}
  includeUncategorized={true}
  placeholder="Filter by category…"
/>
```

`buildFilters()` includes `category_ids: categoryIds`. (Today the page
uses `searchProducts({ query: "" })` for the catalog. The slice **does
not** change the catalog's data source; filtering is client-side by
matching `category_ids` against the catalog rows. A future slice could
push the filter into the SQL but that is out of scope here, per the
proposal: "ProductCatalogPage loads `listCategories()` for the form …
gets the new `category_ids` for free when `ProductResponse` changes." )

Empty selection = no filter (no explicit All chip rendered).

### 10.5 `src/components/ReportsPage.svelte`

- State: `let categoryIds: string[] = [];` (was `categoryId: string`).
- Replace `<select bind:value={categoryId}>` (~line 353) with
  `<CategoryPicker bind:value={categoryIds} includeUncategorized={true} />`.
- `buildFilters()` (~line 143): `category_ids: categoryIds || null` is
  replaced by `category_ids: categoryIds.length > 0 ? categoryIds : null`.
- Filter summary (~line 265) renders:

```ts
if (f.category_ids && f.category_ids.length > 0) {
    const pseudoCount = f.category_ids.filter(id => id === "__uncategorized__").length;
    const realCount = f.category_ids.length - pseudoCount;
    const parts: string[] = [];
    if (realCount > 0) parts.push(`categories=${realCount}`);
    if (pseudoCount > 0) parts.push("categories=uncategorized");
    parts.push(`category=${f.category_ids.slice(0, 1)[0]?.slice(0, 8) ?? ""}…`);
    parts.push(`category=${f.category_ids.slice(0, 1)[0]?.slice(0, 8) ?? ""}…`);
}
```

(Trimmed for design clarity; the implementation renders a stable
short-hand that does not leak raw ids in screenshots. Today the
summary uses `category=<id.slice(0,8)>…`; the multi-category form
becomes `categories (N): <id1.slice(0,4)>, <id2.slice(0,4)>, …` with a
fallback to `categories (N)` if any id is the sentinel.)

### 10.6 TS DTO updates

`src/lib/products.ts`, `src/lib/dashboard.ts`, `src/lib/reports.ts`,
`src/lib/categories.ts` (new) mirror the Rust DTOs. Add:

```ts
export const UNCATEGORIZED_SENTINEL = "__uncategorized__";
```

Existing wrappers `listCategories`, `createCategory`, `updateCategory`
keep their names and signatures.

---

## 11. Filter semantics

### 11.1 ANY-of with EXISTS

Given `category_ids = [c1, c2]`:

```sql
WHERE EXISTS (
  SELECT 1 FROM product_categories pc
  WHERE pc.product_id = el.product_id
    AND pc.category_id IN (c1, c2)
)
```

A product that belongs to both `c1` and `c2` matches once. A product
that belongs only to `c1` matches once. A product in neither matches
zero times. The dashboard's existing sort + urgency grouping is
unchanged; the row shape stays distinct by `lot_id`.

### 11.2 ALL-of (rejected as the slice semantics)

ALL-of would require either:

- `WHERE (SELECT COUNT(*) FROM product_categories WHERE pc.product_id
  = el.product_id AND pc.category_id IN (…)) = N`, which has the
  correct shape but costs a count per product per row.
- Or a materialised intersection.

The slice does not land ALL-of. It's a future enhancement; the
design leaves room for it (the helper is generic) but the UI does not
expose it and the SQL only implements ANY-of.

### 11.3 Empty selection semantics

- `category_ids: None` or `Some(vec![])` → no filter.
- `Some(vec!["c1"])` → ANY-of for `c1` only (preserves back-compat for
  stored single-category filter presets if any exist; none exist
  today).

### 11.4 Uncategorized interpretation

"Uncategorized" means **zero rows in `product_categories` whose
`category.is_active = 1`**. A product that has only archived junction
rows is treated as uncategorized for filter purposes — the
`NOT EXISTS` subquery joins `categories` to filter on `is_active`.
A product that has no junction rows at all is also uncategorized.

When the selection includes `[UNCATEGORIZED_SENTINEL]`:

- Without any real ids: filter is purely "zero active junction rows".
- With real ids: filter is "any-of real OR zero active junction rows".

Composition rule for the user's intuition:

| User picks | What shows |
|---|---|
| `Dairy` only | Products in Dairy (whether or not also in others). |
| `Dairy, Bakery` | Products in Dairy or Bakery (no double count). |
| `Uncategorized` only | Products with no active category relation. |
| `Dairy, Uncategorized` | Products in Dairy, plus products with no active category relation. |
| Empty | All products (no filter). |

### 11.5 Aggregation buckets are unaffected

The dashboard's four urgency cards
(`expired` / `today` / `alert_window` / `next_30_days`) count from the
filtered-or-not rows that come out of `list_dashboard_lots`, then
`get_dashboard` runs the client-side preset matcher (see
`services/dashboard.rs::matches_preset`). When the user selects a
`preset` (one of the six quick-filter buttons), it overrides
`category_ids` only when the user is in the dashboard's local view;
on the reports side the two filters compose (category filter applied
to the SQL, urgency preset applied client-side).

A category-id filter on the dashboard surface therefore does not
change the urgency card counts (the cards count all rows; the table
narrows). This mirrors today's behavior for `store_id` filtering and
matches the project canonical `Dashboard urgency cards remain bucket
counts` requirement from `openspec/specs/.../spec.md`.

---

## 12. CSV / backup / restore / migration compatibility

### 12.1 CSV import (single-category)

`services::csv_io::resolve_category_name` is unchanged. `import_row`
passes the resolved category id as a one-element `Vec<String>` to
`create_product` / `update_product` (the new `category_ids` field).
Existing single-column CSV files continue to parse and produce
single-category products.

Multi-category CSV (semicolon-separated, multiple columns,
`category_2`) is **out of scope** for this slice. The proposal names
this decision explicitly.

### 12.2 CSV export (back-compat)

`list_all_products_for_export` joins through the junction and produces
a comma-joined category-names string per product. The CSV writer
emits a `category` column whose cell is the comma-joined names. A
single-name consumer (today's parser) reads the first name with
`split(',').next()` and gets the expected behavior; multi-name rows
just have a multi-name cell, which is the documented new format.

The wire shape's `category_ids: Vec<String>` is preserved in
`ProductResponse` for clients that want strict multi-name handling.

### 12.3 Backup / restore

The restore flow uses `VACUUM INTO` to copy the live DB file. After
restore, `open_pool` re-runs migrations. Pre-V4 backups:

- Do not have a `product_categories` table → V4 creates it and runs
  the idempotent back-fill from `products.category_id` (already
  present in the backup file).
- Do not have the case-fold fix-up applied (because they predate the
  concern) → V4's case-fold dedup step also applies to the back-filled
  categories. The migration logs the result.
- The legacy `products.category_id` column remains in the schema
  after V4. If the backup predates this column's drop — well, it
  doesn't, since this slice doesn't drop the column. So legacy
  data is reachable only through the back-fill and junction reads.

No new code in `services/backup_restore.rs`. The migrator is the
adapter. The slice's restore test (`restore_from_pre_v4_backup_…)
verifies the end-to-end path.

### 12.4 Pre-V4 CSV exports (forward compatibility for restores)

A user who exported products as CSV in V3 (single-category), then
imports the CSV in V4: works as today. The CSV file has a `category`
column; the import resolves the name case-insensitively to a category
id, places it in a one-element `category_ids` array, and the product
ends up with exactly one junction row. No data loss.

A user who imported products via V3 CSV without categories: works
unchanged. The product ends up with zero junction rows → uncategorized.

### 12.5 Migration compatibility (downgrade)

A V4 backup restored onto a V3 (pre-V4) build: the `product_categories`
table exists and contains rows but the V3 code doesn't query it.
Products appear uncategorized (the V3 code reads `products.category_id`
which is `NULL` in V4 writes). This is documented in the proposal's
rollback section as out of scope.

A V3 backup restored onto a V4 build: handled by §12.3.

---

## 13. Test plan

### 13.1 Backend (Rust) — `cargo test --manifest-path src-tauri/Cargo.toml --lib`

The slice must preserve the existing 2 pre-existing failures
(`preview_report_in_alert_window_returns_alert_lots`,
`preview_report_next_30_days_returns_30d_lots`) and not introduce new
failures. The slice **does** touch a few existing tests:

| Existing test | Change |
|---|---|
| `services/reports::tests::preview_report_custom_with_category_filter` | `category_id: Some(...)` → `category_ids: Some(vec![...])`; the assertion on `filters_used.category_id` becomes `filters_used.category_ids`. |
| `services/reports::tests::preview_report_metadata_captures_effective_filters` | Same rename. |
| `services/reports::tests::seed_report_fixture` | `category_id: Some(dairy.id.clone())` → `category_ids: Some(vec![dairy.id.clone()])` (call the service through the new shape). |
| `db/migrations::tests::*` (V4 entry tests) | New tests added (see §3.5). |
| `services/products::tests::create_and_list_categories` and other category tests | Unchanged; case-fold coverage tests are new. |

#### New tests (`src-tauri/src/db/repositories/products.rs::tests` and `services/products.rs::tests`)

Repository:

- `insert_product_writes_junction_rows`
- `insert_product_with_empty_category_ids_writes_no_junction`
- `insert_product_with_three_category_ids_writes_three_junction_rows`
- `update_product_replaces_full_junction_set`
- `update_product_from_two_to_zero_categories_removes_junction`
- `get_product_returns_category_ids_from_junction`
- `search_products_returns_category_ids_from_junction`
- `find_by_barcode_exact_returns_category_ids_from_junction`
- `find_by_sku_exact_returns_category_ids_from_junction`
- `list_product_category_ids_by_product_ids_empty_input_returns_empty_map`
- `list_product_category_ids_by_product_ids_groups_by_product_id`
- `list_all_products_for_export_includes_csv_only_category_names_string`
- `delete_product_cascades_junction_rows`
- `delete_category_with_junction_is_rejected`

Service:

- `create_product_rejects_nonexistent_category_id`
- `create_product_rejects_archived_category_id_by_default`
- `create_category_rejects_case_collision_with_existing` ("Dairy"
  then "dairy" → second is `DuplicateField { field: "name" }`)
- `update_category_rename_rejects_case_collision`
- `update_category_rename_to_own_current_name_succeeds`
- `get_product_resolves_category_ids_to_categories_vec`
- `get_product_with_no_categories_returns_empty_categories_vec`

Migration (`db/migrations::tests`):

- See §3.5.

Dashboard filter (`db/repositories/dashboard::tests`):

- `list_dashboard_lots_with_no_category_filter_returns_all_lots` (default)
- `list_dashboard_lots_with_empty_category_ids_returns_all_lots`
- `list_dashboard_lots_with_one_category_id_returns_any_of_lots`
- `list_dashboard_lots_with_two_category_ids_returns_any_of_lots_without_double_count`
- `list_dashboard_lots_with_uncategorized_sentinel_returns_products_with_zero_junction`
- `list_dashboard_lots_with_real_id_and_uncategorized_composes_correctly`
- `list_dashboard_lots_with_archived_category_id_in_selection_returns_no_lots_after_archive`

Search service (`services/categories::search::tests`):

- `search_empty_query_returns_active_categories`
- `search_prefix_match_for_partial_query`
- `search_falls_back_to_substring_when_no_prefix`
- `search_excludes_archived_categories`
- `search_respects_limit`
- `search_has_more_true_when_total_exceeds_limit`

Restore (`services/backup_restore::tests`):

- `restore_from_pre_v4_backup_applies_v4_backfill_in_situ`
- (Uses `create_minimal_caduxo_db` plus a V3-only schema fixture;
  after restore + open_pool → migrations, `product_categories` is
  populated from the seeded `products.category_id` rows.)

Reports service (`services/reports::tests`):

- `preview_report_custom_with_multiple_categories_returns_any_of_lots`
- `preview_report_custom_with_uncategorized_returns_zero_category_lots`
- `preview_report_custom_with_real_and_uncategorized_composes`

These three new tests replace `preview_report_custom_with_category_filter`
in shape (it's renamed / generalized; the underlying assertion is the
same `category_ids` path). The original is preserved (as a
single-category case) so the test surface stays coherent.

### 13.2 Frontend — `npx svelte-check` + `npm run build`

- `npx svelte-check --workspace . --threshold error` must pass.
  - All consumers migrated in the same slice; the rename `category_id`
    → `category_ids` shows up in `src/lib/products.ts` /
    `dashboard.ts` / `reports.ts` and is enforced by svelte-check via
    TypeScript-friendly IPC binding.
- `npm run build` must pass.

### 13.3 Manual smoke matrix (Linux/WebKitGTK + Windows/WebView2)

Per canonical `Engineering safety > tests accompany implementation`
requirement, the slice notes list these manual smoke steps. They are
recorded in `apply-progress.md` after the slice applies.

1. Open `ProductForm`, type "Dai", see `Dairy` in the result list.
   Select it; the chip "Dairy" appears above the input.
2. Type "Bak", see `Bakery`. Select it; now two chips.
3. Click × on "Dairy"; it disappears from the selection.
4. Click `Clear all`; chips clear; `value === []`.
5. Type "abc", no match; `Create "abc"` row appears. Click it. The
   picker calls `create_category` and the new chip appears.
6. Open `ProductCatalogPage`. Select `Dairy` from the filter. Verify
   products in `Dairy` show; others do not.
7. Open the `Uncategorized` row in the same picker (Catalog uses
   `includeUncategorized={true}`). Verify products with no active
   categories appear.
8. Open `ReportsPage`. Select `Dairy, Bakery`. Render a custom report.
   Verify lots whose product is in either appear exactly once each
   (no double count for products in both).
9. Verify filter summary shows `categories (2): <id1.slice(0,4)>,
   <id2.slice(0,4)>` or similar (no raw id leak).
10. Empty the selection. Verify the report has no category filter
    (summary no longer mentions categories).
11. Archive a category via `update_category({ is_active: false })` from
    a future code path (today this is exercised through existing
    admin surfaces / dev console; for the slice the manual test calls
    the command directly). Verify the picker no longer surfaces it
    and any product with only this category now appears under
    `Uncategorized` in the filter.
12. Open `ProductDetailPage` for a product with two categories.
    Verify two `.category-badge` chips render.
13. Open the dashboard, click a lot row, open the detail modal.
    Verify comma-joined names render (or a list, whichever the
    implementation chose).
14. Open the calendar tab. Verify it inherits the category filter
    implicitly (it currently has no filter surface; this smoke step
    is a regression check that no UI fell off).
15. **Restore-from-backup with a pre-V4 fixture**: with the app's
    actual DB at V4, simulate restore by copying in a backup file that
    was created before V4 applied (use the test fixture from
    `services::backup_restore::tests`). Confirm the pool reopens,
    V4 applies, junction rows are populated from the legacy column,
    and the picker works.
16. **CSV round-trip**: export a product that has two categories via
    `export_products_csv`. Open the file in a CSV viewer; verify the
    `category` cell is the comma-joined names. Re-import the same
    file with `Skip` strategy; verify the import is a no-op (or with
    `Update` strategy, verify it writes a single-category product with
    only the first name).
17. **Keyboard ergonomics**: Tab into the trigger, press ArrowDown
    repeatedly, press Enter to select, press Esc to close. Verify the
    chip appears, the popover closes, focus returns to the trigger.

### 13.4 Logging discipline

No product data, category names, or junction ids are logged. The
slice's tests use `tracing_test` patterns only where needed;
otherwise `println!` is fine for test assertions.

---

## 14. Spec delta summary

`openspec/specs/caduxo-expiry-tracker/spec.md` updates (apply-phase
carries these out after the implementation lands):

| Capability | Requirement | Action |
|---|---|---|
| Categories | `editable category list` | **MODIFIED** in place to absorb the multi-category model: products may belong to one, many, or zero categories; unassigned is valid; archived categories are hidden from pickers but preserved in history. |
| Categories | `multi-category product model` | **ADDED**: new requirement covering the `product_categories` junction table, composite uniqueness, ON DELETE CASCADE / RESTRICT semantics, and the case-fold guard at the service layer. |
| Categories | `category_filter_sentinel_uncategorized` | **ADDED** scenario under `multi-category product model`: lots whose product has zero junction rows to active categories are included when the picker selects the `Uncategorized` pseudo-row. |
| Product catalog | `mandatory unique SKU` | **MODIFIED** to clarify that `category_ids` is a list of zero or more ids, not a single nullable FK. |
| Reports | `report filters` | **MODIFIED** to describe `category_ids` (any-of), the `Uncategorized` pseudo-row, and the SQL-level `EXISTS` filter. Default `None` / `Some(vec![])` means no filter. |
| Reports | `multi-category filter any-of semantics` | **ADDED** scenario: lots whose product belongs to **any** of the selected categories appear exactly once. |
| Dashboard | `operational main screen` | **MODIFIED** to add a `category_ids` filter (any-of, default no filter, `Uncategorized` allowed). |
| Data persistence and safety | `migrations` | **MODIFIED** to note V4 introduces `product_categories` and the case-fold dedup step. |

`docs/prd.md` adds a small multi-category-picker note (≤ 30 LOC).

---

## 15. Implementation order (apply-phase)

For the parent orchestrator / `tasks.md` plan. The order ensures each
step is independently testable.

1. **Backend — V4 migration.** Add `MIGRATIONS` entry V4 with DDL +
   back-fill + case-fold dedup SQL. Add the V4 migration tests in
   `db::migrations::tests`. Run `cargo test` on the migration file.
   Touches `src-tauri/src/db/migrations.rs` only.

2. **Backend — Junction helpers.** Add
   `list_product_category_ids_by_product_ids`,
   `find_category_by_name_ci`, and the per-product junction SELECT
   helper to `db::repositories/products.rs`. Add focused tests.

3. **Backend — Repo write paths.** Update `insert_product`,
   `update_product` to use the junction. Update `get_product`,
   `search_products`, `find_by_barcode_exact`, `find_by_sku_exact`,
   `list_all_products_for_export` to read from the junction.

4. **Backend — Service write paths.** Update
   `services::products::{create_product, update_product,
   get_product, create_category, update_category}` for the new shape.

5. **Backend — Service report / dashboard.** Remove
   `services::reports::filter_by_category`. Update
   `to_dashboard_filters` to forward `category_ids`. Update
   `DashboardFilters` and `ReportFilters` DTOs.

6. **Backend — Dashboard filter SQL.** Update
   `db::repositories/dashboard.rs::list_dashboard_lots` to apply the
   `EXISTS` filter.

7. **Backend — Search service / command.** Add
   `services::categories::search` and `commands/products.rs::list_categories_search`.
   Register in `lib.rs`.

8. **Backend tests.** Add / update the §13.1 tests.

9. **Frontend — TS DTOs.** Update `src/lib/products.ts`,
   `src/lib/dashboard.ts`, `src/lib/reports.ts`. Add
   `src/lib/categories.ts`.

10. **Frontend — `CategoryPicker.svelte`.** Implement the new
    primitive with the §9 contract.

11. **Frontend — Host adoptions.** Update `ProductForm.svelte`,
    `ProductDetailPage.svelte`, `DashboardPage.svelte`,
    `ProductCatalogPage.svelte`, `ReportsPage.svelte`.

12. **Verify.** `npx svelte-check`, `npm run build`, `cargo test`,
    manual smoke per §13.3.

13. **Spec delta.** Update `openspec/specs/.../spec.md` per §14.

14. **Apply-progress ledger.** Record the manual smoke matrix, the
    backend test commands, and the svelte-check / npm build outcomes.

---

## 16. Risks & mitigations

Carried from the proposal and refined with the design decisions.

| # | Risk | Impact | Mitigation |
|---|---|---|---|
| R1 | Total LOC exceeds the 400/800 project review budgets | Reviewer fatigue; shallow review | User typed a 3,000-line session review budget for this change; the design stays at ~1,400–2,400 LOC. Mitigate further with the single-source-of-truth primitives (§2.3) so the diff is sized by the new primitives + tests, not by parallel re-implementations. |
| R2 | SQLite `ALTER TABLE … DROP COLUMN` fails on the user's build | Migration fails; rollout blocked | The design **does not** drop the legacy column. The column stays in place as ignored data. No migration-time DROP COLUMN. This eliminates the risk class entirely. |
| R3 | Pre-existing case-only duplicates in `categories` (`Dairy` vs `dairy`) | Junction back-fill creates duplicate `(product_id, category_id)` pairs, blocked by PK → back-fill silently drops the duplicate | V4's case-fold dedup picks the **oldest** row as canonical, archives younger ones (`is_active = 0`), remaps junction rows to the canonical id, and remaps the legacy `products.category_id` column too. Migration logs counts. The service-layer guard prevents new duplicates. The risk is fully mitigated. |
| R4 | ANY-of semantics misalign with user intuition | Users expect intersection when they meant union, or vice versa | **ANY-of** is the confirmed default. The picker is labeled "Categories" with no math vocabulary. SQL uses `EXISTS` so there's no double-count incentive. ALL-of is a future enhancement and explicitly out of scope. |
| R5 | Duplicate-count risk in reports | A product in two selected categories counted twice | `EXISTS` at SQL level, **never JOIN**. Rows remain distinct by `lot_id`. The slice removes the per-row post-filter that introduced duplicate-count risk by accident. |
| R6 | N+1 regression in detail-bundle reads | The detail modal regresses when products have many categories | `get_product` resolves the full set in one in-clause SELECT (or the batched helper for cross-product lookups). The N+1 risk disappears with the move to junction reads + a single resolution pass. |
| R7 | New `list_categories_search` command adds command surface | Registry grows; review surface grows | The command follows the existing `list_categories` shape (one entry in `lib.rs`, one service module). Frontend wrapper in `src/lib/categories.ts`. Mirrors the existing pattern; the review surface is small. |
| R8 | Restore-from-backup misses pre-V4 backups | Restore silently drops categories | The migrator runs after every restore (`open_pool` → `run_migrations`). Pre-V4 backups automatically pick up V4 + back-fill + case-fold dedup. **No new code is needed in `services/backup_restore.rs`.** The slice's restore test verifies this end-to-end. |
| R9 | `Uncategorized` pseudo-row mistaken for a real category | A user accidentally edits or deletes the sentinel | Pseudo-row is rendered by the picker; no `id` exists in the `categories` table. The sentinel is a TS constant; it cannot be persisted as a junction row. The picker filters it out of `list_categories_search` results and out of `Value()` programmatically (the picker maintains a stable mapping). |
| R10 | Inline-create adds an arbitrary category name | Controlled-creation product rule violated | Inline-create is **explicit**: the user clicks `Create "X"`. No silent arbitrary-text path. Service-layer case-fold guard rejects duplicates. Empty / whitespace queries are not creatable. |
| R11 | Archived categories appear in the picker and confuse the user | Picker pollutes | Picker filters to `is_active = 1` only. Archived categories are not exposed. Archive / restore is admin work, out of scope for this slice. |
| R12 | Popover positioning clips on small windows | Picker becomes unusable | Reuse the date picker's flip-up math (`getBoundingClientRect` + overflow flip). The popover action is implemented and tested in the date picker prior slice. |
| R13 | Esc inside the search input discards a typed query unexpectedly | User loses confidence | Esc closes the popover without committing the typed query; the input keeps the typed text and re-opens with the same query on next open. Documented in the picker UX. |
| R14 | `category_id` rename breaks an unsynced frontend type | Compile errors in dev / build | All consumers migrate in the same slice; TS types mirror Rust DTOs in lockstep. No deprecation cycle because the app is pre-1.0 with no external consumer. `npx svelte-check` is the canonical gate. |
| R15 | Spec delta drifts from implementation | Capability statement vs. code mismatch | New requirement `multi-category product model` is the canonical place. Pointer edits under existing capabilities are minimal. The next `tasks.md` will execute the spec delta as its own ordered step (last step in §15) so it follows the implementation. |
| R16 | Frontend manual verification only | Regression risk between slices | Recorded in the slice notes and in the apply-progress ledger per §13.3; the canonical `Engineering safety > tests accompany implementation` requirement's manual-verify clause acknowledges this for the slice. |
| R17 | Pre-existing 2-failure baseline shifts | One more or one fewer test "failing" before/after | The slice preserves the existing 2 pre-existing failures (`preview_report_in_alert_window_returns_alert_lots`, `preview_report_next_30_days_returns_30d_lots`) and adds new tests for the new behavior. The verify report records the baseline before and after. |
| R18 | `preview_report_custom_with_category_filter` and `preview_report_metadata_captures_effective_filters` break on rename | Compile / test failures | Both tests are updated alongside `seed_report_fixture` (rename `category_id` → `category_ids`). The two-existing-failures baseline is preserved (those tests are unrelated). |
| R19 | `seed_report_fixture` uses single-FK seeding | Test fixtures break | The fixture is updated to seed via the service-layer API (`create_product({ category_ids })`), not via direct SQL inserts. Same shape as today, just through the service. |
| R20 | `EXISTS` (with `el.product_id`) on a large `expiry_lots` × `product_categories` cross product is slow | Latency degradation for large catalogs | The composite PK `(product_id, category_id)` plus the `idx_product_categories_category` index make the EXISTS subquery O(1) per lot on average. The `category_id IN (…)` branch uses the index. Documented as a future consideration if a catalog with 10k+ categories per product emerges — the schema and SQL are shaped to support it. |
| R21 | `UNCATEGORIZED_SENTINEL` string collides with a legitimate category id | Sentinel treated as a real id | The sentinel string `__uncategorized__` is unlikely to be a real UUID, but `CategoryResponse.id` is a UUID in practice. The frontend adds a defensive check: any id starting with `__` is filtered out of `list_categories_search` results. Backend ignores any `products.category_id` reference to the sentinel (no code path creates one). |
| R22 | `category_unique_error` pattern reused for case-fold collisions | Misleading error message | The guard runs **before** the INSERT and rejects the call with `DuplicateField { field: "name", value: <typed> }` directly. The UNIQUE-violation path stays the canonical race-condition safety net for concurrent writers. The error message stays `"uniqueness violation: categories.name = X"` because the SQLite UNIQUE constraint is the safety net, not the guard. |
| R23 | `value` two-way binding mutates the parent's array on read | Unexpected parent updates | `bind:value={categoryIds}` in Svelte is "two-way" at the property level: the parent reassigns the variable on every internal mutation. The parent should pass a fresh reference (`value = [...categoryIds]`) or use a derived state. Documented in the picker's component header. |
| R24 | Chip removal via Backspace feels like text deletion | Confused users | The picker is clear in its keyboard contract: Backspace in an empty input removes the last chip. The input does not have placeholder text inside the chip row; chips are above the input. The behavior is documented in the picker UX help text (Tooltip / aria-label). |

---

## 17. Decision summary

| Question | Decision |
|---|---|
| Schema: drop legacy `products.category_id` or leave it? | **Leave it.** No `ALTER TABLE … DROP COLUMN`. The column is runtime-ignored (§2.2). Safe single-tenant migration; trivial rollback; zero compatibility risk. |
| Filter semantics: ANY-of or ALL-of? | **ANY-of** with `EXISTS`. ALL-of is out of scope for this slice. |
| `EXISTS` vs `JOIN` for filter | **`EXISTS`** always. No JOIN, no double count. |
| `Uncategorized` representation | Sentinel id `UNCATEGORIZED_SENTINEL = "__uncategorized__"`, never persisted. Backend translates the sentinel into `NOT EXISTS` (zero active junction rows). |
| Empty `category_ids` semantics | `None` or `Some(vec![])` means "no filter". |
| Case-fold dedup resolution | Oldest row by `created_at, id` is canonical; younger rows archived; junction remapped; legacy column remapped. |
| CSV import | Single-category per row only. Multi-category CSV is out of scope. |
| CSV export | Comma-joined category-names cell in the existing `category` column. Back-compat for single-name parsers. |
| Restore-from-backup | Automatic via migrator on `open_pool`. No new code in `services/backup_restore.rs`. |
| Picker primitive location | `src/components/inputs/CategoryPicker.svelte`. The `inputs/` directory is new; the picker is the first tenant. |
| Popover positioning | `position: absolute` with `getBoundingClientRect` + flip-up math (mirroring `caduxo-custom-date-picker`). |
| Inline-create flow | Explicit `Create "X"` button; the picker calls `createCategory` and adds the new id. No silent arbitrary text. |
| Frontend test harness | None. Manual smoke on Linux/WebKitGTK + Windows/WebView2 per canonical `Engineering safety > tests accompany implementation`. |
| Delivery shape | Single PR within the 3,000-line session review budget. No chained-PR split. No `size:exception` needed. |

---

## 18. Open follow-ups (out of scope, recorded for the next slice)

These are deliberate, user-confirmed non-goals. They are recorded
here so the next slice's explore does not re-litigate them.

- **Category admin page.** Virtualized list, archive / restore,
  rename, product-counts-per-category. Future slice.
- **Multi-category CSV import.** Semicolon-separated names or
  `category_2`, `category_3` columns. Future slice.
- **ALL-of filter semantics.** Out of scope; the SQL primitives
  support it but the UI and wire shape do not.
- **Hierarchy / subcategories.** No parent_id; no tree rendering.
- **Per-row urgency classifier changes.** Unchanged; the dashboard
  cards stay bucket counts and the filter matcher stays predicate-
  based.
- **Vitest / Playwright harness.** Out of scope per project config.
- **Fuzzy search for categories.** Out of scope; case-insensitive
  prefix + substring fallback only.
- **Maximum categories per product.** None. The schema allows N.

---

## 19. Acceptance checklist

The slice is accepted for delivery when:

- All §3.5 migration tests pass.
- All §13.1 Rust tests pass, including the new tests.
- The pre-existing 2-failure baseline is preserved
  (`preview_report_in_alert_window_returns_alert_lots`,
  `preview_report_next_30_days_returns_30d_lots`).
- `npx svelte-check --workspace . --threshold error` passes.
- `npm run build` passes.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` passes
  the same baseline plus the new tests.
- Manual smoke per §13.3 is recorded in `apply-progress.md`.
- The spec delta per §14 lands in
  `openspec/specs/caduxo-expiry-tracker/spec.md`.
- `docs/prd.md` is updated with multi-category references.
- The slice's source diff is ≤ 3,000 lines (user-typed review
  budget for this change).

If any item fails, the verify report blocks delivery and the slice
owner iterates.
