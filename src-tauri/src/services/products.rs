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
use crate::pdf::locale::Locale;
use crate::services::user_messages::{user_message, UserMessage};

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

/// Creates a new category after validating the name.
/// Case-fold guard: rejects names that differ only in case from an existing active category.
pub async fn create_category(
    pool: &DbPool,
    input: CategoryCreate,
) -> Result<CategoryResponse, AppError> {
    validate_name(&input.name).map_err(|m| DomainError::Validation { message: m })?;
    let normalized = input.name.trim().to_lowercase();

    // Case-fold guard: refuse names that would collide case-insensitively with
    // an existing active category. The UNIQUE constraint is the safety net for races.
    if let Some(existing) = repo::find_category_by_name_ci(pool, &normalized).await? {
        if existing.name.to_lowercase() == normalized {
            return Err(DomainError::DuplicateField {
                field: "name",
                value: input.name.clone(),
            }
            .into());
        }
    }

    repo::insert_category(pool, &input)
        .await
        .map_err(|e| category_unique_error(e, &input.name))
}

/// Updates a category (rename and/or archive). Returns `NotFound` if the
/// category does not exist.
/// Case-fold guard: rejects renames that would collide case-insensitively
/// with another active category (but allows renaming to the same current name).
pub async fn update_category(
    pool: &DbPool,
    input: CategoryUpdate,
) -> Result<CategoryResponse, AppError> {
    validate_name(&input.name).map_err(|m| DomainError::Validation { message: m })?;
    let normalized = input.name.trim().to_lowercase();

    // Case-fold guard: reject renames that collide with another active category.
    // Allow renaming to the current name (id match means it's the same row).
    if let Some(existing) = repo::find_category_by_name_ci(pool, &normalized).await? {
        if existing.id != input.id && existing.name.to_lowercase() == normalized {
            return Err(DomainError::DuplicateField {
                field: "name",
                value: input.name.clone(),
            }
            .into());
        }
    }

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

    // Resolve catalog unit: prefer explicit FK, then text via find_by_key, else none.
    let (default_unit_id, unit_type) = resolve_unit_fields(
        pool,
        input.default_unit_id.as_deref(),
        input.default_unit.as_deref(),
    )
    .await?;

    repo::insert_product(pool, &input, default_unit_id, unit_type)
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

    // Resolve catalog unit: prefer explicit FK, then text via find_by_key, else none.
    let (default_unit_id, unit_type) = resolve_unit_fields(
        pool,
        input.default_unit_id.as_deref(),
        input.default_unit.as_deref(),
    )
    .await?;

    let row = repo::update_product(pool, &input, default_unit_id, unit_type)
        .await
        .map_err(|e| product_unique_error(e, &input.sku))?
        .ok_or(DomainError::NotFound {
            resource: "product",
            id: input.id,
        })?;
    Ok(row)
}

/// Maps singular Spanish unit text forms to their canonical plural catalog
/// key. The lookup is normalized (trim + lowercase) before being applied.
/// Both forms then resolve to the same catalog row and therefore the same
/// `unit_type`, preserving the existing decimal-unit semantics.
///
/// The aliases are intentionally narrow: only forms that have a clear,
/// pre-existing canonical row. Adding more entries here is safe and
/// idempotent — the migration V16 mirrors this list at the SQL layer.
fn unit_text_aliases() -> &'static std::collections::HashMap<&'static str, &'static str> {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static ALIASES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    ALIASES.get_or_init(|| {
        let mut m = HashMap::new();
        // Integer-family singular forms.
        m.insert("unidad", "units");
        m.insert("caja", "cajas");
        m.insert("botella", "bottles");
        m.insert("bolsa", "bags");
        m.insert("paquete", "packs");
        m.insert("pieza", "pcs");
        m
    })
}

/// Returns the canonical key for a normalized unit text form. Falls back to
/// the input when no alias matches, so existing keys (e.g. "kg") still work.
fn apply_unit_alias(normalized: &str) -> &str {
    unit_text_aliases()
        .get(normalized)
        .copied()
        .unwrap_or(normalized)
}

