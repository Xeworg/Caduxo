//! Product catalog service — categories, products, and barcodes.
//!
//! Business rules (validation, uniqueness-error translation, active-product
//! guards) live here. SQL is delegated to `db::repositories::products`.

use sqlx::Error as SqlxError;

use crate::db::repositories::products as repo;
use crate::db::DbPool;
use crate::domain::validation::{
    validate_barcode, validate_description, validate_name, validate_sku,
};
use crate::dto::products::{
    CategoryCreate, CategoryResponse, CategoryUpdate, ProductBarcodeCreate,
    ProductBarcodeRemoveInput, ProductBarcodeResponse, ProductCreate, ProductDetailResponse,
    ProductResponse, ProductSearchQuery, ProductSearchResult, ProductUpdate,
};
use crate::dto::scanner::ScanSearchResult;
use crate::error::{AppError, DomainError};

/// Software-suggested default for `default_alert_days_before` when the user
/// creates a new product. The schema default is `30`; this constant is the
/// canonical source the frontend should use to seed the field.
pub const SUGGESTED_ALERT_DAYS: i32 = 30;

/// Maximum allowed value for `default_alert_days_before` (~10 years).
const MAX_ALERT_DAYS: i32 = 3650;

/// Returns the software-suggested default alert-days value.
pub fn suggested_alert_days() -> i32 {
    SUGGESTED_ALERT_DAYS
}

fn validate_alert_days(days: i32) -> Result<(), DomainError> {
    if days < 0 {
        return Err(DomainError::Validation {
            message: "Default alert days before cannot be negative".to_string(),
        });
    }
    if days > MAX_ALERT_DAYS {
        return Err(DomainError::Validation {
            message: format!("Default alert days exceeds maximum of {MAX_ALERT_DAYS}"),
        });
    }
    Ok(())
}

