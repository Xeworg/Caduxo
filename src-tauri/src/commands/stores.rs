//! Tauri commands for store and internal location management.

use tauri::State;

use crate::dto::stores::{
    SettingsResponse, SettingsUpdate, StoreCreate, StoreLocationCreate, StoreLocationResponse,
    StoreLocationUpdate, StoreResponse, StoreUpdate,
};
use crate::error::{AppError, CommandError};
use crate::pdf::locale::Locale;
use crate::services::settings as settings_service;
use crate::services::stores as store_service;
use crate::services::user_messages::{
    localize_business_rule, localize_duplicate_field, localize_internal, localize_not_found,
    localize_validation, user_message, UserMessage, ALLOWED_THEMES,
};
use crate::state::AppState;

/// Resolves an optional BCP-47 locale tag into a [`Locale`], falling back to
/// English when the frontend does not supply one. Used at every store and
/// settings command boundary so the catalog validation messages stay
/// frontend-compatible (no required caller-side argument).
fn resolve_locale(locale: Option<String>) -> Locale {
    locale.as_deref().map(Locale::parse).unwrap_or(Locale::En)
}

/// Returns true if this is a first-run (no active stores exist).
#[tauri::command]
pub async fn is_first_run(state: State<'_, AppState>) -> Result<bool, CommandError> {
    let pool = state.pool().await;
    store_service::is_first_run(&pool)
        .await
        .map_err(AppError::into)
}

/// Returns all active stores.
#[tauri::command]
pub async fn list_stores(state: State<'_, AppState>) -> Result<Vec<StoreResponse>, CommandError> {
    let pool = state.pool().await;
    store_service::list_stores(&pool)
        .await
        .map_err(AppError::into)
}

