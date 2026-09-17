//! Repository for app_settings key-value persistence.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::dto::stores::SettingsResponse;

/// Retrieves a single setting value by key, or None if absent.
pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

/// Retrieves the last selected store id from settings.
pub async fn get_last_selected_store_id(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    get_setting(pool, "last_selected_store_id").await
}

/// Retrieves the require_initial_location_on_lot_create setting.
/// Returns true if absent (default).
pub async fn get_require_initial_location_on_lot_create(
    pool: &SqlitePool,
) -> Result<bool, sqlx::Error> {
    let val = get_setting(pool, "require_initial_location_on_lot_create").await?;
    Ok(val.as_deref() != Some("0"))
}

/// Persists the require_initial_location_on_lot_create setting.
pub async fn set_require_initial_location_on_lot_create(
    pool: &SqlitePool,
    value: bool,
) -> Result<(), sqlx::Error> {
    upsert_setting(
        pool,
        "require_initial_location_on_lot_create",
        if value { "1" } else { "0" },
    )
    .await
}

/// Upserts a setting value (insert-or-replace semantics).
pub async fn upsert_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO app_settings (key, value, updated_at) VALUES ($1, $2, $3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(value)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Persists the last-selected store id (None clears the setting).
pub async fn set_last_selected_store_id(
    pool: &SqlitePool,
    store_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    match store_id {
        Some(id) => upsert_setting(pool, "last_selected_store_id", id).await,
        None => {
            sqlx::query("DELETE FROM app_settings WHERE key = 'last_selected_store_id'")
                .execute(pool)
                .await?;
            Ok(())
        }
    }
}

/// Builds the full settings snapshot.
pub async fn get_settings(pool: &SqlitePool) -> Result<SettingsResponse, sqlx::Error> {
    let last_selected_store_id = get_last_selected_store_id(pool).await?;
    let require_initial_location_on_lot_create =
        get_require_initial_location_on_lot_create(pool).await?;
    Ok(SettingsResponse {
        last_selected_store_id,
        require_initial_location_on_lot_create,
    })
}

#[cfg(test)]
mod tests {
    // NOTE: no `use super::*` — avoids bringing the sibling `stores` module
    // into scope, which would shadow local repository functions.
    use crate::db::migrations::fresh_test_pool;

    use super::{
        get_last_selected_store_id, get_setting, get_settings, set_last_selected_store_id,
        upsert_setting,
    };

    #[tokio::test]
    async fn get_missing_setting_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let val = get_setting(&pool, "nonexistent").await?;
        assert!(val.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn upsert_and_get_setting() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        upsert_setting(&pool, "theme", "dark").await?;

        let val = get_setting(&pool, "theme").await?;
        assert_eq!(val, Some("dark".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn upsert_replaces_value() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        upsert_setting(&pool, "theme", "dark").await?;
        upsert_setting(&pool, "theme", "light").await?;

        let val = get_setting(&pool, "theme").await?;
        assert_eq!(val, Some("light".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn settings_set_last_selected_store_id() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Initially none
        assert!(get_last_selected_store_id(&pool).await?.is_none());

        // Set
        set_last_selected_store_id(&pool, Some("store-123")).await?;
        assert_eq!(
            get_last_selected_store_id(&pool).await?,
            Some("store-123".to_string())
        );

        // Replace
        set_last_selected_store_id(&pool, Some("store-456")).await?;
        assert_eq!(
            get_last_selected_store_id(&pool).await?,
            Some("store-456".to_string())
        );

        // Clear
        set_last_selected_store_id(&pool, None).await?;
        assert!(get_last_selected_store_id(&pool).await?.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        upsert_setting(&pool, "last_selected_store_id", "store-abc").await?;

        let settings = get_settings(&pool).await?;
        assert_eq!(
            settings.last_selected_store_id,
            Some("store-abc".to_string())
        );
        Ok(())
    }
}
