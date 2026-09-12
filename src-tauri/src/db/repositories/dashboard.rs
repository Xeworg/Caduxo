//! Repository for dashboard queries: fetching active lots with enriched
//! product/store/location context for the urgency cards and lot table.
//!
//! No business logic here — only SQL, projection, and optional filters.

use sqlx::SqlitePool;

use crate::dto::dashboard::{DashboardFilters, DashboardLotRow};

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
    let rows = sqlx::query_as::<_, DashboardLotRow>(
        r#"
        SELECT
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
            0                 AS days_remaining
        FROM expiry_lots AS el
        JOIN products    AS p  ON p.id = el.product_id
        JOIN stores      AS s  ON s.id = el.store_id
        LEFT JOIN store_locations AS sl ON sl.id = el.location_id
        WHERE el.status = 'active'
          AND (? IS NULL OR el.store_id = ?)
          AND (? IS NULL OR el.location_id = ?)
        ORDER BY el.expiry_date ASC
        "#,
    )
    .bind(&filters.store_id)
    .bind(&filters.store_id)
    .bind(&filters.location_id)
    .bind(&filters.location_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use crate::db::migrations::fresh_test_pool;
    use crate::dto::dashboard::DashboardFilters;

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
    async fn list_filters_by_store_id() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_a, store_b) = seed_schema(&pool).await?;

        let filters = DashboardFilters {
            store_id: Some(store_a.clone()),
            location_id: None,
            preset: None,
            urgency: None,
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
}
