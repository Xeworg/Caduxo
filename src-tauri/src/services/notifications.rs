//! Local notification service — finds due notification candidates and marks
//! them as shown via `notification_log`.
//!
//! Business rules:
//!   1. Only `active` lots are candidates.
//!   2. A candidate is due when `today` falls within the inclusive alert window
//!      `[expiry_date - alert_days_before, expiry_date]`. `alert_days_before = 0`
//!      reduces the window to a single day (today, which must equal expiry).
//!   3. Expired lots (`expiry_date < today`) are excluded.
//!   4. Lots already logged for `notification_date` are excluded so we don't
//!      notify the same lot twice on the same day.
//!   5. `mark_notification_shown` is idempotent for the same (lot, date).
//!   6. Marking a non-existent lot returns `DomainError::NotFound`.
//!
//! The repository owns the SQL; this layer owns date validation, defaulting
//! `notification_date` to today when omitted, and lot-existence checks.

use chrono::{NaiveDate, Utc};

use crate::db::repositories::expiry_lots as lots_repo;
use crate::db::repositories::notification_log as repo;
use crate::db::DbPool;
use crate::dto::notifications::{
    DueNotificationLot, MarkNotificationShownInput, NotificationLogResponse,
};
use crate::error::{AppError, DomainError};

// ============================================================
// Helpers
// ============================================================

/// Validates an ISO-8601 date string (YYYY-MM-DD). Returns `Ok(())` when valid,
/// otherwise a `Validation` error mentioning which field failed.
fn parse_strict_date(label: &str, value: &str) -> Result<(), DomainError> {
    let has_strict_shape = value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(idx, byte)| matches!(idx, 4 | 7) || byte.is_ascii_digit());
    if !has_strict_shape {
        return Err(DomainError::Validation {
            message: format!("Invalid {label} format: `{value}` (expected YYYY-MM-DD)"),
        });
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| DomainError::Validation {
            message: format!("Invalid {label} format: `{value}` (expected YYYY-MM-DD)"),
        })
}

/// Returns today's date as a YYYY-MM-DD string (UTC).
fn today_str() -> String {
    Utc::now().date_naive().format("%Y-%m-%d").to_string()
}

// ============================================================
// Use cases
// ============================================================

/// Returns active lots whose alert window contains `today`, enriched with
/// product/store/location context, excluding any lot already logged for
/// `today`. When `today` is `None`, the current UTC date is used.
///
/// `today` is validated as a strict YYYY-MM-DD date.
pub async fn list_due_notifications(
    pool: &DbPool,
    today: Option<String>,
) -> Result<Vec<DueNotificationLot>, AppError> {
    let today = match today {
        Some(s) if !s.trim().is_empty() => s,
        _ => today_str(),
    };
    parse_strict_date("notification_date", &today).map_err(AppError::Domain)?;
    repo::list_due_notification_lots(pool, &today)
        .await
        .map_err(AppError::from)
}

