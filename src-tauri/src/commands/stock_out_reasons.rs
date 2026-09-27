//! Tauri command adapters for the stock-out reasons catalog.
//!
//! ## Localization note
//!
//! These commands accept an optional `locale: Option<String>` argument (BCP-47
//! tag) so backend error-boundary localization chains can rewrite the
//! user-facing `Validation`, `BusinessRule`, `NotFound`, `DuplicateField`,
//! and `Infrastructure` errors into the active locale.

use crate::dto::stock_out_reasons::{
    StockOutReasonCreateInput, StockOutReasonRenameInput, StockOutReasonResponse,
};
use crate::error::{AppError, CommandError};
use crate::pdf::locale::Locale;
use crate::services::stock_out_reasons as service;
use crate::services::user_messages::{
    localize_business_rule, localize_duplicate_field, localize_internal, localize_not_found,
    localize_validation,
};
use crate::state::AppState;
use tauri::State;

/// Resolves an optional BCP-47 locale tag into a [`Locale`], falling back to
/// English when the frontend does not supply one.
fn resolve_locale(locale: Option<String>) -> Locale {
    locale.as_deref().map(Locale::parse).unwrap_or(Locale::En)
}

/// Lists all active (non-archived) stock-out reasons ordered by sort_order.
#[tauri::command]
pub async fn list_stock_out_reasons(
    state: State<'_, AppState>,
) -> Result<Vec<StockOutReasonResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_active(&pool).await.map_err(Into::into)
}

/// Lists all stock-out reasons including archived ones, ordered by sort_order.
#[tauri::command]
pub async fn list_all_stock_out_reasons(
    state: State<'_, AppState>,
) -> Result<Vec<StockOutReasonResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_all(&pool).await.map_err(Into::into)
}

/// Gets a single stock-out reason by its ID. Returns `None` if not found.
#[tauri::command]
pub async fn get_stock_out_reason(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<StockOutReasonResponse>, CommandError> {
    let pool = state.pool().await;
    service::find_by_id(&pool, &id).await.map_err(Into::into)
}

/// Creates a new custom stock-out reason.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_validation`
/// and `localize_duplicate_field` so the movement-kind / display-name
/// validation and the duplicate-display-name boundaries reach the UI
/// in the active locale. Unknown tags fall back to English.
#[tauri::command]
pub async fn create_stock_out_reason(
    state: State<'_, AppState>,
    input: StockOutReasonCreateInput,
    locale: Option<String>,
) -> Result<StockOutReasonResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::create_custom_reason(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Renames a stock-out reason's display_name. The `movement_kind` is immutable.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_validation`
/// and `localize_not_found` so the display-name validation and the
/// unknown-id boundary reach the UI in the active locale.
#[tauri::command]
pub async fn rename_stock_out_reason(
    state: State<'_, AppState>,
    input: StockOutReasonRenameInput,
    locale: Option<String>,
) -> Result<StockOutReasonResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::rename_reason(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Archives a stock-out reason. No hard delete.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_not_found`
/// so the unknown-id boundary reaches the UI in the active locale.
#[tauri::command]
pub async fn archive_stock_out_reason(
    state: State<'_, AppState>,
    id: String,
    locale: Option<String>,
) -> Result<(), CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::archive_reason(&pool, id)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Restores an archived stock-out reason. Blocked when the display_name
/// conflicts with an existing active reason (case-insensitive).
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_not_found`
/// and `localize_duplicate_field` so the unknown-id and
/// display-name-conflict boundaries reach the UI in the active locale.
#[tauri::command]
pub async fn unarchive_stock_out_reason(
    state: State<'_, AppState>,
    id: String,
    locale: Option<String>,
) -> Result<StockOutReasonResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::unarchive_reason(&pool, id)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}
