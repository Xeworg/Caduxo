//! Tauri commands for lot movement management.

use crate::dto::lot_movements::{LotLocationBalance, LotMovementCreate, LotMovementResponse};
use crate::pdf::locale::Locale;
use crate::services::lot_movements as service;
use crate::services::user_messages::{
    localize_business_rule, localize_duplicate_field, localize_internal, localize_not_found,
    localize_validation,
};
use crate::state::AppState;
use tauri::State;

/// Resolves an optional BCP-47 locale tag into a [`Locale`], falling back to
/// English when the frontend does not supply one. Used at the lot-movement
/// command boundary so user-facing Validation / BusinessRule messages reach
/// the UI in the active locale while English stays the safe default for
/// legacy callers that omit the argument.
fn resolve_locale(locale: Option<String>) -> Locale {
    locale.as_deref().map(Locale::parse).unwrap_or(Locale::En)
}

/// Creates a new lot movement and updates the lot total atomically.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_validation` and
/// `localize_business_rule` so the constant Validation messages
/// (`Unknown movement kind`, `entry:initial must not have a source location`,
/// `transfer requires a source location`, `inventory_adjustment with
/// direction=increase must not have a source location`, the
/// `{kind}` requires a source location` / `must not have a destination
/// location` exit pairs, the `direction is required for inventory_adjustment`
/// / `direction is only valid for inventory_adjustment, not {kind}` pair,
/// the `Notes are required for movement kind {kind}` message, and the
/// canonical `Quantity must be positive / non-negative / whole number` set)
/// and the two BusinessRule messages (`Insufficient balance at source
/// location: ...`, `Location is inactive`) reach the UI in the active
/// locale. Omitting the argument keeps English.
///
/// The string-based IPC contract is preserved: the final error is flattened
/// to `String` via `to_string()`. Non-Validation / non-BusinessRule
/// DomainError variants and InfrastructureError pass through unchanged to
/// keep the pre-localization behaviour for internal errors.
#[tauri::command]
pub async fn create_lot_movement(
    state: State<'_, AppState>,
    input: LotMovementCreate,
    locale: Option<String>,
) -> Result<LotMovementResponse, String> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::create_lot_movement(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
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
