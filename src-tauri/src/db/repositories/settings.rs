//! Repository for app_settings key-value persistence.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::dto::stores::{CloseBehavior, FefoPolicy, SettingsResponse};

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

/// Retrieves the scanner FEFO policy setting, or `None` if absent.
///
/// `None` (or any unrecognised value) is mapped to the documented default
/// (`SuggestFefo`) inside [`get_settings`] and [`FefoPolicy::parse`] — this
/// helper returns the raw stored value so the snapshot logic can apply the
/// same `configured`-style fallback the `language` / `theme` rows use.
pub async fn get_scanner_fefo_policy_setting(
    pool: &SqlitePool,
) -> Result<Option<String>, sqlx::Error> {
    get_setting(pool, "scanner_fefo_policy").await
}

/// Persists the scanner FEFO policy. The command boundary rejects values
/// outside the curated v1 set (`suggest_fefo`, `require_fefo`,
/// `manual_lot_choice`) so the persisted row stays untouched on rejection;
/// this function accepts any non-empty string the caller supplies.
pub async fn set_scanner_fefo_policy_setting(
    pool: &SqlitePool,
    value: &str,
) -> Result<(), sqlx::Error> {
    upsert_setting(pool, "scanner_fefo_policy", value).await
}

/// Retrieves the close-window behaviour setting, or `None` if absent.
///
/// `None` (or any unrecognised value) is mapped to the documented default
/// (`MinimizeToTray`) inside [`get_settings`] and [`CloseBehavior::parse`].
pub async fn get_close_behavior_setting(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    get_setting(pool, "close_behavior").await
}

/// Persists the close-window behaviour. The command boundary rejects values
/// outside the curated v1 set (`minimize_to_tray`, `exit_application`) so
/// the persisted row stays untouched on rejection; this function accepts
/// any non-empty string the caller supplies.
pub async fn set_close_behavior_setting(pool: &SqlitePool, value: &str) -> Result<(), sqlx::Error> {
    upsert_setting(pool, "close_behavior", value).await
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
    // Scanner FEFO policy: a raw `app_settings.scanner_fefo_policy` row
    // (or `None`) is mapped to the documented default via
    // `FefoPolicy::parse`. The helper's lenient matching means an absent
    // row, an empty string, whitespace, and any unrecognised tag all
    // collapse to `SuggestFefo` so the snapshot stays non-null and the
    // frontend never has to defend against `null`.
    let stored_fefo = get_scanner_fefo_policy_setting(pool).await?;
    let scanner_fefo_policy = FefoPolicy::parse(stored_fefo.as_deref());
    // Close-window behaviour: same lenient mapping — `None`, empty, or
    // any unrecognised tag collapses to the documented default
    // (`MinimizeToTray`).
    let stored_close_behavior = get_close_behavior_setting(pool).await?;
    let close_behavior = CloseBehavior::parse(stored_close_behavior.as_deref());
    Ok(SettingsResponse {
        last_selected_store_id,
        require_initial_location_on_lot_create,
        language,
        language_configured,
        theme,
        theme_configured,
        scanner_fefo_policy,
        close_behavior,
    })
}

#[cfg(test)]
mod tests {
    // NOTE: no `use super::*` — avoids bringing the sibling `stores` module
    // into scope, which would shadow local repository functions.
    use crate::db::migrations::fresh_test_pool;

