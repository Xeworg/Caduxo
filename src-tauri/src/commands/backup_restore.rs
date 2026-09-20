//! Tauri commands for backup and restore operations (Slice 12).

use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::dto::backup_restore::{BackupResult, RestoreInput, RestoreResult, RestoreValidation};
use crate::error::{AppError, CommandError};
use crate::pdf::locale::Locale;
use crate::services::backup_restore as service;
use crate::services::user_messages::localize_business_rule;
use crate::state::AppState;

/// Resolves an optional BCP-47 locale tag into a [`Locale`], falling back to
/// English when the frontend does not supply one. Used at the restore
/// command boundary so the simple constant BusinessRule rejection for a
/// missing user confirmation reaches the UI in the active locale. Mirrors
/// the helpers in `commands::products` and `commands::stores`.
fn resolve_locale(locale: Option<String>) -> Locale {
    locale.as_deref().map(Locale::parse).unwrap_or(Locale::En)
}

/// Exports a copy of the active database to `destination` and returns the path,
/// byte count, and schema version at the time of backup.
#[tauri::command]
pub async fn export_backup(
    state: State<'_, AppState>,
    destination: String,
) -> Result<BackupResult, CommandError> {
    let dest = PathBuf::from(&destination);
    let active_db_path = state.pool_path();
    service::export_backup(&active_db_path, &dest)
        .await
        .map_err(AppError::into)
}

/// Validates a backup file before a restore. Performs header check, schema
/// verification, and bounded integrity check without modifying the backup file.
#[tauri::command]
pub async fn validate_backup(backup_path: String) -> Result<RestoreValidation, CommandError> {
    let path = PathBuf::from(&backup_path);
    service::validate_backup(&path)
        .await
        .map_err(AppError::into)
}

/// Restores the database from a validated backup file.
///
/// Requires `confirmed: true` in the input to proceed. This is the explicit
/// destructive-confirmation gate.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_business_rule` so the
/// simple constant BusinessRule rejection for missing user confirmation
/// reaches the UI in the active locale. Unknown tags fall back to English
/// via `Locale::parse`. Callers that omit the argument keep their existing
/// behavior — the optional parameter is fully backwards compatible.
///
/// After a successful restore the application's pool is reopened with the
/// restored database and the frontend should refresh all data views.
#[tauri::command]
pub async fn restore_backup(
    state: State<'_, AppState>,
    input: RestoreInput,
    locale: Option<String>,
) -> Result<RestoreResult, CommandError> {
    let loc = resolve_locale(locale);
    let active_db_path = state.pool_path();
    service::restore_backup(Arc::clone(&state.pool), &active_db_path, input)
        .await
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(AppError::into)
}
