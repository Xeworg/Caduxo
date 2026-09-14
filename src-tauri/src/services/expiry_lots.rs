//! Expiry lot service — create, update, archive, resolve.
//!
//! Business rules (pre-fill, validation, resolution) live here.
//! SQL is delegated to `db::repositories::expiry_lots`.
//! Product/store lookups use `db::repositories::products` and `stores`.

use chrono::NaiveDate;

use crate::db::repositories::expiry_lots as repo;
use crate::db::repositories::products as product_repo;
use crate::db::repositories::stores as store_repo;
use crate::db::repositories::unit_definitions as unit_repo;
use crate::db::DbPool;
use crate::domain::lot_resolution::{
    compute_remaining_quantity, is_fully_resolved, is_valid_quantity,
};
use crate::dto::expiry_lots::{
    ExpiryLotCreate, ExpiryLotResolve, ExpiryLotResolveResult, ExpiryLotResponse, ExpiryLotUpdate,
    LotResolutionEventResponse,
};
use crate::error::{AppError, DomainError};

/// Valid resolution type values.
const VALID_RESOLUTIONS: &[&str] = &["consumed", "sold", "discarded", "donated", "other"];

/// Returns true if the given resolution type is allowed.
fn is_valid_resolution_type(resolution: &str) -> bool {
    VALID_RESOLUTIONS.contains(&resolution.trim().to_ascii_lowercase().as_str())
}

/// Validates that a quantity value is positive.
fn validate_quantity(qty: f64) -> Result<(), DomainError> {
    if !is_valid_quantity(qty) {
        return Err(DomainError::Validation {
            message: format!("Quantity must be positive, got {qty}"),
        });
    }
    Ok(())
}

/// Validates an ISO-8601 date string (YYYY-MM-DD).
fn validate_expiry_date(date: &str) -> Result<(), DomainError> {
    let has_strict_shape = date.len() == 10
        && date.as_bytes()[4] == b'-'
        && date.as_bytes()[7] == b'-'
        && date
            .bytes()
            .enumerate()
            .all(|(idx, byte)| matches!(idx, 4 | 7) || byte.is_ascii_digit());
    if !has_strict_shape {
        return Err(DomainError::Validation {
            message: format!("Invalid expiry date format: `{date}` (expected YYYY-MM-DD)"),
        });
    }

    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| DomainError::Validation {
            message: format!("Invalid expiry date format: `{date}` (expected YYYY-MM-DD)"),
        })
}

/// Resolves the product's default unit from the catalog.
/// Priority: (1) product.default_unit_id → joined display_name from unit_definitions;
/// (2) product.default_unit raw text if no catalog link; (3) "Unidades" preset fallback.
async fn resolve_product_default_unit(
    pool: &DbPool,
    product: &crate::dto::products::ProductResponse,
) -> String {
    if let Some(ref unit_id) = product.default_unit_id {
        if let Ok(Some(unit)) = unit_repo::find_by_id(pool, unit_id).await {
            return unit.display_name;
        }
    }
    // No catalog link: fall back to raw text or the "Unidades" preset.
    if let Some(raw) = product.default_unit.as_deref() {
        if !raw.trim().is_empty() {
            return raw.trim().to_string();
        }
    }
    // Fallback to "Unidades" preset (key: "ud-units").
    unit_repo::find_by_id(pool, "ud-units")
        .await
        .ok()
        .flatten()
        .map(|u| u.display_name)
        .unwrap_or_else(|| "Unidades".to_string())
}

/// Returns the effective alert days for a lot, preferring user-supplied overrides
/// (any non-negative value) and falling back to the product default.
fn resolve_alert_days(user_alert: Option<i32>, product_default: i32) -> i32 {
    match user_alert {
        Some(d) if d >= 0 => d,
        _ => product_default,
    }
}

// ============================================================
// Create
// ============================================================

