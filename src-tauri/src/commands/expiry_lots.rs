//! Tauri commands for expiry lot management: create, update, archive,
//! list, get, and partial resolution.

use tauri::State;

use crate::dto::expiry_lots::{
    ExpiryLotCreate, ExpiryLotResolve, ExpiryLotResolveResult, ExpiryLotResponse, ExpiryLotUpdate,
    LotResolutionEventResponse,
};
use crate::error::{AppError, CommandError};
use crate::services::expiry_lots as service;
use crate::state::AppState;

/// Returns all active expiry lots across all stores, ordered by expiry date.
#[tauri::command]
pub async fn list_expiry_lots(
    state: State<'_, AppState>,
) -> Result<Vec<ExpiryLotResponse>, CommandError> {
    service::list_all_expiry_lots(&state.pool)
        .await
        .map_err(AppError::into)
}

/// Returns all active expiry lots for a given store.
#[tauri::command]
pub async fn list_expiry_lots_by_store(
    state: State<'_, AppState>,
    store_id: String,
) -> Result<Vec<ExpiryLotResponse>, CommandError> {
    service::list_expiry_lots_by_store(&state.pool, store_id)
        .await
        .map_err(AppError::into)
}

/// Returns all active expiry lots for a given product, ordered by expiry date.
#[tauri::command]
pub async fn list_expiry_lots_by_product(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<Vec<ExpiryLotResponse>, CommandError> {
    service::list_expiry_lots_by_product(&state.pool, product_id)
        .await
        .map_err(AppError::into)
}

/// Returns a single expiry lot by id.
#[tauri::command]
pub async fn get_expiry_lot(
    state: State<'_, AppState>,
    id: String,
) -> Result<ExpiryLotResponse, CommandError> {
    service::get_expiry_lot(&state.pool, id)
        .await
        .map_err(AppError::into)
}

/// Creates a new expiry lot. Pre-fills unit and alert-days from the product
/// defaults when those fields are omitted (None). Fails if no active store
/// exists.
#[tauri::command]
pub async fn create_expiry_lot(
    state: State<'_, AppState>,
    input: ExpiryLotCreate,
) -> Result<ExpiryLotResponse, CommandError> {
    service::create_expiry_lot(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Updates an existing active expiry lot.
#[tauri::command]
pub async fn update_expiry_lot(
    state: State<'_, AppState>,
    input: ExpiryLotUpdate,
) -> Result<ExpiryLotResponse, CommandError> {
    service::update_expiry_lot(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Soft-archives an expiry lot (status = 'archived'). Archived lots are
/// excluded from active lists and dashboard queries.
#[tauri::command]
pub async fn archive_expiry_lot(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), CommandError> {
    service::archive_expiry_lot(&state.pool, id)
        .await
        .map_err(AppError::into)
}

/// Resolves (consumes, discards, or transfers) a quantity from an expiry lot.
/// Records a resolution event and marks the lot as fully resolved when
/// remaining quantity reaches zero.
#[tauri::command]
pub async fn resolve_expiry_lot(
    state: State<'_, AppState>,
    input: ExpiryLotResolve,
) -> Result<ExpiryLotResolveResult, CommandError> {
    service::resolve_expiry_lot(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Returns all resolution events for a given expiry lot, newest first.
#[tauri::command]
pub async fn list_lot_resolution_events(
    state: State<'_, AppState>,
    lot_id: String,
) -> Result<Vec<LotResolutionEventResponse>, CommandError> {
    service::list_resolution_events(&state.pool, lot_id)
        .await
        .map_err(AppError::into)
}
