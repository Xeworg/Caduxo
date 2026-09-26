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
        SELECT el.id, el.product_id, el.store_id, el.location_id, sl.name AS location_name,
               el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        LEFT JOIN store_locations sl ON sl.id = el.location_id
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
        SELECT el.id, el.product_id, el.store_id, el.location_id, sl.name AS location_name,
               el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        LEFT JOIN store_locations sl ON sl.id = el.location_id
        WHERE el.product_id = $1 AND el.status = 'active'
        ORDER BY el.expiry_date ASC
        "#,
    )
    .bind(product_id)
    .fetch_all(pool)
    .await
}

/// Returns lots that have a positive balance in at least one location within
/// `store_id`, using the `lot_movements` ledger. This enables cross-store
/// visibility: a lot whose anchor `store_id` is A will still appear in
/// Scanner/Dashboard for store B when stock was transferred to B and the
/// destination balance is still positive.
///
/// The balance per lot is derived from `lot_movements`:
/// - Movements to `destination_location_id` add to the balance.
/// - Movements from `source_location_id` subtract from the balance.
/// A lot is included when the SUM for any location in the store is > 0.
///
/// This is a private helper factored out of the individual query functions so
/// the visibility condition stays in one place.
/// `start_param` is the first parameter number the fragment should use; it uses
/// `$start_param` for the ledger-balance check and `$start_param+1` for the
/// legacy-compat anchor check. Callers must bind `store_id` to both of these
/// positions, in that order.
///
/// Returns a condition that is true when EITHER:
/// (a) the lot has a positive balance in at least one location in `store_id`
///     (derived from lot_movements ledger), OR
/// (b) the lot has NO movements at all AND its anchor `el.store_id` equals
///     `store_id`. This preserves backward compatibility for legacy lots
///     created before the ledger system — they stay visible in their anchor
///     store without requiring ledger entries.
///
/// This means lots are included in a store when they have positive balance,
/// or when they are legacy lots without any ledger history in their anchor store.
fn lot_visible_in_store_condition(start_param: usize) -> String {
    let p1 = start_param;
    let p2 = start_param + 1;
    format!(
        r#"
        EXISTS (
            SELECT 1
            FROM (
                SELECT lm.destination_location_id AS loc, lm.quantity AS delta
                FROM lot_movements lm
                WHERE lm.expiry_lot_id = el.id AND lm.destination_location_id IS NOT NULL

                UNION ALL

                SELECT lm.source_location_id, -lm.quantity
                FROM lot_movements lm
                WHERE lm.expiry_lot_id = el.id AND lm.source_location_id IS NOT NULL
            ) ledger
            JOIN store_locations sl_loc ON sl_loc.id = ledger.loc
            WHERE sl_loc.store_id = ${p1}
            GROUP BY ledger.loc
            HAVING SUM(ledger.delta) > 0
        )
        OR (
            el.store_id = ${p2}
            AND NOT EXISTS (
                SELECT 1 FROM lot_movements lm WHERE lm.expiry_lot_id = el.id
            )
        )
        "#
    )
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
/// Visibility: a lot is included when it has a positive balance in at least
/// one location belonging to `store_id` — not when `expiry_lots.store_id`
/// equals `store_id`. This lets a receiving store discover a lot transferred
/// from another store while the balance in the receiving store's locations
/// is still positive. The anchor `store_id` on `expiry_lots` is unchanged
/// (preserves historical records).
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
    // Fragment starts at $2: $1 is used by batch_code in the outer WHERE.
    let visible = lot_visible_in_store_condition(2);
    let query = format!(
        r#"
        SELECT el.id, el.product_id, el.store_id, el.location_id, sl.name AS location_name,
               el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        LEFT JOIN store_locations sl ON sl.id = el.location_id
        WHERE el.batch_code = $1
          AND el.status = 'active'
          AND (p.id IS NULL OR p.is_active = 1)
          AND {visible}
        ORDER BY el.created_at DESC
        LIMIT 1
        "#,
    );
    sqlx::query_as::<_, ExpiryLotResponse>(&query)
        .bind(batch_code)
        .bind(store_id) // $2: sl_loc.store_id in visibility EXISTS
        .bind(store_id) // $3: el.store_id in legacy fallback
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
/// Visibility: a lot is included when it has a positive balance in at least
/// one location belonging to `store_id` — derived from `lot_movements`
/// rather than scoped by `expiry_lots.store_id`. This preserves the anchor
/// store for all existing records while enabling a receiving store to find
/// and operate transferred lots while their balance there is positive.
///
/// JOINs `products` to expose the unit kind for unit-aware downstream
/// consumers.
pub async fn list_active_lots_for_product_in_store(
    pool: &SqlitePool,
    product_id: &str,
    store_id: &str,
) -> Result<Vec<ExpiryLotResponse>, sqlx::Error> {
    // Fragment starts at $2: $1 is used by product_id in the outer WHERE.
    let visible = lot_visible_in_store_condition(2);
    let query = format!(
        r#"
        SELECT el.id, el.product_id, el.store_id, el.location_id, sl.name AS location_name,
               el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        LEFT JOIN store_locations sl ON sl.id = el.location_id
        WHERE el.product_id = $1
          AND el.status = 'active'
          AND {visible}
        ORDER BY el.expiry_date ASC, el.created_at ASC, el.id ASC
        "#,
    );
    sqlx::query_as::<_, ExpiryLotResponse>(&query)
        .bind(product_id)
        .bind(store_id) // $2: sl_loc.store_id in visibility EXISTS
        .bind(store_id) // $3: el.store_id in legacy fallback
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
        SELECT el.id, el.product_id, el.store_id, el.location_id, sl.name AS location_name,
               el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        LEFT JOIN store_locations sl ON sl.id = el.location_id
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
        SELECT el.id, el.product_id, el.store_id, el.location_id, sl.name AS location_name,
               el.quantity, el.unit,
               el.expiry_date, el.alert_days_before, el.batch_code,
               el.status, el.resolution, el.resolved_at, el.notes,
               el.created_at, el.updated_at,
               p.unit_type AS unit_type
        FROM expiry_lots el
        LEFT JOIN products p ON p.id = el.product_id
        LEFT JOIN store_locations sl ON sl.id = el.location_id
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

// ============================================================
// Tests — location_name projection
// ============================================================

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use crate::db::migrations::fresh_test_pool;

    // NOTE: no `use super::*` — avoids shadowing `fresh_test_pool`.
    // Call repository functions via `super::` explicitly.

    /// Inserts a store row directly so tests stay at the repository level.
    async fn seed_store(pool: &SqlitePool, name: &str) -> Result<String, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
             VALUES ($1, $2, 1, $3, $4)",
        )
        .bind(&id)
        .bind(name)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(id)
    }

    /// Inserts a product row directly. `unit_type` is set explicitly to keep
    /// tests independent of the back-fill migrations.
    async fn seed_product(
        pool: &SqlitePool,
        sku: &str,
        unit_type: &str,
    ) -> Result<String, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO products
                (id, sku, description, default_unit, default_alert_days_before,
                 is_active, unit_type, created_at, updated_at)
             VALUES ($1, $2, 'Test Product', 'kg', 30, 1, $3, $4, $5)",
        )
        .bind(&id)
        .bind(sku)
        .bind(unit_type)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(id)
    }

    /// Inserts an active store_location row.
    async fn seed_location(
        pool: &SqlitePool,
        store_id: &str,
        name: &str,
        is_active: i32,
    ) -> Result<String, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO store_locations
                (id, store_id, name, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&id)
        .bind(store_id)
        .bind(name)
        .bind(is_active)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(id)
    }

    /// Inserts an active expiry lot with an optional location_id.
    async fn seed_lot(
        pool: &SqlitePool,
        product_id: &str,
        store_id: &str,
        location_id: Option<&str>,
    ) -> Result<String, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO expiry_lots
                (id, product_id, store_id, location_id, quantity, unit,
                 expiry_date, alert_days_before, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 10.0, 'kg', '2099-12-31', 7, 'active', $5, $6)",
        )
        .bind(&id)
        .bind(product_id)
        .bind(store_id)
        .bind(location_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(id)
    }

    /// A lot whose `location_id` points at an active `store_locations` row
    /// must project that row's `name` into `location_name`. Guards against
    /// the regression where the backend would expose the UUID to consumers
    /// instead of the human-readable label.
    #[tokio::test]
    async fn get_expiry_lot_projects_real_location_name() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let store_id = seed_store(&pool, "Test Store").await?;
        let product_id = seed_product(&pool, "SKU-LOC-1", "decimal").await?;
        let location_id = seed_location(&pool, &store_id, "Fridge A", 1).await?;
        let lot_id = seed_lot(&pool, &product_id, &store_id, Some(&location_id)).await?;

        let lot = super::get_expiry_lot(&pool, &lot_id)
            .await?
            .expect("lot must exist");

        assert_eq!(lot.location_id.as_deref(), Some(location_id.as_str()));
        assert_eq!(lot.location_name.as_deref(), Some("Fridge A"));
        Ok(())
    }

    /// A lot without a `location_id` must project `location_name = None`. The
    /// scanner/product-detail sentinel `No location` behavior depends on this
    /// contract: any non-null `location_name` would suppress the sentinel.
    #[tokio::test]
    async fn get_expiry_lot_with_null_location_id_returns_null_location_name(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let store_id = seed_store(&pool, "Test Store").await?;
        let product_id = seed_product(&pool, "SKU-LOC-2", "decimal").await?;
        let lot_id = seed_lot(&pool, &product_id, &store_id, None).await?;

        let lot = super::get_expiry_lot(&pool, &lot_id)
            .await?
            .expect("lot must exist");

        assert!(lot.location_id.is_none());
        assert!(
            lot.location_name.is_none(),
            "missing location_id must project location_name = None"
        );
        Ok(())
    }

    /// Historic lots that reference a now-inactive location must still expose
    /// the original name. The decision is "include inactive locations in the
    /// projection"; an `is_active = 0` filter on the join would silently drop
    /// the name and force the UI back to a UUID/sentinel.
    #[tokio::test]
    async fn get_expiry_lot_projects_inactive_location_name(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let store_id = seed_store(&pool, "Test Store").await?;
        let product_id = seed_product(&pool, "SKU-LOC-3", "decimal").await?;
        let location_id = seed_location(&pool, &store_id, "Old Shelf", 0).await?;
        let lot_id = seed_lot(&pool, &product_id, &store_id, Some(&location_id)).await?;

        let lot = super::get_expiry_lot(&pool, &lot_id)
            .await?
            .expect("lot must exist");

        assert_eq!(lot.location_id.as_deref(), Some(location_id.as_str()));
        assert_eq!(
            lot.location_name.as_deref(),
            Some("Old Shelf"),
            "inactive location rows must remain in the projection"
        );
        Ok(())
    }

    /// `list_expiry_lots_by_product` must propagate `location_name` across
    /// every row, including lots whose location was deactivated. Mixed rows
    /// in a single response keep the contract honest.
    #[tokio::test]
    async fn list_expiry_lots_by_product_projects_location_names(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let store_id = seed_store(&pool, "Test Store").await?;
        let product_id = seed_product(&pool, "SKU-LOC-4", "decimal").await?;
        let active_loc = seed_location(&pool, &store_id, "Active Shelf", 1).await?;
        let inactive_loc = seed_location(&pool, &store_id, "Old Shelf", 0).await?;
        seed_lot(&pool, &product_id, &store_id, Some(&active_loc)).await?;
        seed_lot(&pool, &product_id, &store_id, Some(&inactive_loc)).await?;
        seed_lot(&pool, &product_id, &store_id, None).await?;

        let lots = super::list_expiry_lots_by_product(&pool, &product_id).await?;
        assert_eq!(lots.len(), 3);

        let mut names: Vec<Option<&str>> =
            lots.iter().map(|l| l.location_name.as_deref()).collect();
        names.sort();
        assert_eq!(
            names,
            vec![None, Some("Active Shelf"), Some("Old Shelf")],
            "every lot must expose its location name (or None) without dropping inactive rows"
        );
        Ok(())
    }

    // ── Cross-store visibility regression tests (task 6) ───────────────────

    /// Inserts a ledger movement entry directly.
    async fn seed_movement(
        pool: &SqlitePool,
        lot_id: &str,
        movement_kind: &str,
        quantity: f64,
        dest_location_id: Option<&str>,
        src_location_id: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO lot_movements
                (id, expiry_lot_id, movement_kind, direction, quantity,
                 source_location_id, destination_location_id, actor, created_at)
             VALUES ($1, $2, $3, NULL, $4, $5, $6, 'test', $7)",
        )
        .bind(&id)
        .bind(lot_id)
        .bind(movement_kind)
        .bind(quantity)
        .bind(src_location_id)
        .bind(dest_location_id)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Seeds the cross-store fixture:
    ///   Store A (anchor), Store B (receiving), Store C (third/no-balance)
    ///   One active product, one active lot with anchor in Store A.
    ///   Ledger seeds are NOT included — callers add their own movements to
    ///   control which stores have positive balance.
    async fn seed_cross_store_fixture(
        pool: &SqlitePool,
    ) -> Result<(String, String, String, String, String, String), Box<dyn std::error::Error>> {
        // Store A — anchor store
        let store_a = seed_store(&pool, "Anchor Store").await?;

        // Store B — receiving store
        let store_b = seed_store(&pool, "Receiving Store").await?;

        // Store C — third store with no balance
        let store_c = seed_store(&pool, "Third Store").await?;

        // Location in Store A
        let loc_a = seed_location(&pool, &store_a, "Storage A", 1).await?;

        // Location in Store B
        let loc_b = seed_location(&pool, &store_b, "Storage B", 1).await?;

        // Location in Store C
        let _loc_c = seed_location(&pool, &store_c, "Storage C", 1).await?;

        // Product
        let product_id = seed_product(&pool, "CROSS-SKU", "decimal").await?;

        // Lot with anchor in Store A
        let lot_id = seed_lot(&pool, &product_id, &store_a, Some(&loc_a)).await?;

        // Return: (store_a, store_b, store_c, loc_a, loc_b, lot_id)
        Ok((store_a, store_b, store_c, loc_a, loc_b, lot_id))
    }

    /// `find_active_lot_by_batch_code` must resolve a lot when the active store
    /// (B) has a positive balance from a transfer, even though the lot's
    /// anchor `store_id` is A. This is the core cross-store transfer discovery
    /// case for the Scanner batch-code resolution path.
    #[tokio::test]
    async fn find_active_lot_by_batch_code_includes_lot_in_receiving_store(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, store_b, _store_c, loc_a, loc_b, lot_id) =
            seed_cross_store_fixture(&pool).await?;
        let batch_code = "CROSS-BATCH-001";

        // Update the lot's batch code to the test value.
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE expiry_lots SET batch_code = $1, updated_at = $2 WHERE id = $3")
            .bind(batch_code)
            .bind(&now)
            .bind(&lot_id)
            .execute(&pool)
            .await?;

        // Initial stock in Store A (anchor) — loc_a is in Store A
        seed_movement(&pool, &lot_id, "entry:initial", 10.0, Some(&loc_a), None).await?;

        // Transfer 5 units from loc_a (Store A) to loc_b (Store B)
        seed_movement(&pool, &lot_id, "transfer", 5.0, Some(&loc_b), Some(&loc_a)).await?;

        // Scanner in Store B must find the lot by batch code.
        let found = super::find_active_lot_by_batch_code(&pool, &store_b, batch_code).await?;
        assert!(
            found.is_some(),
            "lot with positive balance in Store B must be resolved from Store B"
        );
        assert_eq!(found.unwrap().id, lot_id);
        Ok(())
    }

    /// `find_active_lot_by_batch_code` must NOT resolve a lot when the
    /// scanning store (C) has no positive balance for that lot. The lot may
    /// exist with positive balance in Store A and B, but Store C cannot
    /// discover it until stock is transferred there.
    #[tokio::test]
    async fn find_active_lot_by_batch_code_excludes_lot_from_store_without_balance(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, store_b, store_c, loc_a, loc_b, lot_id) =
            seed_cross_store_fixture(&pool).await?;
        let batch_code = "CROSS-BATCH-002";

        sqlx::query("UPDATE expiry_lots SET batch_code = $1, updated_at = $2 WHERE id = $3")
            .bind(batch_code)
            .bind(&Utc::now().to_rfc3339())
            .bind(&lot_id)
            .execute(&pool)
            .await?;

        // Initial stock in loc_a (Store A)
        seed_movement(&pool, &lot_id, "entry:initial", 10.0, Some(&loc_a), None).await?;

        // Transfer 5 units from loc_a to loc_b (Store B has balance, Store C does not)
        seed_movement(&pool, &lot_id, "transfer", 5.0, Some(&loc_b), Some(&loc_a)).await?;

        // Scanner in Store C must NOT find the lot (no balance there).
        let found = super::find_active_lot_by_batch_code(&pool, &store_c, batch_code).await?;
        assert!(
            found.is_none(),
            "lot without balance in Store C must not be resolved from Store C"
        );
        Ok(())
    }

    /// `find_active_lot_by_batch_code` must NOT resolve a lot when the
    /// destination store's balance has been fully drained (all stock moved out).
    #[tokio::test]
    async fn find_active_lot_by_batch_code_excludes_lot_with_zero_balance_in_target_store(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, store_b, _store_c, loc_a, loc_b, lot_id) =
            seed_cross_store_fixture(&pool).await?;
        let batch_code = "CROSS-BATCH-003";

        sqlx::query("UPDATE expiry_lots SET batch_code = $1, updated_at = $2 WHERE id = $3")
            .bind(batch_code)
            .bind(&Utc::now().to_rfc3339())
            .bind(&lot_id)
            .execute(&pool)
            .await?;

        // Initial stock in loc_a (Store A)
        seed_movement(&pool, &lot_id, "entry:initial", 10.0, Some(&loc_a), None).await?;

        // Transfer 10 units from loc_a to loc_b (Store B)
        seed_movement(&pool, &lot_id, "transfer", 10.0, Some(&loc_b), Some(&loc_a)).await?;

        // Transfer all 10 units from loc_b back to loc_a (drain B)
        seed_movement(&pool, &lot_id, "transfer", 10.0, Some(&loc_a), Some(&loc_b)).await?;

        // Scanner in Store B must NOT find the lot (balance is 0).
        let found = super::find_active_lot_by_batch_code(&pool, &store_b, batch_code).await?;
        assert!(
            found.is_none(),
            "lot with zero balance in Store B must not be resolved from Store B"
        );
        Ok(())
    }

    /// `list_active_lots_for_product_in_store` must include a lot when the
    /// target store has a positive balance, even if the lot's anchor store
    /// is different. This covers the `ProductMatch` Scanner path.
    #[tokio::test]
    async fn list_active_lots_for_product_in_store_includes_lot_in_receiving_store(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, store_b, _store_c, loc_a, loc_b, lot_id) =
            seed_cross_store_fixture(&pool).await?;

        // Get the product_id from the lot.
        let lot = super::get_expiry_lot(&pool, &lot_id)
            .await?
            .expect("lot must exist");
        let product_id = lot.product_id;

        // Initial stock in loc_a (Store A)
        seed_movement(&pool, &lot_id, "entry:initial", 10.0, Some(&loc_a), None).await?;

        // Transfer 5 units from loc_a to loc_b (Store B)
        seed_movement(&pool, &lot_id, "transfer", 5.0, Some(&loc_b), Some(&loc_a)).await?;

        // List lots for this product from Store B.
        let lots =
            super::list_active_lots_for_product_in_store(&pool, &product_id, &store_b).await?;
        assert!(
            lots.iter().any(|l| l.id == lot_id),
            "lot with positive balance in Store B must appear in Store B product list"
        );
        Ok(())
    }

    /// `list_active_lots_for_product_in_store` must exclude a lot when the
    /// target store has no positive balance for that lot.
    #[tokio::test]
    async fn list_active_lots_for_product_in_store_excludes_lot_from_store_without_balance(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, store_b, store_c, loc_a, loc_b, lot_id) =
            seed_cross_store_fixture(&pool).await?;

        let lot = super::get_expiry_lot(&pool, &lot_id)
            .await?
            .expect("lot must exist");
        let product_id = lot.product_id;

        // Initial stock in loc_a (Store A)
        seed_movement(&pool, &lot_id, "entry:initial", 10.0, Some(&loc_a), None).await?;

        // Transfer 5 units from loc_a to loc_b (Store B has balance, Store C does not)
        seed_movement(&pool, &lot_id, "transfer", 5.0, Some(&loc_b), Some(&loc_a)).await?;

        // List lots for this product from Store C (no balance there).
        let lots =
            super::list_active_lots_for_product_in_store(&pool, &product_id, &store_c).await?;
        assert!(
            !lots.iter().any(|l| l.id == lot_id),
            "lot without balance in Store C must not appear in Store C product list"
        );
        Ok(())
    }
}
