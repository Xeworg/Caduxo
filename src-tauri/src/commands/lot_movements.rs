//! Tauri commands for lot movement management.

use crate::dto::lot_movements::{LotLocationBalance, LotMovementCreate, LotMovementResponse};
use crate::services::lot_movements as service;
use crate::state::AppState;
use tauri::State;

/// Creates a new lot movement and updates the lot total atomically.
#[tauri::command]
pub async fn create_lot_movement(
    state: State<'_, AppState>,
    input: LotMovementCreate,
) -> Result<LotMovementResponse, String> {
    let pool = state.pool().await;
    service::create_lot_movement(&pool, input)
        .await
        .map_err(|e| e.to_string())
}

/// Lists all movements for a lot, newest first.
#[tauri::command]
pub async fn list_lot_movements(
    state: State<'_, AppState>,
    lot_id: String,
) -> Result<Vec<LotMovementResponse>, String> {
    let pool = state.pool().await;
    service::list_lot_movements(&pool, &lot_id)
        .await
        .map_err(|e| e.to_string())
}

/// Returns per-location balances for a lot.
#[tauri::command]
pub async fn get_lot_location_balances(
    state: State<'_, AppState>,
    lot_id: String,
) -> Result<Vec<LotLocationBalance>, String> {
    let pool = state.pool().await;
    service::get_lot_location_balances(&pool, &lot_id)
        .await
        .map_err(|e| e.to_string())
}