/// Creates a new expiry lot. Fails if no active store exists (precondition).
/// Unit and alert_days are pre-filled from the product defaults when omitted.
pub async fn create_expiry_lot(
    pool: &DbPool,
    input: ExpiryLotCreate,
) -> Result<ExpiryLotResponse, AppError> {
    // ── Precondition: at least one active store must exist. ──────────────────
    let store_exists = store_repo::has_active_store(pool).await?;
    if !store_exists {
        return Err(DomainError::BusinessRule {
            message: "Cannot create expiry lot: at least one store must exist first".to_string(),
        }
        .into());
    }

    // ── Validate input fields. ────────────────────────────────────────────────
    validate_quantity(input.quantity).map_err(AppError::Domain)?;
    validate_expiry_date(&input.expiry_date).map_err(AppError::Domain)?;

    // ── Look up the product to pre-fill defaults. ──────────────────────────
    let product = product_repo::get_product(pool, &input.product_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| DomainError::NotFound {
            resource: "product",
            id: input.product_id.clone(),
        })?;

    // ── Resolve unit: user-supplied wins; otherwise resolve via catalog. ──────
    let unit = match input.unit.as_deref() {
        Some(u) if !u.trim().is_empty() => u.trim().to_string(),
        _ => resolve_product_default_unit(pool, &product).await,
    };
    let alert_days_before =
        resolve_alert_days(input.alert_days_before, product.default_alert_days_before);

    repo::insert_expiry_lot(pool, &input, &unit, alert_days_before)
        .await
        .map_err(AppError::from)
}

// ============================================================
// Update
// ============================================================

/// Updates an existing active expiry lot. Returns `NotFound` if the lot does
/// not exist or is already archived.
pub async fn update_expiry_lot(
    pool: &DbPool,
    input: ExpiryLotUpdate,
) -> Result<ExpiryLotResponse, AppError> {
    validate_quantity(input.quantity).map_err(AppError::Domain)?;
    validate_expiry_date(&input.expiry_date).map_err(AppError::Domain)?;

    let row = repo::update_expiry_lot(pool, &input)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "expiry_lot",
            id: input.id.clone(),
        })?;
    Ok(row)
}

// ============================================================
// Archive
// ============================================================

/// Soft-archives an expiry lot. Returns `NotFound` if the lot does not exist.
pub async fn archive_expiry_lot(pool: &DbPool, id: String) -> Result<(), AppError> {
    let archived = repo::archive_expiry_lot(pool, &id)
        .await
        .map_err(AppError::from)?;
    if !archived {
        return Err(DomainError::NotFound {
            resource: "expiry_lot",
            id,
        }
        .into());
    }
    Ok(())
}

// ============================================================
// Get / List
// ============================================================

/// Returns a single expiry lot by id.
pub async fn get_expiry_lot(pool: &DbPool, id: String) -> Result<ExpiryLotResponse, AppError> {
    let row = repo::get_expiry_lot(pool, &id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "expiry_lot",
            id,
        })?;
    Ok(row)
}

/// Returns all active expiry lots for a product, ordered by expiry date.
pub async fn list_expiry_lots_by_product(
    pool: &DbPool,
    product_id: String,
) -> Result<Vec<ExpiryLotResponse>, AppError> {
    repo::list_expiry_lots_by_product(pool, &product_id)
        .await
        .map_err(AppError::from)
}

/// Returns all active expiry lots for a store, ordered by expiry date.
pub async fn list_expiry_lots_by_store(
    pool: &DbPool,
    store_id: String,
) -> Result<Vec<ExpiryLotResponse>, AppError> {
    repo::list_expiry_lots_by_store(pool, &store_id)
        .await
        .map_err(AppError::from)
}

/// Returns all active expiry lots across all stores, ordered by expiry date.
pub async fn list_all_expiry_lots(pool: &DbPool) -> Result<Vec<ExpiryLotResponse>, AppError> {
    repo::list_all_active_expiry_lots(pool)
        .await
        .map_err(AppError::from)
}

// ============================================================
// Partial resolution
// ============================================================

