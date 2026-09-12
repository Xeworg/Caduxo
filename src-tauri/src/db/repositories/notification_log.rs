//! Repository for `notification_log` writes and the due-notification query.
//!
//! The due-notification query implements the alert-window rules:
//!   - Only `status = 'active'` lots are candidates.
//!   - The lot must not be expired (`expiry_date >= today`).
//!   - Today must fall within the inclusive alert window
//!     `[expiry_date - alert_days_before, expiry_date]`.
//!   - Lots already present in `notification_log` for `today` are excluded so
//!     we never notify the same lot twice on the same day.
//!
//! `alert_days_before = 0` reduces the window to a single day (`today`),
//! which is the correct "due only on expiry day" behavior.

use sqlx::SqlitePool;

use crate::dto::notifications::{DueNotificationLot, NotificationLogResponse};

// ============================================================
// Due notification candidate query
// ============================================================

/// Returns active lots whose alert window contains `today`, enriched with
/// product and store context, and excluding any lot already logged for
/// `today` in `notification_log`.
///
/// `today` MUST be a YYYY-MM-DD date string. Rows are ordered by
/// `expiry_date ASC` (most urgent first).
pub async fn list_due_notification_lots(
    pool: &SqlitePool,
    today: &str,
) -> Result<Vec<DueNotificationLot>, sqlx::Error> {
    sqlx::query_as::<_, DueNotificationLot>(
        r#"
        SELECT
            el.id         AS lot_id,
            el.product_id,
            p.sku         AS sku,
            p.description AS description,
            el.store_id,
            s.name        AS store_name,
            el.location_id,
            sl.name       AS location_name,
            el.quantity,
            el.unit,
            el.expiry_date,
            el.alert_days_before,
            $1            AS notification_date
        FROM expiry_lots AS el
        JOIN products    AS p  ON p.id = el.product_id
        JOIN stores      AS s  ON s.id = el.store_id
        LEFT JOIN store_locations AS sl ON sl.id = el.location_id
        WHERE el.status = 'active'
          AND el.expiry_date >= $1
          AND el.expiry_date <= date($1, '+' || el.alert_days_before || ' days')
          AND NOT EXISTS (
              SELECT 1
              FROM notification_log nl
              WHERE nl.expiry_lot_id = el.id
                AND nl.notification_date = $1
          )
        ORDER BY el.expiry_date ASC, el.created_at ASC
        "#,
    )
    .bind(today)
    .fetch_all(pool)
    .await
}

// ============================================================
// notification_log read + idempotent upsert
// ============================================================

/// Fetches a single `notification_log` row by lot+date, or `None`.
pub async fn get_notification_log(
    pool: &SqlitePool,
    expiry_lot_id: &str,
    notification_date: &str,
) -> Result<Option<NotificationLogResponse>, sqlx::Error> {
    sqlx::query_as::<_, NotificationLogResponse>(
        r#"
        SELECT id, expiry_lot_id, notification_date, shown_at
        FROM notification_log
        WHERE expiry_lot_id = $1 AND notification_date = $2
        "#,
    )
    .bind(expiry_lot_id)
    .bind(notification_date)
    .fetch_optional(pool)
    .await
}

