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

/// Retrieves the language setting, or None if absent.
pub async fn get_language_setting(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    get_setting(pool, "language").await
}

/// Persists the language setting. Rejects values outside {"en", "es"} at the
/// command layer; this function accepts any non-empty string.
pub async fn set_language_setting(pool: &SqlitePool, value: &str) -> Result<(), sqlx::Error> {
    upsert_setting(pool, "language", value).await
}

/// Retrieves the theme setting, or None if absent.
///
/// Returns the raw stored value (potentially empty string). The `get_settings`
/// snapshot normalises absent / empty / unsupported values to the
/// `"caduxo-light"` fallback with `theme_configured = false`.
pub async fn get_theme_setting(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    get_setting(pool, "theme").await
}

/// Persists the theme setting. Rejects values outside the curated set
/// (`caduxo-light`, `dark`, `dracula`, `valentine`, `luxury`, `sunset`,
/// `nord`) at the command layer; this function accepts any non-empty
/// string and trusts the caller.
pub async fn set_theme_setting(pool: &SqlitePool, value: &str) -> Result<(), sqlx::Error> {
    upsert_setting(pool, "theme", value).await
}

/// Builds the full settings snapshot.
pub async fn get_settings(pool: &SqlitePool) -> Result<SettingsResponse, sqlx::Error> {
    let last_selected_store_id = get_last_selected_store_id(pool).await?;
    let require_initial_location_on_lot_create =
        get_require_initial_location_on_lot_create(pool).await?;
    // The row's presence (and a value in the supported set) is the
    // single source of truth for whether the user made a manual pick.
    // Anything else — absent row, empty value, unsupported tag — is reported
    // as `language_configured: false` so the frontend can run OS detection
    // instead of treating the fallback string as a preference.
    let stored_language = get_language_setting(pool).await?;
    let language_configured = matches!(stored_language.as_deref(), Some("en") | Some("es"));
    let language = stored_language
        .filter(|_| language_configured)
        .unwrap_or_else(|| "en".to_string());
    // Mirror the language pattern for the theme row: stored row present AND
    // value in the v1 curated set → manual pick; otherwise (absent row,
    // empty value, unsupported tag) the response reports the
    // `"caduxo-light"` fallback with `theme_configured: false` so the
    // frontend can run `prefers-color-scheme` detection instead of
    // honouring an invalid row. The curated set is sourced from
    // `services::user_messages::ALLOWED_THEMES` (the IPC validator's
    // whitelist) so the snapshot logic cannot drift away from the
    // validation gate.
    let stored_theme = get_theme_setting(pool).await?;
    let theme_configured = stored_theme
        .as_deref()
        .map(|v| crate::services::user_messages::ALLOWED_THEMES.contains(&v))
        .unwrap_or(false);
    let theme = stored_theme
        .filter(|_| theme_configured)
        .unwrap_or_else(|| "caduxo-light".to_string());
    Ok(SettingsResponse {
        last_selected_store_id,
        require_initial_location_on_lot_create,
        language,
        language_configured,
        theme,
        theme_configured,
    })
}

#[cfg(test)]
mod tests {
    // NOTE: no `use super::*` — avoids bringing the sibling `stores` module
    // into scope, which would shadow local repository functions.
    use crate::db::migrations::fresh_test_pool;

    use super::{
        get_last_selected_store_id, get_setting, get_settings, get_theme_setting,
        set_language_setting, set_last_selected_store_id, set_theme_setting, upsert_setting,
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

    #[tokio::test]
    async fn get_settings_language_unconfigured_on_fresh_install(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // No `app_settings.language` row yet: the response must still expose
        // a non-null language (fallback "en"), but the configured flag must
        // be false so the frontend can run OS detection.
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.language, "en");
        assert!(!settings.language_configured);
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_language_configured_when_persisted(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Persist a manual pick: configured flag flips true and the value
        // is returned as-is (no fallback masking).
        set_language_setting(&pool, "es").await?;
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.language, "es");
        assert!(settings.language_configured);

        // Same for the other supported locale.
        set_language_setting(&pool, "en").await?;
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.language, "en");
        assert!(settings.language_configured);
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_language_unconfigured_for_unsupported_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // A persisted but unsupported tag (e.g. legacy "fr") must NOT be
        // reported as a manual preference — the frontend falls back to
        // detection instead of honouring an invalid row.
        upsert_setting(&pool, "language", "fr").await?;
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.language, "en");
        assert!(!settings.language_configured);
        Ok(())
    }

    // ─── Theme handling (PR 2 of caduxo-daisyui-redesign) ──────────────

    #[tokio::test]
    async fn get_theme_setting_missing_row_returns_none() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let val = get_theme_setting(&pool).await?;
        assert!(val.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_theme_defaults_on_fresh_install() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;

        // No `app_settings.theme` row yet: the response must expose a
        // non-null theme (fallback `"caduxo-light"`), but the configured
        // flag must be false so the frontend can run `prefers-color-scheme`
        // detection instead of locking the UI to the fallback theme.
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.theme, "caduxo-light");
        assert!(!settings.theme_configured);
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_theme_unconfigured_for_empty_string(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // An empty stored value (e.g. a row created before the validation
        // gate landed, or a manual DB poke) must NOT be reported as a
        // manual preference — the frontend falls back to detection /
        // fallback instead of honouring an empty string.
        upsert_setting(&pool, "theme", "").await?;
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.theme, "caduxo-light");
        assert!(!settings.theme_configured);
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_theme_unconfigured_for_unsupported_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // A persisted but unsupported tag (e.g. a real DaisyUI theme like
        // `synthwave` that is NOT in the curated v1 set, or a typo) must
        // NOT be reported as a manual preference — the frontend falls back
        // to the curated set via detection instead of honouring an invalid
        // row.
        upsert_setting(&pool, "theme", "synthwave").await?;
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.theme, "caduxo-light");
        assert!(!settings.theme_configured);
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_theme_configured_when_persisted() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;

        // Manual pick: configured flag flips true and the value is returned
        // as-is (no fallback masking). Covers the legacy v1 pair plus each
        // new built-in so the snapshot recognises every curated theme.
        for value in [
            "dark",
            "caduxo-light",
            "dracula",
            "valentine",
            "luxury",
            "sunset",
            "nord",
        ] {
            set_theme_setting(&pool, value).await?;
            let settings = get_settings(&pool).await?;
            assert_eq!(settings.theme, value, "theme mismatch for `{value}`");
            assert!(
                settings.theme_configured,
                "theme_configured must be true for `{value}`"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_language_and_theme_configure_independently(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Persist only the theme; language must stay unconfigured so the
        // frontend still runs OS detection for locale.
        set_theme_setting(&pool, "dark").await?;
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.theme, "dark");
        assert!(settings.theme_configured);
        assert_eq!(settings.language, "en");
        assert!(!settings.language_configured);

        // Persist only the language; theme must stay unconfigured.
        upsert_setting(&pool, "language", "es").await?;
        let settings = get_settings(&pool).await?;
        assert_eq!(settings.language, "es");
        assert!(settings.language_configured);
        assert_eq!(settings.theme, "dark");
        assert!(settings.theme_configured);
        Ok(())
    }
}
