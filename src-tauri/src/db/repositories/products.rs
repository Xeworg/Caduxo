//! Repository for product catalog persistence: categories, products, barcodes.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::dto::products::{
    CategoryCreate, CategoryResponse, CategoryUpdate, ProductBarcodeCreate, ProductBarcodeResponse,
    ProductCreate, ProductResponse, ProductSearchQuery, ProductSearchResult, ProductUpdate,
};

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
// Products
// ============================================================

/// Inserts a new active product.
pub async fn insert_product(
    pool: &SqlitePool,
    input: &ProductCreate,
) -> Result<ProductResponse, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO products (
            id, sku, description, category_id, default_unit,
            default_alert_days_before, notes, is_active,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8, $9)
        "#,
    )
    .bind(&id)
    .bind(&input.sku)
    .bind(&input.description)
    .bind(&input.category_id)
    .bind(&input.default_unit)
    .bind(input.default_alert_days_before)
    .bind(&input.notes)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(ProductResponse {
        id,
        sku: input.sku.clone(),
        description: input.description.clone(),
        category_id: input.category_id.clone(),
        default_unit: input.default_unit.clone(),
        default_alert_days_before: input.default_alert_days_before,
        notes: input.notes.clone(),
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Fetches a single product by id (active or archived).
pub async fn get_product(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<ProductResponse>, sqlx::Error> {
    sqlx::query_as::<_, ProductResponse>(
        r#"
        SELECT id, sku, description, category_id, default_unit,
               default_alert_days_before, notes, is_active,
               created_at, updated_at
        FROM products
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Updates a product by id and returns the refreshed row, or `None` if the
/// row did not exist.
pub async fn update_product(
    pool: &SqlitePool,
    input: &ProductUpdate,
) -> Result<Option<ProductResponse>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    let affected = sqlx::query(
        r#"
        UPDATE products
        SET sku = $1, description = $2, category_id = $3, default_unit = $4,
            default_alert_days_before = $5, notes = $6, is_active = $7,
            updated_at = $8
        WHERE id = $9
        "#,
    )
    .bind(&input.sku)
    .bind(&input.description)
    .bind(&input.category_id)
    .bind(&input.default_unit)
    .bind(input.default_alert_days_before)
    .bind(&input.notes)
    .bind(i32::from(input.is_active))
    .bind(&now)
    .bind(&input.id)
    .execute(pool)
    .await?;

    if affected.rows_affected() == 0 {
        return Ok(None);
    }
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

/// Searches products by description, SKU, or barcode (case-insensitive
/// `LIKE` match). An empty query returns all products.
pub async fn search_products(
    pool: &SqlitePool,
    query: &ProductSearchQuery,
) -> Result<Vec<ProductSearchResult>, sqlx::Error> {
    // Wrap the user query in `%...%` for a partial match; an empty input
    // becomes `%%`, which `LIKE` matches against every row.
    let pattern = format!("%{}%", query.query.trim());

    sqlx::query_as::<_, ProductSearchResult>(
        r#"
        SELECT
            p.id, p.sku, p.description, p.category_id, p.is_active,
            (
                SELECT pb.barcode FROM product_barcodes pb
                WHERE pb.product_id = p.id AND pb.is_primary = 1
                LIMIT 1
            ) AS primary_barcode
        FROM products p
        WHERE p.description LIKE $1
           OR p.sku           LIKE $1
           OR EXISTS (
                SELECT 1 FROM product_barcodes pb2
                WHERE pb2.product_id = p.id AND pb2.barcode LIKE $1
           )
        ORDER BY p.description ASC
        "#,
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await
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
///
/// The barcode column is `UNIQUE`, so this is a single-row lookup.
pub async fn find_by_barcode_exact(
    pool: &SqlitePool,
    barcode: &str,
) -> Result<Option<ProductSearchResult>, sqlx::Error> {
    sqlx::query_as::<_, ProductSearchResult>(
        r#"
        SELECT
p.id, p.sku, p.description, p.category_id, p.is_active,
$1 AS primary_barcode
        FROM products p
        INNER JOIN product_barcodes pb ON pb.product_id = p.id
        WHERE pb.barcode = $1
        "#,
    )
    .bind(barcode)
    .fetch_optional(pool)
    .await
}

/// Looks up a product by exact SKU. Returns the product row (active or
/// archived) if the SKU exists, `None` otherwise.
pub async fn find_by_sku_exact(
    pool: &SqlitePool,
    sku: &str,
) -> Result<Option<ProductSearchResult>, sqlx::Error> {
    sqlx::query_as::<_, ProductSearchResult>(
        r#"
        SELECT
p.id, p.sku, p.description, p.category_id, p.is_active,
(
SELECT pb.barcode FROM product_barcodes pb
WHERE pb.product_id = p.id AND pb.is_primary = 1
LIMIT 1
) AS primary_barcode
        FROM products p
        WHERE p.sku = $1
        "#,
    )
    .bind(sku)
    .fetch_optional(pool)
    .await
}

/// Checks whether a product has at least one active expiry lot.
/// Used by the scanner workflow to determine whether to jump to lot entry.
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
/// CSV export. Each product appears once; barcodes are joined in `list_barcodes`.
pub async fn list_all_products_for_export(
    pool: &SqlitePool,
) -> Result<Vec<ProductResponse>, sqlx::Error> {
    sqlx::query_as::<_, ProductResponse>(
        r#"
        SELECT id, sku, description, category_id, default_unit,
               default_alert_days_before, notes, is_active,
               created_at, updated_at
        FROM products
        ORDER BY sku ASC
        "#,
    )
    .fetch_all(pool)
    .await
}
