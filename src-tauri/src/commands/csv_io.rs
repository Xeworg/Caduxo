//! Tauri commands for CSV import/export.
//!
//! Slice 10a: backend preview and exports only.
//! Slice 10b: import commit with conflict strategies (skip, update, review).

use std::path::PathBuf;

use tauri::State;

use crate::dto::csv_io::{
    CsvExportResult, CsvImportInput, CsvImportResult, CsvPreviewInput, CsvPreviewResponse,
    ReportExportInput,
};
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
    let pool = state.pool().await;
    service::preview_product_csv(&pool, input)
        .await
        .map_err(AppError::into)
}

/// Commits a product CSV import using the provided column mapping and conflict
/// strategy. Creates new products (and their barcodes), updates existing products
/// when the strategy is `Update`, or returns conflict rows for review without
/// making any database changes when the strategy is `Review`.
#[tauri::command]
pub async fn import_product_csv(
    state: State<'_, AppState>,
    input: CsvImportInput,
) -> Result<CsvImportResult, CommandError> {
    let pool = state.pool().await;
    service::import_product_csv(&pool, input)
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
    let pool = state.pool().await;
    service::export_products_csv(&pool, &p)
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
    let pool = state.pool().await;
    service::export_report_csv(&pool, input)
        .await
        .map_err(AppError::into)
}