/// Resolves a quantity from an expiry lot, records a resolution event, and
/// marks the lot as fully resolved when remaining quantity reaches zero.
///
/// Returns the resolution event id, resolved quantity, remaining quantity, and
/// whether the lot is now fully resolved.
pub async fn resolve_expiry_lot(
    pool: &DbPool,
    input: ExpiryLotResolve,
) -> Result<ExpiryLotResolveResult, AppError> {
    // ── Validate. ────────────────────────────────────────────────────────────
    validate_quantity(input.quantity).map_err(AppError::Domain)?;
    if !is_valid_resolution_type(&input.resolution) {
        return Err(AppError::Domain(DomainError::Validation {
            message: format!(
                "Resolution must be one of {:?}, got `{}`",
                VALID_RESOLUTIONS, input.resolution
            ),
        }));
    }

    // ── Fetch the lot and validate it exists and is active. ─────────────────
    let lot = repo::get_expiry_lot(pool, &input.lot_id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "expiry_lot",
            id: input.lot_id.clone(),
        })?;

    if lot.status != "active" {
        return Err(DomainError::BusinessRule {
            message: format!("Cannot resolve lot: lot is already `{}`", lot.status),
        }
        .into());
    }

    // ── Validate resolved_qty does not exceed remaining quantity. ────────────
    if input.quantity > lot.quantity {
        return Err(AppError::Domain(DomainError::Validation {
            message: format!(
                "Cannot resolve {:.2} {}: only {:.2} {} remain",
                input.quantity, lot.unit, lot.quantity, lot.unit
            ),
        }));
    }

    // ── Record the resolution event first (event is the source of truth). ──
    let event = repo::insert_resolution_event(pool, &input)
        .await
        .map_err(AppError::from)?;

    // ── Compute remaining quantity and update the lot. ───────────────────────
    // SAFETY: `input.quantity <= lot.quantity` is enforced by the guard above,
    // so `compute_remaining_quantity` is guaranteed to return `Some`.
    let remaining = match compute_remaining_quantity(lot.quantity, input.quantity) {
        Some(v) => v,
        None => unreachable!(),
    };
    let now = chrono::Utc::now();
    let resolved_at = now.to_rfc3339();

    repo::apply_partial_resolution(
        pool,
        &input.lot_id,
        input.quantity,
        &input.resolution,
        &resolved_at,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ExpiryLotResolveResult {
        lot_id: input.lot_id,
        resolved_quantity: input.quantity,
        remaining_quantity: remaining,
        is_fully_resolved: is_fully_resolved(remaining),
        resolution_event_id: event.id,
    })
}

