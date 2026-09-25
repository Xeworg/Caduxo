//! Repository for dashboard queries: fetching active lots with enriched
//! product/store/location context for the urgency cards and lot table.
//!
//! No business logic here — only SQL, projection, and optional filters.

use sqlx::SqlitePool;

use crate::dto::dashboard::{DashboardFilters, DashboardLotRow};
use crate::dto::products::UNCATEGORIZED_SENTINEL;
// sqlx needs this in scope to find UnitKind's Decode impl.

/// Returns the SQL predicate fragment that selects lots that are visible in
/// a given store. A lot is visible when EITHER:
/// (a) it has a positive balance in at least one location in the store
///     (derived from `lot_movements` ledger), OR
/// (b) it has NO movements at all AND its anchor `el.store_id` equals the
///     target store. This preserves backward compatibility for legacy lots
///     created before the ledger system — they stay visible in their anchor
///     store without requiring ledger entries.
///
/// `start_param` is the first parameter number the fragment should use; it uses
/// `$start_param` for the ledger-balance check and `$start_param+1` for the
/// legacy-compat anchor check. Callers must bind `store_id` to both positions.
fn store_has_positive_balance_condition(start_param: usize) -> String {
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

/// Fetches active expiry lots with enriched context (product, store, location).
///
/// Returns rows ordered by expiry_date ASC so callers can sort/ bucket further.
/// Filtering by `store_id` / `location_id` is optional; all active lots are
/// returned when neither is specified.
///
/// Caller is responsible for computing urgency groups and counts.
pub async fn list_dashboard_lots(
    pool: &SqlitePool,
    filters: &DashboardFilters,
) -> Result<Vec<DashboardLotRow>, sqlx::Error> {
    // Build the base SELECT with LEFT JOINs for optional location.
    // We always filter to status = 'active' so resolved/archived lots never appear.
    //
    // Store visibility: when `filters.store_id` is Some, a lot is included only
    // when it has a positive balance in at least one location belonging to that
    // store (derived from `lot_movements`, not from `expiry_lots.store_id`).
    // When `filters.store_id` is None, every active lot is returned once per
    // lot id — the anchor `store_id` on `expiry_lots` governs which store's
    // row is projected (preserving existing no-filter behavior).
    // Fragment starts at $3: $1 and $2 are used by the location filter.
    let store_visible = match &filters.store_id {
        Some(_store_id) => format!(
            "              AND (\n                  {}\n              )\n",
            store_has_positive_balance_condition(3)
        ),
        None => String::new(),
    };

    let query = format!(
        "SELECT
                el.id             AS lot_id,
                el.product_id,
                p.sku,
                p.description,
                el.store_id,
                s.name            AS store_name,
                el.location_id,
                sl.name           AS location_name,
                el.quantity,
                el.unit,
                el.expiry_date,
                el.alert_days_before,
                el.batch_code,
                el.status,
                ''                AS urgency,
                0                 AS days_remaining,
                p.default_unit_id,
                p.unit_type
            FROM expiry_lots AS el
            JOIN products    AS p  ON p.id = el.product_id
            JOIN stores      AS s  ON s.id = el.store_id
            LEFT JOIN store_locations AS sl ON sl.id = el.location_id
            WHERE el.status = 'active'
              AND p.lifecycle != 'retired'
              AND ($1 IS NULL OR el.location_id = $2)
{store_visible}",
    );
    let mut query = query;

    // Category filter with sentinel-aware ANY-of semantics.
    // - The sentinel (`UNCATEGORIZED_SENTINEL`) is filter-only and means
    //   "products that have NO active category memberships".
    // - Real ids use a single `EXISTS` over `product_categories` so we get
    //   ANY-of without a nested `IN ($5, $6, ...)`.
    // - When both are present we OR them: products matching the real ids OR
    //   products with no junction rows at all.
    // - Sentinel-only also matches products with zero junction rows.
    //
    // Category filter placeholders start at $5 (after $1-$4 used by location
    // and store visibility). Bound in the same order after those bindings.
    if let Some(ref cat_ids) = filters.category_ids {
        if !cat_ids.is_empty() {
            let real_ids: Vec<&String> = cat_ids
                .iter()
                .filter(|id| id.as_str() != UNCATEGORIZED_SENTINEL)
                .collect();
            let include_uncategorized = cat_ids.iter().any(|id| id == UNCATEGORIZED_SENTINEL);

            // Count existing positional bindings before category filter.
            // Location filter: 2 bindings ($1, $2). Store visibility (if present): 2 ($3, $4).
            let cat_start = if filters.store_id.is_some() { 5 } else { 3 };

            // Pre-compute the IN-clause placeholders for real ids.
            let in_placeholders: String = (0..real_ids.len())
                .map(|i| format!("${}", cat_start + i))
                .collect::<Vec<_>>()
                .join(", ");

            match (real_ids.is_empty(), include_uncategorized) {
                (true, true) => {
                    // Only the sentinel: every active lot whose product has
                    // no junction rows.
                    query.push_str(
                        "              AND NOT EXISTS (
                  SELECT 1 FROM product_categories pc
                  WHERE pc.product_id = p.id
              )
            ",
                    );
                }
                (false, false) => {
                    // Only real ids: ANY-of via EXISTS over the junction.
                    query.push_str(&format!(
                        "              AND EXISTS (
                  SELECT 1 FROM product_categories pc
                  WHERE pc.product_id = p.id
                    AND pc.category_id IN ({})
              )
            ",
                        in_placeholders
                    ));
                }
                (false, true) => {
                    // Real ids + sentinel: ANY-of real ids OR no junction rows.
                    query.push_str(&format!(
                        "              AND (
                  EXISTS (
                      SELECT 1 FROM product_categories pc
                      WHERE pc.product_id = p.id
                        AND pc.category_id IN ({})
                  )
                  OR NOT EXISTS (
                      SELECT 1 FROM product_categories pc
                      WHERE pc.product_id = p.id
                  )
              )
            ",
                        in_placeholders
                    ));
                }
                (true, false) => {
                    // Empty after filtering out sentinel: treat as no filter.
                }
            }
        }
    }

    query.push_str("            ORDER BY el.expiry_date ASC\n            ");

    // Collect real category ids (excluding sentinel) to bind in query.
    let real_cat_ids: Vec<String> = filters
        .category_ids
        .as_ref()
        .map(|ids| {
            ids.iter()
                .filter(|id| id.as_str() != UNCATEGORIZED_SENTINEL)
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    let mut q = sqlx::query_as::<_, DashboardLotRow>(&query)
        .bind(&filters.location_id) // $1: IS NULL check for optional location filter
        .bind(&filters.location_id); // $2: el.location_id = $2

    // Bind store_id for the visibility predicate ($3 and $4 in the compiled query).
    if let Some(ref store_id) = filters.store_id {
        q = q.bind(store_id).bind(store_id);
    }

    // Bind category ids in the same order as their placeholders appear.
    // Placeholders start at $5 if store filter present, else $3.
    for cid in &real_cat_ids {
        q = q.bind(cid);
    }

    let rows = q.fetch_all(pool).await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use crate::db::migrations::fresh_test_pool;
    use crate::dto::dashboard::DashboardFilters;
    use crate::dto::products::ProductCreate;

    /// Seeds a schema suitable for ledger-visibility tests:
    ///   Store A (anchor), Store B (receiving), Store C (third/no-balance)
    ///   One product, one lot with anchor in Store A.
    ///   The lot is NOT inserted into lot_movements here — callers add their
    ///   own movement entries to control the balance scenario.
    async fn seed_cross_store_fixture(
        pool: &SqlitePool,
    ) -> Result<(String, String, String, String, String), Box<dyn std::error::Error>> {
        let now = Utc::now().to_rfc3339();

        // Store A — anchor store
        let store_a = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
             VALUES ($1, 'Store A', 1, $2, $3)",
        )
        .bind(&store_a)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Store B — receiving store
        let store_b = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
             VALUES ($1, 'Store B', 1, $2, $3)",
        )
        .bind(&store_b)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Store C — third store, no balance
        let store_c = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
             VALUES ($1, 'Store C', 1, $2, $3)",
        )
        .bind(&store_c)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Location in Store A
        let loc_a = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at)
             VALUES ($1, $2, 'Storage A', 1, $3, $4)",
        )
        .bind(&loc_a)
        .bind(&store_a)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Location in Store B
        let loc_b = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at)
             VALUES ($1, $2, 'Storage B', 1, $3, $4)",
        )
        .bind(&loc_b)
        .bind(&store_b)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Product
        let product_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO products (id, sku, description, default_alert_days_before,
                                  is_active, created_at, updated_at)
             VALUES ($1, 'XSTORE-001', 'Cross-Store Test Product', 7, 1, $2, $3)",
        )
        .bind(&product_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Lot with anchor in Store A — expiry far future
        let lot_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO expiry_lots (id, product_id, store_id, location_id, quantity, unit,
                                      expiry_date, alert_days_before, status,
                                      created_at, updated_at)
             VALUES ($1, $2, $3, $4, 5.0, 'kg', '2099-06-30', 7, 'active', $5, $6)",
        )
        .bind(&lot_id)
        .bind(&product_id)
        .bind(&store_a)
        .bind(&loc_a)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok((store_a, store_b, store_c, loc_a, lot_id))
    }

    /// Inserts a ledger movement entry directly.
    async fn insert_movement(
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

    // ── Store-visibility regression tests ───────────────────────────────

    /// A lot with positive balance in Store B's locations (via ledger) must
    /// appear in Dashboard when filtered to Store B, even though its anchor
    /// store_id is Store A. This is the core cross-store transfer discovery case.
    #[tokio::test]
    async fn list_dashboard_lots_includes_lot_in_receiving_store(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, store_b, _store_c, loc_a, lot_id) = seed_cross_store_fixture(&pool).await?;

        // Initial stock in Store A (anchor)
        insert_movement(&pool, &lot_id, "entry:initial", 10.0, Some(&loc_a), None).await?;

        // Transfer 5 units to Store B
        let loc_b_id: String = {
            // Look up loc_b by querying store_locations for store_b
            let rows: Vec<(String,)> =
                sqlx::query_as("SELECT id FROM store_locations WHERE store_id = $1")
                    .bind(&store_b)
                    .fetch_all(&pool)
                    .await?;
            rows[0].0.clone()
        };
        insert_movement(
            &pool,
            &lot_id,
            "transfer",
            5.0,
            Some(&loc_b_id),
            Some(&loc_a),
        )
        .await?;

        // Dashboard filtered to Store B should include the lot.
        let filters = DashboardFilters {
            store_id: Some(store_b.clone()),
            location_id: None,
            preset: None,
            urgency: None,
            category_ids: None,
        };
        let rows = super::list_dashboard_lots(&pool, &filters).await?;
        assert!(
            rows.iter().any(|r| r.lot_id == lot_id),
            "lot transferred to Store B must appear in Dashboard for Store B"
        );
        Ok(())
    }

    /// A lot that was transferred to Store C but has no positive balance there
    /// (fully transferred out or never transferred) must NOT appear in Dashboard
    /// when filtered to Store C.
    #[tokio::test]
    async fn list_dashboard_lots_excludes_lot_from_store_without_balance(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, _store_b, store_c, loc_a, lot_id) = seed_cross_store_fixture(&pool).await?;

        // Initial stock in Store A (anchor) — nothing in Store C
        insert_movement(&pool, &lot_id, "entry:initial", 10.0, Some(&loc_a), None).await?;

        // Dashboard filtered to Store C must NOT include the lot.
        let filters = DashboardFilters {
            store_id: Some(store_c.clone()),
            location_id: None,
            preset: None,
            urgency: None,
            category_ids: None,
        };
        let rows = super::list_dashboard_lots(&pool, &filters).await?;
        assert!(
            !rows.iter().any(|r| r.lot_id == lot_id),
            "lot without balance in Store C must not appear in Dashboard for Store C"
        );
        Ok(())
    }

    /// A lot whose balance was fully drained from Store B (all stock transferred
    /// out) must NOT appear in Dashboard for Store B.
    #[tokio::test]
    async fn list_dashboard_lots_excludes_lot_with_zero_balance_in_target_store(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, store_b, _store_c, loc_a, lot_id) = seed_cross_store_fixture(&pool).await?;

        // Initial stock in Store A
        insert_movement(&pool, &lot_id, "entry:initial", 10.0, Some(&loc_a), None).await?;

        // Transfer all 10 units to Store B
        let loc_b_id: String = {
            let rows: Vec<(String,)> =
                sqlx::query_as("SELECT id FROM store_locations WHERE store_id = $1")
                    .bind(&store_b)
                    .fetch_all(&pool)
                    .await?;
            rows[0].0.clone()
        };
        insert_movement(
            &pool,
            &lot_id,
            "transfer",
            10.0,
            Some(&loc_b_id),
            Some(&loc_a),
        )
        .await?;

        // Now transfer all 10 units from Store B back to Store A (drain B)
        insert_movement(
            &pool,
            &lot_id,
            "transfer",
            10.0,
            Some(&loc_a),
            Some(&loc_b_id),
        )
        .await?;

        // Dashboard filtered to Store B must NOT include the lot (B is drained).
        let filters = DashboardFilters {
            store_id: Some(store_b.clone()),
            location_id: None,
            preset: None,
            urgency: None,
            category_ids: None,
        };
        let rows = super::list_dashboard_lots(&pool, &filters).await?;
        assert!(
            !rows.iter().any(|r| r.lot_id == lot_id),
            "drained lot must not appear in Dashboard for the drained store"
        );
        Ok(())
    }

    /// When the no-filter case is used (store_id = None), the lot must appear
    /// once, governed by its anchor store — preserving backward compatibility.
    #[tokio::test]
    async fn list_dashboard_lots_no_filter_returns_one_row_per_lot(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (_store_a, store_b, _store_c, loc_a, lot_id) = seed_cross_store_fixture(&pool).await?;

        // Initial stock in Store A
        insert_movement(&pool, &lot_id, "entry:initial", 10.0, Some(&loc_a), None).await?;

        // Transfer 5 units to Store B
        let loc_b_id: String = {
            let rows: Vec<(String,)> =
                sqlx::query_as("SELECT id FROM store_locations WHERE store_id = $1")
                    .bind(&store_b)
                    .fetch_all(&pool)
                    .await?;
            rows[0].0.clone()
        };
        insert_movement(
            &pool,
            &lot_id,
            "transfer",
            5.0,
            Some(&loc_b_id),
            Some(&loc_a),
        )
        .await?;

        // No filter — lot should appear once (anchor store governs projection).
        let rows = super::list_dashboard_lots(&pool, &DashboardFilters::default()).await?;
        let matching: Vec<_> = rows.iter().filter(|r| r.lot_id == lot_id).collect();
        assert_eq!(
            matching.len(),
            1,
            "no-filter dashboard must return each lot exactly once"
        );
        assert_eq!(
            matching[0].store_id, _store_a,
            "anchor store_id governs projection when no store filter is applied"
        );
        Ok(())
    }

    // ── Pre-existing regression tests (require ledger seeds) ──────────────

    /// Seeds a full schema: stores, locations, products, lots, AND the
    /// `entry:initial` ledger entry for each lot so that the store-visibility
    /// predicate is satisfied.
    async fn seed_schema(pool: &SqlitePool) -> Result<(String, String), sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        // Store A
        let store_a = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
             VALUES ($1, 'Store A', 1, $2, $3)",
        )
        .bind(&store_a)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Store B
        let store_b = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
             VALUES ($1, 'Store B', 1, $2, $3)",
        )
        .bind(&store_b)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Location for Store A
        let loc_a = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at)
             VALUES ($1, $2, 'Fridge A', 1, $3, $4)",
        )
        .bind(&loc_a)
        .bind(&store_a)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Product
        let product_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO products (id, sku, description, default_alert_days_before,
                                  is_active, created_at, updated_at)
             VALUES ($1, 'DASH-001', 'Dashboard Test Product', 7, 1, $2, $3)",
        )
        .bind(&product_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Lot in Store A — expires far future (no urgency)
        let lot_future = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO expiry_lots (id, product_id, store_id, location_id, quantity, unit,
                                      expiry_date, alert_days_before, status,
                                      created_at, updated_at)
             VALUES ($1, $2, $3, $4, 5.0, 'kg', '2099-06-30', 7, 'active', $5, $6)",
        )
        .bind(&lot_future)
        .bind(&product_id)
        .bind(&store_a)
        .bind(&loc_a)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Seed entry:initial for lot_future in loc_a
        sqlx::query(
            "INSERT INTO lot_movements
                (id, expiry_lot_id, movement_kind, direction, quantity,
                 source_location_id, destination_location_id, actor, created_at)
             VALUES ($1, $2, 'entry:initial', NULL, 5.0, NULL, $3, 'test', $4)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&lot_future)
        .bind(&loc_a)
        .bind(&now)
        .execute(pool)
        .await?;

        // Location for Store B (needed for lot_today's ledger entry)
        let loc_b = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at)
             VALUES ($1, $2, 'Fridge B', 1, $3, $4)",
        )
        .bind(&loc_b)
        .bind(&store_b)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Lot in Store B — expires today (today bucket)
        let lot_today = Uuid::new_v4().to_string();
        let today_str = Utc::now().date_naive().format("%Y-%m-%d").to_string();
        sqlx::query(
            "INSERT INTO expiry_lots (id, product_id, store_id, location_id, quantity, unit,
                                      expiry_date, alert_days_before, status,
                                      created_at, updated_at)
             VALUES ($1, $2, $3, NULL, 3.0, 'L', $4, 14, 'active', $5, $6)",
        )
        .bind(&lot_today)
        .bind(&product_id)
        .bind(&store_b)
        .bind(&today_str)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Seed entry:initial for lot_today — use loc_b so CHECK constraint passes
        sqlx::query(
            "INSERT INTO lot_movements
                (id, expiry_lot_id, movement_kind, direction, quantity,
                 source_location_id, destination_location_id, actor, created_at)
             VALUES ($1, $2, 'entry:initial', NULL, 3.0, NULL, $3, 'test', $4)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&lot_today)
        .bind(&loc_b)
        .bind(&now)
        .execute(pool)
        .await?;

        // Lot in Store A — expired
        let lot_expired = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO expiry_lots (id, product_id, store_id, location_id, quantity, unit,
                                      expiry_date, alert_days_before, status,
                                      created_at, updated_at)
             VALUES ($1, $2, $3, NULL, 2.0, 'pcs', '2020-01-01', 30, 'active', $4, $5)",
        )
        .bind(&lot_expired)
        .bind(&product_id)
        .bind(&store_a)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Seed entry:initial for lot_expired — use loc_a so CHECK constraint passes
        sqlx::query(
            "INSERT INTO lot_movements
                (id, expiry_lot_id, movement_kind, direction, quantity,
                 source_location_id, destination_location_id, actor, created_at)
             VALUES ($1, $2, 'entry:initial', NULL, 2.0, NULL, $3, 'test', $4)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&lot_expired)
        .bind(&loc_a)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok((store_a.clone(), store_b.clone()))
    }

    #[tokio::test]
    async fn list_returns_all_active_lots_ordered_by_expiry(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (_store_a, _store_b) = seed_schema(&pool).await?;

        let rows = super::list_dashboard_lots(&pool, &DashboardFilters::default()).await?;
        // 3 active lots seeded above
        assert_eq!(rows.len(), 3, "all 3 active lots should be returned");
        // Ordered by expiry_date ASC
        assert_eq!(rows[0].expiry_date, "2020-01-01", "expired first");
        assert_eq!(rows[1].expiry_date.len(), 10, "today expiry has valid date");
        Ok(())
    }

    #[tokio::test]
    async fn excludes_lots_whose_product_is_retired() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, _store_b) = seed_schema(&pool).await?;
        let now = Utc::now().to_rfc3339();
        let retired_product_id = Uuid::new_v4().to_string();
        let retired_lot_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO products (id, sku, description, default_alert_days_before,
                                  is_active, lifecycle, created_at, updated_at)
             VALUES ($1, 'DASH-RETIRED', 'Retired Dashboard Product', 7, 1, 'retired', $2, $3)",
        )
        .bind(&retired_product_id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO expiry_lots (id, product_id, store_id, location_id, quantity, unit,
                                      expiry_date, alert_days_before, status,
                                      created_at, updated_at)
             VALUES ($1, $2, $3, NULL, 1.0, 'pcs', '2099-12-31', 7, 'active', $4, $5)",
        )
        .bind(&retired_lot_id)
        .bind(&retired_product_id)
        .bind(&store_a)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

        let rows = super::list_dashboard_lots(&pool, &DashboardFilters::default()).await?;
        assert!(
            rows.iter().all(|r| r.product_id != retired_product_id),
            "dashboard operational lots must exclude retired parent products"
        );
        Ok(())
    }

    #[tokio::test]
    async fn includes_lots_whose_product_is_archived() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, _store_b) = seed_schema(&pool).await?;
        let now = Utc::now().to_rfc3339();
        let archived_product_id = Uuid::new_v4().to_string();
        let archived_lot_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO products (id, sku, description, default_alert_days_before,
                                  is_active, lifecycle, created_at, updated_at)
             VALUES ($1, 'DASH-ARCHIVED', 'Archived Dashboard Product', 7, 0, 'archived', $2, $3)",
        )
        .bind(&archived_product_id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO expiry_lots (id, product_id, store_id, location_id, quantity, unit,
                                      expiry_date, alert_days_before, status,
                                      created_at, updated_at)
             VALUES ($1, $2, $3, NULL, 1.0, 'pcs', '2099-12-31', 7, 'active', $4, $5)",
        )
        .bind(&archived_lot_id)
        .bind(&archived_product_id)
        .bind(&store_a)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

        let rows = super::list_dashboard_lots(&pool, &DashboardFilters::default()).await?;
        assert!(
            rows.iter().any(|r| r.product_id == archived_product_id),
            "archived products are recoverable and their active lots stay visible operationally"
        );
        Ok(())
    }

    #[tokio::test]
    async fn list_filters_by_store_id() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, store_b) = seed_schema(&pool).await?;

        let filters = DashboardFilters {
            store_id: Some(store_a.clone()),
            location_id: None,
            preset: None,
            urgency: None,
            category_ids: None,
        };
        let rows = super::list_dashboard_lots(&pool, &filters).await?;
        assert!(
            rows.iter().all(|r| r.store_id == store_a),
            "all rows should belong to Store A"
        );
        // Store A has 2 lots (future + expired)
        assert_eq!(rows.len(), 2, "Store A has 2 active lots");

        let filters_b = DashboardFilters {
            store_id: Some(store_b.clone()),
            location_id: None,
            preset: None,
            urgency: None,
            category_ids: None,
        };
        let rows_b = super::list_dashboard_lots(&pool, &filters_b).await?;
        assert_eq!(rows_b.len(), 1, "Store B has 1 active lot (today)");
        Ok(())
    }

    #[tokio::test]
    async fn list_joins_store_and_location_names() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, _store_b) = seed_schema(&pool).await?;

        let filters = DashboardFilters {
            store_id: Some(store_a),
            location_id: None,
            preset: None,
            urgency: None,
            category_ids: None,
        };
        let rows = super::list_dashboard_lots(&pool, &filters).await?;
        assert!(
            rows.iter().all(|r| r.store_name == "Store A"),
            "store_name should be resolved"
        );
        // The lot with location has Fridge A; the expired one has no location
        for r in &rows {
            if r.location_id.is_some() {
                assert_eq!(
                    r.location_name.as_deref(),
                    Some("Fridge A"),
                    "location_name should match"
                );
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn resolved_lots_excluded() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let now = Utc::now().to_rfc3339();
        let store_a = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
                 VALUES ($1, 'Store X', 1, $2, $3)",
        )
        .bind(&store_a)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

        let product_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO products (id, sku, description, default_alert_days_before,
                                      is_active, created_at, updated_at)
                 VALUES ($1, 'RES-001', 'Resolved Product', 7, 1, $2, $3)",
        )
        .bind(&product_id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

        let lot_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit,
                                          expiry_date, alert_days_before, status,
                                          created_at, updated_at)
                 VALUES ($1, $2, $3, 1.0, 'kg', '2025-12-31', 7, 'resolved', $4, $5)",
        )
        .bind(&lot_id)
        .bind(&product_id)
        .bind(&store_a)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

        let rows = super::list_dashboard_lots(&pool, &DashboardFilters::default()).await?;
        assert!(
            rows.iter().all(|r| r.status != "resolved"),
            "resolved lots must not appear in dashboard"
        );
        Ok(())
    }

    // ── Category filter (UNCATEGORIZED_SENTINEL handling) ───────────────

    /// Seeds a tiny category-filter fixture:
    ///   - Dairy category
    ///   - Bakery category
    ///   - Tagged product (in Dairy)
    ///   - Untagged product (no junction rows)
    ///   - One lot per product
    async fn seed_category_filter_fixture(
        pool: &SqlitePool,
    ) -> Result<(String, String, String, String), Box<dyn std::error::Error>> {
        use crate::services::products as products_service;

        let now = Utc::now().to_rfc3339();
        let store_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
                 VALUES ($1, 'Filter Store', 1, $2, $3)",
        )
        .bind(&store_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        let dairy = products_service::create_category(
            pool,
            crate::dto::products::CategoryCreate {
                name: "Dairy".into(),
            },
        )
        .await?;
        let bakery = products_service::create_category(
            pool,
            crate::dto::products::CategoryCreate {
                name: "Bakery".into(),
            },
        )
        .await?;

        // Tagged product — in Dairy only.
        let tagged = products_service::create_product(
            pool,
            ProductCreate {
                sku: "CAT-TAGGED".into(),
                description: "Tagged product".into(),
                category_ids: Some(vec![dairy.id.clone()]),
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        // Untagged product — no junction rows.
        let untagged = products_service::create_product(
            pool,
            ProductCreate {
                sku: "CAT-UNTAGGED".into(),
                description: "Untagged product".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        for pid in [&tagged.id, &untagged.id] {
            let lot_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit,
                                              expiry_date, alert_days_before, status,
                                              created_at, updated_at)
                     VALUES ($1, $2, $3, 1.0, 'pcs', '2099-12-31', 7, 'active', $4, $5)",
            )
            .bind(&lot_id)
            .bind(pid)
            .bind(&store_id)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
            // No ledger entries needed — the legacy-compatibility clause
            // (el.store_id = store AND NOT EXISTS movements) handles these lots
            // when no store filter is applied.
        }

        Ok((dairy.id, bakery.id, tagged.id, untagged.id))
    }

    #[tokio::test]
    async fn list_filters_by_real_category_returns_only_tagged_lots(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (dairy_id, _bakery_id, tagged_id, _untagged_id) =
            seed_category_filter_fixture(&pool).await?;

        let filters = DashboardFilters {
            category_ids: Some(vec![dairy_id.clone()]),
            ..Default::default()
        };
        let rows = super::list_dashboard_lots(&pool, &filters).await?;
        assert_eq!(rows.len(), 1, "only the tagged lot should match");
        assert_eq!(rows[0].product_id, tagged_id);
        Ok(())
    }

    #[tokio::test]
    async fn list_filters_by_uncategorized_sentinel_returns_only_untagged_lots(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (_dairy_id, _bakery_id, _tagged_id, untagged_id) =
            seed_category_filter_fixture(&pool).await?;

        let filters = DashboardFilters {
            category_ids: Some(vec![
                crate::dto::products::UNCATEGORIZED_SENTINEL.to_string()
            ]),
            ..Default::default()
        };
        let rows = super::list_dashboard_lots(&pool, &filters).await?;
        assert_eq!(
            rows.len(),
            1,
            "only the untagged lot should match the sentinel"
        );
        assert_eq!(rows[0].product_id, untagged_id);
        Ok(())
    }

    #[tokio::test]
    async fn list_filters_by_real_and_uncategorized_returns_either(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (dairy_id, _bakery_id, tagged_id, untagged_id) =
            seed_category_filter_fixture(&pool).await?;

        let filters = DashboardFilters {
            category_ids: Some(vec![
                dairy_id.clone(),
                crate::dto::products::UNCATEGORIZED_SENTINEL.to_string(),
            ]),
            ..Default::default()
        };
        let rows = super::list_dashboard_lots(&pool, &filters).await?;
        assert_eq!(rows.len(), 2, "tagged + untagged lots should both match");
        let product_ids: std::collections::HashSet<&str> =
            rows.iter().map(|r| r.product_id.as_str()).collect();
        assert!(product_ids.contains(tagged_id.as_str()));
        assert!(product_ids.contains(untagged_id.as_str()));
        Ok(())
    }

    #[tokio::test]
    async fn list_does_not_bind_sentinel_as_real_category_id(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (_dairy_id, _bakery_id, tagged_id, _untagged_id) =
            seed_category_filter_fixture(&pool).await?;

        // Sentinel-only filter must not raise a sqlx binding error and must
        // not match any lot whose product has a real category tag.
        let filters = DashboardFilters {
            category_ids: Some(vec![
                crate::dto::products::UNCATEGORIZED_SENTINEL.to_string()
            ]),
            ..Default::default()
        };
        let rows = super::list_dashboard_lots(&pool, &filters).await?;
        assert!(
            rows.iter().all(|r| r.product_id != tagged_id),
            "tagged product must not be returned by sentinel-only filter"
        );
        Ok(())
    }

    #[tokio::test]
    async fn list_any_of_multiple_real_categories_returns_union(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (dairy_id, bakery_id, tagged_id, _untagged_id) =
            seed_category_filter_fixture(&pool).await?;

        // Both real categories selected — tagged lot belongs to Dairy, so it
        // should appear even though Bakery has no tagged products.
        let filters = DashboardFilters {
            category_ids: Some(vec![dairy_id.clone(), bakery_id.clone()]),
            ..Default::default()
        };
        let rows = super::list_dashboard_lots(&pool, &filters).await?;
        assert_eq!(rows.len(), 1, "only the Dairy-tagged lot matches");
        assert_eq!(rows[0].product_id, tagged_id);
        Ok(())
    }
}
