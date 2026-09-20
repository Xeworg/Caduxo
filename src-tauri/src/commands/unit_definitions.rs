//! Tauri command adapters for the unit definitions catalog.
//!
//! ## Localization note
//!
//! These commands do **not** accept a `locale` argument today, so the
//! backend error boundaries stay on the canonical English path
//! (`CommandError::NotFound`, `CommandError::DuplicateField`,
//! `CommandError::Internal` with their default messages). Wiring
//! `localize_not_found`, `localize_duplicate_field`, and
//! `localize_internal` here would require an IPC signature change for
//! every command in this module, which the active slice deliberately
//! avoids to keep the frontend contract narrow. The helper chain
//! (`localize_validation` / `localize_business_rule`) is also left
//! un-wired for the same reason — the unit-definitions service surfaces
//! `BusinessRule` messages that this slice does not localize. A follow-up
//! slice can introduce an optional `locale: Option<String>` argument per
//! command (matching the pattern used by `commands::products`,
//! `commands::stores`, and `commands::expiry_lots`) without breaking
//! existing callers.

use crate::dto::unit_definitions::{
    UnitAuditBannerState, UnitDefinitionCreateInput, UnitDefinitionRenameInput,
    UnitDefinitionResponse, UnitReviewAction, UnitReviewActionResult, UnrecognizedUnitGroup,
};
use crate::error::CommandError;
use crate::state::AppState;
use tauri::State;

/// Lists all active (non-archived) unit definitions ordered by kind then display_name.
#[tauri::command]
pub async fn list_unit_definitions(
    state: State<'_, AppState>,
) -> Result<Vec<UnitDefinitionResponse>, CommandError> {
    let pool = state.pool().await;
    crate::services::unit_definitions::list_active(&pool)
        .await
        .map_err(Into::into)
}

/// Creates a new custom unit. The key must be unique (case-insensitive) and
/// lowercase alphanumeric with optional hyphen/underscore (max 16 chars).
#[tauri::command]
pub async fn create_unit_definition(
    state: State<'_, AppState>,
    input: UnitDefinitionCreateInput,
) -> Result<UnitDefinitionResponse, CommandError> {
    let pool = state.pool().await;
    crate::services::unit_definitions::create_custom_unit(&pool, input)
        .await
        .map_err(Into::into)
}

/// Renames a unit's display_name. The key is immutable in this slice.
#[tauri::command]
pub async fn rename_unit_definition(
    state: State<'_, AppState>,
    input: UnitDefinitionRenameInput,
) -> Result<UnitDefinitionResponse, CommandError> {
    let pool = state.pool().await;
    crate::services::unit_definitions::rename_unit(&pool, input)
        .await
        .map_err(Into::into)
}

/// Returns all unrecognized unit groups: products with no catalog link but a
/// non-empty `default_unit` text value.
#[tauri::command]
pub async fn list_unrecognized_units(
    state: State<'_, AppState>,
) -> Result<Vec<UnrecognizedUnitGroup>, CommandError> {
    let pool = state.pool().await;
    crate::services::unit_audit::unrecognized_units(&pool)
        .await
        .map_err(Into::into)
}

/// Returns the banner visibility state: whether the banner should be shown,
/// the current signature, and the count of unrecognized groups.
#[tauri::command]
pub async fn unit_audit_banner_state(
    state: State<'_, AppState>,
) -> Result<UnitAuditBannerState, CommandError> {
    let pool = state.pool().await;
    crate::services::unit_audit::banner_state(&pool)
        .await
        .map_err(Into::into)
}

/// Dismisses the audit banner by persisting the current signature.
/// The banner stays hidden until a new unrecognized unit is introduced.
#[tauri::command]
pub async fn dismiss_unit_audit_banner(state: State<'_, AppState>) -> Result<(), CommandError> {
    let pool = state.pool().await;
    crate::services::unit_audit::dismiss_banner(&pool)
        .await
        .map_err(Into::into)
}

/// Applies a review action from the UnitReviewPage: map-to-preset,
/// keep-as-custom, or leave-for-later.
#[tauri::command]
pub async fn apply_unit_review_action(
    state: State<'_, AppState>,
    action: UnitReviewAction,
) -> Result<UnitReviewActionResult, CommandError> {
    let pool = state.pool().await;
    crate::services::unit_audit::apply_review_action(&pool, action)
        .await
        .map_err(Into::into)
}
