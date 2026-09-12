//! Repository for expiry lot persistence: expiry_lots and lot_resolution_events.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::dto::expiry_lots::{
    ExpiryLotCreate, ExpiryLotResolve, ExpiryLotResponse, ExpiryLotUpdate,
    LotResolutionEventResponse,
};

// ============================================================
// Expiry lots — create / update / archive
// ============================================================

/// Inserts a new active expiry lot and returns the inserted row.
pub async fn insert_expiry_lot(
    pool: &SqlitePool,
    input: &ExpiryLotCreate,
    unit: &str,
    alert_days_before: i32,
) -> Result<ExpiryLotResponse, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO expiry_lots (
            id, product_id, store_id, location_id, quantity, unit,
            expiry_date, alert_days_before, batch_code, notes,
            status, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'active', $11, $12)
        "#,
    )
    .bind(&id)
    .bind(&input.product_id)
    .bind(&input.store_id)
    .bind(&input.location_id)
    .bind(input.quantity)
    .bind(unit)
    .bind(&input.expiry_date)
    .bind(alert_days_before)
    .bind(&input.batch_code)
    .bind(&input.notes)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_expiry_lot(pool, &id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

/// Fetches a single expiry lot by id, or `None` if it does not exist.
pub async fn get_expiry_lot(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<ExpiryLotResponse>, sqlx::Error> {
    sqlx::query_as::<_, ExpiryLotResponse>(
        r#"
        SELECT id, product_id, store_id, location_id, quantity, unit,
               expiry_date, alert_days_before, batch_code,
               status, resolution, resolved_at, notes,
               created_at, updated_at
        FROM expiry_lots
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Updates an existing expiry lot and returns the refreshed row, or `None`
/// if the lot did not exist.
pub async fn update_expiry_lot(
    pool: &SqlitePool,
    input: &ExpiryLotUpdate,
) -> Result<Option<ExpiryLotResponse>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    let affected = sqlx::query(
        r#"
        UPDATE expiry_lots
        SET location_id = $1, quantity = $2, unit = $3,
            expiry_date = $4, alert_days_before = $5,
            batch_code = $6, notes = $7, updated_at = $8
        WHERE id = $9 AND status = 'active'
        "#,
    )
    .bind(&input.location_id)
    .bind(input.quantity)
    .bind(&input.unit)
    .bind(&input.expiry_date)
    .bind(input.alert_days_before)
    .bind(&input.batch_code)
    .bind(&input.notes)
    .bind(&now)
    .bind(&input.id)
    .execute(pool)
    .await?;

    if affected.rows_affected() == 0 {
        return Ok(None);
    }
    get_expiry_lot(pool, &input.id).await
}

/// Soft-archives an expiry lot by setting status = 'archived'. Returns `true`
/// if a row was updated, `false` if the lot did not exist or was already
/// archived.
pub async fn archive_expiry_lot(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let affected = sqlx::query(
        r#"
        UPDATE expiry_lots
        SET status = 'archived', updated_at = $1
        WHERE id = $2 AND status = 'active'
        "#,
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(affected.rows_affected() > 0)
}

// ============================================================
// Expiry lots — list queries
// ============================================================

/// Returns all active expiry lots for a given product.
pub async fn list_expiry_lots_by_product(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<Vec<ExpiryLotResponse>, sqlx::Error> {
    sqlx::query_as::<_, ExpiryLotResponse>(
        r#"
        SELECT id, product_id, store_id, location_id, quantity, unit,
               expiry_date, alert_days_before, batch_code,
               status, resolution, resolved_at, notes,
               created_at, updated_at
        FROM expiry_lots
        WHERE product_id = $1 AND status = 'active'
        ORDER BY expiry_date ASC
        "#,
    )
    .bind(product_id)
    .fetch_all(pool)
    .await
}

/// Returns all active expiry lots for a given store.
pub async fn list_expiry_lots_by_store(
    pool: &SqlitePool,
    store_id: &str,
) -> Result<Vec<ExpiryLotResponse>, sqlx::Error> {
    sqlx::query_as::<_, ExpiryLotResponse>(
        r#"
        SELECT id, product_id, store_id, location_id, quantity, unit,
               expiry_date, alert_days_before, batch_code,
               status, resolution, resolved_at, notes,
               created_at, updated_at
        FROM expiry_lots
        WHERE store_id = $1 AND status = 'active'
        ORDER BY expiry_date ASC
        "#,
    )
    .bind(store_id)
    .fetch_all(pool)
    .await
}

/// Returns all active expiry lots (no filter).
pub async fn list_all_active_expiry_lots(
    pool: &SqlitePool,
) -> Result<Vec<ExpiryLotResponse>, sqlx::Error> {
    sqlx::query_as::<_, ExpiryLotResponse>(
        r#"
        SELECT id, product_id, store_id, location_id, quantity, unit,
               expiry_date, alert_days_before, batch_code,
               status, resolution, resolved_at, notes,
               created_at, updated_at
        FROM expiry_lots
        WHERE status = 'active'
        ORDER BY expiry_date ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

// ============================================================
// Partial resolution
// ============================================================

/// Subtracts `resolved_qty` from the lot's remaining quantity, sets
/// `status = 'resolved'` and fills `resolution`/`resolved_at` when remaining
/// quantity reaches zero. Returns the updated lot row.
pub async fn apply_partial_resolution(
    pool: &SqlitePool,
    lot_id: &str,
    resolved_qty: f64,
    resolution: &str,
    resolved_at: &str,
) -> Result<ExpiryLotResponse, sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    // Read current quantity first.
    let lot = get_expiry_lot(pool, lot_id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)?;

    let new_remaining = lot.quantity - resolved_qty;
    // Clamp to 1.0 so the DB CHECK(quantity > 0) constraint is not violated.
    // The lot is marked status='resolved' so it is excluded from active queries;
    // `lot_resolution_events` is the authoritative record of exact resolved qty.
    let qty_to_store = if new_remaining <= 0.0 {
        1.0
    } else {
        new_remaining
    };
    let new_status = if new_remaining <= 0.0 {
        "resolved"
    } else {
        "active"
    };

    sqlx::query(
        r#"
        UPDATE expiry_lots
        SET quantity = $1, status = $2, resolution = $3, resolved_at = $4,
            updated_at = $5
        WHERE id = $6
        "#,
    )
    .bind(qty_to_store)
    .bind(new_status)
    .bind(resolution)
    .bind(resolved_at)
    .bind(&now)
    .bind(lot_id)
    .execute(pool)
    .await?;

    get_expiry_lot(pool, lot_id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

/// Records a resolution event for an expiry lot.
pub async fn insert_resolution_event(
    pool: &SqlitePool,
    input: &ExpiryLotResolve,
) -> Result<LotResolutionEventResponse, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO lot_resolution_events
            (id, expiry_lot_id, quantity, resolution, notes, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(&id)
    .bind(&input.lot_id)
    .bind(input.quantity)
    .bind(&input.resolution)
    .bind(&input.notes)
    .bind(&now)
    .execute(pool)
    .await?;

    get_resolution_event(pool, &id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

/// Fetches a single resolution event by id.
pub async fn get_resolution_event(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<LotResolutionEventResponse>, sqlx::Error> {
    sqlx::query_as::<_, LotResolutionEventResponse>(
        r#"
        SELECT id, expiry_lot_id, quantity, resolution, notes, created_at
        FROM lot_resolution_events
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Returns all resolution events for a given expiry lot, newest first.
pub async fn list_resolution_events_by_lot(
    pool: &SqlitePool,
    lot_id: &str,
) -> Result<Vec<LotResolutionEventResponse>, sqlx::Error> {
    sqlx::query_as::<_, LotResolutionEventResponse>(
        r#"
        SELECT id, expiry_lot_id, quantity, resolution, notes, created_at
        FROM lot_resolution_events
        WHERE expiry_lot_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(lot_id)
    .fetch_all(pool)
    .await
}
