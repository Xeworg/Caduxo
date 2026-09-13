//! Data Transfer Objects for CSV import/export (Slice 10a).
//!
//! These types define the Tauri IPC boundary for the Slice 10a backend:
//! - `preview_product_csv` — backend preview/classification only (no commit).
//! - `export_products_csv` — canonical products CSV export.
//! - `export_report_csv` — dashboard report CSV export.
//!
//! Mapping modal, import commit, and conflict strategies are deferred to Slice 10b.

use serde::{Deserialize, Serialize};

// ============================================================
// Column mapping
// ============================================================

/// Optional column mapping override. When `None`, the backend detects the
/// mapping from the CSV header row using canonical aliases.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CsvColumnMapping {
    /// Column index (0-based) of the SKU column. Required.
    pub sku: Option<usize>,
    /// Column index (0-based) of the description column. Required.
    pub description: Option<usize>,
    /// Column index (0-based) of the barcode/UPC column. Optional.
    pub barcode: Option<usize>,
    /// Column index (0-based) of the category name column. Optional.
    pub category: Option<usize>,
    /// Column index (0-based) of the default unit column. Optional.
    pub default_unit: Option<usize>,
    /// Column index (0-based) of the default alert-days column. Optional.
    pub default_alert_days_before: Option<usize>,
    /// Column index (0-based) of the notes column. Optional.
    pub notes: Option<usize>,
}

// ============================================================
// Preview
// ============================================================

/// Input for `preview_product_csv`.
#[derive(Debug, Deserialize)]
pub struct CsvPreviewInput {
    /// Raw CSV text including the header row.
    pub content: String,
    /// Optional explicit column mapping. When omitted or partial, missing
    /// fields are auto-detected from the header row.
    pub mapping: Option<CsvColumnMapping>,
}

/// Per-row classification produced by `preview_product_csv`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CsvPreviewRowStatus {
    /// Row is structurally valid and does not collide with existing data.
    Ok,
    /// SKU already exists in the database. `existing_product_id` is the row's
    /// internal id so the UI can show a link / resolve strategy later.
    DuplicateSku {
        existing_product_id: String,
        existing_sku: String,
    },
    /// Barcode already exists in the database.
    DuplicateBarcode {
        existing_product_id: String,
        existing_barcode: String,
    },
    /// A required field is missing or empty.
    MissingRequired { field: String },
    /// A value failed validation (e.g. negative alert days, malformed barcode).
    Invalid { reason: String },
}

/// A single preview row. `raw` mirrors the original cell text per header so
/// the UI can show a faithful preview without re-parsing the CSV.
#[derive(Debug, Clone, Serialize)]
pub struct CsvPreviewRow {
    /// 1-based row index in the source CSV (excluding the header row).
    pub row_index: usize,
    /// Original cell text per header name.
    pub raw: Vec<String>,
    /// Parsed SKU.
    pub sku: Option<String>,
    /// Parsed description.
    pub description: Option<String>,
    /// Parsed barcode (single value; multi-barcode rows not supported here).
    pub barcode: Option<String>,
    /// Parsed category name (not resolved to a category id yet).
    pub category: Option<String>,
    /// Parsed default unit.
    pub default_unit: Option<String>,
    /// Parsed default alert-days value.
    pub default_alert_days_before: Option<i32>,
    /// Parsed notes.
    pub notes: Option<String>,
    /// Classification status.
    pub status: CsvPreviewRowStatus,
}

/// Full preview summary returned by `preview_product_csv`.
#[derive(Debug, Serialize)]
pub struct CsvPreviewResponse {
    /// Detected header names from the source CSV.
    pub headers: Vec<String>,
    /// Mapping used to extract fields (post auto-detection).
    pub mapping_used: CsvColumnMapping,
    /// Total data rows (excluding the header row).
    pub total_rows: usize,
    /// Rows with `Ok` status.
    pub valid_rows: usize,
    /// Rows with any non-Ok status.
    pub invalid_rows: usize,
    /// Rows whose SKU collides with an existing product.
    pub duplicate_sku_count: usize,
    /// Rows whose barcode collides with an existing barcode.
    pub duplicate_barcode_count: usize,
    /// Rows that are missing a required field (SKU or description).
    pub missing_required_count: usize,
    /// Detailed per-row classification.
    pub rows: Vec<CsvPreviewRow>,
}

// ============================================================
// Exports
// ============================================================

/// Input for `export_report_csv`: same shape as `DashboardFilters` plus the
/// destination path. Re-uses the dashboard query so the export reflects the
/// current dashboard report.
#[derive(Debug, Deserialize)]
pub struct ReportExportInput {
    /// Optional store filter.
    pub store_id: Option<String>,
    /// Optional location filter.
    pub location_id: Option<String>,
    /// Optional urgency preset filter.
    pub preset: Option<crate::dto::dashboard::DashboardPreset>,
    /// Optional urgency string filter (snake_case).
    pub urgency: Option<String>,
    /// Destination file path. Must be writable.
    pub path: String,
}

/// Generic export outcome. Returned by both `export_products_csv` and
/// `export_report_csv`.
#[derive(Debug, Serialize)]
pub struct CsvExportResult {
    /// Absolute path the file was written to.
    pub path: String,
    /// Number of data rows written (excluding the header).
    pub rows_written: usize,
    /// Number of bytes written to disk.
    pub bytes_written: u64,
}

// ============================================================
// Import
// ============================================================

/// How to handle rows whose SKU or barcode already exists in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    /// Skip all rows that collide with an existing SKU or barcode.
    Skip,
    /// Update existing products and add barcodes to existing products.
    Update,
    /// Return the list of conflicting rows for manual review without
    /// modifying any data.
    Review,
}

/// Input for `import_product_csv`.
#[derive(Debug, Deserialize)]
pub struct CsvImportInput {
    /// Raw CSV text including the header row.
    pub content: String,
    /// Column mapping (already validated in preview).
    pub mapping: CsvColumnMapping,
    /// How to handle SKU/barcode conflicts.
    pub strategy: ConflictStrategy,
}

/// Outcome of a single imported row.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CsvImportRowOutcome {
    /// Row was successfully created as a new product.
    Created { product_id: String, sku: String },
    /// Row was skipped because the SKU or barcode already exists.
    Skipped { reason: String },
    /// Row was updated (product fields changed, barcodes added).
    Updated { product_id: String, sku: String },
    /// Row was skipped because it was invalid.
    Invalid { reason: String },
}

/// Summary of a single imported row (minimal, for the result table).
#[derive(Debug, Clone, Serialize)]
pub struct CsvImportRowResult {
    /// 1-based row index in the source CSV (excluding the header row).
    pub row_index: usize,
    /// Parsed SKU (or empty when missing).
    pub sku: String,
    /// Parsed description (or empty when missing).
    pub description: String,
    /// Parsed barcode (or empty when missing).
    pub barcode: String,
    /// Outcome.
    pub outcome: CsvImportRowOutcome,
}

/// Result returned by `import_product_csv`.
#[derive(Debug, Serialize)]
pub struct CsvImportResult {
    /// Total data rows processed.
    pub total_rows: usize,
    /// Rows successfully created.
    pub created: usize,
    /// Rows skipped (duplicate or invalid).
    pub skipped: usize,
    /// Rows updated.
    pub updated: usize,
    /// Rows that could not be committed (validation errors).
    pub invalid: usize,
    /// Detailed per-row outcomes (all rows, ordered by row_index).
    pub rows: Vec<CsvImportRowResult>,
}
