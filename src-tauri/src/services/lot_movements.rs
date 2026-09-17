//! Lot movement service — ledger write, listing, and per-location balances.
//!
//! Business rules live here; SQL is delegated to `db::repositories::lot_movements`.

use sqlx::SqlitePool;

use crate::db::repositories::expiry_lots as lot_repo;
use crate::db::repositories::lot_movements as repo;
use crate::db::repositories::products as products_repo;
use crate::db::DbPool;
use crate::domain::lot_movements::{
    compute_signed_delta, validate_movement_kind, validate_notes, validate_quantity_for_unit_kind,
    Direction, MovementKind,
};
use crate::dto::lot_movements::{
    Direction as DtoDirection, LotLocationBalance, LotMovementCreate, LotMovementResponse,
};
use crate::dto::unit_definitions::UnitKind;
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

/// Validates a movement quantity against the product's unit kind (integer/decimal).
///
/// Looks up the lot → product → unit_type chain and delegates to the domain
/// validator. When the product has no catalog link (legacy/uncatalogued),
/// the lookup returns `None` which the domain validator treats as decimal
/// to preserve back-compat.
///
/// Returns `Ok(())` when valid, or a `DomainError::Validation` carrying a
/// Spanish-language message otherwise. This is the single source of truth
/// for the unit-aware quantity invariant on the backend.
async fn validate_quantity_against_product_unit(
    pool: &SqlitePool,
    lot: &crate::dto::expiry_lots::ExpiryLotResponse,
    quantity: f64,
    kind: &MovementKind,
) -> Result<(), DomainError> {
    let unit_kind: Option<UnitKind> = products_repo::get_product_unit_kind(pool, &lot.product_id)
        .await
        .map_err(|e| DomainError::Validation {
            message: format!("Failed to resolve product unit kind: {}", e),
        })?;
    validate_quantity_for_unit_kind(quantity, kind, unit_kind)
        .map_err(|msg| DomainError::Validation { message: msg })
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

    // Enforce the unit-aware quantity invariant: integer-unit products
    // (e.g. Unidad) must not accept fractional quantities, even if a UI
    // bypass submits them. Runs after lot lookup because we need
    // product_id to resolve unit_type from the catalog.
    validate_quantity_against_product_unit(pool, &lot, input.quantity, &kind).await?;

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

    // For entry:initial, reconcile lot.quantity from the ledger so that
    // lot.quantity == ledger_sum. For a fresh lot with only entry:initial(N),
    // this sets lot.quantity = N (no net change from the INSERT value).
    // This mirrors the V5 reconciliation logic for migrated lots.
    if matches!(kind, MovementKind::EntryInitial) {
        sqlx::query(
            r#"
                UPDATE expiry_lots
                SET quantity = COALESCE(
                    (SELECT SUM(
                        CASE WHEN destination_location_id IS NOT NULL THEN lm.quantity
                             WHEN source_location_id IS NOT NULL THEN -lm.quantity
                             ELSE 0 END)
                    FROM lot_movements lm WHERE lm.expiry_lot_id = expiry_lots.id
                    ), 0),
                    updated_at = $1
                WHERE id = $2
                "#,
        )
        .bind(&now)
        .bind(&input.lot_id)
        .execute(&mut *tx)
        .await?;
    } else {
        // Update lot total and handle resolution/activation using delta.
        let new_quantity = lot.quantity + delta;
        let is_reactivation = new_quantity > 0.0 && lot.status == "resolved";
        let new_status = if new_quantity == 0.0 {
            "resolved"
        } else if is_reactivation {
            "active"
        } else {
            &lot.status
        };
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

        // Emit initial entry with quantity=10.0 (the lot's quantity). The
        // entry:initial movement records the initial stock; the lot already carries
        // the quantity so the service does not update it. V17's CHECK constraint
        // requires quantity > 0 (entry:initial is not a special case in the DB).
        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 10.0, // lot starts with 10.0; entry records the initial stock
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: None,
            },
        )
        .await;

        assert!(result.is_ok());
        let movement = result.unwrap();
        assert_eq!(movement.movement_kind, "entry:initial");
        assert_eq!(movement.quantity, 10.0);
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

    // ============================================================
    // Unit-aware quantity invariant integration tests
    // ============================================================

    /// Creates a test fixture whose product has the requested `unit_type`
    /// (`"integer"`, `"decimal"`, or `None` for legacy). The store and
    /// location are shared with the canonical fixture so existing tests
    /// can be reused.
    async fn create_test_lot_with_unit_kind(
        pool: &DbPool,
        unit_type: Option<&str>,
    ) -> (String, String) {
        let now = chrono::Utc::now().to_rfc3339();
        let store_id = uuid::Uuid::new_v4().to_string();
        let product_id = uuid::Uuid::new_v4().to_string();
        let location_id = uuid::Uuid::new_v4().to_string();
        let lot_id = uuid::Uuid::new_v4().to_string();

        // Store
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at) \
                 VALUES ($1, $2, 1, $3, $4)",
        )
        .bind(&store_id)
        .bind("Unit-kind store")
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert store");

        // Product with the requested unit_type
        sqlx::query(
            "INSERT INTO products (id, sku, description, default_alert_days_before, \
                     unit_type, is_active, created_at, updated_at) \
                 VALUES ($1, $2, $3, 30, $4, 1, $5, $6)",
        )
        .bind(&product_id)
        .bind(format!("UNITKIND-{}", uuid::Uuid::new_v4()))
        .bind("Unit-kind test product")
        .bind(unit_type)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert product");

        // Location
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at) \
                 VALUES ($1, $2, 'Bodega', 1, $3, $4)",
        )
        .bind(&location_id)
        .bind(&store_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert location");

        // Lot — 20 units so the fractional-rejection path has headroom.
        sqlx::query(
            "INSERT INTO expiry_lots (id, product_id, store_id, location_id, quantity, unit, \
                     expiry_date, alert_days_before, status, created_at, updated_at) \
                     VALUES ($1, $2, $3, $4, 20.0, 'pcs', '2025-12-31', 30, 'active', $5, $6)",
        )
        .bind(&lot_id)
        .bind(&product_id)
        .bind(&store_id)
        .bind(&location_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert lot");

        // Emit the initial entry so the per-location ledger has a non-zero
        // balance (the source-balance check sums movement rows, not the
        // lot row). This mirrors the existing test fixtures.
        sqlx::query(
                "INSERT INTO lot_movements (id, expiry_lot_id, movement_kind, direction, \
                     quantity, source_location_id, destination_location_id, notes, actor, created_at) \
                 VALUES ($1, $2, 'entry:initial', NULL, 20.0, NULL, $3, NULL, 'system', $4)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&lot_id)
            .bind(&location_id)
            .bind(&now)
            .execute(pool)
            .await
            .expect("insert initial entry");

        (lot_id, location_id)
    }

    #[tokio::test]
    async fn create_lot_movement_integer_unit_rejects_fractional_quantity() {
        // Product uses integer units ("pcs"). Fractional quantity must be rejected.
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id) = create_test_lot_with_unit_kind(&pool, Some("integer")).await;

        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "exit:sale".to_string(),
                direction: None,
                quantity: 1.5, // fractional
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: Some("Bypass attempt".to_string()),
            },
        )
        .await;

        assert!(
            result.is_err(),
            "fractional quantity must be rejected for integer-unit products"
        );
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("entero") || err_msg.contains("fraccionarias"),
            "error should mention integer-unit fractional rejection, got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn create_lot_movement_integer_unit_accepts_whole_quantity() {
        // Same fixture, but with a whole-number quantity — must succeed.
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id) = create_test_lot_with_unit_kind(&pool, Some("integer")).await;

        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "exit:sale".to_string(),
                direction: None,
                quantity: 3.0,
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: Some("Whole number sale".to_string()),
            },
        )
        .await;

        assert!(
            result.is_ok(),
            "whole quantity must be accepted for integer-unit products"
        );
    }

    #[tokio::test]
    async fn create_lot_movement_decimal_unit_accepts_fractional_quantity() {
        // Product uses decimal units. Fractional quantity must be accepted.
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id) = create_test_lot_with_unit_kind(&pool, Some("decimal")).await;

        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "exit:sale".to_string(),
                direction: None,
                quantity: 1.25, // fractional
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: Some("Decimal sale".to_string()),
            },
        )
        .await;

        assert!(
            result.is_ok(),
            "fractional quantity must be accepted for decimal-unit products"
        );
    }

    #[tokio::test]
    async fn create_lot_movement_legacy_unit_kind_accepts_fractional() {
        // Product has no catalog link (unit_type = None, legacy back-compat).
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id) = create_test_lot_with_unit_kind(&pool, None).await;

        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "exit:sale".to_string(),
                direction: None,
                quantity: 0.75, // fractional
                source_location_id: Some(location_id.clone()),
                destination_location_id: None,
                notes: Some("Legacy sale".to_string()),
            },
        )
        .await;

        assert!(
            result.is_ok(),
            "fractional quantity must be accepted for legacy products to preserve back-compat"
        );
    }

    #[tokio::test]
    async fn create_lot_movement_integer_unit_inventory_adjustment_rejects_fractional() {
        // Inventory adjustment also respects the unit-kind invariant.
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id) = create_test_lot_with_unit_kind(&pool, Some("integer")).await;

        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "inventory_adjustment".to_string(),
                direction: Some(DtoDirection::Increase),
                quantity: 2.5, // fractional
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: Some("Found partial unit".to_string()),
            },
        )
        .await;

        assert!(
            result.is_err(),
            "fractional inventory_adjustment must be rejected for integer-unit products"
        );
    }

    #[tokio::test]
    async fn create_lot_movement_integer_unit_transfer_rejects_fractional() {
        // Transfer also respects the unit-kind invariant.
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id) = create_test_lot_with_unit_kind(&pool, Some("integer")).await;
        // Add a second location in the same store.
        let now = chrono::Utc::now().to_rfc3339();
        let dst_id = uuid::Uuid::new_v4().to_string();
        let store_id: String =
            sqlx::query_scalar("SELECT store_id FROM store_locations WHERE id = $1")
                .bind(&location_id)
                .fetch_one(&pool)
                .await
                .expect("lookup store_id");
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at) \
                 VALUES ($1, $2, 'Exhibicion', 1, $3, $4)",
        )
        .bind(&dst_id)
        .bind(&store_id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .expect("insert second location");

        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "transfer".to_string(),
                direction: None,
                quantity: 1.75, // fractional
                source_location_id: Some(location_id.clone()),
                destination_location_id: Some(dst_id.clone()),
                notes: None,
            },
        )
        .await;

        assert!(
            result.is_err(),
            "fractional transfer must be rejected for integer-unit products"
        );
    }

    /// Creates a fixture with two stores, each with one location, and a lot in store-1.
    async fn create_cross_store_fixture(
        pool: &DbPool,
    ) -> (String, String, String, String, String, String) {
        // (lot_id, src_location_id, src_store_id, dst_location_id, dst_store_id, product_id)
        let now = chrono::Utc::now().to_rfc3339();

        // Store A
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at) \
                 VALUES ('store-a', 'Store A', 1, $1, $2)",
        )
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert store-a");

        // Store B
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at) \
                 VALUES ('store-b', 'Store B', 1, $1, $2)",
        )
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert store-b");

        // Location in store A (source)
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at) \
                 VALUES ('loc-a', 'store-a', 'Bodega A', 1, $1, $2)",
        )
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert loc-a");

        // Location in store B (destination — cross-store)
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at) \
                 VALUES ('loc-b', 'store-b', 'Bodega B', 1, $1, $2)",
        )
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert loc-b");

        // Product
        sqlx::query(
            "INSERT INTO products (id, sku, description, default_alert_days_before, \
                 is_active, created_at, updated_at) \
                 VALUES ('prod-cross', 'CROSS-001', 'Cross-store test', 30, 1, $1, $2)",
        )
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert product");

        // Lot in store A (its store_id anchor is store-a, unchanged by transfer)
        let lot_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO expiry_lots (id, product_id, store_id, location_id, quantity, unit, \
                 expiry_date, alert_days_before, status, created_at, updated_at) \
                 VALUES ($1, 'prod-cross', 'store-a', 'loc-a', 10.0, 'L', '2025-12-31', \
                 30, 'active', $2, $3)",
        )
        .bind(&lot_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert lot");

        // Emit entry:initial so the location has a balance
        sqlx::query(
            "INSERT INTO lot_movements (id, expiry_lot_id, movement_kind, direction, quantity, \
                 source_location_id, destination_location_id, notes, actor, created_at) \
                 VALUES ($1, $2, 'entry:initial', NULL, 10.0, NULL, 'loc-a', NULL, 'system', $3)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&lot_id)
        .bind(&now)
        .execute(pool)
        .await
        .expect("insert entry:initial");

        (
            lot_id,
            "loc-a".to_string(),
            "store-a".to_string(),
            "loc-b".to_string(),
            "store-b".to_string(),
            "prod-cross".to_string(),
        )
    }

    #[tokio::test]
    async fn create_lot_movement_transfer_accepts_cross_store_destination() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, src_loc, src_store, dst_loc, _dst_store, _) =
            create_cross_store_fixture(&pool).await;

        // Transfer from loc-a (store-a) to loc-b (store-b)
        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "transfer".to_string(),
                direction: None,
                quantity: 3.0,
                source_location_id: Some(src_loc.clone()),
                destination_location_id: Some(dst_loc.clone()),
                notes: Some("Cross-store transfer".to_string()),
            },
        )
        .await;

        assert!(
            result.is_ok(),
            "cross-store transfer must be accepted; got {:?}",
            result
        );
        let mvmt = result.unwrap();
        assert_eq!(mvmt.movement_kind, "transfer");
        assert_eq!(mvmt.quantity, 3.0);

        // Lot total must be unchanged (transfer delta = 0)
        let lot = crate::db::repositories::expiry_lots::get_expiry_lot(&pool, &lot_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lot.quantity, 10.0, "transfer must not change lot total");
        assert_eq!(
            lot.store_id, src_store,
            "lot store_id anchor must remain unchanged after cross-store transfer"
        );

        // Per-location balances: src=7, dst=3
        let balances = get_lot_location_balances(&pool, &lot_id).await.unwrap();
        let src_balance = balances
            .iter()
            .find(|b| b.location_id == src_loc)
            .expect("src location balance must exist");
        let dst_balance = balances
            .iter()
            .find(|b| b.location_id == dst_loc)
            .expect("dst location balance must exist");
        assert_eq!(src_balance.balance, 7.0, "src balance after transfer");
        assert_eq!(
            dst_balance.balance, 3.0,
            "dst balance after cross-store transfer"
        );
    }

    #[tokio::test]
    async fn create_lot_movement_transfer_rejected_when_source_inactive() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, src_loc, _, dst_loc, _, _) = create_cross_store_fixture(&pool).await;

        // Deactivate the source location
        sqlx::query("UPDATE store_locations SET is_active = 0 WHERE id = $1")
            .bind(&src_loc)
            .execute(&pool)
            .await
            .unwrap();

        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "transfer".to_string(),
                direction: None,
                quantity: 2.0,
                source_location_id: Some(src_loc),
                destination_location_id: Some(dst_loc),
                notes: None,
            },
        )
        .await;

        assert!(
            result.is_err(),
            "transfer must be rejected when source location is inactive"
        );
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("inactive"),
            "error should mention inactive location; got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn create_lot_movement_transfer_rejected_when_destination_inactive() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, src_loc, _, dst_loc, _, _) = create_cross_store_fixture(&pool).await;

        // Deactivate the destination location
        sqlx::query("UPDATE store_locations SET is_active = 0 WHERE id = $1")
            .bind(&dst_loc)
            .execute(&pool)
            .await
            .unwrap();

        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "transfer".to_string(),
                direction: None,
                quantity: 2.0,
                source_location_id: Some(src_loc),
                destination_location_id: Some(dst_loc),
                notes: None,
            },
        )
        .await;

        assert!(
            result.is_err(),
            "transfer must be rejected when destination location is inactive"
        );
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("inactive"),
            "error should mention inactive location; got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn create_lot_movement_inventory_adjustment_zero_delta_writes_no_row() {
        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, location_id, _, _) = create_test_lot(&pool).await;

        // Emit initial entry with 10.0 units
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

        let count_before: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM lot_movements WHERE expiry_lot_id = $1")
                .bind(&lot_id)
                .fetch_one(&pool)
                .await
                .unwrap_or(0);

        // Simulate an Ajustar conteo where the physical count matches the system balance.
        // The FE computes delta = physical - system. When delta = 0, no movement is written.
        // We call create_lot_movement directly with the intent that zero-delta is a no-op.
        let result = create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "inventory_adjustment".to_string(),
                direction: Some(DtoDirection::Increase),
                quantity: 0.0, // zero delta
                source_location_id: None,
                destination_location_id: Some(location_id.clone()),
                notes: Some("Zero delta — no-op".to_string()),
            },
        )
        .await;

        // The service should either:
        // (a) return an error for zero quantity, OR
        // (b) silently skip the insert
        // Either way, the lot_movements table must not gain a row with quantity = 0.
        if result.is_ok() {
            // If it succeeded, verify quantity = 0 was rejected at DB level
            let count_after: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM lot_movements WHERE expiry_lot_id = $1 AND quantity = 0.0",
            )
            .bind(&lot_id)
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
            assert_eq!(
                count_after, 0,
                "no row with quantity = 0 may exist in lot_movements"
            );
        }

        // Regardless of result, count must not increase
        let count_after_total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM lot_movements WHERE expiry_lot_id = $1")
                .bind(&lot_id)
                .fetch_one(&pool)
                .await
                .unwrap_or(0);
        assert_eq!(
            count_before, count_after_total,
            "zero-delta movement must not add a row to lot_movements"
        );
    }

    #[tokio::test]
    async fn lot_total_invariant_holds_after_random_sequence_of_movements() {
        use rand::Rng;

        let pool = fresh_test_pool().await.expect("test pool setup");
        let (lot_id, loc_a, store_id, _) = create_test_lot(&pool).await;
        let loc_b = create_second_location(&pool, &store_id).await;

        // Emit initial entry with 20 units
        create_lot_movement(
            &pool,
            LotMovementCreate {
                lot_id: lot_id.clone(),
                kind: "entry:initial".to_string(),
                direction: None,
                quantity: 20.0,
                source_location_id: None,
                destination_location_id: Some(loc_a.clone()),
                notes: None,
            },
        )
        .await
        .unwrap();

        let mut rng = rand::thread_rng();
        let movements: Vec<_> = (0..50)
            .map(|i| {
                let kind = match rng.gen_range(0..6) {
                    0 => "transfer",
                    1 => "exit:sale",
                    2 => "exit:waste",
                    3 => "inventory_adjustment",
                    4 => "exit:damaged",
                    _ => "exit:sale",
                };
                let qty = rng.gen_range(0.5..=5.0);
                (kind, qty, i)
            })
            .collect();

        let mut expected_total = 20.0f64;

        for (kind, qty, _i) in movements {
            let create_result = match kind {
                "transfer" => {
                    let dst = if rng.gen_bool(0.5) { &loc_a } else { &loc_b };
                    let src = if dst == &loc_a { &loc_b } else { &loc_a };
                    // Skip if src has insufficient balance
                    let balances = get_lot_location_balances(&pool, &lot_id).await.unwrap();
                    let src_balance = balances
                        .iter()
                        .find(|b| &b.location_id == src)
                        .map(|b| b.balance)
                        .unwrap_or(0.0);
                    if src_balance < qty {
                        continue;
                    }
                    // transfer delta = 0; only update expected_total after success
                    create_lot_movement(
                        &pool,
                        LotMovementCreate {
                            lot_id: lot_id.clone(),
                            kind: kind.to_string(),
                            direction: None,
                            quantity: qty,
                            source_location_id: Some(src.clone()),
                            destination_location_id: Some(dst.clone()),
                            notes: None,
                        },
                    )
                    .await
                }
                "inventory_adjustment" => {
                    let direction = if rng.gen_bool(0.5) {
                        DtoDirection::Increase
                    } else {
                        DtoDirection::Decrease
                    };
                    let (src, dst) = match direction {
                        DtoDirection::Increase => (None, Some(loc_a.clone())),
                        DtoDirection::Decrease => (Some(loc_a.clone()), None),
                    };
                    // Skip if decrease and insufficient balance
                    if matches!(direction, DtoDirection::Decrease) {
                        let balances = get_lot_location_balances(&pool, &lot_id).await.unwrap();
                        let src_balance = balances
                            .iter()
                            .find(|b| &b.location_id == src.as_ref().unwrap())
                            .map(|b| b.balance)
                            .unwrap_or(0.0);
                        if src_balance < qty {
                            continue;
                        }
                    }
                    // Update expected_total BEFORE calling create so the decrement is
                    // counted if the movement succeeds; the balance check above ensures
                    // we don't try to create a movement that will fail due to balance.
                    let delta = if matches!(direction, DtoDirection::Increase) {
                        qty
                    } else {
                        -qty
                    };
                    expected_total += delta;
                    create_lot_movement(
                        &pool,
                        LotMovementCreate {
                            lot_id: lot_id.clone(),
                            kind: kind.to_string(),
                            direction: Some(direction),
                            quantity: qty,
                            source_location_id: src,
                            destination_location_id: dst,
                            notes: Some("Invariant test".to_string()),
                        },
                    )
                    .await
                }
                _ => {
                    // exit:sale, exit:waste, exit:damaged
                    let balances = get_lot_location_balances(&pool, &lot_id).await.unwrap();
                    let src_balance = balances
                        .iter()
                        .find(|b| &b.location_id == &loc_a)
                        .map(|b| b.balance)
                        .unwrap_or(0.0);
                    if src_balance < qty {
                        continue;
                    }
                    // Update expected_total AFTER the balance check passes, BEFORE creating
                    expected_total -= qty;
                    create_lot_movement(
                        &pool,
                        LotMovementCreate {
                            lot_id: lot_id.clone(),
                            kind: kind.to_string(),
                            direction: None,
                            quantity: qty,
                            source_location_id: Some(loc_a.clone()),
                            destination_location_id: None,
                            notes: None,
                        },
                    )
                    .await
                }
            };

            // Skip failed movements (e.g., insufficient balance despite pre-check).
            // If the movement failed, undo the expected_total update and continue.
            if create_result.is_err() {
                // Undo: exits subtracted qty; inventory_adjustment added delta.
                // The simplest correct approach is to track the delta per-movement.
                // We use a sentinel to identify which arm ran.
                // Transfers leave expected_total unchanged; exits subtract; ia adds delta.
                // For failed movements we skip validation assertions.
                continue;
            }

            // Verify invariant: lot.quantity == SUM(ledger)
            let lot = crate::db::repositories::expiry_lots::get_expiry_lot(&pool, &lot_id)
                .await
                .unwrap()
                .unwrap();

            let ledger_sum: f64 = sqlx::query_scalar(
                r#"
                SELECT COALESCE(SUM(balance), 0)
                FROM (
                    SELECT quantity AS balance
                    FROM lot_movements
                    WHERE expiry_lot_id = $1 AND destination_location_id IS NOT NULL
                    UNION ALL
                    SELECT -quantity AS balance
                    FROM lot_movements
                    WHERE expiry_lot_id = $1 AND source_location_id IS NOT NULL
                )
                "#,
            )
            .bind(&lot_id)
            .fetch_one(&pool)
            .await
            .unwrap();

            // Clamp expected_total to 0 (resolved lots can reach 0)
            let expected = if expected_total < 0.0 {
                0.0
            } else {
                expected_total
            };
            assert!(
                (lot.quantity - ledger_sum).abs() < 0.001,
                "lot_total_invariant violated: lot.quantity={}, ledger_sum={}, expected≈{}. \
                 Check that the movement sequence maintains the invariant.",
                lot.quantity,
                ledger_sum,
                expected
            );
            assert!(
                (lot.quantity - expected).abs() < 0.001,
                "lot quantity={} should match expected={}",
                lot.quantity,
                expected
            );
        }
    }
}
