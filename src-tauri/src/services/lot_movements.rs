//! Lot movement service — ledger write, listing, and per-location balances.
//!
//! Business rules live here; SQL is delegated to `db::repositories::lot_movements`.

use chrono::Local;
use sqlx::SqlitePool;

use crate::db::repositories::expiry_lots as lot_repo;
use crate::db::repositories::lot_movements as repo;
use crate::db::repositories::settings as settings_repo;
use crate::db::DbPool;
use crate::domain::lot_movements::{
    compute_signed_delta, validate_movement_kind, validate_notes, Direction, MovementKind,
};
use crate::domain::lot_movements::{derive_batch_prefix, next_batch_candidate};
use crate::dto::lot_movements::{
    Direction as DtoDirection, LotLocationBalance, LotMovementCreate, LotMovementResponse,
};
use crate::error::{AppError, DomainError};

// ============================================================
// Validation helpers
// ============================================================

/// Validates that the movement kind is known.
fn validate_kind(kind: &str) -> Result<MovementKind, DomainError> {
    validate_movement_kind(kind).ok_or_else(|| DomainError::Validation {
        message: format!("Unknown movement kind: `{}`", kind),
    })
}

/// Validates direction usage: inventory_adjustment requires direction, others must not have one.
fn validate_direction_usage(
    kind: &MovementKind,
    direction: Option<&DtoDirection>,
) -> Result<(), DomainError> {
    match kind {
        MovementKind::InventoryAdjustment => {
            if direction.is_none() {
                return Err(DomainError::Validation {
                    message: "direction is required for inventory_adjustment".to_string(),
                });
            }
        }
        _ => {
            if direction.is_some() {
                return Err(DomainError::Validation {
                    message: format!(
                        "direction is only valid for inventory_adjustment, not `{}`",
                        kind
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Validates location nullability for the movement kind.
fn validate_location_nullability(
    kind: &MovementKind,
    source: Option<&str>,
    dest: Option<&str>,
    direction: Option<&DtoDirection>,
) -> Result<(), DomainError> {
    match kind {
        MovementKind::EntryInitial => {
            if source.is_some() {
                return Err(DomainError::Validation {
                    message: "entry:initial must not have a source location".to_string(),
                });
            }
            if dest.is_none() {
                return Err(DomainError::Validation {
                    message: "entry:initial requires a destination location".to_string(),
                });
            }
        }
        MovementKind::Transfer => {
            if source.is_none() {
                return Err(DomainError::Validation {
                    message: "transfer requires a source location".to_string(),
                });
            }
            if dest.is_none() {
                return Err(DomainError::Validation {
                    message: "transfer requires a destination location".to_string(),
                });
            }
            if source == dest {
                return Err(DomainError::Validation {
                    message: "transfer source and destination must differ".to_string(),
                });
            }
        }
        MovementKind::InventoryAdjustment => match direction {
            Some(DtoDirection::Increase) => {
                if source.is_some() {
                    return Err(DomainError::Validation {
                            message: "inventory_adjustment with direction=increase must not have a source location".to_string(),
                        });
                }
                if dest.is_none() {
                    return Err(DomainError::Validation {
                            message: "inventory_adjustment with direction=increase requires a destination location".to_string(),
                        });
                }
            }
            Some(DtoDirection::Decrease) => {
                if dest.is_some() {
                    return Err(DomainError::Validation {
                            message: "inventory_adjustment with direction=decrease must not have a destination location".to_string(),
                        });
                }
                if source.is_none() {
                    return Err(DomainError::Validation {
                            message: "inventory_adjustment with direction=decrease requires a source location".to_string(),
                        });
                }
            }
            None => {
                return Err(DomainError::Validation {
                    message: "direction is required for inventory_adjustment".to_string(),
                });
            }
        },
        _ => {
            // All exit kinds require source, no destination
            if source.is_none() {
                return Err(DomainError::Validation {
                    message: format!("`{}` requires a source location", kind),
                });
            }
            if dest.is_some() {
                return Err(DomainError::Validation {
                    message: format!("`{}` must not have a destination location", kind),
                });
            }
        }
    }
    Ok(())
}

/// Validates quantity is positive. entry:initial is allowed to have quantity = 0
/// since the lot already has the quantity from the INSERT (no double-counting).
fn validate_quantity(qty: f64, kind: &MovementKind) -> Result<(), DomainError> {
    if qty < 0.0 {
        return Err(DomainError::Validation {
            message: format!("Quantity must be non-negative, got {}", qty),
        });
    }
    if qty == 0.0 && !matches!(kind, MovementKind::EntryInitial) {
        return Err(DomainError::Validation {
            message: format!("Quantity must be positive, got {}", qty),
        });
    }
    Ok(())
}

/// Validates source balance coverage for the movement.
async fn validate_source_balance(
    pool: &SqlitePool,
    lot_id: &str,
    source_location_id: &str,
    quantity: f64,
) -> Result<(), AppError> {
    let balances = repo::location_balances_for_lot(pool, lot_id).await?;

    let current_balance = balances
        .iter()
        .find(|b| b.location_id == source_location_id)
        .map(|b| b.balance)
        .unwrap_or(0.0);

    if current_balance < quantity {
        return Err(DomainError::BusinessRule {
            message: format!(
                "Insufficient balance at source location: available={}, requested={}",
                current_balance, quantity
            ),
        }
        .into());
    }
    Ok(())
}

/// Validates location is active.
async fn validate_location_active(pool: &SqlitePool, location_id: &str) -> Result<(), AppError> {
    let is_active: bool = sqlx::query_scalar("SELECT is_active FROM store_locations WHERE id = $1")
        .bind(location_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or(false);

    if !is_active {
        return Err(DomainError::BusinessRule {
            message: "Location is inactive".to_string(),
        }
        .into());
    }
    Ok(())
}

// ============================================================
// Public service API
// ============================================================

/// Creates a new lot movement and updates the lot total atomically.
///
/// # Arguments
/// * `pool` - Database pool
/// * `input` - Movement creation input
///
/// # Returns
/// The created movement response.
pub async fn create_lot_movement(
    pool: &DbPool,
    input: LotMovementCreate,
) -> Result<LotMovementResponse, AppError> {
    // ── Validate movement kind ─────────────────────────────────────────────────
    let kind = validate_kind(&input.kind)?;
    validate_direction_usage(&kind, input.direction.as_ref())?;
    validate_quantity(input.quantity, &kind)?;
    validate_location_nullability(
        &kind,
        input.source_location_id.as_deref(),
        input.destination_location_id.as_deref(),
        input.direction.as_ref(),
    )?;
    validate_notes(&kind, input.notes.as_deref())
        .map_err(|msg| DomainError::Validation { message: msg })?;

    // ── Validate lot exists ───────────────────────────────────────────────────
    let lot = lot_repo::get_expiry_lot(pool, &input.lot_id)
        .await?
        .ok_or_else(|| DomainError::NotFound {
            resource: "lot",
            id: input.lot_id.clone(),
        })?;

    // ── Validate source balance for exits and transfers ───────────────────────
    if let Some(ref src) = input.source_location_id {
        // Check location is active
        validate_location_active(pool, src).await?;

        // Check balance is sufficient for non-entry movements
        if !matches!(kind, MovementKind::EntryInitial) {
            validate_source_balance(pool, &input.lot_id, src, input.quantity).await?;
        }
    }

    // ── Validate destination location is active ────────────────────────────────
    if let Some(ref dst) = input.destination_location_id {
        validate_location_active(pool, dst).await?;
    }

    // ── Compute signed delta ───────────────────────────────────────────────────
    let direction_domain = input.direction.as_ref().map(|d| match d {
        DtoDirection::Increase => Direction::Increase,
        DtoDirection::Decrease => Direction::Decrease,
    });
    let delta = compute_signed_delta(&kind, direction_domain.as_ref(), input.quantity);

    // ── Insert movement and update lot total in a transaction ──────────────────
    let mut tx = pool.begin().await?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let direction_str: Option<String> = input.direction.map(|d| match d {
        DtoDirection::Increase => "increase".to_string(),
        DtoDirection::Decrease => "decrease".to_string(),
    });

    sqlx::query(
        r#"
            INSERT INTO lot_movements (
                id, expiry_lot_id, movement_kind, direction, quantity,
                source_location_id, destination_location_id, notes, actor, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
    )
    .bind(&id)
    .bind(&input.lot_id)
    .bind(&input.kind)
    .bind(&direction_str)
    .bind(input.quantity)
    .bind(&input.source_location_id)
    .bind(&input.destination_location_id)
    .bind(&input.notes)
    .bind("system")
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // For entry:initial, the lot already has the correct quantity from INSERT.
    // We only update status/resolved_at, NOT the quantity (to avoid double-counting).
    // For all other movements, we update the quantity.
    if matches!(kind, MovementKind::EntryInitial) {
        // Only update timestamp, status, and resolution for entry:initial
        let new_status = if lot.status == "resolved" && lot.quantity > 0.0 {
            "active"
        } else {
            &lot.status
        };
        sqlx::query(
            r#"
                UPDATE expiry_lots
                SET status = $1,
                    resolution = NULL,
                    resolved_at = NULL,
                    updated_at = $2
                WHERE id = $3
                "#,
        )
        .bind(new_status)
        .bind(&now)
        .bind(&input.lot_id)
        .execute(&mut *tx)
        .await?;
    } else {
        // Update lot total and handle resolution/activation
        let new_quantity = lot.quantity + delta;
        let is_reactivation = new_quantity > 0.0 && lot.status == "resolved";
        let new_status = if new_quantity == 0.0 {
            "resolved"
        } else if is_reactivation {
            "active"
        } else {
            &lot.status
        };
        // Clear resolution on reactivation; set on resolution to 0
        let new_resolution = if new_quantity == 0.0 {
            Some(input.kind.clone())
        } else if is_reactivation {
            None
        } else {
            lot.resolution.clone()
        };
        let new_resolved_at = if new_quantity == 0.0 {
            Some(now.clone())
        } else if is_reactivation {
            None
        } else {
            lot.resolved_at.clone()
        };

        sqlx::query(
            r#"
                UPDATE expiry_lots
                SET quantity = $1,
                    status = $2,
                    resolution = $3,
                    resolved_at = $4,
                    updated_at = $5
                WHERE id = $6
                "#,
        )
        .bind(new_quantity)
        .bind(new_status)
        .bind(&new_resolution)
        .bind(&new_resolved_at)
        .bind(&now)
        .bind(&input.lot_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // Fetch and return the created movement
    repo::get_movement(pool, &id)
        .await?
        .ok_or(AppError::from(sqlx::Error::RowNotFound))
}

/// Lists all movements for a lot, newest first.
pub async fn list_lot_movements(
    pool: &DbPool,
    lot_id: &str,
) -> Result<Vec<LotMovementResponse>, AppError> {
    repo::list_movements_by_lot(pool, lot_id)
        .await
        .map_err(AppError::from)
}

/// Returns per-location balances for a lot.
pub async fn get_lot_location_balances(
    pool: &DbPool,
    lot_id: &str,
) -> Result<Vec<LotLocationBalance>, AppError> {
    repo::location_balances_for_lot(pool, lot_id)
        .await
        .map_err(AppError::from)
}

/// Derives an auto-generated batch code for a product.
///
/// Format: PREFIX-YYYYMMDD-NNN
///
/// This is called when the user leaves batch_code blank during lot creation.
pub async fn derive_auto_batch_code(pool: &DbPool, product_id: &str) -> Result<String, AppError> {
    // Get product SKU
    let product = crate::db::repositories::products::get_product(pool, product_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| DomainError::NotFound {
            resource: "product",
            id: product_id.to_string(),
        })?;

    let prefix = derive_batch_prefix(Some(product.sku.as_str()));
    let date = Local::now().format("%Y%m%d").to_string();

    // Find max NNN for this prefix and date
    let max_nnn = repo::find_max_nnn_for_prefix_on_date(pool, &prefix, &date)
        .await
        .map_err(AppError::from)?
        .unwrap_or(0);

    let candidate = next_batch_candidate(&prefix, &date, max_nnn);

    // Check for collisions with manually entered batch codes
    // If the candidate already exists, increment until we find a free one
    let mut batch_code = candidate.clone();
    let mut attempts = 0;
    while attempts < 100 {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM expiry_lots WHERE batch_code = $1)")
                .bind(&batch_code)
                .fetch_one(pool)
                .await
                .map_err(AppError::from)?;

        if !exists {
            return Ok(batch_code);
        }

        // Increment NNN and try again
        let current_nnn = max_nnn + attempts + 1;
        batch_code = next_batch_candidate(&prefix, &date, current_nnn);
        attempts += 1;
    }

    // Fallback: use a UUID suffix if we hit 100 collisions
    Ok(format!(
        "{}-{}-{}-{}",
        prefix,
        date,
        999,
        uuid::Uuid::new_v4().to_string()[..6].to_uppercase()
    ))
}

/// Creates the sentinel location for a store if it doesn't exist.
/// Returns the sentinel location ID.
pub async fn ensure_sentinel_location(pool: &DbPool, store_id: &str) -> Result<String, AppError> {
    let sentinel_id = format!("loc-sentinel-{}", store_id);

    // Check if sentinel exists
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM store_locations WHERE id = $1)")
            .bind(&sentinel_id)
            .fetch_one(pool)
            .await
            .map_err(AppError::from)?;

    if exists {
        return Ok(sentinel_id);
    }

    // Create sentinel
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO store_locations (id, store_id, name, notes, is_active, created_at, updated_at)
        VALUES ($1, $2, 'Sin ubicacion', NULL, 1, $3, $4)
        "#,
    )
    .bind(&sentinel_id)
    .bind(store_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(sentinel_id)
}

/// Checks if the require_initial_location_on_lot_create setting is enabled.
/// Returns `true` if the setting is '1' or absent (default).
pub async fn is_initial_location_required(pool: &DbPool) -> Result<bool, AppError> {
    let value = settings_repo::get_setting(pool, "require_initial_location_on_lot_create")
        .await
        .map_err(AppError::from)?;

    Ok(value.as_deref() != Some("0"))
}

#[cfg(test)]
mod tests {
    // Service tests live in the integration test section of migrations.rs
    // since they require a database pool.

    use super::*;

    #[test]
    fn validate_kind_valid() {
        assert!(validate_kind("entry:initial").is_ok());
        assert!(validate_kind("transfer").is_ok());
        assert!(validate_kind("exit:sale").is_ok());
        assert!(validate_kind("inventory_adjustment").is_ok());
    }

    #[test]
    fn validate_kind_unknown() {
        assert!(validate_kind("unknown").is_err());
    }

    #[test]
    fn validate_direction_usage_inventory_adjustment_requires_direction() {
        let kind = MovementKind::InventoryAdjustment;
        assert!(validate_direction_usage(&kind, None).is_err());
        assert!(validate_direction_usage(&kind, Some(&DtoDirection::Increase)).is_ok());
        assert!(validate_direction_usage(&kind, Some(&DtoDirection::Decrease)).is_ok());
    }

    #[test]
    fn validate_direction_usage_other_kinds_must_not_have_direction() {
        let kind = MovementKind::ExitSale;
        assert!(validate_direction_usage(&kind, None).is_ok());
        assert!(validate_direction_usage(&kind, Some(&DtoDirection::Increase)).is_err());
    }

    #[test]
    fn validate_quantity_positive() {
        // entry:initial allows quantity = 0
        assert!(validate_quantity(0.0, &MovementKind::EntryInitial).is_ok());
        assert!(validate_quantity(1.0, &MovementKind::EntryInitial).is_ok());
        // other kinds require quantity > 0
        assert!(validate_quantity(1.0, &MovementKind::ExitSale).is_ok());
        assert!(validate_quantity(0.001, &MovementKind::ExitSale).is_ok());
        assert!(validate_quantity(0.0, &MovementKind::ExitSale).is_err());
        assert!(validate_quantity(-1.0, &MovementKind::ExitSale).is_err());
    }

    #[test]
    fn validate_location_nullability_entry_initial() {
        let kind = MovementKind::EntryInitial;
        // source=None, dest=Some is valid
        assert!(validate_location_nullability(&kind, None, Some("loc1"), None).is_ok());
        // source=Some is invalid
        assert!(validate_location_nullability(&kind, Some("loc1"), Some("loc2"), None).is_err());
        // dest=None is invalid
        assert!(validate_location_nullability(&kind, None, None, None).is_err());
    }

    #[test]
    fn validate_location_nullability_transfer() {
        let kind = MovementKind::Transfer;
        // source=Some, dest=Some (different) is valid
        assert!(validate_location_nullability(&kind, Some("loc1"), Some("loc2"), None).is_ok());
        // same source and dest is invalid
        assert!(validate_location_nullability(&kind, Some("loc1"), Some("loc1"), None).is_err());
        // missing source or dest is invalid
        assert!(validate_location_nullability(&kind, None, Some("loc2"), None).is_err());
        assert!(validate_location_nullability(&kind, Some("loc1"), None, None).is_err());
    }

    #[test]
    fn validate_location_nullability_inventory_adjustment() {
        // increase: source=None, dest=Some
        assert!(validate_location_nullability(
            &MovementKind::InventoryAdjustment,
            None,
            Some("loc1"),
            Some(&DtoDirection::Increase)
        )
        .is_ok());
        // increase: source=Some is invalid
        assert!(validate_location_nullability(
            &MovementKind::InventoryAdjustment,
            Some("loc1"),
            Some("loc2"),
            Some(&DtoDirection::Increase)
        )
        .is_err());

        // decrease: source=Some, dest=None
        assert!(validate_location_nullability(
            &MovementKind::InventoryAdjustment,
            Some("loc1"),
            None,
            Some(&DtoDirection::Decrease)
        )
        .is_ok());
        // decrease: dest=Some is invalid
        assert!(validate_location_nullability(
            &MovementKind::InventoryAdjustment,
            Some("loc1"),
            Some("loc2"),
            Some(&DtoDirection::Decrease)
        )
        .is_err());
    }

    #[test]
    fn validate_location_nullability_exit_sale() {
        let kind = MovementKind::ExitSale;
        // source=Some, dest=None is valid
        assert!(validate_location_nullability(&kind, Some("loc1"), None, None).is_ok());
        // dest=Some is invalid
        assert!(validate_location_nullability(&kind, Some("loc1"), Some("loc2"), None).is_err());
        // source=None is invalid
        assert!(validate_location_nullability(&kind, None, None, None).is_err());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::db::migrations::fresh_test_pool;

    /// Helper to create a minimal test fixture: store, product, location, and lot.
    /// Uses fresh_test_pool() (all migrations) but creates lot AFTER migrations
    /// so the V5 backfill doesn't interfere with test data.
    async fn create_test_lot(pool: &DbPool) -> (String, String, String, String) {
        let now = chrono::Utc::now().to_rfc3339();

        // Create store
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at) \
                     VALUES ('s1', 'Test Store', 1, $1, $2)",
        )
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert store");

        // Create product
        sqlx::query(
                "INSERT INTO products (id, sku, description, default_alert_days_before, is_active, created_at, updated_at) \
                     VALUES ('p1', 'SKU001', 'Test Product', 30, 1, $1, $2)",
            )
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .expect("insert product");

        // Create location
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at) \
                     VALUES ('loc1', 's1', 'Bodega', 1, $1, $2)",
        )
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert location");

        // Create lot - this is created AFTER migrations, so no V5 backfill applies to it
        let lot_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, location_id, quantity, unit, \
                     expiry_date, alert_days_before, status, created_at, updated_at) \
                     VALUES ($1, 'p1', 's1', 'loc1', 10.0, 'L', '2025-12-31', 30, 'active', $2, $3)",
            )
            .bind(&lot_id)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .expect("insert lot");

        (
            lot_id,
            "loc1".to_string(),
            "s1".to_string(),
            "p1".to_string(),
        )
    }

    /// Helper to create a second location in the same store.
    async fn create_second_location(pool: &DbPool, store_id: &str) -> String {
        let now = chrono::Utc::now().to_rfc3339();
        let loc_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at) \
                     VALUES ($1, $2, 'Exhibicion', 1, $3, $4)",
        )
        .bind(&loc_id)
        .bind(store_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert second location");
        loc_id
    }

    #[tokio::test]
    async fn create_lot_movement_emits_initial_entry_on_lot_creation() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with quantity=0 since lot already has quantity
        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 0.0, // lot already starts with 10.0, entry records history
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await;

        assert!(result.is_ok());
        let movement = result.unwrap();
        assert_eq!(movement.movement_kind, "entry:initial");
        assert_eq!(movement.quantity, 0.0);
        assert_eq!(movement.destination_location_id, Some(location_id));
    }

    #[tokio::test]
    async fn create_lot_movement_transfer_preserves_lot_total() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, src_location, store_id, _) = create_test_lot(&pool).await;
        let dst_location = create_second_location(&pool, &store_id).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(src_location.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Record transfer of 3 units
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "transfer".to_string(),
                direction: None,
                quantity: 3.0,
                source_location_id: Some(src_location.clone()),
                destination_location_id: Some(dst_location.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Check lot total unchanged at 10.0 (transfer preserves total)
        let lot = crate::db::repositories::expiry_lots::get_expiry_lot(&pool, &lot_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lot.quantity, 10.0);

        // Check per-location balances: src=7, dst=3
        let balances = get_lot_location_balances(&pool, &lot_id).await.unwrap();
        assert_eq!(balances.len(), 2);
        let src_bal = balances
            .iter()
            .find(|b| b.location_id == src_location)
            .unwrap();
        let dst_bal = balances
            .iter()
            .find(|b| b.location_id == dst_location)
            .unwrap();
        assert_eq!(src_bal.balance, 7.0);
        assert_eq!(dst_bal.balance, 3.0);
    }

    #[tokio::test]
    async fn create_lot_movement_exit_reduces_lot_total() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Record exit sale for 4 units
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "exit:sale".to_string(),
                direction: None,
                quantity: 4.0,
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: Some("Customer purchase".to_string()),
            },
        )
        .await
        .unwrap();

        // Check lot total reduced to 6.0
        let lot = crate::db::repositories::expiry_lots::get_expiry_lot(&pool, &lot_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lot.quantity, 6.0);
    }

    #[tokio::test]
    async fn create_lot_movement_exit_rejected_when_notes_blank_for_other() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Record exit:other without notes should fail
        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "exit:other".to_string(),
                direction: None,
                quantity: 2.0,
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: None,
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_lot_movement_inventory_adjustment_increase() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Record inventory adjustment (increase) for 3 units
        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "inventory_adjustment".to_string(),
                direction: Some(DtoDirection::Increase),
                quantity: 3.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: Some("Found extra units during count".to_string()),
            },
        )
        .await;

        assert!(result.is_ok());
        let movement = result.unwrap();
        assert_eq!(movement.movement_kind, "inventory_adjustment");
        assert_eq!(movement.direction, Some("increase".to_string()));

        // Check lot total increased to 13.0
        let lot = crate::db::repositories::expiry_lots::get_expiry_lot(&pool, &lot_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lot.quantity, 13.0);
    }

    #[tokio::test]
    async fn create_lot_movement_inventory_adjustment_decrease() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Record inventory adjustment (decrease) for 2 units
        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "inventory_adjustment".to_string(),
                direction: Some(DtoDirection::Decrease),
                quantity: 2.0,
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: Some("Damaged goods found".to_string()),
            },
        )
        .await;

        assert!(result.is_ok());
        let movement = result.unwrap();
        assert_eq!(movement.movement_kind, "inventory_adjustment");
        assert_eq!(movement.direction, Some("decrease".to_string()));

        // Check lot total decreased to 8.0
        let lot = crate::db::repositories::expiry_lots::get_expiry_lot(&pool, &lot_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lot.quantity, 8.0);
    }

    #[tokio::test]
    async fn create_lot_movement_inventory_adjustment_requires_notes() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Inventory adjustment without notes should fail
        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "inventory_adjustment".to_string(),
                direction: Some(DtoDirection::Increase),
                quantity: 2.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_lot_movement_inventory_adjustment_requires_direction() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Inventory adjustment without direction should fail
        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "inventory_adjustment".to_string(),
                direction: None,
                quantity: 2.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: Some("Note".to_string()),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_movements_by_lot_orders_newest_first() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Wait a bit to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Record exit sale for 3 units
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "exit:sale".to_string(),
                direction: None,
                quantity: 3.0,
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: Some("Sale".to_string()),
            },
        )
        .await
        .unwrap();

        // List movements
        let movements = list_lot_movements(&pool, &lot_id).await.unwrap();
        assert_eq!(movements.len(), 2);
        // First (newest) should be the exit sale
        assert_eq!(movements[0].movement_kind, "exit:sale");
        // Second (oldest) should be the initial entry
        assert_eq!(movements[1].movement_kind, "entry:initial");
    }

    #[tokio::test]
    async fn location_balances_for_lot_matches_ledger_sum() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, src_location, store_id, _) = create_test_lot(&pool).await;
        let dst_location = create_second_location(&pool, &store_id).await;

        // Emit initial entry at src with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(src_location.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Transfer 4 units to dst
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "transfer".to_string(),
                direction: None,
                quantity: 4.0,
                source_location_id: Some(src_location.clone()),
                destination_location_id: Some(dst_location.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Get balances
        let balances = get_lot_location_balances(&pool, &lot_id).await.unwrap();
        assert_eq!(balances.len(), 2);

        // Find src and dst balances
        let src_balance = balances
            .iter()
            .find(|b| b.location_id == src_location)
            .unwrap();
        let dst_balance = balances
            .iter()
            .find(|b| b.location_id == dst_location)
            .unwrap();
        assert_eq!(src_balance.balance, 6.0); // 10 - 4
        assert_eq!(dst_balance.balance, 4.0); // 0 + 4
    }

    #[tokio::test]
    async fn create_lot_movement_exit_resolves_lot_at_zero() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Exit sale for 10 units (all stock)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "exit:sale".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: Some("Full lot sold".to_string()),
            },
        )
        .await
        .unwrap();

        // Check lot is resolved with quantity 0
        let lot = crate::db::repositories::expiry_lots::get_expiry_lot(&pool, &lot_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lot.quantity, 0.0);
        assert_eq!(lot.status, "resolved");
        assert_eq!(lot.resolution, Some("exit:sale".to_string()));
    }

    #[tokio::test]
    async fn create_lot_movement_reactivates_resolved_lot() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Exit sale for 10 units (all stock) - lot becomes resolved
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "exit:sale".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: Some("Full lot sold".to_string()),
            },
        )
        .await
        .unwrap();

        // Verify lot is resolved
        let lot = crate::db::repositories::expiry_lots::get_expiry_lot(&pool, &lot_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lot.status, "resolved");

        // Inventory adjustment increase to reactivate with 5 units
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "inventory_adjustment".to_string(),
                direction: Some(DtoDirection::Increase),
                quantity: 5.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: Some("Stock found during recount".to_string()),
            },
        )
        .await
        .unwrap();

        // Check lot is reactivated with quantity 5.0
        let lot = crate::db::repositories::expiry_lots::get_expiry_lot(&pool, &lot_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lot.quantity, 5.0);
        assert_eq!(lot.status, "active");
        assert!(lot.resolution.is_none());
    }

    #[tokio::test]
    async fn create_lot_movement_rejected_for_insufficient_balance() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with the lot's quantity (10.0)
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0,
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        // Try to exit 15 units (more than available 10.0)
        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "exit:sale".to_string(),
                direction: None,
                quantity: 15.0,
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: Some("Too many".to_string()),
            },
        )
        .await;

        assert!(result.is_err());
    }
}
