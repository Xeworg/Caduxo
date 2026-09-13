//! Tauri commands for backup and restore operations (Slice 12).

use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::dto::backup_restore::{BackupResult, RestoreInput, RestoreResult, RestoreValidation};
use crate::error::{AppError, CommandError};
use crate::services::backup_restore as service;
use crate::state::AppState;

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
/// After a successful restore the application's pool is reopened with the
/// restored database and the frontend should refresh all data views.
#[tauri::command]
pub async fn restore_backup(
    state: State<'_, AppState>,
    input: RestoreInput,
) -> Result<RestoreResult, CommandError> {
    let active_db_path = state.pool_path();
    service::restore_backup(Arc::clone(&state.pool), &active_db_path, input)
        .await
        .map_err(AppError::into)
}
