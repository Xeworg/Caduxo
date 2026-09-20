//! Tauri commands for the local notification pipeline.
//!
//! ## Localization note
//!
//! These commands accept an optional `locale: Option<String>` argument
//! (BCP-47 tag) so backend error-boundary localization chains can rewrite
//! the user-facing `Validation`, `NotFound`, and `Infrastructure` errors
//! into the active locale. Unknown tags fall back to English via
//! `Locale::parse`; omitting the argument keeps English to preserve
//! backward compatibility with callers that have not yet been updated.
//!
//! OS notification copy, scheduling, and the developer-facing health
//! command are intentionally untouched by this slice.
//!
//! These commands are thin adapters over `services::notifications`. They are
//! designed for later frontend wiring once the notification permission flow
//! lands; the backend contract is stable today so the UI can be built against
//! these IPC shapes without further backend changes.
//!
//! - `list_due_notifications(today?, locale?)` — returns enriched candidate
//!   rows whose alert window contains `today`, excluding any lot already
//!   logged for `today`. Defaults to today (UTC) when `today` is omitted or
//!   blank.
//! - `mark_notification_shown(input, locale?)` — records `notification_log`
//!   so the lot is not surfaced again for the same day. Idempotent. Returns
//!   `NotFound` for unknown lot ids.

use tauri::State;

use crate::dto::notifications::{
    DueNotificationLot, MarkNotificationShownInput, NotificationLogResponse,
};
use crate::error::{AppError, CommandError};
use crate::pdf::locale::Locale;
use crate::services::notifications as service;
use crate::services::user_messages::{
    localize_business_rule, localize_duplicate_field, localize_internal, localize_not_found,
    localize_validation,
};
use crate::state::AppState;

/// Resolves an optional BCP-47 locale tag into a [`Locale`], falling back to
/// English when the frontend does not supply one. Mirrors the helper used
/// in `commands::products` and `commands::expiry_lots`.
fn resolve_locale(locale: Option<String>) -> Locale {
    locale.as_deref().map(Locale::parse).unwrap_or(Locale::En)
}

/// Returns active lots whose alert window contains `today`, enriched with
/// product, store, and location context, excluding any lot already logged
/// for `today`. When `today` is omitted (or blank), the current UTC date is
/// used.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_validation`
/// so the strict `YYYY-MM-DD` `notification_date` validation raised by the
/// service reaches the UI in the active locale. Unknown tags fall back to
/// English via `Locale::parse`. Omitting the argument keeps English for
/// backward compatibility.
#[tauri::command]
pub async fn list_due_notifications(
    state: State<'_, AppState>,
    today: Option<String>,
    locale: Option<String>,
) -> Result<Vec<DueNotificationLot>, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::list_due_notifications(&pool, today)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}

/// Records that a notification was shown for `(expiry_lot_id, notification_date)`.
/// Idempotent: repeat calls for the same pair return the same row without
/// raising an error. `notification_date` defaults to today (UTC) when omitted.
/// Returns `NotFound` for unknown lot ids.
///
/// `locale` (BCP-47 tag, optional) is forwarded to `localize_validation`
/// and `localize_not_found` so the strict `YYYY-MM-DD` `notification_date`
/// validation and the unknown-lot-id boundary raised by the service reach
/// the UI in the active locale. Unknown tags fall back to English via
/// `Locale::parse`. Omitting the argument keeps English for backward
/// compatibility.
#[tauri::command]
pub async fn mark_notification_shown(
    state: State<'_, AppState>,
    input: MarkNotificationShownInput,
    locale: Option<String>,
) -> Result<NotificationLogResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::mark_notification_shown(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(|e| localize_business_rule(e, loc))
        .map_err(|e| localize_not_found(e, loc))
        .map_err(|e| localize_duplicate_field(e, loc))
        .map_err(|e| localize_internal(e, loc))
        .map_err(AppError::into)
}
