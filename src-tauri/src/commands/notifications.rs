//! Tauri commands for the local notification pipeline.
//!
//! ## Localization note
//!
//! These commands do **not** accept a `locale` argument today, so the
//! backend error boundaries stay on the canonical English path
//! (`CommandError::NotFound` for unknown lot ids, `CommandError::Internal`
//! for infrastructure errors). Wiring `localize_not_found` and
//! `localize_internal` here would require an IPC signature change for
//! every command in this module, which the active slice deliberately
//! avoids to keep the frontend contract narrow. A follow-up slice can
//! introduce an optional `locale: Option<String>` argument per command
//! (matching the pattern used by `commands::products`, `commands::stores`,
//! and `commands::expiry_lots`) without breaking existing callers.
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
    let pool = state.pool().await;
    service::list_due_notifications(&pool, today)
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
    let pool = state.pool().await;
    service::mark_notification_shown(&pool, input)
        .await
        .map_err(AppError::into)
}
