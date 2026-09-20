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
/// language, and theme.
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
    if let Some(value) = input.theme.as_deref() {
        repo::set_theme_setting(pool, value)
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
        // Default: language is reported as the "en" fallback but NOT
        // configured — the frontend uses the configured flag to decide
        // whether to honour the value or run OS detection.
        assert_eq!(settings.language, "en");
        assert!(!settings.language_configured);
        Ok(())
    }

    #[tokio::test]
    async fn update_settings_persists_language_and_marks_configured(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // After a manual pick, the snapshot must surface both the value and
        // the configured flag so the frontend stops calling detection.
        let updated = update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: None,
                require_initial_location_on_lot_create: None,
                language: Some("es".to_string()),
                theme: None,
            },
        )
        .await?;
        assert_eq!(updated.language, "es");
        assert!(updated.language_configured);

        // And subsequent reads reflect the same state.
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.language, "es");
        assert!(settings.language_configured);
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
                theme: None,
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
                theme: None,
            },
        )
        .await?;
        assert!(cleared.last_selected_store_id.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn update_settings_partial_theme_update_preserves_other_keys(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Seed every other setting first so the partial update can prove
        // it does not touch any sibling key.
        let store = store_create(
            &pool,
            StoreCreate {
                name: "Theme Test Shop".into(),
                code: Some("THEME-01".into()),
                notes: None,
            },
        )
        .await?;
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store.id.clone()),
                require_initial_location_on_lot_create: Some(false),
                language: Some("es".to_string()),
                theme: None,
            },
        )
        .await?;

        // Send a theme-only partial update. Language, store id, and
        // require_initial_location_on_lot_create MUST be preserved.
        //
        // Note: `last_selected_store_id: None` is the existing "clear"
        // semantic — the implementation branch only protects the
        // Option-typed keys (language, theme, require_initial_location_*),
        // mirroring the language pattern. To prove the theme update is
        // partial, we re-supply the store id verbatim so the assertion
        // below confirms it round-trips unchanged.
        let updated = update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store.id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
                theme: Some("dark".to_string()),
            },
        )
        .await?;
        assert_eq!(updated.theme, "dark");
        assert!(updated.theme_configured);
        assert_eq!(updated.last_selected_store_id, Some(store.id.clone()));
        assert!(!updated.require_initial_location_on_lot_create);
        assert_eq!(updated.language, "es");
        assert!(updated.language_configured);

        // Re-read to make sure the partial update persisted the way the
        // snapshot reports.
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.theme, "dark");
        assert!(settings.theme_configured);
        assert_eq!(settings.last_selected_store_id, Some(store.id.clone()));
        assert!(!settings.require_initial_location_on_lot_create);
        assert_eq!(settings.language, "es");
        assert!(settings.language_configured);
        Ok(())
    }
}
