//! Tauri commands for the local notification pipeline.
//!
//! These commands are thin adapters over `services::notifications`. They are
//! designed for later frontend wiring once the notification permission flow
//! lands; the backend contract is stable today so the UI can be built against
//! these IPC shapes without further backend changes.
//!
//! - `list_due_notifications(today?)` — returns enriched candidate rows whose
//!   alert window contains `today`, excluding any lot already logged for
//!   `today`. Defaults to today (UTC) when `today` is omitted or blank.
//! - `mark_notification_shown(input)` — records `notification_log` so the lot
//!   is not surfaced again for the same day. Idempotent. Returns
//!   `NotFound` for unknown lot ids.

use tauri::State;

use crate::dto::notifications::{
    DueNotificationLot, MarkNotificationShownInput, NotificationLogResponse,
};
use crate::error::{AppError, CommandError};
use crate::services::notifications as service;
use crate::state::AppState;

/// Returns active lots whose alert window contains `today`, enriched with
/// product, store, and location context, excluding any lot already logged
/// for `today`. When `today` is omitted (or blank), the current UTC date is
/// used.
#[tauri::command]
pub async fn list_due_notifications(
    state: State<'_, AppState>,
    today: Option<String>,
) -> Result<Vec<DueNotificationLot>, CommandError> {
    service::list_due_notifications(&state.pool, today)
        .await
        .map_err(AppError::into)
}

/// Records that a notification was shown for `(expiry_lot_id, notification_date)`.
/// Idempotent: repeat calls for the same pair return the same row without
/// raising an error. `notification_date` defaults to today (UTC) when omitted.
/// Returns `NotFound` for unknown lot ids.
#[tauri::command]
pub async fn mark_notification_shown(
    state: State<'_, AppState>,
    input: MarkNotificationShownInput,
) -> Result<NotificationLogResponse, CommandError> {
    service::mark_notification_shown(&state.pool, input)
        .await
        .map_err(AppError::into)
}
