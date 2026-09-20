//! Health-check command for verifying the backend is reachable.
//!
//! ## Localization note
//!
//! The `Database unreachable` text returned from the `health_check`
//! command is a constant developer-facing string, not a localized user
//! message. The frontend does not display it as a user-facing notice;
//! it is the raw payload a `ping_db` failure surfaces for debugging. A
//! future slice can route the error through `CommandError::Internal` via
//! `localize_internal` if the UI starts rendering it; the active slice
//! leaves the constant English intact to avoid silent behaviour changes
//! in the IPC payload.

use crate::error::CommandError;
use crate::state::AppState;
use tauri::State;

/// Returns a simple health payload confirming the backend is running.
#[tauri::command]
pub async fn health_check(state: State<'_, AppState>) -> Result<HealthResponse, CommandError> {
    let db_alive = state.ping_db().await.map_err(|e| {
        tracing::warn!(error = %e, "Health check: database ping failed");
        CommandError::Internal {
            message: "Database unreachable".to_string(),
        }
    })?;

    Ok(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_alive,
    })
}

#[derive(serde::Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub db_alive: bool,
}
