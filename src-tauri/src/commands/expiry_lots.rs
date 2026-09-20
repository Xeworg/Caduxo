//! Tauri commands for expiry lot management: create, update, archive,
//! list, get, and partial resolution.

use tauri::State;

use crate::dto::expiry_lots::{
    ArchiveLotInput, ExpiryLotCreate, ExpiryLotResolve, ExpiryLotResolveResult, ExpiryLotResponse,
    ExpiryLotUpdate, LotResolutionEventResponse,
};
use crate::error::{AppError, CommandError};
use crate::services::expiry_lots as service;
use crate::services::user_messages::localize_validation;
use crate::state::AppState;

/// Returns all active expiry lots across all stores, ordered by expiry date.
#[tauri::command]
pub async fn list_expiry_lots(
    state: State<'_, AppState>,
) -> Result<Vec<ExpiryLotResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_all_expiry_lots(&pool)
        .await
        .map_err(AppError::into)
}

/// Returns all active expiry lots for a given store.
#[tauri::command]
pub async fn list_expiry_lots_by_store(
    state: State<'_, AppState>,
    store_id: String,
) -> Result<Vec<ExpiryLotResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_expiry_lots_by_store(&pool, store_id)
        .await
        .map_err(AppError::into)
}

/// Returns all active expiry lots for a given product, ordered by expiry date.
#[tauri::command]
pub async fn list_expiry_lots_by_product(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<Vec<ExpiryLotResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_expiry_lots_by_product(&pool, product_id)
        .await
        .map_err(AppError::into)
}

/// Returns a single expiry lot by id.
#[tauri::command]
pub async fn get_expiry_lot(
    state: State<'_, AppState>,
    id: String,
) -> Result<ExpiryLotResponse, CommandError> {
    let pool = state.pool().await;
    service::get_expiry_lot(&pool, id)
        .await
        .map_err(AppError::into)
}

/// Creates a new expiry lot. Pre-fills unit and alert-days from the product
/// defaults when those fields are omitted (None). Fails if no active store
/// exists.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` so the
/// `LocationRequired` validation surfaced by the service reaches the UI in
/// the active locale. Unknown tags fall back to English via `Locale::parse`.
#[tauri::command]
pub async fn create_expiry_lot(
    state: State<'_, AppState>,
    input: ExpiryLotCreate,
    locale: String,
) -> Result<ExpiryLotResponse, CommandError> {
    let pool = state.pool().await;
    let loc = crate::pdf::locale::Locale::parse(&locale);
    service::create_expiry_lot(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(AppError::into)
}

/// Updates an existing active expiry lot.
#[tauri::command]
pub async fn update_expiry_lot(
    state: State<'_, AppState>,
    input: ExpiryLotUpdate,
) -> Result<ExpiryLotResponse, CommandError> {
    let pool = state.pool().await;
    service::update_expiry_lot(&pool, input)
        .await
        .map_err(AppError::into)
}

/// Soft-archives an expiry lot (status = 'archived') with a required
/// justification. Persists the archive reason + notes in `lot_movements`
/// as an `exit:other` marker in the same transaction. Archived lots are
/// excluded from active lists and dashboard queries.
#[tauri::command]
pub async fn archive_expiry_lot(
    state: State<'_, AppState>,
    input: ArchiveLotInput,
) -> Result<(), CommandError> {
    let pool = state.pool().await;
    service::archive_expiry_lot(&pool, input)
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
    let pool = state.pool().await;
    service::resolve_expiry_lot(&pool, input)
        .await
        .map_err(AppError::into)
}

/// Returns all resolution events for a given expiry lot, newest first.
#[tauri::command]
pub async fn list_lot_resolution_events(
    state: State<'_, AppState>,
    lot_id: String,
) -> Result<Vec<LotResolutionEventResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_resolution_events(&pool, lot_id)
        .await
        .map_err(AppError::into)
}