/// Resolves `default_unit_id` and `unit_type` from the provided inputs.
///
/// Priority:
/// 1. If `explicit_unit_id` is Some, look it up in the catalog and use its kind.
/// 2. Else if `default_unit_text` is Some, do a case-insensitive lookup:
///    apply a singular→plural alias (e.g. `Unidad` → `units`) and then
///    `find_by_key` against the canonical key; fall back to
///    `find_by_display_name_ci` so display names like `Kilogramo` or
///    `Unidades` resolve to their catalog row.
///    On hit: use the catalog id and kind. On miss: leave both None.
/// 3. Else (no unit info): both None.
async fn resolve_unit_fields(
    pool: &DbPool,
    explicit_unit_id: Option<&str>,
    default_unit_text: Option<&str>,
) -> Result<(Option<String>, Option<String>), AppError> {
    use crate::db::repositories::unit_definitions as units_repo;
    use crate::dto::unit_definitions::UnitKind;

    // Path 1: explicit unit_id.
    if let Some(id) = explicit_unit_id {
        let unit = units_repo::find_by_id(pool, id)
            .await
            .map_err(AppError::from)?
            .ok_or(DomainError::NotFound {
                resource: "unit_definition",
                id: id.to_string(),
            })?;
        let kind_str = match unit.kind {
            UnitKind::Integer => "integer",
            UnitKind::Decimal => "decimal",
        };
        return Ok((Some(unit.id), Some(kind_str.to_string())));
    }

    // Path 2: text-based lookup.
    if let Some(text) = default_unit_text {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            let normalized = trimmed.to_lowercase();
            let canonical = apply_unit_alias(&normalized);

            // 2a: alias-canonicalized key lookup (handles "Unidad" → "units").
            if let Some(unit) = units_repo::find_by_key(pool, canonical)
                .await
                .map_err(AppError::from)?
            {
                let kind_str = match unit.kind {
                    UnitKind::Integer => "integer",
                    UnitKind::Decimal => "decimal",
                };
                return Ok((Some(unit.id), Some(kind_str.to_string())));
            }

            // 2b: display_name lookup (handles "Kilogramo", "Unidades", etc.).
            if let Some(unit) = units_repo::find_by_display_name_ci(pool, trimmed)
                .await
                .map_err(AppError::from)?
            {
                let kind_str = match unit.kind {
                    UnitKind::Integer => "integer",
                    UnitKind::Decimal => "decimal",
                };
                return Ok((Some(unit.id), Some(kind_str.to_string())));
            }
        }
    }

    // Path 3: no unit info.
    Ok((None, None))
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