/// Inserts a `notification_log` row for `(expiry_lot_id, notification_date)`.
/// If a row already exists (UNIQUE constraint), this is a no-op and the
/// existing row is returned — the call is fully idempotent.
///
/// Uses a deterministic id of the form `{expiry_lot_id}|{notification_date}`,
/// which keeps `INSERT OR IGNORE` semantically clean (one row per lot+date,
/// always the same id) and avoids any need for a follow-up SELECT to detect
/// duplicates.
pub async fn upsert_notification_log(
    pool: &SqlitePool,
    expiry_lot_id: &str,
    notification_date: &str,
    shown_at: &str,
) -> Result<NotificationLogResponse, sqlx::Error> {
    let deterministic_id = format!("{}|{}", expiry_lot_id, notification_date);

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO notification_log
            (id, expiry_lot_id, notification_date, shown_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(&deterministic_id)
    .bind(expiry_lot_id)
    .bind(notification_date)
    .bind(shown_at)
    .execute(pool)
    .await?;

    get_notification_log(pool, expiry_lot_id, notification_date)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use crate::db::migrations::fresh_test_pool;

    /// Seeds one store, one location, one product, and returns
    /// `(store_id, location_id, product_id)`.
    async fn seed_product_and_store(
        pool: &SqlitePool,
    ) -> Result<(String, String, String), sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        let store_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
             VALUES ($1, 'S', 1, $2, $3)",
        )
        .bind(&store_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        let location_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at)
             VALUES ($1, $2, 'Fridge', 1, $3, $4)",
        )
        .bind(&location_id)
        .bind(&store_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        let product_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO products (id, sku, description, default_alert_days_before,
                                   is_active, created_at, updated_at)
             VALUES ($1, 'NOTIF-001', 'Test Product', 7, 1, $2, $3)",
        )
        .bind(&product_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok((store_id, location_id, product_id))
    }

    /// Inserts an active expiry lot with the given parameters.
    #[allow(clippy::too_many_arguments)]
    async fn insert_active_lot(
        pool: &SqlitePool,
        product_id: &str,
        store_id: &str,
        location_id: Option<&str>,
        expiry_date: &str,
        alert_days_before: i32,
        status: &str,
    ) -> Result<String, sqlx::Error> {
        let lot_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO expiry_lots
                 (id, product_id, store_id, location_id, quantity, unit,
                  expiry_date, alert_days_before, status,
                  created_at, updated_at)
             VALUES ($1, $2, $3, $4, 1.0, 'pcs', $5, $6, $7, $8, $9)",
        )
        .bind(&lot_id)
        .bind(product_id)
        .bind(store_id)
        .bind(location_id)
        .bind(expiry_date)
        .bind(alert_days_before)
        .bind(status)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(lot_id)
    }

    fn today_str() -> String {
        Utc::now().date_naive().format("%Y-%m-%d").to_string()
    }

    // --- list_due_notification_lots --------------------------------------

    #[tokio::test]
    async fn list_includes_lot_with_today_expiry() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, location_id, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id = insert_active_lot(
            &pool,
            &product_id,
            &store_id,
            Some(&location_id),
            &today,
            7,
            "active",
        )
        .await?;

        let rows = super::list_due_notification_lots(&pool, &today).await?;
        assert_eq!(rows.len(), 1, "today-expiry active lot must be a candidate");
        assert_eq!(rows[0].lot_id, lot_id);
        assert_eq!(rows[0].notification_date, today);
        assert_eq!(rows[0].sku, "NOTIF-001");
        assert_eq!(rows[0].store_name, "S");
        assert_eq!(rows[0].location_name.as_deref(), Some("Fridge"));
        Ok(())
    }

    #[tokio::test]
    async fn list_includes_lot_within_alert_window() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let five_days_later = (Utc::now().date_naive() + chrono::Duration::days(5))
            .format("%Y-%m-%d")
            .to_string();

        let lot_id = insert_active_lot(
            &pool,
            &product_id,
            &store_id,
            None,
            &five_days_later,
            14,
            "active",
        )
        .await?;

        let rows = super::list_due_notification_lots(&pool, &today).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].lot_id, lot_id,
            "5 days out w/ alert_days=14 is in window"
        );
        Ok(())
    }

    #[tokio::test]
    async fn list_excludes_lot_outside_alert_window() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let far_future = (Utc::now().date_naive() + chrono::Duration::days(60))
            .format("%Y-%m-%d")
            .to_string();

        insert_active_lot(
            &pool,
            &product_id,
            &store_id,
            None,
            &far_future,
            14,
            "active",
        )
        .await?;

        let rows = super::list_due_notification_lots(&pool, &today).await?;
        assert!(
            rows.is_empty(),
            "60 days out w/ alert_days=14 is outside window"
        );
        Ok(())
    }

    #[tokio::test]
    async fn list_excludes_expired_lots() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let yesterday = (Utc::now().date_naive() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let long_ago = (Utc::now().date_naive() - chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();

        // expired yesterday — must not appear even with alert_days_before=30
        insert_active_lot(
            &pool,
            &product_id,
            &store_id,
            None,
            &yesterday,
            30,
            "active",
        )
        .await?;
        // expired 30 days ago — must not appear
        insert_active_lot(&pool, &product_id, &store_id, None, &long_ago, 90, "active").await?;

        let rows = super::list_due_notification_lots(&pool, &today).await?;
        assert!(
            rows.is_empty(),
            "expired lots must never be notification candidates"
        );
        Ok(())
    }

    #[tokio::test]
    async fn list_excludes_resolved_and_archived_lots() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();

        insert_active_lot(&pool, &product_id, &store_id, None, &today, 7, "resolved").await?;
        insert_active_lot(&pool, &product_id, &store_id, None, &today, 7, "archived").await?;

        let rows = super::list_due_notification_lots(&pool, &today).await?;
        assert!(
            rows.is_empty(),
            "non-active lots must never be notification candidates"
        );
        Ok(())
    }

    #[tokio::test]
    async fn list_alert_days_zero_only_includes_today_expiry(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let tomorrow = (Utc::now().date_naive() + chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        // tomorrow + alert_days_before=0 → window is [tomorrow, tomorrow] = no match
        insert_active_lot(&pool, &product_id, &store_id, None, &tomorrow, 0, "active").await?;
        // today + alert_days_before=0 → window is [today, today] = match
        let lot_today =
            insert_active_lot(&pool, &product_id, &store_id, None, &today, 0, "active").await?;

        let rows = super::list_due_notification_lots(&pool, &today).await?;
        assert_eq!(
            rows.len(),
            1,
            "only today expiry must match with alert_days=0"
        );
        assert_eq!(rows[0].lot_id, lot_today);
        Ok(())
    }

    #[tokio::test]
    async fn list_excludes_lot_already_in_notification_log_today(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id =
            insert_active_lot(&pool, &product_id, &store_id, None, &today, 7, "active").await?;

        // First call: includes the lot.
        let rows = super::list_due_notification_lots(&pool, &today).await?;
        assert_eq!(rows.len(), 1);

        // Mark shown → second call must exclude it.
        super::upsert_notification_log(&pool, &lot_id, &today, &Utc::now().to_rfc3339()).await?;

        let rows = super::list_due_notification_lots(&pool, &today).await?;
        assert!(
            rows.is_empty(),
            "lot already logged for today must not appear again"
        );
        Ok(())
    }

    #[tokio::test]
    async fn list_includes_lot_logged_on_different_date() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id =
            insert_active_lot(&pool, &product_id, &store_id, None, &today, 7, "active").await?;

        let yesterday = (Utc::now().date_naive() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        super::upsert_notification_log(&pool, &lot_id, &yesterday, &Utc::now().to_rfc3339())
            .await?;

        let rows = super::list_due_notification_lots(&pool, &today).await?;
        assert_eq!(
            rows.len(),
            1,
            "yesterday's log row must not block today's notification"
        );
        Ok(())
    }

    // --- upsert_notification_log -----------------------------------------

    #[tokio::test]
    async fn upsert_notification_log_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id =
            insert_active_lot(&pool, &product_id, &store_id, None, &today, 7, "active").await?;

        // First call: inserts.
        let first =
            super::upsert_notification_log(&pool, &lot_id, &today, "2025-06-30T12:00:00Z").await?;
        assert_eq!(first.expiry_lot_id, lot_id);
        assert_eq!(first.notification_date, today);
        assert_eq!(first.shown_at, "2025-06-30T12:00:00Z");

        // Second call: must not fail and must return the SAME existing row.
        let second = super::upsert_notification_log(
            &pool,
            &lot_id,
            &today,
            "2099-01-01T00:00:00Z", // different timestamp
        )
        .await?;
        assert_eq!(second.id, first.id, "id must be stable across calls");
        assert_eq!(
            second.shown_at, first.shown_at,
            "first shown_at must win — log is not overwritten"
        );

        // Exactly one row in the table.
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM notification_log WHERE expiry_lot_id = ?")
                .bind(&lot_id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(count.0, 1);
        Ok(())
    }
}
