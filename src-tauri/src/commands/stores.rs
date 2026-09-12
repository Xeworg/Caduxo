//! Tauri commands for store and internal location management.

use tauri::State;

use crate::dto::stores::{
    SettingsResponse, SettingsUpdate, StoreCreate, StoreLocationCreate, StoreLocationResponse,
    StoreLocationUpdate, StoreResponse, StoreUpdate,
};
use crate::error::{AppError, CommandError};
use crate::services::settings as settings_service;
use crate::services::stores as store_service;
use crate::state::AppState;

/// Returns true if this is a first-run (no active stores exist).
#[tauri::command]
pub async fn is_first_run(state: State<'_, AppState>) -> Result<bool, CommandError> {
    store_service::is_first_run(&state.pool)
        .await
        .map_err(AppError::into)
}

/// Returns all active stores.
#[tauri::command]
pub async fn list_stores(state: State<'_, AppState>) -> Result<Vec<StoreResponse>, CommandError> {
    store_service::list_stores(&state.pool)
        .await
        .map_err(AppError::into)
}

/// Creates a new store.
#[tauri::command]
pub async fn create_store(
    state: State<'_, AppState>,
    input: StoreCreate,
) -> Result<StoreResponse, CommandError> {
    store_service::create_store(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Updates an existing store.
#[tauri::command]
pub async fn update_store(
    state: State<'_, AppState>,
    input: StoreUpdate,
) -> Result<StoreResponse, CommandError> {
    store_service::update_store(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Returns active locations for a store.
#[tauri::command]
pub async fn list_store_locations(
    state: State<'_, AppState>,
    store_id: String,
) -> Result<Vec<StoreLocationResponse>, CommandError> {
    store_service::list_locations(&state.pool, &store_id)
        .await
        .map_err(AppError::into)
}

/// Creates a new internal location under a store.
#[tauri::command]
pub async fn create_store_location(
    state: State<'_, AppState>,
    input: StoreLocationCreate,
) -> Result<StoreLocationResponse, CommandError> {
    store_service::create_location(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Updates an existing internal location.
#[tauri::command]
pub async fn update_store_location(
    state: State<'_, AppState>,
    input: StoreLocationUpdate,
) -> Result<StoreLocationResponse, CommandError> {
    store_service::update_location(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Returns the current application settings.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<SettingsResponse, CommandError> {
    settings_service::get_settings(&state.pool)
        .await
        .map_err(AppError::into)
}

/// Updates application settings.
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    input: SettingsUpdate,
) -> Result<SettingsResponse, CommandError> {
    settings_service::update_settings(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Returns true if at least one active store exists (used as a precondition check).
#[tauri::command]
pub async fn has_store(state: State<'_, AppState>) -> Result<bool, CommandError> {
    store_service::has_store(&state.pool)
        .await
        .map_err(AppError::into)
}