/// If `err` is a SQLite `UNIQUE constraint failed: <table>.<column>` error,
/// returns the column name (e.g. `products.sku`). Returns `None` otherwise.
fn unique_violation_column(err: &SqlxError) -> Option<String> {
    if let SqlxError::Database(db_err) = err {
        let msg = db_err.message();
        if let Some(rest) = msg.strip_prefix("UNIQUE constraint failed: ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn product_unique_error(err: SqlxError, sku: &str) -> AppError {
    if let Some(column) = unique_violation_column(&err) {
        if column.ends_with(".sku") {
            return AppError::Domain(DomainError::DuplicateField {
                field: "sku",
                value: sku.to_string(),
            });
        }
    }
    AppError::from(err)
}

fn barcode_unique_error(err: SqlxError, barcode: &str) -> AppError {
    if let Some(column) = unique_violation_column(&err) {
        if column.ends_with(".barcode") {
            return AppError::Domain(DomainError::DuplicateField {
                field: "barcode",
                value: barcode.to_string(),
            });
        }
    }
    AppError::from(err)
}

fn category_unique_error(err: SqlxError, name: &str) -> AppError {
    if let Some(column) = unique_violation_column(&err) {
        if column.ends_with(".name") {
            return AppError::Domain(DomainError::DuplicateField {
                field: "name",
                value: name.to_string(),
            });
        }
    }
    AppError::from(err)
}

// ============================================================
// Categories
// ============================================================

/// Lists active categories, ordered alphabetically.
pub async fn list_categories(pool: &DbPool) -> Result<Vec<CategoryResponse>, AppError> {
    repo::list_active_categories(pool)
        .await
        .map_err(AppError::from)
}

/// Lists all categories (active first), ordered alphabetically.
pub async fn list_all_categories(pool: &DbPool) -> Result<Vec<CategoryResponse>, AppError> {
    repo::list_all_categories(pool)
        .await
        .map_err(AppError::from)
}

/// Creates a new category after validating the name.
pub async fn create_category(
    pool: &DbPool,
    input: CategoryCreate,
) -> Result<CategoryResponse, AppError> {
    validate_name(&input.name).map_err(|m| DomainError::Validation { message: m })?;
    repo::insert_category(pool, &input)
        .await
        .map_err(|e| category_unique_error(e, &input.name))
}

/// Updates a category (rename and/or archive). Returns `NotFound` if the
/// category does not exist.
pub async fn update_category(
    pool: &DbPool,
    input: CategoryUpdate,
) -> Result<CategoryResponse, AppError> {
    validate_name(&input.name).map_err(|m| DomainError::Validation { message: m })?;

    let row = repo::update_category(pool, &input)
        .await
        .map_err(|e| category_unique_error(e, &input.name))?
        .ok_or(DomainError::NotFound {
            resource: "category",
            id: input.id,
        })?;
    Ok(row)
}

// ============================================================
// Products
// ============================================================

/// Creates a new active product.
pub async fn create_product(
    pool: &DbPool,
    input: ProductCreate,
) -> Result<ProductResponse, AppError> {
    validate_sku(&input.sku).map_err(|m| DomainError::Validation { message: m })?;
    validate_description(&input.description).map_err(|m| DomainError::Validation { message: m })?;
    validate_alert_days(input.default_alert_days_before)?;

    repo::insert_product(pool, &input)
        .await
        .map_err(|e| product_unique_error(e, &input.sku))
}

/// Updates an existing product. Returns `NotFound` if the id does not exist.
pub async fn update_product(
    pool: &DbPool,
    input: ProductUpdate,
) -> Result<ProductResponse, AppError> {
    validate_sku(&input.sku).map_err(|m| DomainError::Validation { message: m })?;
    validate_description(&input.description).map_err(|m| DomainError::Validation { message: m })?;
    validate_alert_days(input.default_alert_days_before)?;

    let row = repo::update_product(pool, &input)
        .await
        .map_err(|e| product_unique_error(e, &input.sku))?
        .ok_or(DomainError::NotFound {
            resource: "product",
            id: input.id,
        })?;
    Ok(row)
}

/// Soft-archives a product. Returns `NotFound` if the id does not exist.
pub async fn archive_product(pool: &DbPool, id: String) -> Result<(), AppError> {
    let found = repo::archive_product(pool, &id).await?;
    if !found {
        return Err(DomainError::NotFound {
            resource: "product",
            id,
        }
        .into());
    }
    Ok(())
}

/// Returns the full product detail (product + barcodes + resolved category).
pub async fn get_product(pool: &DbPool, id: String) -> Result<ProductDetailResponse, AppError> {
    let product = repo::get_product(pool, &id)
        .await?
        .ok_or(DomainError::NotFound {
            resource: "product",
            id: id.clone(),
        })?;

    let barcodes = repo::list_barcodes(pool, &id).await?;
    let category = match product.category_id.as_deref() {
        Some(cid) => repo::get_category(pool, cid).await?,
        None => None,
    };

    Ok(ProductDetailResponse {
        product,
        barcodes,
        category,
    })
}

/// Searches products by description, SKU, or barcode.
pub async fn search_products(
    pool: &DbPool,
    query: ProductSearchQuery,
) -> Result<Vec<ProductSearchResult>, AppError> {
    repo::search_products(pool, &query)
        .await
        .map_err(AppError::from)
}

/// Scanner workflow: barcode-first exact lookup, then SKU-second exact lookup.
///
/// 1. Search `product_barcodes` for an exact barcode match.
/// 2. If not found, search `products` for an exact SKU match.
/// 3. If neither matches, return `NotFound` with the scanned value for pre-fill.
///
/// Returns `Found { product, has_lots }` when a match is found, or
/// `NotFound { scanned_value }` when nothing matched.
pub async fn find_product_by_scan(
    pool: &DbPool,
    scanned_value: &str,
) -> Result<ScanSearchResult, AppError> {
    let trimmed = scanned_value.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Validation {
            message: "Scan value cannot be empty".to_string(),
        }
        .into());
    }

    // Step 1: exact barcode match
    if let Some(product) = repo::find_by_barcode_exact(pool, trimmed)
        .await
        .map_err(AppError::from)?
    {
        let has_lots = repo::product_has_active_lots(pool, &product.id)
            .await
            .map_err(AppError::from)?;
        return Ok(ScanSearchResult::Found { product, has_lots });
    }

    // Step 2: exact SKU match
    if let Some(product) = repo::find_by_sku_exact(pool, trimmed)
        .await
        .map_err(AppError::from)?
    {
        let has_lots = repo::product_has_active_lots(pool, &product.id)
            .await
            .map_err(AppError::from)?;
        return Ok(ScanSearchResult::Found { product, has_lots });
    }

    // Step 3: no match
    Ok(ScanSearchResult::NotFound {
        scanned_value: trimmed.to_string(),
    })
}