/// Creates a new store.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` so the store
/// `Name` validation surfaced by the service reaches the UI in the active
/// locale. Unknown tags fall back to English via `Locale::parse`.
#[tauri::command]
pub async fn create_store(
    state: State<'_, AppState>,
    input: StoreCreate,
    locale: Option<String>,
) -> Result<StoreResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    store_service::create_store(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Updates an existing store.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` for the same
/// reason as `create_store`.
#[tauri::command]
pub async fn update_store(
    state: State<'_, AppState>,
    input: StoreUpdate,
    locale: Option<String>,
) -> Result<StoreResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    store_service::update_store(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Returns active locations for a store.
#[tauri::command]
pub async fn list_store_locations(
    state: State<'_, AppState>,
    store_id: String,
) -> Result<Vec<StoreLocationResponse>, CommandError> {
    let pool = state.pool().await;
    store_service::list_locations(&pool, &store_id)
        .await
        .map_err(AppError::into)
}

/// Creates a new internal location under a store.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` and
/// `localize_business_rule` so both the location `Name` validation and the
/// simple constant BusinessRule rejection for an inactive store reach the UI
/// in the active locale. Unknown tags fall back to English via
/// `Locale::parse`.
#[tauri::command]
pub async fn create_store_location(
    state: State<'_, AppState>,
    input: StoreLocationCreate,
    locale: Option<String>,
) -> Result<StoreLocationResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    store_service::create_location(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Updates an existing internal location.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` for the same
/// reason as `create_store_location`.
#[tauri::command]
pub async fn update_store_location(
    state: State<'_, AppState>,
    input: StoreLocationUpdate,
    locale: Option<String>,
) -> Result<StoreLocationResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    store_service::update_location(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Returns the current application settings.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<SettingsResponse, CommandError> {
    let pool = state.pool().await;
    settings_service::get_settings(&pool)
        .await
        .map_err(AppError::into)
}

/// Updates application settings.
///
/// `locale` (BCP-47 tag) is forwarded to the inline `LanguageNotAllowed`
/// validation so the rejection reaches the UI in the active locale. Unknown
/// tags fall back to English via `Locale::parse`. PR 1 of
/// `scanner-quick-operations` adds equivalent `scanner_fefo_policy` and
/// `close_behavior` validation here so the persisted row stays untouched
/// when an unrecognised value slips through.
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    input: SettingsUpdate,
    locale: Option<String>,
) -> Result<SettingsResponse, CommandError> {
    let loc = resolve_locale(locale);
    if let Some(ref value) = input.language {
        if value != "en" && value != "es" {
            return Err(CommandError::Validation {
                message: user_message(
                    UserMessage::LanguageNotAllowed {
                        value: value.clone(),
                    },
                    loc,
                ),
            });
        }
    }
    if let Some(ref value) = input.theme {
        validate_theme_value(value, loc)?;
    }
    if let Some(ref value) = input.scanner_fefo_policy {
        validate_scanner_fefo_policy_value(*value, loc)?;
    }
    if let Some(ref value) = input.close_behavior {
        validate_close_behavior_value(*value, loc)?;
    }
    let pool = state.pool().await;
    // `?` uses the `From<AppError> for CommandError` impl the function
    // return type expects, avoiding the `AppError::into` ambiguity that
    // arises with explicit `.map_err(...)` when two `From` impls exist.
    let updated: SettingsResponse = settings_service::update_settings(&pool, input).await?;

    // Refresh the close-window behaviour cache so the next close attempt
    // observes the new setting without an async DB read inside the
    // lifecycle handler. PR 3 of `scanner-quick-operations` wires the
    // desktop close-window handler against this cache.
    state.set_close_behavior(updated.close_behavior);

    Ok(updated)
}

/// Validates a scanner FEFO policy value at the IPC boundary.
///
/// The curated v1 set is `{"suggest_fefo", "require_fefo",
/// "manual_lot_choice"}`. Any other value is rejected as
/// `CommandError::Validation` with a locale-aware message so the persisted
/// row stays untouched. PR 1 of `scanner-quick-operations` introduces the
/// setting; the function is the single source of truth for IPC validation
/// and is mirrored by tests in `commands::stores::tests`.
fn validate_scanner_fefo_policy_value(
    value: crate::dto::stores::FefoPolicy,
    loc: Locale,
) -> Result<(), CommandError> {
    // `FefoPolicy::parse` is lenient; the curated v1 set is the same three
    // variants the wire shape recognises — accepting the parsed value back
    // round-trips through the same set. We surface the raw value in the
    // rejection text so a stale unknown wire tag (which serde would have
    // rejected before reaching the command) does not silently fall back
    // to `SuggestFefo`.
    let wire = value.as_wire();
    match wire {
        "suggest_fefo" | "require_fefo" | "manual_lot_choice" => Ok(()),
        _ => Err(CommandError::Validation {
            message: user_message(
                UserMessage::ScannerFefoPolicyNotAllowed {
                    value: wire.to_string(),
                },
                loc,
            ),
        }),
    }
}

/// Validates a close-window behaviour value at the IPC boundary.
///
/// The curated v1 set is `{"minimize_to_tray", "exit_application"}`. Any
/// other value is rejected as `CommandError::Validation` with a
/// locale-aware message so the persisted row stays untouched.
fn validate_close_behavior_value(
    value: crate::dto::stores::CloseBehavior,
    loc: Locale,
) -> Result<(), CommandError> {
    let wire = value.as_wire();
    match wire {
        "minimize_to_tray" | "exit_application" => Ok(()),
        _ => Err(CommandError::Validation {
            message: user_message(
                UserMessage::CloseBehaviorNotAllowed {
                    value: wire.to_string(),
                },
                loc,
            ),
        }),
    }
}

/// Validates a theme preference at the IPC boundary.
///
/// The curated v1 set is `{"caduxo-light", "dark", "dracula", "valentine",
/// "luxury", "sunset", "nord"}` (see `ALLOWED_THEMES` in
/// `services::user_messages`, which is also the source the rejection
/// message uses). Any other value is rejected as `CommandError::Validation`
/// with a locale-aware message so the persisted row stays untouched. The
/// function is the single source of truth for theme validation;
/// `update_settings` and the unit tests both call it so a future change
/// to the curated set lands in one place.
///
/// PR 2 (caduxo-daisyui-redesign) wires the first theme-aware setting.
fn validate_theme_value(value: &str, loc: Locale) -> Result<(), CommandError> {
    if ALLOWED_THEMES.contains(&value) {
        return Ok(());
    }
    Err(CommandError::Validation {
        message: user_message(
            UserMessage::ThemeNotAllowed {
                value: value.to_string(),
            },
            loc,
        ),
    })
}

/// Returns true if at least one active store exists (used as a precondition check).
#[tauri::command]
pub async fn has_store(state: State<'_, AppState>) -> Result<bool, CommandError> {
    let pool = state.pool().await;
    store_service::has_store(&pool)
        .await
        .map_err(AppError::into)
}

#[cfg(test)]
mod tests {
    // PR 2 (caduxo-daisyui-redesign) — command-boundary theme validation.
    // The IPC gate rejects values outside the curated v1 set
    // (`caduxo-light`, `dark`, `dracula`, `valentine`, `luxury`, `sunset`,
    // `nord`) at the boundary so the persisted row stays untouched and the
    // rejection reaches the UI in the active locale. The list is owned by
    // `services::user_messages::ALLOWED_THEMES`; the IPC validator, the
    // rejection message, and these tests all read from it (via the
    // constant's display form for the literal string assertions) so a
    // future theme addition cannot drift the message away from the
    // whitelist.
    use super::validate_theme_value;
    use crate::error::CommandError;
    use crate::pdf::locale::Locale;
    use crate::services::user_messages::ALLOWED_THEMES_DISPLAY;

    #[test]
    fn validate_theme_accepts_caduxo_light_in_english() {
        let result = validate_theme_value("caduxo-light", Locale::En);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_theme_accepts_caduxo_light_in_spanish() {
        let result = validate_theme_value("caduxo-light", Locale::Es);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_theme_accepts_dark_in_english() {
        let result = validate_theme_value("dark", Locale::En);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_theme_accepts_dark_in_spanish() {
        let result = validate_theme_value("dark", Locale::Es);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_theme_accepts_dracula_in_english() {
        let result = validate_theme_value("dracula", Locale::En);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_theme_accepts_valentine_in_spanish() {
        let result = validate_theme_value("valentine", Locale::Es);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_theme_accepts_luxury_in_english() {
        let result = validate_theme_value("luxury", Locale::En);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_theme_accepts_sunset_in_spanish() {
        let result = validate_theme_value("sunset", Locale::Es);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_theme_accepts_nord_in_english() {
        let result = validate_theme_value("nord", Locale::En);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_theme_rejects_synthwave_in_english() {
        // `synthwave` is a real DaisyUI theme but is intentionally NOT in
        // the curated v1 set — the user explicitly asked for `dracula`,
        // `valentine`, `luxury`, `sunset`, `nord` and not the full
        // catalog. Keeping a non-selected DaisyUI theme as the rejection
        // sample also doubles as a regression guard: if someone ever swaps
        // `ALLOWED_THEMES` for `themes: all`-style coverage, this assertion
        // would need to change too.
        let err = validate_theme_value("synthwave", Locale::En).unwrap_err();
        match err {
            CommandError::Validation { message } => {
                assert_eq!(
                    message,
                    format!(
                        "theme must be one of {{{}}}, got `synthwave`",
                        ALLOWED_THEMES_DISPLAY
                    )
                );
            }
            other => panic!("expected CommandError::Validation, got {other:?}"),
        }
    }

    #[test]
    fn validate_theme_rejects_synthwave_in_spanish() {
        let err = validate_theme_value("synthwave", Locale::Es).unwrap_err();
        match err {
            CommandError::Validation { message } => {
                assert_eq!(
                    message,
                    format!(
                        "el tema debe ser uno de {{{}}}, se recibió `synthwave`",
                        ALLOWED_THEMES_DISPLAY
                    )
                );
            }
            other => panic!("expected CommandError::Validation, got {other:?}"),
        }
    }

    #[test]
    fn validate_theme_rejects_empty_string() {
        let err = validate_theme_value("", Locale::En).unwrap_err();
        assert!(matches!(err, CommandError::Validation { .. }));
    }

    #[test]
    fn validate_theme_rejects_uppercase_dark() {
        // Curation is case-sensitive on purpose; the v1 set is the literal
        // strings in `ALLOWED_THEMES`. An uppercase variant must be
        // rejected so a typo never reaches persistence.
        let err = validate_theme_value("Dark", Locale::En).unwrap_err();
        assert!(matches!(err, CommandError::Validation { .. }));
    }

    #[test]
    fn validate_theme_rejects_uppercase_dracula() {
        // Same case-sensitivity guarantee for the new built-ins.
        let err = validate_theme_value("Dracula", Locale::En).unwrap_err();
        assert!(matches!(err, CommandError::Validation { .. }));
    }
}