/// Returns the full product detail (product + barcodes + resolved categories).
pub async fn get_product(pool: &DbPool, id: String) -> Result<ProductDetailResponse, AppError> {
    let product = repo::get_product(pool, &id)
        .await?
        .ok_or(DomainError::NotFound {
            resource: "product",
            id: id.clone(),
        })?;

    let barcodes = repo::list_barcodes(pool, &id).await?;

    // Resolve all category ids to CategoryResponse objects.
    // For typical small category sets, fetch each id individually.
    // This is not N+1 for products (see the batched helper for batch reads).
    let categories = if product.category_ids.is_empty() {
        Vec::new()
    } else {
        let mut cats = Vec::with_capacity(product.category_ids.len());
        for cid in &product.category_ids {
            if let Some(row) = repo::get_category(pool, cid).await? {
                if row.is_active {
                    cats.push(row);
                }
            }
        }
        cats
    };

    Ok(ProductDetailResponse {
        product,
        barcodes,
        categories,
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
            message: user_message(UserMessage::ScanValueEmpty, Locale::En),
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
            category_ids: None,
            default_unit: Some("L".into()),
            default_unit_id: None,
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
                category_ids: None,
                default_unit: Some("kg".into()),
                default_unit_id: None,
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
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
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
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
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
                category_ids: None,
                default_unit: Some("L".into()),
                default_unit_id: None,
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
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
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

    // ====================================================================
    // Unit resolver — singular/plural Spanish + display_name coverage
    //
    // Regression for SKU 1106000626: the product has `default_unit = "Unidad"`
    // (singular Spanish) and was previously unlinked, so Product Detail showed
    // `—` and LotForm accepted fractional quantities.
    // ====================================================================

    // Singular Spanish `Unidad` resolves to integer preset `ud-units`.
    #[tokio::test]
    async fn create_product_with_unidad_singular_resolves_to_integer(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(
            &pool,
            ProductCreate {
                sku: "1106000626".into(),
                description: "SKU 1106000626".into(),
                category_ids: None,
                default_unit: Some("Unidad".into()),
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            p.default_unit_id.as_deref(),
            Some("ud-units"),
            "singular `Unidad` must resolve to ud-units via alias"
        );
        assert_eq!(
            p.unit_type,
            Some(crate::dto::unit_definitions::UnitKind::Integer),
            "singular `Unidad` must yield UnitKind::Integer"
        );
        Ok(())
    }

    // Plural Spanish `Unidades` resolves to integer preset `ud-units` via display_name.
    #[tokio::test]
    async fn create_product_with_unidades_plural_resolves_to_integer(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-UNIDADES".into(),
                description: "Plural form".into(),
                category_ids: None,
                default_unit: Some("Unidades".into()),
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            p.default_unit_id.as_deref(),
            Some("ud-units"),
            "plural `Unidades` must resolve to ud-units via display_name"
        );
        assert_eq!(
            p.unit_type,
            Some(crate::dto::unit_definitions::UnitKind::Integer),
            "plural `Unidades` must yield UnitKind::Integer"
        );
        Ok(())
    }

    // Decimal units (`kg`) still resolve to decimal via catalog key. Preserves prior semantics.
    #[tokio::test]
    async fn create_product_with_kg_key_resolves_to_decimal(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-KG".into(),
                description: "Decimal unit".into(),
                category_ids: None,
                default_unit: Some("kg".into()),
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        assert_eq!(p.default_unit_id.as_deref(), Some("ud-kg"));
        assert_eq!(
            p.unit_type,
            Some(crate::dto::unit_definitions::UnitKind::Decimal),
            "decimal-unit semantics must be preserved"
        );
        Ok(())
    }

    // Display_name match: `Kilogramo` resolves to `ud-kg` (decimal).
    #[tokio::test]
    async fn create_product_with_kilogramo_display_name_resolves_to_decimal(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-KILO-DN".into(),
                description: "Display name match".into(),
                category_ids: None,
                default_unit: Some("Kilogramo".into()),
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            p.default_unit_id.as_deref(),
            Some("ud-kg"),
            "`Kilogramo` (display_name) must resolve to ud-kg"
        );
        assert_eq!(
            p.unit_type,
            Some(crate::dto::unit_definitions::UnitKind::Decimal)
        );
        Ok(())
    }

    // Truly-unknown values remain unlinked (no catalog row to point at).
    #[tokio::test]
    async fn create_product_with_unknown_unit_remains_unlinked(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-UNK".into(),
                description: "Unknown unit".into(),
                category_ids: None,
                default_unit: Some("totally-unknown".into()),
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        assert!(
            p.default_unit_id.is_none(),
            "unknown unit must not link to any catalog row"
        );
        assert!(
            p.unit_type.is_none(),
            "unknown unit must keep unit_type as None"
        );
        // Raw text is preserved verbatim for the audit banner / review surface.
        assert_eq!(p.default_unit.as_deref(), Some("totally-unknown"));
        Ok(())
    }

    // Updating an existing product with the singular form also links it.
    #[tokio::test]
    async fn update_product_with_unidad_singular_links_to_integer_preset(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-UP".into(),
                description: "Update path".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        assert!(created.default_unit_id.is_none());
        assert!(created.unit_type.is_none());

        let updated = update_product(
            &pool,
            ProductUpdate {
                id: created.id.clone(),
                sku: created.sku.clone(),
                description: created.description.clone(),
                category_ids: None,
                default_unit: Some("Unidad".into()),
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
                is_active: true,
            },
        )
        .await?;

        assert_eq!(updated.default_unit_id.as_deref(), Some("ud-units"));
        assert_eq!(
            updated.unit_type,
            Some(crate::dto::unit_definitions::UnitKind::Integer)
        );
        Ok(())
    }

    // Singular Spanish `Caja` (alias for `cajas`) resolves to integer preset.
    #[tokio::test]
    async fn create_product_with_caja_singular_resolves_to_integer(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-CAJA-S".into(),
                description: "Caja singular".into(),
                category_ids: None,
                default_unit: Some("Caja".into()),
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            p.default_unit_id.as_deref(),
            Some("ud-cajas"),
            "singular `Caja` must alias to ud-cajas"
        );
        assert_eq!(
            p.unit_type,
            Some(crate::dto::unit_definitions::UnitKind::Integer)
        );
        Ok(())
    }

    // Idempotency: a known FK supplied explicitly bypasses the alias logic but
    // still yields the correct kind. Confirms explicit_unit_id path stays intact.
    #[tokio::test]
    async fn create_product_with_explicit_decimal_fk_resolves_to_decimal(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = create_product(
            &pool,
            ProductCreate {
                sku: "SKU-FK".into(),
                description: "Explicit FK".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: Some("ud-kg".into()),
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        assert_eq!(p.default_unit_id.as_deref(), Some("ud-kg"));
        assert_eq!(
            p.unit_type,
            Some(crate::dto::unit_definitions::UnitKind::Decimal)
        );
        Ok(())
    }
}
