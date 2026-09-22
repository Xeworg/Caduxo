//! Repository for product catalog persistence: categories, products, barcodes.
//!
//! ## Runtime invariant (V4 onwards)
//!
//! The `product_categories` junction table is the canonical source of truth for
//! product-category membership. The legacy `products.category_id` column is
//! schema-present but **never read or written by runtime code** after V4
//! applies. This comment is the split-brain prevention guard; see design §2.2.

use std::collections::HashMap;

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::dto::products::{
    CategoryCreate, CategoryResponse, CategoryUpdate, ProductBarcodeCreate, ProductBarcodeResponse,
    ProductCreate, ProductResponse, ProductSearchQuery, ProductSearchResult, ProductUpdate,
};
use crate::dto::unit_definitions::UnitKind;

// ============================================================
// Categories
// ============================================================

/// Inserts a new active category.
pub async fn insert_category(
    pool: &SqlitePool,
    input: &CategoryCreate,
) -> Result<CategoryResponse, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO categories (id, name, is_active, created_at, updated_at)
        VALUES ($1, $2, 1, $3, $4)
        "#,
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(CategoryResponse {
        id,
        name: input.name.clone(),
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Returns all active categories, ordered alphabetically.
pub async fn list_active_categories(
    pool: &SqlitePool,
) -> Result<Vec<CategoryResponse>, sqlx::Error> {
    sqlx::query_as::<_, CategoryResponse>(
        r#"
        SELECT id, name, is_active, created_at, updated_at
        FROM categories
        WHERE is_active = 1
        ORDER BY name ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Returns every category (active first), ordered alphabetically.
pub async fn list_all_categories(pool: &SqlitePool) -> Result<Vec<CategoryResponse>, sqlx::Error> {
    sqlx::query_as::<_, CategoryResponse>(
        r#"
        SELECT id, name, is_active, created_at, updated_at
        FROM categories
        ORDER BY is_active DESC, name ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Fetches a single category by id (active or inactive).
pub async fn get_category(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<CategoryResponse>, sqlx::Error> {
    sqlx::query_as::<_, CategoryResponse>(
        r#"
        SELECT id, name, is_active, created_at, updated_at
        FROM categories
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Case-insensitive exact match for a category name.
/// Used by the service-layer case-fold guard.
pub async fn find_category_by_name_ci(
    pool: &SqlitePool,
    normalized: &str,
) -> Result<Option<CategoryResponse>, sqlx::Error> {
    sqlx::query_as::<_, CategoryResponse>(
        r#"
        SELECT id, name, is_active, created_at, updated_at
        FROM categories
        WHERE lower(name) = lower($1)
        LIMIT 1
        "#,
    )
    .bind(normalized)
    .fetch_optional(pool)
    .await
}

/// Updates a category by id and returns the refreshed row, or `None` if the
/// row did not exist.
pub async fn update_category(
    pool: &SqlitePool,
    input: &CategoryUpdate,
) -> Result<Option<CategoryResponse>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    let affected = sqlx::query(
        r#"
        UPDATE categories
        SET name = $1, is_active = $2, updated_at = $3
        WHERE id = $4
        "#,
    )
    .bind(&input.name)
    .bind(i32::from(input.is_active))
    .bind(&now)
    .bind(&input.id)
    .execute(pool)
    .await?;

    if affected.rows_affected() == 0 {
        return Ok(None);
    }
    get_category(pool, &input.id).await
}

// ============================================================
// Junction helpers
// ============================================================

/// Reads the full category-id set for each product id in the input slice.
/// Returns a map from product_id → Vec<category_id>. Used for batch reads
/// so callers can resolve `category_ids` for many products in one round-trip.
pub async fn list_product_category_ids_by_product_ids(
    pool: &SqlitePool,
    product_ids: &[String],
) -> Result<HashMap<String, Vec<String>>, sqlx::Error> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for product_id in product_ids {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT category_id FROM product_categories WHERE product_id = $1")
                .bind(product_id)
                .fetch_all(pool)
                .await?;
        out.insert(product_id.clone(), rows.into_iter().map(|(c,)| c).collect());
    }
    Ok(out)
}

/// Reads the category ids for a single product from the junction table.
async fn get_product_category_ids(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT category_id FROM product_categories WHERE product_id = $1")
            .bind(product_id)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|(c,)| c).collect())
}

/// Writes the full set of category ids for a product into the junction table.
/// Uses INSERT OR IGNORE so re-assigning to the same category is a no-op
/// (the composite PK handles integrity). Called within a transaction.
async fn write_junction(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    product_id: &str,
    category_ids: &[String],
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    for cid in category_ids {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO product_categories (product_id, category_id, created_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(product_id)
        .bind(cid)
        .bind(&now)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

// ============================================================
// Products
// ============================================================

/// Inserts a new active product.
///
/// The junction write is inside the transaction so a junction-write failure
/// rolls back the product insert. The legacy `products.category_id` column is
/// **not written** — see the module-level invariant comment.
pub async fn insert_product(
    pool: &SqlitePool,
    input: &ProductCreate,
    default_unit_id: Option<String>,
    unit_type: Option<String>,
) -> Result<ProductResponse, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

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
    .bind(&default_unit_id)
    .bind(&unit_type)
    .bind(input.default_alert_days_before)
    .bind(&input.notes)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    let category_ids = input.category_ids.clone().unwrap_or_default();
    write_junction(&mut tx, &id, &category_ids).await?;

    tx.commit().await?;

    // Fetch back with resolved category_ids from the junction.
    get_product(pool, &id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

/// Fetches a single product by id (active or archived), including the
/// joined catalog columns `default_unit_id` and `unit_type`.
/// `category_ids` is resolved from the junction table.
pub async fn get_product(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<ProductResponse>, sqlx::Error> {
    let row: Option<RawProductRow> = sqlx::query_as(
        r#"
        SELECT id, sku, description, category_id, default_unit,
               default_unit_id, unit_type,
               default_alert_days_before, notes, is_active,
               created_at, updated_at
        FROM products
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let category_ids = get_product_category_ids(pool, id).await?;
            Ok(Some(r.into_response(category_ids)))
        }
        None => Ok(None),
    }
}

/// Updates a product by id and returns the refreshed row, or `None` if the
/// row did not exist. Performs a full-set junction replacement inside the
/// same transaction: DELETE existing + INSERT new.
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
    .bind(&default_unit_id)
    .bind(&unit_type)
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

    // Full set: delete all existing junction rows then write the new set.
    sqlx::query("DELETE FROM product_categories WHERE product_id = $1")
        .bind(&input.id)
        .execute(&mut *tx)
        .await?;

    let category_ids = input.category_ids.clone().unwrap_or_default();
    write_junction(&mut tx, &input.id, &category_ids).await?;

    tx.commit().await?;

    get_product(pool, &input.id).await
}

/// Soft-archives a product (`is_active = 0`). Returns `true` if a row was
/// updated, `false` if the product did not exist.
pub async fn archive_product(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let affected = sqlx::query(
        r#"
        UPDATE products SET is_active = 0, updated_at = $1 WHERE id = $2
        "#,
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(affected.rows_affected() > 0)
}

/// Returns the catalog-resolved unit kind for a product, or `None` when the
/// product has no catalog link (legacy / uncatalogued).
///
/// Lightweight accessor used by the lot movement service to enforce
/// unit-aware quantity validation without loading the full `ProductResponse`.
/// Returns `Ok(None)` when the product does not exist, so callers can decide
/// how to surface the missing-product case (typically a `NotFound` error).
pub async fn get_product_unit_kind(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<Option<UnitKind>, sqlx::Error> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT unit_type FROM products WHERE id = $1")
            .bind(product_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|(raw,)| {
        raw.and_then(|k| match k.as_str() {
            "integer" => Some(UnitKind::Integer),
            "decimal" => Some(UnitKind::Decimal),
            _ => None,
        })
    }))
}

/// Searches products by description, SKU, or barcode (case-insensitive
/// `LIKE` match). An empty query returns all products.
/// `category_ids` is resolved from the junction table for each hit.
pub async fn search_products(
    pool: &SqlitePool,
    query: &ProductSearchQuery,
) -> Result<Vec<ProductSearchResult>, sqlx::Error> {
    let pattern = format!("%{}%", query.query.trim());

    let rows: Vec<RawProductRow> = sqlx::query_as(
        r#"
        SELECT id, sku, description, category_id, default_unit,
               default_unit_id, unit_type,
               default_alert_days_before, notes, is_active,
               created_at, updated_at
        FROM products
        WHERE description LIKE $1
           OR sku           LIKE $1
           OR EXISTS (
                SELECT 1 FROM product_barcodes pb2
                WHERE pb2.product_id = products.id AND pb2.barcode LIKE $1
           )
        ORDER BY description ASC
        "#,
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let product_ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    let junction = list_product_category_ids_by_product_ids(pool, &product_ids).await?;
    let primary_barcodes = list_primary_barcodes_for_products(pool, &product_ids).await?;

    let mut results = Vec::with_capacity(rows.len());
    for r in rows {
        let id = r.id.clone();
        let category_ids = junction.get(&id).cloned().unwrap_or_default();
        let primary_barcode = primary_barcodes.get(&id).cloned();
        results.push(r.into_search_result(category_ids, primary_barcode));
    }
    Ok(results)
}

/// Batch-fetches the primary barcode for each product in `product_ids`.
/// Returns a map from product_id → barcode value. Products with no barcode
/// or no primary barcode are absent from the map.
async fn list_primary_barcodes_for_products(
    pool: &SqlitePool,
    product_ids: &[String],
) -> Result<std::collections::HashMap<String, String>, sqlx::Error> {
    if product_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let mut out = std::collections::HashMap::new();
    for id in product_ids {
        // Fetch the primary barcode (is_primary = 1) for this product.
        // `list_barcodes` returns rows ordered by is_primary DESC, so the first
        // row is always the primary if one exists.
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT barcode FROM product_barcodes WHERE product_id = $1 AND is_primary = 1 LIMIT 1",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;
        if let Some((barcode,)) = rows.into_iter().next() {
            out.insert(id.clone(), barcode);
        }
    }
    Ok(out)
}

// ============================================================
// Barcodes
// ============================================================

/// Inserts a barcode. If `is_primary` is `true`, any existing primary barcode
/// for the same product is demoted first so that each product has at most one
/// primary barcode.
pub async fn insert_barcode(
    pool: &SqlitePool,
    input: &ProductBarcodeCreate,
) -> Result<ProductBarcodeResponse, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    if input.is_primary {
        sqlx::query(
            r#"
            UPDATE product_barcodes SET is_primary = 0 WHERE product_id = $1
            "#,
        )
        .bind(&input.product_id)
        .execute(pool)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO product_barcodes
            (id, product_id, barcode, barcode_type, is_primary, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(&id)
    .bind(&input.product_id)
    .bind(&input.barcode)
    .bind(&input.barcode_type)
    .bind(i32::from(input.is_primary))
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(ProductBarcodeResponse {
        id,
        product_id: input.product_id.clone(),
        barcode: input.barcode.clone(),
        barcode_type: input.barcode_type.clone(),
        is_primary: input.is_primary,
        created_at: now,
    })
}

/// Lists all barcodes for a product (primary first, then by creation order).
pub async fn list_barcodes(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<Vec<ProductBarcodeResponse>, sqlx::Error> {
    sqlx::query_as::<_, ProductBarcodeResponse>(
        r#"
        SELECT id, product_id, barcode, barcode_type, is_primary, created_at
        FROM product_barcodes
        WHERE product_id = $1
        ORDER BY is_primary DESC, created_at ASC
        "#,
    )
    .bind(product_id)
    .fetch_all(pool)
    .await
}

/// Deletes a barcode by id. Returns `true` if a row was removed.
pub async fn remove_barcode(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let affected = sqlx::query(
        r#"
        DELETE FROM product_barcodes WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(affected.rows_affected() > 0)
}

// ============================================================
// Exact lookups for scanner workflow
// ============================================================

/// Looks up a product by exact barcode. Returns the product row (active or
/// archived) if the barcode exists, `None` otherwise.
pub async fn find_by_barcode_exact(
    pool: &SqlitePool,
    barcode: &str,
) -> Result<Option<ProductSearchResult>, sqlx::Error> {
    let row: Option<RawProductRow> = sqlx::query_as(
        r#"
        SELECT products.id, sku, description, category_id, default_unit,
               default_unit_id, unit_type,
               default_alert_days_before, notes, is_active,
               products.created_at, products.updated_at
        FROM products
        INNER JOIN product_barcodes pb ON pb.product_id = products.id
        WHERE pb.barcode = $1
        "#,
    )
    .bind(barcode)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let category_ids = get_product_category_ids(pool, &r.id).await?;
            // Primary barcode: look up the primary for this product.
            let primary_barcodes =
                list_primary_barcodes_for_products(pool, &[r.id.clone()]).await?;
            let primary_barcode = primary_barcodes.get(&r.id).cloned();
            Ok(Some(r.into_search_result(category_ids, primary_barcode)))
        }
        None => Ok(None),
    }
}

/// Looks up a product by exact SKU. Returns the product row (active or
/// archived) if the SKU exists, `None` otherwise.
pub async fn find_by_sku_exact(
    pool: &SqlitePool,
    sku: &str,
) -> Result<Option<ProductSearchResult>, sqlx::Error> {
    let row: Option<RawProductRow> = sqlx::query_as(
        r#"
        SELECT id, sku, description, category_id, default_unit,
               default_unit_id, unit_type,
               default_alert_days_before, notes, is_active,
               created_at, updated_at
        FROM products
        WHERE sku = $1
        "#,
    )
    .bind(sku)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let category_ids = get_product_category_ids(pool, &r.id).await?;
            let primary_barcodes =
                list_primary_barcodes_for_products(pool, &[r.id.clone()]).await?;
            let primary_barcode = primary_barcodes.get(&r.id).cloned();
            Ok(Some(r.into_search_result(category_ids, primary_barcode)))
        }
        None => Ok(None),
    }
}

/// Scanner-only: looks up an **active** product by exact barcode. Returns
/// the product row only when `is_active = 1`; archived products are
/// ignored. The existing `find_by_barcode_exact` (used by
/// `find_product_by_scan` for the dashboard scan/search surface) is
/// intentionally left untouched because the dashboard must keep surfacing
/// archived products so the user can locate them in the catalog.
pub async fn find_active_product_by_barcode_exact(
    pool: &SqlitePool,
    barcode: &str,
) -> Result<Option<ProductSearchResult>, sqlx::Error> {
    let row: Option<RawProductRow> = sqlx::query_as(
        r#"
        SELECT products.id, sku, description, category_id, default_unit,
               default_unit_id, unit_type,
               default_alert_days_before, notes, is_active,
               products.created_at, products.updated_at
        FROM products
        INNER JOIN product_barcodes pb ON pb.product_id = products.id
        WHERE pb.barcode = $1 AND products.is_active = 1
        "#,
    )
    .bind(barcode)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let category_ids = get_product_category_ids(pool, &r.id).await?;
            let primary_barcodes =
                list_primary_barcodes_for_products(pool, &[r.id.clone()]).await?;
            let primary_barcode = primary_barcodes.get(&r.id).cloned();
            Ok(Some(r.into_search_result(category_ids, primary_barcode)))
        }
        None => Ok(None),
    }
}

/// Scanner-only: looks up an **active** product by exact SKU. Returns the
/// product row only when `is_active = 1`; archived products are ignored.
/// The existing `find_by_sku_exact` (used by `find_product_by_scan` for
/// the dashboard scan/search surface) is intentionally left untouched.
pub async fn find_active_product_by_sku_exact(
    pool: &SqlitePool,
    sku: &str,
) -> Result<Option<ProductSearchResult>, sqlx::Error> {
    let row: Option<RawProductRow> = sqlx::query_as(
        r#"
        SELECT id, sku, description, category_id, default_unit,
               default_unit_id, unit_type,
               default_alert_days_before, notes, is_active,
               created_at, updated_at
        FROM products
        WHERE sku = $1 AND is_active = 1
        "#,
    )
    .bind(sku)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let category_ids = get_product_category_ids(pool, &r.id).await?;
            let primary_barcodes =
                list_primary_barcodes_for_products(pool, &[r.id.clone()]).await?;
            let primary_barcode = primary_barcodes.get(&r.id).cloned();
            Ok(Some(r.into_search_result(category_ids, primary_barcode)))
        }
        None => Ok(None),
    }
}

/// Checks whether a product has at least one active expiry lots.
pub async fn product_has_active_lots(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<bool, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
            SELECT COUNT(*) FROM expiry_lots
            WHERE product_id = $1 AND status = 'active'
            "#,
    )
    .bind(product_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(n,)| n > 0).unwrap_or(false))
}

/// Returns every product (active and archived) ordered by SKU, suitable for
/// CSV export. Each product appears once; `category_ids` is resolved from
/// the junction in one batched read.
pub async fn list_all_products_for_export(
    pool: &SqlitePool,
) -> Result<Vec<ProductResponse>, sqlx::Error> {
    let rows: Vec<RawProductRow> = sqlx::query_as(
        r#"
        SELECT id, sku, description, category_id, default_unit,
               default_unit_id, unit_type,
               default_alert_days_before, notes, is_active,
               created_at, updated_at
        FROM products
        ORDER BY sku ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let product_ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    let junction = list_product_category_ids_by_product_ids(pool, &product_ids).await?;

    let mut results = Vec::with_capacity(rows.len());
    for r in rows {
        let id = r.id.clone();
        let category_ids = junction.get(&id).cloned().unwrap_or_default();
        results.push(r.into_response(category_ids));
    }
    Ok(results)
}

// ============================================================
// Raw row helpers
// ============================================================

/// Intermediate row type for product queries. `category_id` is kept so the
/// raw SELECT can use `SELECT *` without schema errors; it is NOT used in
/// the runtime model — see the module-level invariant comment.
#[derive(Debug, sqlx::FromRow)]
struct RawProductRow {
    id: String,
    sku: String,
    description: String,
    #[sqlx(default)]
    _category_id: Option<String>, // kept so SELECT * maps cleanly; NOT used by runtime — see module-level invariant
    default_unit: Option<String>,
    default_unit_id: Option<String>,
    unit_type: Option<String>,
    default_alert_days_before: i32,
    notes: Option<String>,
    is_active: bool,
    created_at: String,
    updated_at: String,
}

impl RawProductRow {
    /// Converts the raw row to a `ProductResponse`, injecting `category_ids`
    /// from the junction table read.
    fn into_response(self, category_ids: Vec<String>) -> ProductResponse {
        ProductResponse {
            id: self.id,
            sku: self.sku,
            description: self.description,
            category_ids,
            default_unit: self.default_unit,
            default_unit_id: self.default_unit_id,
            unit_type: self.unit_type.map(|k| match k.as_str() {
                "integer" => UnitKind::Integer,
                _ => UnitKind::Decimal,
            }),
            default_alert_days_before: self.default_alert_days_before,
            notes: self.notes,
            is_active: self.is_active,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    /// Converts the raw row to a `ProductSearchResult`, injecting `category_ids`.
    /// `primary_barcode` is pre-resolved by callers for efficiency.
    ///
    /// `default_unit` and `default_alert_days_before` are forwarded verbatim
    /// from the raw row so the catalog list can render the same unit / alert
    /// metadata the detail page already shows.
    fn into_search_result(
        self,
        category_ids: Vec<String>,
        primary_barcode: Option<String>,
    ) -> ProductSearchResult {
        ProductSearchResult {
            id: self.id,
            sku: self.sku,
            description: self.description,
            category_ids,
            default_unit: self.default_unit,
            default_alert_days_before: self.default_alert_days_before,
            primary_barcode,
            is_active: self.is_active,
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::products::{CategoryCreate, ProductCreate, ProductUpdate};

    // ── Junction batch helper ─────────────────────────────────────────────────

    #[tokio::test]
    async fn list_product_category_ids_by_product_ids_empty_input_returns_empty_map() {
        use super::list_product_category_ids_by_product_ids;
        let pool = fresh_test_pool().await.unwrap();
        let result = list_product_category_ids_by_product_ids(&pool, &[])
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn list_product_category_ids_by_product_ids_groups_by_product_id() {
        use super::list_product_category_ids_by_product_ids;
        use crate::services::products::{create_category, create_product};

        let pool = fresh_test_pool().await.unwrap();

        let cat_a = create_category(&pool, CategoryCreate { name: "A".into() })
            .await
            .unwrap();
        let cat_b = create_category(&pool, CategoryCreate { name: "B".into() })
            .await
            .unwrap();

        let p1 = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-1".into(),
                description: "Product 1".into(),
                category_ids: Some(vec![cat_a.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();
        let p2 = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-2".into(),
                description: "Product 2".into(),
                category_ids: Some(vec![cat_a.id.clone(), cat_b.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        let result =
            list_product_category_ids_by_product_ids(&pool, &[p1.id.clone(), p2.id.clone()])
                .await
                .unwrap();

        assert_eq!(result.get(&p1.id).unwrap().len(), 1);
        assert_eq!(result.get(&p2.id).unwrap().len(), 2);
    }

    // ── Junction write ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn insert_product_writes_junction_rows() {
        use crate::services::products::{create_category, create_product};

        let pool = fresh_test_pool().await.unwrap();
        let cat = create_category(
            &pool,
            CategoryCreate {
                name: "Dairy".into(),
            },
        )
        .await
        .unwrap();

        let product = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-D".into(),
                description: "Dairy product".into(),
                category_ids: Some(vec![cat.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(product.category_ids, vec![cat.id]);
    }

    #[tokio::test]
    async fn insert_product_with_empty_category_ids_writes_no_junction() {
        use crate::services::products::create_product;

        let pool = fresh_test_pool().await.unwrap();
        let product = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-E".into(),
                description: "No category product".into(),
                category_ids: Some(vec![]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        assert!(product.category_ids.is_empty());
    }

    #[tokio::test]
    async fn insert_product_with_three_category_ids_writes_three_junction_rows() {
        use crate::services::products::{create_category, create_product};

        let pool = fresh_test_pool().await.unwrap();
        let cat_a = create_category(&pool, CategoryCreate { name: "A".into() })
            .await
            .unwrap();
        let cat_b = create_category(&pool, CategoryCreate { name: "B".into() })
            .await
            .unwrap();
        let cat_c = create_category(&pool, CategoryCreate { name: "C".into() })
            .await
            .unwrap();

        let product = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-3".into(),
                description: "Three categories".into(),
                category_ids: Some(vec![cat_a.id.clone(), cat_b.id.clone(), cat_c.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(product.category_ids.len(), 3);
    }

    #[tokio::test]
    async fn update_product_replaces_full_junction_set() {
        use crate::services::products::{create_category, create_product, update_product};

        let pool = fresh_test_pool().await.unwrap();
        let cat_a = create_category(&pool, CategoryCreate { name: "A".into() })
            .await
            .unwrap();
        let cat_b = create_category(&pool, CategoryCreate { name: "B".into() })
            .await
            .unwrap();

        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-U".into(),
                description: "Update test".into(),
                category_ids: Some(vec![cat_a.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(p.category_ids, vec![cat_a.id]);

        let updated = update_product(
            &pool,
            ProductUpdate {
                id: p.id.clone(),
                sku: "SKU-U".into(),
                description: "Updated".into(),
                category_ids: Some(vec![cat_b.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
                is_active: true,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.category_ids, vec![cat_b.id]);
    }

    #[tokio::test]
    async fn update_product_from_two_to_zero_categories_removes_junction() {
        use crate::services::products::{create_category, create_product, update_product};

        let pool = fresh_test_pool().await.unwrap();
        let cat_a = create_category(&pool, CategoryCreate { name: "A".into() })
            .await
            .unwrap();
        let cat_b = create_category(&pool, CategoryCreate { name: "B".into() })
            .await
            .unwrap();

        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-R".into(),
                description: "Remove categories".into(),
                category_ids: Some(vec![cat_a.id.clone(), cat_b.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(p.category_ids.len(), 2);

        let updated = update_product(
            &pool,
            ProductUpdate {
                id: p.id.clone(),
                sku: "SKU-R".into(),
                description: "Updated".into(),
                category_ids: Some(vec![]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
                is_active: true,
            },
        )
        .await
        .unwrap();

        assert!(updated.category_ids.is_empty());
    }

    // ── Read paths ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_product_returns_category_ids_from_junction() {
        use crate::services::products::{create_category, create_product, get_product};

        let pool = fresh_test_pool().await.unwrap();
        let cat = create_category(
            &pool,
            CategoryCreate {
                name: "Bakery".into(),
            },
        )
        .await
        .unwrap();

        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-G".into(),
                description: "Get test".into(),
                category_ids: Some(vec![cat.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        let detail = get_product(&pool, p.id.clone()).await.unwrap();
        assert_eq!(detail.product.category_ids, vec![cat.id]);
    }

    #[tokio::test]
    async fn search_products_returns_category_ids_from_junction() {
        use crate::dto::products::ProductSearchQuery;
        use crate::services::products::{create_category, create_product};

        let pool = fresh_test_pool().await.unwrap();
        let cat = create_category(
            &pool,
            CategoryCreate {
                name: "Produce".into(),
            },
        )
        .await
        .unwrap();

        let _p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-SE".into(),
                description: "Searchable Produce Item".into(),
                category_ids: Some(vec![cat.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        let hits = super::search_products(
            &pool,
            &ProductSearchQuery {
                query: "Produce".into(),
            },
        )
        .await
        .unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].category_ids, vec![cat.id]);
    }

    #[tokio::test]
    async fn find_by_barcode_exact_returns_category_ids_from_junction() {
        use crate::dto::products::ProductBarcodeCreate;
        use crate::services::products::{add_barcode, create_category, create_product};

        let pool = fresh_test_pool().await.unwrap();
        let cat = create_category(
            &pool,
            CategoryCreate {
                name: "Frozen".into(),
            },
        )
        .await
        .unwrap();

        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-BC".into(),
                description: "Barcode test".into(),
                category_ids: Some(vec![cat.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "1234567890123".into(),
                barcode_type: Some("EAN13".into()),
                is_primary: true,
            },
        )
        .await
        .unwrap();

        let found = super::find_by_barcode_exact(&pool, "1234567890123")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(found.category_ids, vec![cat.id]);
    }

    #[tokio::test]
    async fn find_by_sku_exact_returns_category_ids_from_junction() {
        use crate::services::products::{create_category, create_product};

        let pool = fresh_test_pool().await.unwrap();
        let cat = create_category(
            &pool,
            CategoryCreate {
                name: "Snacks".into(),
            },
        )
        .await
        .unwrap();

        let _p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-SKU-EXACT".into(),
                description: "SKU exact test".into(),
                category_ids: Some(vec![cat.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        let found = super::find_by_sku_exact(&pool, "SKU-SKU-EXACT")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(found.category_ids, vec![cat.id]);
    }

    #[tokio::test]
    async fn delete_product_cascades_junction_rows() {
        use crate::services::products::{create_category, create_product};

        let pool = fresh_test_pool().await.unwrap();
        let cat = create_category(
            &pool,
            CategoryCreate {
                name: "Beverages".into(),
            },
        )
        .await
        .unwrap();

        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-DEL".into(),
                description: "Delete cascade test".into(),
                category_ids: Some(vec![cat.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        sqlx::query("DELETE FROM products WHERE id = $1")
            .bind(&p.id)
            .execute(&pool)
            .await
            .unwrap();

        let remaining: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM product_categories WHERE product_id = $1")
                .bind(&p.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(remaining.0, 0);
    }

    #[tokio::test]
    async fn delete_category_with_junction_is_rejected() {
        use crate::services::products::{create_category, create_product};

        let pool = fresh_test_pool().await.unwrap();
        let cat = create_category(
            &pool,
            CategoryCreate {
                name: "Tied".into(),
            },
        )
        .await
        .unwrap();

        let _p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-TIED".into(),
                description: "Category tied product".into(),
                category_ids: Some(vec![cat.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        // Hard delete of a category with junction rows is rejected by ON DELETE RESTRICT.
        let err = sqlx::query("DELETE FROM categories WHERE id = $1")
            .bind(&cat.id)
            .execute(&pool)
            .await
            .unwrap_err();

        // ON DELETE RESTRICT produces a FOREIGN KEY constraint violation.
        assert!(
            matches!(err, sqlx::Error::Database(ref db_err)
                    if db_err.code().map_or(false, |c| c == "787" || c == "27500")
                    || db_err.to_string().contains("FOREIGN KEY constraint")),
            "Expected FK constraint error, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn list_all_products_for_export_includes_category_ids() {
        use crate::services::products::create_product;

        let pool = fresh_test_pool().await.unwrap();
        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-EXP".into(),
                description: "Export product".into(),
                category_ids: Some(vec![]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await
        .unwrap();

        let products = super::list_all_products_for_export(&pool).await.unwrap();
        let found = products.iter().find(|product| product.id == p.id).unwrap();
        // New DTO has `category_ids` field (Vec<String>), not `category_id`.
        assert!(found.category_ids.is_empty());
    }
}