// ============================================================
// Barcodes
// ============================================================

/// Adds a barcode to an active product. Refuses archived products and
/// translates UNIQUE barcode collisions to `DuplicateField`.
pub async fn add_barcode(
    pool: &DbPool,
    input: ProductBarcodeCreate,
) -> Result<ProductBarcodeResponse, AppError> {
    validate_barcode(&input.barcode).map_err(|m| DomainError::Validation { message: m })?;

    let product = repo::get_product(pool, &input.product_id)
        .await?
        .ok_or_else(|| DomainError::NotFound {
            resource: "product",
            id: input.product_id.clone(),
        })?;

    if !product.is_active {
        return Err(DomainError::BusinessRule {
            message: "Cannot add barcode to an archived product".to_string(),
        }
        .into());
    }

    repo::insert_barcode(pool, &input)
        .await
        .map_err(|e| barcode_unique_error(e, &input.barcode))
}

/// Lists all barcodes for an existing product.
pub async fn list_barcodes(
    pool: &DbPool,
    product_id: String,
) -> Result<Vec<ProductBarcodeResponse>, AppError> {
    let _ = repo::get_product(pool, &product_id)
        .await?
        .ok_or_else(|| DomainError::NotFound {
            resource: "product",
            id: product_id.clone(),
        })?;

    repo::list_barcodes(pool, &product_id)
        .await
        .map_err(AppError::from)
}

