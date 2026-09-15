//! Repository for dashboard queries: fetching active lots with enriched
//! product/store/location context for the urgency cards and lot table.
//!
//! No business logic here — only SQL, projection, and optional filters.

use sqlx::SqlitePool;

use crate::dto::dashboard::{DashboardFilters, DashboardLotRow};
use crate::dto::products::UNCATEGORIZED_SENTINEL;
// sqlx needs this in scope to find UnitKind's Decode impl.

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
    let mut query =
            "SELECT\n                el.id             AS lot_id,\n                el.product_id,\n                p.sku,\n                p.description,\n                el.store_id,\n                s.name            AS store_name,\n                el.location_id,\n                sl.name           AS location_name,\n                el.quantity,\n                el.unit,\n                el.expiry_date,\n                el.alert_days_before,\n                el.batch_code,\n                el.status,\n                ''                AS urgency,\n                0                 AS days_remaining,\n                p.default_unit_id,\n                p.unit_type\n            FROM expiry_lots AS el\n            JOIN products    AS p  ON p.id = el.product_id\n            JOIN stores      AS s  ON s.id = el.store_id\n            LEFT JOIN store_locations AS sl ON sl.id = el.location_id\n            WHERE el.status = 'active'\n              AND (? IS NULL OR el.store_id = ?)\n              AND (? IS NULL OR el.location_id = ?)\n            "
            .to_string();

    // Category filter with sentinel-aware ANY-of semantics.
    // - The sentinel (`UNCATEGORIZED_SENTINEL`) is filter-only and means
    //   "products that have NO active category memberships".
    // - Real ids use a single `EXISTS` over `product_categories` so we get
    //   ANY-of without a nested `IN (?, ?, ...)`.
    // - When both are present we OR them: products matching the real ids OR
    //   products with no junction rows at all.
    // - Sentinel-only also matches products with zero junction rows.
    if let Some(ref cat_ids) = filters.category_ids {
        if !cat_ids.is_empty() {
            let real_ids: Vec<&String> = cat_ids
                .iter()
                .filter(|id| id.as_str() != UNCATEGORIZED_SENTINEL)
                .collect();
            let include_uncategorized = cat_ids.iter().any(|id| id == UNCATEGORIZED_SENTINEL);

            match (real_ids.is_empty(), include_uncategorized) {
                (true, true) => {
                    // Only the sentinel: every active lot whose product has
                    // no junction rows.
                    query.push_str(
"              AND NOT EXISTS (\n                  SELECT 1 FROM product_categories pc\n                  WHERE pc.product_id = p.id\n              )\n            ",
                    );
                }
                (false, false) => {
                    // Only real ids: ANY-of via EXISTS over the junction.
                    let placeholders: Vec<&str> = real_ids.iter().map(|_| "?").collect();
                    query.push_str(&format!(
"              AND EXISTS (\n                  SELECT 1 FROM product_categories pc\n                  WHERE pc.product_id = p.id\n                    AND pc.category_id IN ({})\n              )\n            ",
placeholders.join(", ")
                    ));
                }
                (false, true) => {
                    // Real ids + sentinel: ANY-of real ids OR no junction rows.
                    let placeholders: Vec<&str> = real_ids.iter().map(|_| "?").collect();
                    query.push_str(&format!(
"              AND (\n                  EXISTS (\n                      SELECT 1 FROM product_categories pc\n                      WHERE pc.product_id = p.id\n                        AND pc.category_id IN ({})\n                  )\n                  OR NOT EXISTS (\n                      SELECT 1 FROM product_categories pc\n                      WHERE pc.product_id = p.id\n                  )\n              )\n            ",
placeholders.join(", ")
                    ));
                }
                (true, false) => {
                    // Empty after filtering out sentinel: treat as no filter.
                }
            }
        }
    }

    query.push_str("            ORDER BY el.expiry_date ASC\n            ");

    let mut q = sqlx::query_as::<_, DashboardLotRow>(&query)
        .bind(&filters.store_id)
        .bind(&filters.store_id)
        .bind(&filters.location_id)
        .bind(&filters.location_id);

    if let Some(ref cat_ids) = filters.category_ids {
        // Only real ids are bound; sentinel is filter-only.
        for cid in cat_ids {
            if cid != UNCATEGORIZED_SENTINEL {
                q = q.bind(cid);
            }
        }
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
        let (_dairy_id, _bakery_id, _tagged_id, _untagged_id) =
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
            rows.iter().all(|r| r.product_id != _tagged_id),
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
