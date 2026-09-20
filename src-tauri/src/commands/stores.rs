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
use crate::services::user_messages::{localize_validation, user_message, UserMessage};
use crate::state::AppState;

/// Resolves an optional BCP-47 locale tag into a [`Locale`], falling back to
/// English when the frontend does not supply one. Used at every store and
/// settings command boundary so the catalog validation messages stay
/// frontend-compatible (no required caller-side argument).
fn resolve_locale(locale: Option<String>) -> Locale {
    locale
        .as_deref()
        .map(Locale::parse)
        .unwrap_or(Locale::En)
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
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` so the
/// location `Name` validation surfaced by the service reaches the UI in the
/// active locale. Unknown tags fall back to English via `Locale::parse`.
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
/// tags fall back to English via `Locale::parse`.
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
    let pool = state.pool().await;
    settings_service::update_settings(&pool, input)
        .await
        .map_err(AppError::into)
}

/// Returns true if at least one active store exists (used as a precondition check).
#[tauri::command]
pub async fn has_store(state: State<'_, AppState>) -> Result<bool, CommandError> {
    let pool = state.pool().await;
    store_service::has_store(&pool)
        .await
        .map_err(AppError::into)
}
