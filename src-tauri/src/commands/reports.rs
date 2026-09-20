//! Tauri commands for report preview and PDF export (Slice 11a/11b).
//!
//! Thin IPC adapters only. All business logic lives in
//! `services::reports::{preview_report, export_report_pdf}`.

use std::path::PathBuf;

use tauri::State;

use crate::dto::reports::{ReportData, ReportRequest};
use crate::error::{AppError, CommandError};
use crate::services::reports as service;
use crate::state::AppState;

/// Returns a report preview payload (metadata + sorted lot rows) for the given
/// request. Reuses `services::dashboard::get_dashboard` so urgency
/// classification, sort order, and active-lot filtering match the dashboard
/// UI and any future CSV / PDF consumer.
///
/// Reports supported:
///   - `in_alert_window` — lots in their per-lot alert window
///   - `expired` — lots past their expiry date
///   - `next_30_days` — lots expiring within 30 calendar days
///   - `custom` — arbitrary combination of store / location / category /
///     urgency / date-range filters
///
/// The metadata block carries the report type, a human-readable description,
/// the effective filters, the generation timestamp, and the row count so the
/// preview UI / PDF header can render everything in a single round-trip.
#[tauri::command]
pub async fn preview_report(
    state: State<'_, AppState>,
    request: ReportRequest,
    locale: String,
) -> Result<ReportData, CommandError> {
    let loc = crate::pdf::locale::Locale::parse(&locale);
    let pool = state.pool().await;
    service::preview_report(&pool, request, loc)
        .await
        .map_err(AppError::into)
}

/// Renders the same report payload to a PDF file on disk. The PDF rows mirror
/// `preview_report` because both paths run the same data-fetch service.
///
/// `file_path` is the absolute destination path chosen by the user (typically
/// through a native save dialog on the frontend). The file is truncated if it
/// already exists.
#[tauri::command]
pub async fn export_report_pdf(
    state: State<'_, AppState>,
    request: ReportRequest,
    file_path: String,
    locale: String,
) -> Result<crate::pdf::report_pdf::RenderedReport, CommandError> {
    let loc = crate::pdf::locale::Locale::parse(&locale);
    let path = PathBuf::from(&file_path);
    let pool = state.pool().await;
    service::export_report_pdf(&pool, request, path, loc)
        .await
        .map_err(AppError::into)
}
