//! Tauri command adapters for the unit definitions catalog.
//!
//! ## Localization note
//!
//! These commands accept an optional `locale: Option<String>` argument (BCP-47
//! tag) so backend error-boundary localization chains can rewrite the
//! user-facing `Validation`, `BusinessRule`, `NotFound`, `DuplicateField`,
//! and `Infrastructure` errors into the active locale. Unknown tags fall
//! back to English via `Locale::parse`; omitting the argument keeps English
//! to preserve backward compatibility with callers that have not yet
//! been updated.

use crate::dto::unit_definitions::{
    UnitAuditBannerState, UnitDefinitionCreateInput, UnitDefinitionRenameInput,
    UnitDefinitionResponse, UnitReviewAction, UnitReviewActionResult, UnrecognizedUnitGroup,
};
use crate::error::{AppError, CommandError};
use crate::pdf::locale::Locale;
use crate::services::unit_audit;
use crate::services::unit_definitions as service;
use crate::services::user_messages::{
    localize_business_rule, localize_duplicate_field, localize_internal, localize_not_found,
    localize_validation,
};
use crate::state::AppState;
use tauri::State;

/// Resolves an optional BCP-47 locale tag into a [`Locale`], falling back to
/// English when the frontend does not supply one. Mirrors the helper used
/// in `commands::products` and `commands::expiry_lots`.
fn resolve_locale(locale: Option<String>) -> Locale {
    locale.as_deref().map(Locale::parse).unwrap_or(Locale::En)
}

/// Lists all active (non-archived) unit definitions ordered by kind then display_name.
#[tauri::command]
pub async fn list_unit_definitions(
    state: State<'_, AppState>,
) -> Result<Vec<UnitDefinitionResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_active(&pool)
        .await
        .map_err(Into::into)
}

/// Creates a new custom unit. The key must be unique (case-insensitive) and
/// lowercase alphanumeric with optional hyphen/underscore (max 16 chars).
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_validation`
/// and `localize_duplicate_field` so the key / display-name validation and
/// the duplicate-key boundary reach the UI in the active locale. Unknown
/// tags fall back to English via `Locale::parse`. Omitting the argument
/// keeps English for backward compatibility.
#[tauri::command]
pub async fn create_unit_definition(
    state: State<'_, AppState>,
    input: UnitDefinitionCreateInput,
    locale: Option<String>,
) -> Result<UnitDefinitionResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::create_custom_unit(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Renames a unit's display_name. The key is immutable in this slice.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_validation`
/// and `localize_not_found` so the display-name validation and the
/// unknown-id boundary reach the UI in the active locale. Unknown tags
/// fall back to English via `Locale::parse`. Omitting the argument keeps
/// English for backward compatibility.
#[tauri::command]
pub async fn rename_unit_definition(
    state: State<'_, AppState>,
    input: UnitDefinitionRenameInput,
    locale: Option<String>,
) -> Result<UnitDefinitionResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::rename_unit(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Returns all unrecognized unit groups: products with no catalog link but a
/// non-empty `default_unit` text value.
#[tauri::command]
pub async fn list_unrecognized_units(
    state: State<'_, AppState>,
) -> Result<Vec<UnrecognizedUnitGroup>, CommandError> {
    let pool = state.pool().await;
    unit_audit::unrecognized_units(&pool)
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
    unit_audit::banner_state(&pool)
        .await
        .map_err(Into::into)
}

/// Dismisses the audit banner by persisting the current signature.
/// The banner stays hidden until a new unrecognized unit is introduced.
#[tauri::command]
pub async fn dismiss_unit_audit_banner(state: State<'_, AppState>) -> Result<(), CommandError> {
    let pool = state.pool().await;
    unit_audit::dismiss_banner(&pool)
        .await
        .map_err(Into::into)
}

/// Applies a review action from the UnitReviewPage: map-to-preset,
/// keep-as-custom, or leave-for-later.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_not_found`
/// so the unknown-preset-id boundary raised by `map_to_preset` reaches
/// the UI in the active locale. Unknown tags fall back to English via
/// `Locale::parse`. Omitting the argument keeps English for backward
/// compatibility.
#[tauri::command]
pub async fn apply_unit_review_action(
    state: State<'_, AppState>,
    action: UnitReviewAction,
    locale: Option<String>,
) -> Result<UnitReviewActionResult, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    unit_audit::apply_review_action(&pool, action)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}
