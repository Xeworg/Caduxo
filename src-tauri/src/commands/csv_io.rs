//! Tauri commands for CSV import/export (Slice 10a).
//!
//! Backend-only slice: implements preview, export-products, and export-report.
//! Import commit and conflict strategies are deferred to Slice 10b.

use std::path::PathBuf;

use tauri::State;

use crate::dto::csv_io::{CsvExportResult, CsvPreviewInput, CsvPreviewResponse, ReportExportInput};
use crate::error::{AppError, CommandError};
use crate::services::csv_io as service;
use crate::state::AppState;

/// Reads a UTF-8 CSV file from disk and returns its contents. Used by the
/// frontend after a `plugin-dialog` open() to feed the file content into
/// `preview_product_csv`. Keeping the read in Rust avoids adding
/// `@tauri-apps/plugin-fs`.
#[tauri::command]
pub async fn read_csv_text(path: String) -> Result<String, CommandError> {
    let p = PathBuf::from(&path);
    service::read_csv_text(&p).map_err(AppError::into)
}

/// Backend preview for a product CSV. Detects headers, validates the column
/// mapping, classifies each row (duplicate SKU/barcode, missing required,
/// invalid values), and returns a row-by-row summary plus aggregate counts.
/// Does NOT modify the database — preview only.
#[tauri::command]
pub async fn preview_product_csv(
    state: State<'_, AppState>,
    input: CsvPreviewInput,
) -> Result<CsvPreviewResponse, CommandError> {
    service::preview_product_csv(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Writes the canonical products CSV (every product, active + archived) to
/// the given path. Returns the absolute path, the row count, and the byte
/// count written.
#[tauri::command]
pub async fn export_products_csv(
    state: State<'_, AppState>,
    path: String,
) -> Result<CsvExportResult, CommandError> {
    let p = PathBuf::from(&path);
    service::export_products_csv(&state.pool, &p)
        .await
        .map_err(AppError::into)
}

/// Writes the dashboard report CSV to the given path using the same filter
/// shape as `list_dashboard_lots`. Reuses `services::dashboard::get_dashboard`
/// so urgency, sorting, and counts match the dashboard UI.
#[tauri::command]
pub async fn export_report_csv(
    state: State<'_, AppState>,
    input: ReportExportInput,
) -> Result<CsvExportResult, CommandError> {
    service::export_report_csv(&state.pool, input)
        .await
        .map_err(AppError::into)
}