/// Returns all resolution events for a given expiry lot.
pub async fn list_resolution_events(
    pool: &DbPool,
    lot_id: String,
) -> Result<Vec<LotResolutionEventResponse>, AppError> {
    // Verify the lot exists first.
    let _ = repo::get_expiry_lot(pool, &lot_id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "expiry_lot",
            id: lot_id.clone(),
        })?;

    repo::list_resolution_events_by_lot(pool, &lot_id)
        .await
        .map_err(AppError::from)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::expiry_lots::{ExpiryLotCreate, ExpiryLotResolve, ExpiryLotUpdate};
    use crate::dto::products::ProductCreate;
    use crate::dto::stores::StoreCreate;
    use crate::services::expiry_lots as svc;
    use crate::services::stores::create_store as create_store_svc;

    /// Helper: creates a store + product, returning (store_id, product_id).
    async fn seed_product(
        pool: &crate::db::DbPool,
    ) -> Result<(String, String), Box<dyn std::error::Error>> {
        // Create a store first (required by lot creation).
        let store = create_store_svc(
            pool,
            StoreCreate {
                name: "Test Store".into(),
                code: None,
                notes: None,
            },
        )
        .await?;

        // Create a product with a known default_unit and alert_days.
        let product = crate::services::products::create_product(
            pool,
            ProductCreate {
                sku: "SEED-SKU-001".into(),
                description: "Test Product".into(),
                category_id: None,
                default_unit: Some("kg".into()),
                default_unit_id: None,
                default_alert_days_before: 14,
                notes: None,
            },
        )
        .await?;
        Ok((store.id, product.id))
    }

    /// Helper: creates a store + product + one expiry lot, returning (store_id, product_id, lot).
    async fn seed_lot(
        pool: &crate::db::DbPool,
        expiry_date: &str,
    ) -> Result<
        (String, String, crate::dto::expiry_lots::ExpiryLotResponse),
        Box<dyn std::error::Error>,
    > {
        let (store_id, product_id) = seed_product(pool).await?;
        let lot = svc::create_expiry_lot(
            pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 10.0,
                unit: None,
                expiry_date: expiry_date.into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;
        Ok((store_id, product_id, lot))
    }

    // ------------------------------------------------------------------
    // Create — precondition: store must exist
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn create_lot_requires_store() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // No store → must fail with BusinessRule.
        let err = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: "fake".into(),
                store_id: "no-such-store".into(),
                location_id: None,
                quantity: 5.0,
                unit: Some("L".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(7),
                batch_code: None,
                notes: None,
            },
        )
        .await
        .expect_err("creating lot without a store must fail");

        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::BusinessRule { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(), // pool has one store seeded
                location_id: None,
                quantity: 10.0,
                unit: Some("L".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(7),
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(lot.product_id, product_id);
        assert_eq!(lot.quantity, 10.0);
        assert_eq!(lot.unit, "L");
        assert_eq!(lot.alert_days_before, 7);
        assert_eq!(lot.status, "active");
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_pre_fills_unit_from_product() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        // User omits unit → product default "kg" should be used.
        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 2.0,
                unit: None, // not provided
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            lot.unit, "Kilogramo",
            "unit should be pre-filled from product default via catalog display_name"
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_user_unit_overrides_product_default(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        // Product has default "kg" but user specifies "g" → user wins.
        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 2.0,
                unit: Some("g".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            lot.unit, "g",
            "user-supplied unit must override product default"
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_pre_fills_alert_days_from_product() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        // Product has default 14 alert days; user omits → should use 14.
        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 3.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None, // not provided
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            lot.alert_days_before, 14,
            "alert_days should be pre-filled from product default"
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_user_alert_days_overrides_product_default(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        // Product has default 14 but user specifies 30 → user wins.
        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 3.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(30),
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            lot.alert_days_before, 30,
            "user-supplied alert_days must override product default"
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_rejects_negative_quantity() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let err = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: -1.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await
        .expect_err("negative quantity must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_rejects_zero_quantity() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let err = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 0.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await
        .expect_err("zero quantity must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_rejects_invalid_expiry_date() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        for bad_date in ["not-a-date", "2025/12/31", "25-12-31", ""] {
            let err = svc::create_expiry_lot(
                &pool,
                ExpiryLotCreate {
                    product_id: product_id.clone(),
                    store_id: store_id.clone(),
                    location_id: None,
                    quantity: 5.0,
                    unit: None,
                    expiry_date: bad_date.into(),
                    alert_days_before: None,
                    batch_code: None,
                    notes: None,
                },
            )
            .await
            .expect_err(&format!("invalid date `{bad_date}` must be rejected"));
            assert!(matches!(
                err,
                crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
            ));
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Update
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn update_lot_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 10.0,
                unit: Some("L".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(7),
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let updated = svc::update_expiry_lot(
            &pool,
            ExpiryLotUpdate {
                id: lot.id.clone(),
                location_id: None,
                quantity: 8.0,
                unit: "mL".into(),
                expiry_date: "2026-01-15".into(),
                alert_days_before: 14,
                batch_code: Some("BATCH-001".into()),
                notes: Some("Updated notes".into()),
            },
        )
        .await?;

        assert_eq!(updated.quantity, 8.0);
        assert_eq!(updated.unit, "mL");
        assert_eq!(updated.expiry_date, "2026-01-15");
        assert_eq!(updated.alert_days_before, 14);
        assert_eq!(updated.batch_code.as_deref(), Some("BATCH-001"));
        Ok(())
    }

    #[tokio::test]
    async fn update_missing_lot_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::update_expiry_lot(
            &pool,
            ExpiryLotUpdate {
                id: "nope".into(),
                location_id: None,
                quantity: 5.0,
                unit: "kg".into(),
                expiry_date: "2025-12-31".into(),
                alert_days_before: 30,
                batch_code: None,
                notes: None,
            },
        )
        .await
        .expect_err("updating missing lot must be NotFound");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::NotFound { .. })
        ));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Archive
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn archive_lot_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        svc::archive_expiry_lot(&pool, lot.id.clone()).await?;

        // Archived lot should no longer appear in the active list.
        let lots = svc::list_expiry_lots_by_product(&pool, lot.product_id.clone()).await?;
        assert!(
            lots.is_empty(),
            "archived lot should not appear in active list"
        );
        Ok(())
    }

    #[tokio::test]
    async fn archive_missing_lot_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::archive_expiry_lot(&pool, "nope".into())
            .await
            .expect_err("archiving missing lot must be NotFound");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::NotFound { .. })
        ));
        Ok(())
    }

    // ------------------------------------------------------------------
    // List
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn list_lots_by_product_ordered_by_expiry() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        // Create two lots with different expiry dates.
        let _early = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-06-01".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;
        let _late = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 3.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let lots = svc::list_expiry_lots_by_product(&pool, product_id).await?;
        assert_eq!(lots.len(), 2);
        assert_eq!(lots[0].expiry_date, "2025-06-01", "early expiry first");
        assert_eq!(lots[1].expiry_date, "2025-12-31", "late expiry second");
        Ok(())
    }

    // ------------------------------------------------------------------
    // Partial resolution
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn partial_resolution_reduces_quantity_and_records_event(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 10.0,
                unit: Some("L".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(7),
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // Resolve 3 units.
        let result = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 3.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await?;

        assert_eq!(result.resolved_quantity, 3.0);
        assert_eq!(result.remaining_quantity, 7.0);
        assert!(!result.is_fully_resolved);

        // Verify the lot quantity was updated.
        let updated = svc::get_expiry_lot(&pool, lot.id.clone()).await?;
        assert_eq!(updated.quantity, 7.0);
        assert_eq!(updated.status, "active");

        // Verify the resolution event was recorded.
        let events = svc::list_resolution_events(&pool, lot.id.clone()).await?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].quantity, 3.0);
        assert_eq!(events[0].resolution, "consumed");
        Ok(())
    }

    #[tokio::test]
    async fn full_resolution_marks_lot_resolved() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // Resolve all 5 units at once.
        let result = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 5.0,
                resolution: "discarded".into(),
                notes: Some("All expired".into()),
            },
        )
        .await?;

        assert_eq!(result.remaining_quantity, 0.0);
        assert!(result.is_fully_resolved);

        let updated = svc::get_expiry_lot(&pool, lot.id.clone()).await?;
        assert_eq!(updated.status, "resolved");
        // DB clamps quantity to 1.0 to satisfy CHECK(quantity > 0); remaining is tracked via events.
        assert_eq!(updated.quantity, 1.0);
        assert_eq!(updated.resolution.as_deref(), Some("discarded"));
        assert!(updated.resolved_at.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn cannot_resolve_more_than_remaining() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let err = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 10.0, // more than the 5 remaining
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await
        .expect_err("resolving more than remaining must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn cannot_resolve_archived_or_resolved_lot() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // Resolve fully → status becomes "resolved".
        svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 5.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await?;

        // Second resolution attempt must fail.
        let err = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 1.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await
        .expect_err("resolving already-resolved lot must fail");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::BusinessRule { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn resolve_rejects_invalid_resolution_type() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let err = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 2.0,
                resolution: "eaten".into(), // not a valid type
                notes: None,
            },
        )
        .await
        .expect_err("invalid resolution type must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn resolve_rejects_zero_quantity() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let err = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 0.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await
        .expect_err("zero resolution quantity must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn multiple_partial_resolutions_accumulate() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 10.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // First partial resolution.
        let r1 = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 3.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await?;
        assert_eq!(r1.remaining_quantity, 7.0);

        // Second partial resolution.
        let r2 = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 4.0,
                resolution: "discarded".into(),
                notes: None,
            },
        )
        .await?;
        assert_eq!(r2.remaining_quantity, 3.0);
        assert!(!r2.is_fully_resolved);

        // Both events recorded.
        let events = svc::list_resolution_events(&pool, lot.id.clone()).await?;
        assert_eq!(events.len(), 2);

        // Final resolution.
        let r3 = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 3.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await?;
        assert!(r3.is_fully_resolved);
        assert_eq!(r3.remaining_quantity, 0.0);

        let updated = svc::get_expiry_lot(&pool, lot.id.clone()).await?;
        assert_eq!(updated.status, "resolved");
        Ok(())
    }
}