/// Removes a barcode by id. Returns `NotFound` if the id does not exist.
pub async fn remove_barcode(
    pool: &DbPool,
    input: ProductBarcodeRemoveInput,
) -> Result<(), AppError> {
    let removed = repo::remove_barcode(pool, &input.id).await?;
    if !removed {
        return Err(DomainError::NotFound {
            resource: "product_barcode",
            id: input.id,
        }
        .into());
    }
    Ok(())
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    // NOTE: avoid `use super::*` to keep `fresh_test_pool` unambiguous.
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::products::{
        CategoryCreate, CategoryUpdate, ProductBarcodeCreate, ProductBarcodeRemoveInput,
        ProductCreate, ProductSearchQuery, ProductUpdate,
    };
    use crate::error::AppError;
    use crate::services::products::{
        add_barcode, archive_product, create_category, create_product, get_product, list_barcodes,
        list_categories, remove_barcode, search_products, suggested_alert_days, update_category,
        update_product,
    };

    // ------------------------------------------------------------------
    // Categories
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn create_and_list_categories() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        create_category(
            &pool,
            CategoryCreate {
                name: "Dairy".into(),
            },
        )
        .await?;
        create_category(
            &pool,
            CategoryCreate {
                name: "Bakery".into(),
            },
        )
        .await?;

        let cats = list_categories(&pool).await?;
        assert_eq!(cats.len(), 2);
        assert_eq!(cats[0].name, "Bakery");
        assert_eq!(cats[1].name, "Dairy");
        assert!(cats.iter().all(|c| c.is_active));
        Ok(())
    }

    #[tokio::test]
    async fn create_category_rejects_duplicate_name() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        create_category(
            &pool,
            CategoryCreate {
                name: "Frozen".into(),
            },
        )
        .await?;

        let err = create_category(
            &pool,
            CategoryCreate {
                name: "Frozen".into(),
            },
        )
        .await
        .expect_err("duplicate name must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::DuplicateField { field, value }) => {
                assert_eq!(field, "name");
                assert_eq!(value, "Frozen");
            }
            other => panic!("expected DuplicateField, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn create_category_rejects_empty_name() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = create_category(&pool, CategoryCreate { name: "   ".into() })
            .await
            .expect_err("empty name must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn update_category_renames_and_archives() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let cat = create_category(
            &pool,
            CategoryCreate {
                name: "Original".into(),
            },
        )
        .await?;

        let updated = update_category(
            &pool,
            CategoryUpdate {
                id: cat.id.clone(),
                name: "Renamed".into(),
                is_active: false,
            },
        )
        .await?;
        assert_eq!(updated.name, "Renamed");
        assert!(!updated.is_active);

        // Default list omits archived entries.
        let active = list_categories(&pool).await?;
        assert!(active.iter().all(|c| c.id != cat.id));
        Ok(())
    }

    #[tokio::test]
    async fn update_category_missing_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = update_category(
            &pool,
            CategoryUpdate {
                id: "nope".into(),
                name: "X".into(),
                is_active: true,
            },
        )
        .await
        .expect_err("missing category must be NotFound");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::NotFound { .. })
        ));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Products — create / update / archive / validation
    // ------------------------------------------------------------------

    fn basic_product(sku: &str) -> ProductCreate {
        ProductCreate {
            sku: sku.into(),
            description: "Whole Milk 1L".into(),
            category_id: None,
            default_unit: Some("L".into()),
            default_alert_days_before: suggested_alert_days(),
            notes: None,
        }
    }

    #[tokio::test]
    async fn suggested_alert_days_is_thirty() {
        assert_eq!(suggested_alert_days(), 30);
    }

    #[tokio::test]
    async fn create_product_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(&pool, basic_product("SKU-001")).await?;
        assert_eq!(p.sku, "SKU-001");
        assert_eq!(p.default_alert_days_before, 30);
        assert!(p.is_active);
        Ok(())
    }

    #[tokio::test]
    async fn create_product_rejects_empty_sku() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = create_product(&pool, basic_product("   "))
            .await
            .expect_err("empty SKU must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_product_rejects_empty_description() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let mut input = basic_product("SKU-001");
        input.description = "".into();
        let err = create_product(&pool, input)
            .await
            .expect_err("empty description must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_product_rejects_negative_alert_days() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let mut input = basic_product("SKU-001");
        input.default_alert_days_before = -1;
        let err = create_product(&pool, input)
            .await
            .expect_err("negative alert days must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_product_rejects_excessive_alert_days() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let mut input = basic_product("SKU-001");
        input.default_alert_days_before = 5000;
        let err = create_product(&pool, input)
            .await
            .expect_err("alert days above cap must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_product_enforces_sku_uniqueness() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        create_product(&pool, basic_product("SKU-DUP")).await?;

        let err = create_product(&pool, basic_product("SKU-DUP"))
            .await
            .expect_err("duplicate SKU must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::DuplicateField { field, value }) => {
                assert_eq!(field, "sku");
                assert_eq!(value, "SKU-DUP");
            }
            other => panic!("expected DuplicateField(sku), got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn update_product_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(&pool, basic_product("SKU-U")).await?;

        let updated = update_product(
            &pool,
            ProductUpdate {
                id: p.id.clone(),
                sku: "SKU-U-NEW".into(),
                description: "Updated description".into(),
                category_id: None,
                default_unit: Some("kg".into()),
                default_alert_days_before: 14,
                notes: Some("note".into()),
                is_active: true,
            },
        )
        .await?;

        assert_eq!(updated.sku, "SKU-U-NEW");
        assert_eq!(updated.default_alert_days_before, 14);
        Ok(())
    }

    #[tokio::test]
    async fn update_product_enforces_sku_uniqueness() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let _a = create_product(&pool, basic_product("SKU-A")).await?;
        let b = create_product(&pool, basic_product("SKU-B")).await?;

        let err = update_product(
            &pool,
            ProductUpdate {
                id: b.id.clone(),
                sku: "SKU-A".into(),
                description: "desc".into(),
                category_id: None,
                default_unit: None,
                default_alert_days_before: 30,
                notes: None,
                is_active: true,
            },
        )
        .await
        .expect_err("updating to a colliding SKU must fail");

        match err {
            AppError::Domain(crate::error::DomainError::DuplicateField { field, .. }) => {
                assert_eq!(field, "sku");
            }
            other => panic!("expected DuplicateField(sku), got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn update_product_missing_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = update_product(
            &pool,
            ProductUpdate {
                id: "nope".into(),
                sku: "SKU-X".into(),
                description: "d".into(),
                category_id: None,
                default_unit: None,
                default_alert_days_before: 30,
                notes: None,
                is_active: true,
            },
        )
        .await
        .expect_err("missing product must be NotFound");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::NotFound { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn archive_product_soft_deletes() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(&pool, basic_product("SKU-ARCH")).await?;

        archive_product(&pool, p.id.clone()).await?;

        let detail = get_product(&pool, p.id.clone()).await?;
        assert!(!detail.product.is_active, "product should be archived");
        Ok(())
    }

    #[tokio::test]
    async fn archive_product_missing_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = archive_product(&pool, "nope".into())
            .await
            .expect_err("missing product must be NotFound");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::NotFound { .. })
        ));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Barcodes — add / list / remove / uniqueness across products
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn add_list_remove_barcodes() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(&pool, basic_product("SKU-BC")).await?;

        let b1 = add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "1111111111".into(),
                barcode_type: Some("EAN13".into()),
                is_primary: true,
            },
        )
        .await?;
        let b2 = add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "2222222222".into(),
                barcode_type: None,
                is_primary: false,
            },
        )
        .await?;

        let list = list_barcodes(&pool, p.id.clone()).await?;
        assert_eq!(list.len(), 2);
        // Primary first.
        assert!(list[0].is_primary);

        remove_barcode(&pool, ProductBarcodeRemoveInput { id: b2.id.clone() }).await?;
        remove_barcode(&pool, ProductBarcodeRemoveInput { id: b1.id.clone() }).await?;

        let list = list_barcodes(&pool, p.id.clone()).await?;
        assert_eq!(list.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn add_barcode_validates_value() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(&pool, basic_product("SKU-BCV")).await?;

        let err = add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "".into(),
                barcode_type: None,
                is_primary: false,
            },
        )
        .await
        .expect_err("empty barcode must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn add_barcode_to_archived_product_is_rejected() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let p = create_product(&pool, basic_product("SKU-BCA")).await?;
        archive_product(&pool, p.id.clone()).await?;

        let err = add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "9999999999".into(),
                barcode_type: None,
                is_primary: false,
            },
        )
        .await
        .expect_err("adding barcode to archived product must fail");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::BusinessRule { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn add_barcode_to_missing_product_returns_not_found(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: "nope".into(),
                barcode: "1111111111".into(),
                barcode_type: None,
                is_primary: false,
            },
        )
        .await
        .expect_err("missing product must be NotFound");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::NotFound { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn barcode_uniqueness_across_products() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p1 = create_product(&pool, basic_product("SKU-U1")).await?;
        let p2 = create_product(&pool, basic_product("SKU-U2")).await?;

        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p1.id.clone(),
                barcode: "7501234567890".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;

        let err = add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p2.id.clone(),
                barcode: "7501234567890".into(),
                barcode_type: None,
                is_primary: false,
            },
        )
        .await
        .expect_err("barcode must be unique across products");

        match err {
            AppError::Domain(crate::error::DomainError::DuplicateField { field, value }) => {
                assert_eq!(field, "barcode");
                assert_eq!(value, "7501234567890");
            }
            other => panic!("expected DuplicateField(barcode), got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn add_secondary_barcode_to_same_product_is_allowed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(&pool, basic_product("SKU-MULTI")).await?;

        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "ABC-001".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;
        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "ABC-002".into(),
                barcode_type: None,
                is_primary: false,
            },
        )
        .await?;

        let list = list_barcodes(&pool, p.id.clone()).await?;
        assert_eq!(list.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn remove_barcode_missing_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = remove_barcode(&pool, ProductBarcodeRemoveInput { id: "nope".into() })
            .await
            .expect_err("missing barcode must be NotFound");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::NotFound { .. })
        ));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Search
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn search_finds_by_description_sku_and_barcode() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let milk = create_product(
            &pool,
            ProductCreate {
                sku: "MILK-1L".into(),
                description: "Whole Milk 1L".into(),
                category_id: None,
                default_unit: Some("L".into()),
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        let bread = create_product(
            &pool,
            ProductCreate {
                sku: "BREAD-001".into(),
                description: "Sourdough Bread".into(),
                category_id: None,
                default_unit: None,
                default_alert_days_before: 14,
                notes: None,
            },
        )
        .await?;
        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: milk.id.clone(),
                barcode: "7500000000001".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;
        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: bread.id.clone(),
                barcode: "7500000000002".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;

        // Description match (case-insensitive partial)
        let hits = search_products(
            &pool,
            ProductSearchQuery {
                query: "milk".into(),
            },
        )
        .await?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].sku, "MILK-1L");

        // SKU partial match
        let hits = search_products(
            &pool,
            ProductSearchQuery {
                query: "BREAD".into(),
            },
        )
        .await?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].sku, "BREAD-001");

        // Exact barcode match
        let hits = search_products(
            &pool,
            ProductSearchQuery {
                query: "7500000000001".into(),
            },
        )
        .await?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].sku, "MILK-1L");

        // Barcode partial match
        let hits = search_products(
            &pool,
            ProductSearchQuery {
                query: "750000".into(),
            },
        )
        .await?;
        assert_eq!(hits.len(), 2);

        // Empty query returns everything
        let hits = search_products(&pool, ProductSearchQuery { query: "".into() }).await?;
        assert_eq!(hits.len(), 2);

        // No match
        let hits = search_products(
            &pool,
            ProductSearchQuery {
                query: "zzz-nothing".into(),
            },
        )
        .await?;
        assert_eq!(hits.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn search_includes_primary_barcode_in_results() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let p = create_product(&pool, basic_product("SKU-PB")).await?;
        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "PB-001".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;
        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "PB-002".into(),
                barcode_type: None,
                is_primary: false,
            },
        )
        .await?;

        let hits = search_products(
            &pool,
            ProductSearchQuery {
                query: "SKU-PB".into(),
            },
        )
        .await?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].primary_barcode.as_deref(), Some("PB-001"));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Scanner workflow — find_product_by_scan (barcode-first, SKU-second)
    // ------------------------------------------------------------------

    use crate::dto::scanner::ScanSearchResult;
    use crate::services::products::find_product_by_scan;

    #[tokio::test]
    async fn scan_barcode_exact_returns_found_with_barcode_match(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(&pool, basic_product("SCAN-SKU")).await?;
        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "0123456789012".into(),
                barcode_type: Some("EAN13".into()),
                is_primary: true,
            },
        )
        .await?;

        let result = find_product_by_scan(&pool, "0123456789012").await?;
        match result {
            ScanSearchResult::Found { product, has_lots } => {
                assert_eq!(product.sku, "SCAN-SKU");
                assert!(!has_lots, "new product should not have lots");
            }
            ScanSearchResult::NotFound { .. } => {
                panic!("expected Found for barcode match");
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn scan_sku_exact_when_no_barcode_match_returns_found(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let _p = create_product(&pool, basic_product("SCAN-SKU-EXACT")).await?;

        let result = find_product_by_scan(&pool, "SCAN-SKU-EXACT").await?;
        match result {
            ScanSearchResult::Found {
                product,
                has_lots: _,
            } => {
                assert_eq!(product.sku, "SCAN-SKU-EXACT");
            }
            ScanSearchResult::NotFound { .. } => {
                panic!("expected Found for SKU match");
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn scan_unknown_value_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let _p = create_product(&pool, basic_product("KNOWN-SKU")).await?;

        let result = find_product_by_scan(&pool, "totally-unknown-value").await?;
        match result {
            ScanSearchResult::NotFound { scanned_value } => {
                assert_eq!(scanned_value, "totally-unknown-value");
            }
            ScanSearchResult::Found { .. } => {
                panic!("expected NotFound for unknown scan");
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn scan_rejects_empty_value() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = find_product_by_scan(&pool, "   ")
            .await
            .expect_err("empty scan must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn scan_barcode_takes_precedence_over_sku() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // Create a product whose SKU is also a valid barcode value for another product.
        let p = create_product(&pool, basic_product("BARCODE-VALUE")).await?;
        add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "SOME-REAL-BC".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;
        // Create a second product whose SKU is "SOME-REAL-BC" (same as barcode above).
        let _p2 = create_product(&pool, basic_product("SOME-REAL-BC")).await?;

        // Scanning "SOME-REAL-BC" must match the barcode product (first), not the SKU product.
        let result = find_product_by_scan(&pool, "SOME-REAL-BC").await?;
        match result {
            ScanSearchResult::Found {
                product,
                has_lots: _,
            } => {
                assert_eq!(
                    product.sku, "BARCODE-VALUE",
                    "barcode match must win over SKU"
                );
            }
            ScanSearchResult::NotFound { .. } => {
                panic!("expected Found");
            }
        }
        Ok(())
    }
}