    use super::{
        get_close_behavior_setting, get_last_selected_store_id, get_scanner_fefo_policy_setting,
        get_setting, get_settings, get_theme_setting, set_close_behavior_setting,
        set_language_setting, set_last_selected_store_id, set_scanner_fefo_policy_setting,
        set_theme_setting, upsert_setting,
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

    // ─── Scanner FEFO policy (PR 1 of scanner-quick-operations) ────────────

    #[tokio::test]
    async fn get_scanner_fefo_policy_missing_returns_none() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let val = get_scanner_fefo_policy_setting(&pool).await?;
        assert!(val.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn set_and_get_scanner_fefo_policy_round_trips() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        set_scanner_fefo_policy_setting(&pool, "require_fefo").await?;
        let val = get_scanner_fefo_policy_setting(&pool).await?;
        assert_eq!(val, Some("require_fefo".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_scanner_fefo_defaults_to_suggest_on_fresh_install(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // No `app_settings.scanner_fefo_policy` row: the snapshot must
        // expose the documented default (`SuggestFefo` / `suggest_fefo`).
        let settings = get_settings(&pool).await?;
        assert_eq!(
            settings.scanner_fefo_policy,
            crate::dto::stores::FefoPolicy::SuggestFefo
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_scanner_fefo_round_trips_every_curated_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // Each curated value must round-trip verbatim so the snapshot
        // doesn't fallback-mask the user's manual pick.
        for (raw, expected) in [
            ("suggest_fefo", crate::dto::stores::FefoPolicy::SuggestFefo),
            ("require_fefo", crate::dto::stores::FefoPolicy::RequireFefo),
            (
                "manual_lot_choice",
                crate::dto::stores::FefoPolicy::ManualLotChoice,
            ),
        ] {
            set_scanner_fefo_policy_setting(&pool, raw).await?;
            let settings = get_settings(&pool).await?;
            assert_eq!(
                settings.scanner_fefo_policy, expected,
                "scanner_fefo_policy mismatch for `{raw}`"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_scanner_fefo_falls_back_to_suggest_for_unknown_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // A persisted but unrecognised tag (e.g. legacy `"force_fefo"`,
        // empty string, or whitespace) must NOT be reported as a manual
        // preference; the snapshot falls back to `SuggestFefo` so the
        // frontend never sees a null / unmapped value.
        for raw in ["force_fefo", "", "  ", "SuggestFefo"] {
            upsert_setting(&pool, "scanner_fefo_policy", raw).await?;
            let settings = get_settings(&pool).await?;
            assert_eq!(
                settings.scanner_fefo_policy,
                crate::dto::stores::FefoPolicy::SuggestFefo,
                "scanner_fefo_policy must fall back to SuggestFefo for raw=`{raw}`"
            );
        }
        Ok(())
    }

    // ─── Close-window behaviour (PR 1 of scanner-quick-operations) ──────────

    #[tokio::test]
    async fn get_close_behavior_missing_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let val = get_close_behavior_setting(&pool).await?;
        assert!(val.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn set_and_get_close_behavior_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        set_close_behavior_setting(&pool, "exit_application").await?;
        let val = get_close_behavior_setting(&pool).await?;
        assert_eq!(val, Some("exit_application".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_close_behavior_defaults_to_minimize_to_tray_on_fresh_install(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // No `app_settings.close_behavior` row: the snapshot must expose
        // the documented default (`MinimizeToTray` / `minimize_to_tray`).
        let settings = get_settings(&pool).await?;
        assert_eq!(
            settings.close_behavior,
            crate::dto::stores::CloseBehavior::MinimizeToTray
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_close_behavior_round_trips_every_curated_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        for (raw, expected) in [
            (
                "minimize_to_tray",
                crate::dto::stores::CloseBehavior::MinimizeToTray,
            ),
            (
                "exit_application",
                crate::dto::stores::CloseBehavior::ExitApplication,
            ),
        ] {
            set_close_behavior_setting(&pool, raw).await?;
            let settings = get_settings(&pool).await?;
            assert_eq!(
                settings.close_behavior, expected,
                "close_behavior mismatch for `{raw}`"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_close_behavior_falls_back_to_minimize_to_tray_for_unknown_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // Unrecognised tags (e.g. legacy `"kill"`, empty, whitespace) must
        // NOT be reported as a manual preference; the snapshot falls back
        // to `MinimizeToTray` so the close-window handler never sees a
        // null / unmapped value.
        for raw in ["kill", "", "  ", "MinimizeToTray"] {
            upsert_setting(&pool, "close_behavior", raw).await?;
            let settings = get_settings(&pool).await?;
            assert_eq!(
                settings.close_behavior,
                crate::dto::stores::CloseBehavior::MinimizeToTray,
                "close_behavior must fall back to MinimizeToTray for raw=`{raw}`"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn get_settings_all_four_keys_configure_independently(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // Persist only one of the four new keys; the others must stay at
        // their documented defaults so the frontend can detect a partial
        // update without seeing unrelated state change.
        set_scanner_fefo_policy_setting(&pool, "require_fefo").await?;
        let settings = get_settings(&pool).await?;
        assert_eq!(
            settings.scanner_fefo_policy,
            crate::dto::stores::FefoPolicy::RequireFefo
        );
        assert_eq!(
            settings.close_behavior,
            crate::dto::stores::CloseBehavior::MinimizeToTray
        );

        // Now flip only the close behaviour; FEFO must stay at RequireFefo.
        set_close_behavior_setting(&pool, "exit_application").await?;
        let settings = get_settings(&pool).await?;
        assert_eq!(
            settings.scanner_fefo_policy,
            crate::dto::stores::FefoPolicy::RequireFefo
        );
        assert_eq!(
            settings.close_behavior,
            crate::dto::stores::CloseBehavior::ExitApplication
        );
        Ok(())
    }
}
