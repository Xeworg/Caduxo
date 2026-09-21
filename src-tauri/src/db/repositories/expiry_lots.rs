//! Repository for expiry lot persistence: expiry_lots and lot_resolution_events.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::dto::expiry_lots::{
    ExpiryLotResolve, ExpiryLotResponse, ExpiryLotUpdate, LotResolutionEventResponse,
};

// ============================================================
// Expiry lots — create / update / archive
// ============================================================

/// Fetches a single expiry lot by id, or `None` if it does not exist.
/// JOINs `products` to expose the unit kind for unit-aware downstream consumers.
pub async fn get_expiry_lot(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<ExpiryLotResponse>, sqlx::Error> {
    sqlx::query_as::<_, ExpiryLotResponse>(
        r#"
        SELECT el.id, el.product_id, el.store_id, el.location_id, el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        WHERE el.id = $1
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

// ============================================================
// Expiry lots — list queries
// ============================================================

/// Returns all active expiry lots for a given product.
/// JOINs `products` to expose the unit kind for unit-aware downstream consumers.
pub async fn list_expiry_lots_by_product(
    pool: &SqlitePool,
    product_id: &str,
) -> Result<Vec<ExpiryLotResponse>, sqlx::Error> {
    sqlx::query_as::<_, ExpiryLotResponse>(
        r#"
        SELECT el.id, el.product_id, el.store_id, el.location_id, el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        WHERE el.product_id = $1 AND el.status = 'active'
        ORDER BY el.expiry_date ASC
        "#,
    )
    .bind(product_id)
    .fetch_all(pool)
    .await
}

/// Returns the active lot in the given store whose `batch_code` matches
/// `batch_code` exactly. Trimming is the caller's responsibility; the
/// comparison is verbatim so the spec's "no upper-casing, lower-casing, or
/// whitespace normalization" rule is preserved.
///
/// Introduced by `scanner-quick-operations` (PR 1) for the
/// `resolve_scanner_code` lot-code lookup branch. Returns `None` when no
/// active lot matches; the caller falls back to barcode → SKU lookup.
/// Returns the most recently created lot when multiple active rows carry the
/// same `batch_code` in the same store (a defensive `LIMIT 1` keeps the
/// service-layer contract deterministic; the unique-batch-code invariant
/// is not enforced at the schema level).
///
/// JOINs `products` to expose the unit kind for unit-aware downstream
/// consumers and to enforce the scanner's "active product only" rule via
/// `products.is_active = 1`. The Scanner tab MUST NOT resolve lot codes
/// whose parent product has been archived (the lot remains in the
/// database for stock-history audit, but the scanner lookup ignores it).
pub async fn find_active_lot_by_batch_code(
    pool: &SqlitePool,
    store_id: &str,
    batch_code: &str,
) -> Result<Option<ExpiryLotResponse>, sqlx::Error> {
    sqlx::query_as::<_, ExpiryLotResponse>(
        r#"
        SELECT el.id, el.product_id, el.store_id, el.location_id, el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        WHERE el.store_id = $1
          AND el.batch_code = $2
          AND el.status = 'active'
          AND (p.id IS NULL OR p.is_active = 1)
        ORDER BY el.created_at DESC
        LIMIT 1
        "#,
    )
    .bind(store_id)
    .bind(batch_code)
    .fetch_optional(pool)
    .await
}

/// Returns the active lots for a given product in the given store, ordered
/// by stable FEFO order `(expiry_date ASC, created_at ASC, id ASC)`.
///
/// Introduced by `scanner-quick-operations` (PR 1) for the
/// `resolve_scanner_code` `ProductMatch` branch. The ordering matches the
/// spec's `expiry_date ASC, created_at ASC, id ASC` tie-break so the
/// frontend can render the FEFO suggestion without re-sorting; `id ASC`
/// is the deterministic tie-breaker when two lots share an expiry date
/// and a creation timestamp (rare in practice, but the contract demands
/// stability for snapshot-style lookups).
///
/// JOINs `products` to expose the unit kind for unit-aware downstream
/// consumers.
pub async fn list_active_lots_for_product_in_store(
    pool: &SqlitePool,
    product_id: &str,
    store_id: &str,
) -> Result<Vec<ExpiryLotResponse>, sqlx::Error> {
    sqlx::query_as::<_, ExpiryLotResponse>(
        r#"
        SELECT el.id, el.product_id, el.store_id, el.location_id, el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        WHERE el.product_id = $1
          AND el.store_id = $2
          AND el.status = 'active'
        ORDER BY el.expiry_date ASC, el.created_at ASC, el.id ASC
        "#,
    )
    .bind(product_id)
    .bind(store_id)
    .fetch_all(pool)
    .await
}

/// Returns all active expiry lots for a given store.
/// JOINs `products` to expose the unit kind for unit-aware downstream consumers.
pub async fn list_expiry_lots_by_store(
    pool: &SqlitePool,
    store_id: &str,
) -> Result<Vec<ExpiryLotResponse>, sqlx::Error> {
    sqlx::query_as::<_, ExpiryLotResponse>(
        r#"
        SELECT el.id, el.product_id, el.store_id, el.location_id, el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        WHERE el.store_id = $1 AND el.status = 'active'
        ORDER BY el.expiry_date ASC
        "#,
    )
    .bind(store_id)
    .fetch_all(pool)
    .await
}

/// Returns all active expiry lots (no filter).
/// JOINs `products` to expose the unit kind for unit-aware downstream consumers.
pub async fn list_all_active_expiry_lots(
    pool: &SqlitePool,
) -> Result<Vec<ExpiryLotResponse>, sqlx::Error> {
    sqlx::query_as::<_, ExpiryLotResponse>(
        r#"
        SELECT el.id, el.product_id, el.store_id, el.location_id, el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        WHERE el.status = 'active'
        ORDER BY el.expiry_date ASC
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
