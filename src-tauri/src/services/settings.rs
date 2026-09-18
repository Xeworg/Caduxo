//! Settings management service.

use crate::db::repositories::settings as repo;
use crate::db::DbPool;
use crate::dto::stores::{SettingsResponse, SettingsUpdate};
use crate::error::AppError;

/// Returns the full settings snapshot.
pub async fn get_settings(pool: &DbPool) -> Result<SettingsResponse, AppError> {
    repo::get_settings(pool).await.map_err(AppError::from)
}

/// Updates settings: last_selected_store_id, require_initial_location_on_lot_create,
/// and language.
pub async fn update_settings(
    pool: &DbPool,
    input: SettingsUpdate,
) -> Result<SettingsResponse, AppError> {
    repo::set_last_selected_store_id(pool, input.last_selected_store_id.as_deref())
        .await
        .map_err(AppError::from)?;
    if let Some(value) = input.require_initial_location_on_lot_create {
        repo::set_require_initial_location_on_lot_create(pool, value)
            .await
            .map_err(AppError::from)?;
    }
    if let Some(value) = input.language.as_deref() {
        repo::set_language_setting(pool, value)
            .await
            .map_err(AppError::from)?;
    }
    repo::get_settings(pool).await.map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    // NOTE: avoid `use super::*` — avoid import-shadowing surprises.
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::stores::{SettingsUpdate, StoreCreate};
    use crate::services::settings::{get_settings, update_settings};
    use crate::services::stores::create_store as store_create;

    #[tokio::test]
    async fn get_settings_initially_empty() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let settings = get_settings(&pool).await?;
        assert!(settings.last_selected_store_id.is_none());
        // Default: require_initial_location_on_lot_create = true
        assert!(settings.require_initial_location_on_lot_create);
        Ok(())
    }

    #[tokio::test]
    async fn update_settings_sets_last_store() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        let store = store_create(
            &pool,
            StoreCreate {
                name: "My Shop".into(),
                code: Some("SHOP-01".into()),
                notes: None,
            },
        )
        .await?;

        let updated = update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store.id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
            },
        )
        .await?;
        assert_eq!(updated.last_selected_store_id, Some(store.id));

        let cleared = update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: None,
                require_initial_location_on_lot_create: None,
                language: None,
            },
        )
        .await?;
        assert!(cleared.last_selected_store_id.is_none());
        Ok(())
    }
}