/// Records `notification_log(expiry_lot_id, notification_date)` so that the
/// lot is not surfaced again for the same day. Idempotent for the same
/// `(lot, date)` pair.
///
/// Defaults `notification_date` to today (UTC) when omitted. The lot must
/// exist in `expiry_lots` — unknown ids return `DomainError::NotFound`.
pub async fn mark_notification_shown(
    pool: &DbPool,
    input: MarkNotificationShownInput,
) -> Result<NotificationLogResponse, AppError> {
    // 1. Validate the lot exists. (We do not check status here — even resolved
    //    or archived lots that were just shown on a previous attempt may
    //    legitimately need their log row written. This keeps the contract
    //    idempotent.)
    let _lot = lots_repo::get_expiry_lot(pool, &input.expiry_lot_id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "expiry_lot",
            id: input.expiry_lot_id.clone(),
        })?;

    // 2. Resolve + validate date.
    let notification_date = match input.notification_date.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => today_str(),
    };
    parse_strict_date("notification_date", &notification_date).map_err(AppError::Domain)?;

    // 3. Idempotent upsert.
    let now = Utc::now().to_rfc3339();
    repo::upsert_notification_log(pool, &input.expiry_lot_id, &notification_date, &now)
        .await
        .map_err(AppError::from)
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
    use crate::dto::notifications::MarkNotificationShownInput;
    use crate::dto::products::ProductCreate;
    use crate::dto::stores::StoreCreate;
    use crate::services::notifications as svc;
    use crate::services::products::create_product as create_product_svc;
    use crate::services::stores::create_store as create_store_svc;

    async fn seed_product_and_store(
        pool: &crate::db::DbPool,
    ) -> Result<(String, String, String), Box<dyn std::error::Error>> {
        let store = create_store_svc(
            pool,
            StoreCreate {
                name: "S".into(),
                code: None,
                notes: None,
            },
        )
        .await?;
        let location_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at)
             VALUES ($1, $2, 'Fridge', 1, $3, $4)",
        )
        .bind(&location_id)
        .bind(&store.id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        let product = create_product_svc(
            pool,
            ProductCreate {
                sku: "NOTIF-001".into(),
                description: "Test Product".into(),
                category_id: None,
                default_unit: Some("pcs".into()),
                default_unit_id: None,
                default_alert_days_before: 7,
                notes: None,
            },
        )
        .await?;
        Ok((store.id, location_id, product.id))
    }

    async fn make_lot(
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

    // --- list_due_notifications ------------------------------------------------

    #[tokio::test]
    async fn list_due_returns_active_lot_in_window() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, location_id, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id = make_lot(
            &pool,
            &product_id,
            &store_id,
            Some(&location_id),
            &today,
            7,
            "active",
        )
        .await?;

        let rows = svc::list_due_notifications(&pool, None).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].lot_id, lot_id);
        assert_eq!(rows[0].notification_date, today);
        assert_eq!(rows[0].location_name.as_deref(), Some("Fridge"));
        Ok(())
    }

    #[tokio::test]
    async fn list_due_accepts_explicit_today() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id = make_lot(&pool, &product_id, &store_id, None, &today, 7, "active").await?;

        // Explicit today equivalent to None.
        let rows_explicit = svc::list_due_notifications(&pool, Some(today.clone())).await?;
        let rows_implicit = svc::list_due_notifications(&pool, None).await?;
        assert_eq!(rows_explicit.len(), 1);
        assert_eq!(rows_explicit[0].lot_id, lot_id);
        // Both calls should match; an empty-string is treated as None.
        let rows_blank = svc::list_due_notifications(&pool, Some("   ".into())).await?;
        assert_eq!(rows_blank.len(), 1);
        assert_eq!(rows_implicit.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn list_due_rejects_malformed_date() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        for bad in ["not-a-date", "2025/06/30", "25-06-30", "2025-6-30"] {
            let err = svc::list_due_notifications(&pool, Some(bad.into()))
                .await
                .expect_err(&format!("malformed date `{bad}` must be rejected"));
            assert!(matches!(
                err,
                crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
            ));
        }
        Ok(())
    }

    #[tokio::test]
    async fn list_due_excludes_expired_lots() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let yesterday = (Utc::now().date_naive() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        make_lot(
            &pool,
            &product_id,
            &store_id,
            None,
            &yesterday,
            30,
            "active",
        )
        .await?;

        let rows = svc::list_due_notifications(&pool, None).await?;
        assert!(rows.is_empty(), "yesterday expiry is expired → excluded");
        Ok(())
    }

    #[tokio::test]
    async fn list_due_excludes_non_active_lots() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();

        make_lot(&pool, &product_id, &store_id, None, &today, 7, "resolved").await?;
        make_lot(&pool, &product_id, &store_id, None, &today, 7, "archived").await?;

        let rows = svc::list_due_notifications(&pool, None).await?;
        assert!(rows.is_empty(), "resolved/archived lots are not candidates");
        Ok(())
    }

    #[tokio::test]
    async fn list_due_excludes_lot_already_marked_for_today(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id = make_lot(&pool, &product_id, &store_id, None, &today, 7, "active").await?;

        // First list: lot is present.
        let first = svc::list_due_notifications(&pool, None).await?;
        assert_eq!(first.len(), 1);

        // Mark shown → second list is empty.
        svc::mark_notification_shown(
            &pool,
            MarkNotificationShownInput {
                expiry_lot_id: lot_id.clone(),
                notification_date: Some(today.clone()),
            },
        )
        .await?;

        let second = svc::list_due_notifications(&pool, None).await?;
        assert!(
            second.is_empty(),
            "after mark_notification_shown, lot must not appear again"
        );
        Ok(())
    }

    #[tokio::test]
    async fn list_due_with_alert_days_zero_only_today() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let tomorrow = (Utc::now().date_naive() + chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        let lot_today = make_lot(&pool, &product_id, &store_id, None, &today, 0, "active").await?;
        make_lot(&pool, &product_id, &store_id, None, &tomorrow, 0, "active").await?;

        let rows = svc::list_due_notifications(&pool, None).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].lot_id, lot_today);
        Ok(())
    }

    #[tokio::test]
    async fn list_due_window_within_alert_days_includes_lot(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let five_days = (Utc::now().date_naive() + chrono::Duration::days(5))
            .format("%Y-%m-%d")
            .to_string();

        let lot = make_lot(
            &pool,
            &product_id,
            &store_id,
            None,
            &five_days,
            14,
            "active",
        )
        .await?;

        let rows = svc::list_due_notifications(&pool, None).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].lot_id, lot);
        Ok(())
    }

    #[tokio::test]
    async fn list_due_window_outside_alert_days_excludes_lot(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let ten_days = (Utc::now().date_naive() + chrono::Duration::days(10))
            .format("%Y-%m-%d")
            .to_string();

        // alert window = 7 days, expiry in 10 days → out of window.
        make_lot(&pool, &product_id, &store_id, None, &ten_days, 7, "active").await?;

        let rows = svc::list_due_notifications(&pool, None).await?;
        assert!(rows.is_empty());
        Ok(())
    }

    // --- mark_notification_shown ----------------------------------------------

    #[tokio::test]
    async fn mark_shown_records_log() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id = make_lot(&pool, &product_id, &store_id, None, &today, 7, "active").await?;

        let resp = svc::mark_notification_shown(
            &pool,
            MarkNotificationShownInput {
                expiry_lot_id: lot_id.clone(),
                notification_date: Some(today.clone()),
            },
        )
        .await?;

        assert_eq!(resp.expiry_lot_id, lot_id);
        assert_eq!(resp.notification_date, today);
        assert!(!resp.shown_at.is_empty());

        // The row must exist in notification_log.
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM notification_log WHERE expiry_lot_id = ?")
                .bind(&lot_id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(count.0, 1);
        Ok(())
    }

    #[tokio::test]
    async fn mark_shown_defaults_to_today() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id = make_lot(&pool, &product_id, &store_id, None, &today, 7, "active").await?;

        let resp = svc::mark_notification_shown(
            &pool,
            MarkNotificationShownInput {
                expiry_lot_id: lot_id.clone(),
                notification_date: None,
            },
        )
        .await?;
        assert_eq!(resp.notification_date, today);
        Ok(())
    }

    #[tokio::test]
    async fn mark_shown_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id = make_lot(&pool, &product_id, &store_id, None, &today, 7, "active").await?;

        let first = svc::mark_notification_shown(
            &pool,
            MarkNotificationShownInput {
                expiry_lot_id: lot_id.clone(),
                notification_date: Some(today.clone()),
            },
        )
        .await?;
        let second = svc::mark_notification_shown(
            &pool,
            MarkNotificationShownInput {
                expiry_lot_id: lot_id.clone(),
                notification_date: Some(today.clone()),
            },
        )
        .await?;
        // Same row returned both times — same id, same first shown_at.
        assert_eq!(first.id, second.id);
        assert_eq!(first.shown_at, second.shown_at);

        // Only one row persisted.
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM notification_log WHERE expiry_lot_id = ?")
                .bind(&lot_id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(count.0, 1);
        Ok(())
    }

    #[tokio::test]
    async fn mark_shown_unknown_lot_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let today = today_str();
        let err = svc::mark_notification_shown(
            &pool,
            MarkNotificationShownInput {
                expiry_lot_id: "no-such-lot".into(),
                notification_date: Some(today),
            },
        )
        .await
        .expect_err("unknown lot must return NotFound");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::NotFound { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn mark_shown_rejects_malformed_date() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let lot_id = make_lot(&pool, &product_id, &store_id, None, &today, 7, "active").await?;

        let err = svc::mark_notification_shown(
            &pool,
            MarkNotificationShownInput {
                expiry_lot_id: lot_id,
                notification_date: Some("definitely-not-a-date".into()),
            },
        )
        .await
        .expect_err("malformed date must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn mark_shown_allows_different_date_after_dedup() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();
        let yesterday = (Utc::now().date_naive() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let lot_id = make_lot(&pool, &product_id, &store_id, None, &today, 7, "active").await?;

        svc::mark_notification_shown(
            &pool,
            MarkNotificationShownInput {
                expiry_lot_id: lot_id.clone(),
                notification_date: Some(today.clone()),
            },
        )
        .await?;

        // Marking same lot for a different date must succeed and produce a distinct row.
        let resp = svc::mark_notification_shown(
            &pool,
            MarkNotificationShownInput {
                expiry_lot_id: lot_id.clone(),
                notification_date: Some(yesterday.clone()),
            },
        )
        .await?;
        assert_eq!(resp.notification_date, yesterday);

        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM notification_log WHERE expiry_lot_id = ?")
                .bind(&lot_id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(count.0, 2, "two distinct dates → two log rows");
        Ok(())
    }

    #[tokio::test]
    async fn mark_shown_works_for_archived_or_resolved_lot(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, _, product_id) = seed_product_and_store(&pool).await?;
        let today = today_str();

        // Both non-active lots still exist in the table — marking must succeed.
        let archived = make_lot(&pool, &product_id, &store_id, None, &today, 7, "archived").await?;
        let resolved = make_lot(&pool, &product_id, &store_id, None, &today, 7, "resolved").await?;

        for lot_id in [archived, resolved] {
            let resp = svc::mark_notification_shown(
                &pool,
                MarkNotificationShownInput {
                    expiry_lot_id: lot_id.clone(),
                    notification_date: Some(today.clone()),
                },
            )
            .await?;
            assert_eq!(resp.expiry_lot_id, lot_id);
        }
        Ok(())
    }
}
