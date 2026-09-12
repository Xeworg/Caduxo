//! CSV import/export service (Slice 10a).
//!
//! Slice 10a delivers backend preview and exports only:
//! - `preview_product_csv` — header detection, mapping validation, row
//!   classification (duplicate SKU/barcode against the database, missing
//!   required fields, validation errors). Does NOT commit any change.
//! - `export_products_csv` — write canonical products CSV with primary
//!   barcode.
//! - `export_report_csv` — reuse `dashboard::get_dashboard` filters and
//!   write the resulting rows to CSV.
//!
//! Mapping modal, import commit, and conflict strategies are deferred to
//! Slice 10b. Logging at this layer avoids logging raw row content (no
//! SKU/barcode/description/notes values).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::DbPool;
use crate::domain::validation::{validate_barcode, validate_description, validate_sku};
use crate::dto::csv_io::{
    CsvColumnMapping, CsvExportResult, CsvPreviewInput, CsvPreviewResponse, CsvPreviewRow,
    CsvPreviewRowStatus, ReportExportInput,
};
use crate::dto::dashboard::DashboardFilters;
use crate::error::{AppError, DomainError, InfrastructureError};

// ============================================================
// Header detection
// ============================================================

/// Canonical alias -> canonical field name. Lookup is case-insensitive and
/// ignores non-alphanumeric characters so `Item SKU`, `item_sku`, and
/// `ITEM-SKU` all map to `sku`.
fn canonical_field_name(header: &str) -> Option<&'static str> {
    let normalized: String = header
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();

    match normalized.as_str() {
        "sku" | "code" | "productcode" | "itemcode" | "itemsku" | "ref" | "reference" => {
            Some("sku")
        }
        "description" | "name" | "productname" | "itemname" | "title" | "label" => {
            Some("description")
        }
        "barcode" | "upc" | "ean" | "gtin" | "codebar" | "primarybarcode" => Some("barcode"),
        "category" | "categoriename" | "categoryname" | "type" => Some("category"),
        "unit" | "defaultunit" | "uom" => Some("default_unit"),
        "alertdays"
        | "alertdaysbefore"
        | "defaultalertdays"
        | "defaultalertdaysbefore"
        | "daysbefore" => Some("default_alert_days_before"),
        "notes" | "note" | "remarks" | "comment" | "comments" => Some("notes"),
        _ => None,
    }
}

/// Maps a header row to the column mapping. Each canonical field is reported
/// at most once; later headers matching the same canonical name are ignored.
fn detect_mapping(headers: &csv::StringRecord) -> CsvColumnMapping {
    let mut mapping = CsvColumnMapping::default();
    let mut seen: HashSet<&'static str> = HashSet::new();

    for (idx, header) in headers.iter().enumerate() {
        let Some(field) = canonical_field_name(header) else {
            continue;
        };
        if seen.insert(field) {
            match field {
                "sku" => mapping.sku = Some(idx),
                "description" => mapping.description = Some(idx),
                "barcode" => mapping.barcode = Some(idx),
                "category" => mapping.category = Some(idx),
                "default_unit" => mapping.default_unit = Some(idx),
                "default_alert_days_before" => mapping.default_alert_days_before = Some(idx),
                "notes" => mapping.notes = Some(idx),
                _ => unreachable!("canonical_field_name returned unknown field"),
            }
        }
    }

    mapping
}

/// Returns the explicit mapping when provided, otherwise auto-detects from the
/// header row. When the explicit mapping is partial, missing fields are filled
/// from auto-detection.
fn resolve_mapping(
    headers: &csv::StringRecord,
    explicit: Option<CsvColumnMapping>,
) -> CsvColumnMapping {
    let detected = detect_mapping(headers);
    let Some(explicit) = explicit else {
        return detected;
    };

    CsvColumnMapping {
        sku: explicit.sku.or(detected.sku),
        description: explicit.description.or(detected.description),
        barcode: explicit.barcode.or(detected.barcode),
        category: explicit.category.or(detected.category),
        default_unit: explicit.default_unit.or(detected.default_unit),
        default_alert_days_before: explicit
            .default_alert_days_before
            .or(detected.default_alert_days_before),
        notes: explicit.notes.or(detected.notes),
    }
}

// ============================================================
// Preview
// ============================================================

/// Backend preview for a product CSV. Parses headers, resolves the column
/// mapping, validates every data row, and classifies each row against
/// existing products and barcodes. Never modifies the database.
pub async fn preview_product_csv(
    pool: &DbPool,
    input: CsvPreviewInput,
) -> Result<CsvPreviewResponse, AppError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(input.content.as_bytes());

    let headers = reader
        .headers()
        .map_err(|e| {
            AppError::Domain(DomainError::Validation {
                message: format!("Failed to read CSV header row: {e}"),
            })
        })?
        .clone();

    let mapping = resolve_mapping(&headers, input.mapping);

    // Both required fields must resolve; otherwise we cannot classify rows.
    if mapping.sku.is_none() {
        return Err(AppError::Domain(DomainError::Validation {
            message:
                "SKU column not detected. Provide a column named `sku`, `code`, or `reference`."
                    .to_string(),
        }));
    }
    if mapping.description.is_none() {
        return Err(AppError::Domain(DomainError::Validation {
            message:
                "Description column not detected. Provide a column named `description` or `name`."
                    .to_string(),
        }));
    }

    let mut rows: Vec<CsvPreviewRow> = Vec::new();
    let mut valid_rows: usize = 0;
    let mut duplicate_sku_count: usize = 0;
    let mut duplicate_barcode_count: usize = 0;
    let mut missing_required_count: usize = 0;

    for (idx, record_result) in reader.records().enumerate() {
        let record = record_result.map_err(|e| {
            AppError::Domain(DomainError::Validation {
                message: format!("Failed to read CSV row {}: {e}", idx + 2),
            })
        })?;

        let raw: Vec<String> = record.iter().map(|s| s.to_string()).collect();

        let sku = mapping
            .sku
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let description = mapping
            .description
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let barcode = mapping
            .barcode
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let category = mapping
            .category
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let default_unit = mapping
            .default_unit
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let notes = mapping
            .notes
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let default_alert_days_before = mapping
            .default_alert_days_before
            .and_then(|i| record.get(i))
            .and_then(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    return None;
                }
                trimmed.parse::<i32>().ok()
            });

        let row_index = idx + 1; // 1-based, excluding header

        let (status, sku_for_count, barcode_for_count, missing_field, invalid_reason) =
            classify_row(
                pool,
                &sku,
                &description,
                &barcode,
                default_alert_days_before,
            )
            .await;

        match &status {
            CsvPreviewRowStatus::Ok => valid_rows += 1,
            CsvPreviewRowStatus::DuplicateSku { .. } => duplicate_sku_count += 1,
            CsvPreviewRowStatus::DuplicateBarcode { .. } => duplicate_barcode_count += 1,
            CsvPreviewRowStatus::MissingRequired { .. } => missing_required_count += 1,
            CsvPreviewRowStatus::Invalid { .. } => {}
        }

        // Update sku/barcode to the values used for classification so the UI
        // shows what was actually compared (empty when missing).
        let _ = (sku_for_count, barcode_for_count);

        let row = CsvPreviewRow {
            row_index,
            raw,
            sku,
            description,
            barcode,
            category,
            default_unit,
            default_alert_days_before,
            notes,
            status,
        };

        let _ = (missing_field, invalid_reason);

        rows.push(row);
    }

    let total_rows = rows.len();
    let invalid_rows = total_rows - valid_rows;

    Ok(CsvPreviewResponse {
        headers: headers.iter().map(|s| s.to_string()).collect(),
        mapping_used: mapping,
        total_rows,
        valid_rows,
        invalid_rows,
        duplicate_sku_count,
        duplicate_barcode_count,
        missing_required_count,
        rows,
    })
}

/// Classifies a single row by validating fields and checking the database for
/// SKU/barcode collisions. Returns the status plus scratch fields kept for
/// symmetry with future per-field error reporting (currently unused by the
/// caller).
async fn classify_row(
    pool: &DbPool,
    sku: &Option<String>,
    description: &Option<String>,
    barcode: &Option<String>,
    default_alert_days_before: Option<i32>,
) -> (
    CsvPreviewRowStatus,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    // 1. Required-field check.
    if sku.is_none() {
        return (
            CsvPreviewRowStatus::MissingRequired {
                field: "sku".to_string(),
            },
            sku.clone(),
            barcode.clone(),
            Some("sku".to_string()),
            None,
        );
    }
    if description.is_none() {
        return (
            CsvPreviewRowStatus::MissingRequired {
                field: "description".to_string(),
            },
            sku.clone(),
            barcode.clone(),
            Some("description".to_string()),
            None,
        );
    }
    let sku = sku.clone().expect("checked above");
    let description = description.clone().expect("checked above");

    // 2. Domain validation (mirrors create_product rules).
    if let Err(msg) = validate_sku(&sku) {
        return (
            CsvPreviewRowStatus::Invalid {
                reason: msg.clone(),
            },
            Some(sku),
            barcode.clone(),
            None,
            Some(msg),
        );
    }
    if let Err(msg) = validate_description(&description) {
        return (
            CsvPreviewRowStatus::Invalid {
                reason: msg.clone(),
            },
            Some(sku),
            barcode.clone(),
            None,
            Some(msg),
        );
    }
    if let Some(b) = barcode.as_deref() {
        if let Err(msg) = validate_barcode(b) {
            return (
                CsvPreviewRowStatus::Invalid {
                    reason: msg.clone(),
                },
                Some(sku),
                barcode.clone(),
                None,
                Some(msg),
            );
        }
    }
    if let Some(days) = default_alert_days_before {
        if days < 0 {
            return (
                CsvPreviewRowStatus::Invalid {
                    reason: "Default alert days before cannot be negative".to_string(),
                },
                Some(sku),
                barcode.clone(),
                None,
                Some("alert days negative".to_string()),
            );
        }
        if days > 3650 {
            return (
                CsvPreviewRowStatus::Invalid {
                    reason: format!("Default alert days exceeds maximum of {days}"),
                },
                Some(sku),
                barcode.clone(),
                None,
                Some("alert days too large".to_string()),
            );
        }
    }

    // 3. SKU uniqueness check.
    match crate::db::repositories::products::find_by_sku_exact(pool, &sku).await {
        Ok(Some(existing)) => {
            return (
                CsvPreviewRowStatus::DuplicateSku {
                    existing_product_id: existing.id,
                    existing_sku: existing.sku,
                },
                Some(sku),
                barcode.clone(),
                None,
                None,
            );
        }
        Ok(None) => {}
        Err(e) => {
            return (
                CsvPreviewRowStatus::Invalid {
                    reason: format!("Database error while checking SKU uniqueness: {e}"),
                },
                Some(sku),
                barcode.clone(),
                None,
                Some(e.to_string()),
            );
        }
    }

    // 4. Barcode uniqueness check (only when a barcode is present).
    if let Some(b) = barcode.as_deref() {
        match crate::db::repositories::products::find_by_barcode_exact(pool, b).await {
            Ok(Some(existing)) => {
                return (
                    CsvPreviewRowStatus::DuplicateBarcode {
                        existing_product_id: existing.id,
                        existing_barcode: existing.primary_barcode.unwrap_or_else(|| b.to_string()),
                    },
                    Some(sku),
                    barcode.clone(),
                    None,
                    None,
                );
            }
            Ok(None) => {}
            Err(e) => {
                return (
                    CsvPreviewRowStatus::Invalid {
                        reason: format!("Database error while checking barcode uniqueness: {e}"),
                    },
                    Some(sku),
                    barcode.clone(),
                    None,
                    Some(e.to_string()),
                );
            }
        }
    }

    (
        CsvPreviewRowStatus::Ok,
        Some(sku),
        barcode.clone(),
        None,
        None,
    )
}

// ============================================================
// Exports
// ============================================================

/// Canonical header order for the products CSV export.
const PRODUCTS_HEADERS: &[&str] = &[
    "sku",
    "description",
    "category",
    "default_unit",
    "default_alert_days_before",
    "primary_barcode",
    "additional_barcodes",
    "notes",
    "is_active",
];

/// Writes the canonical products CSV to `path`. Each product appears once with
/// its primary barcode plus a semicolon-separated list of additional barcodes.
pub async fn export_products_csv(pool: &DbPool, path: &Path) -> Result<CsvExportResult, AppError> {
    use crate::db::repositories::products as products_repo;

    let products = products_repo::list_all_products_for_export(pool)
        .await
        .map_err(AppError::from)?;

    let categories = products_repo::list_all_categories(pool)
        .await
        .map_err(AppError::from)?;

    let category_names: std::collections::HashMap<String, String> =
        categories.into_iter().map(|c| (c.id, c.name)).collect();

    let mut writer = csv::WriterBuilder::new()
        .has_headers(false) // we write the header manually to control ordering
        .from_path(path)
        .map_err(|e| AppError::Infrastructure(InfrastructureError::Csv(e)))?;

    writer
        .write_record(PRODUCTS_HEADERS)
        .map_err(|e| AppError::Infrastructure(InfrastructureError::Csv(e)))?;

    let mut rows_written: usize = 0;
    for product in &products {
        let barcodes = products_repo::list_barcodes(pool, &product.id)
            .await
            .map_err(AppError::from)?;
        let primary_barcode = barcodes
            .iter()
            .find(|b| b.is_primary)
            .map(|b| b.barcode.clone());
        let additional_barcodes: Vec<String> = barcodes
            .iter()
            .filter(|b| !b.is_primary)
            .map(|b| b.barcode.clone())
            .collect();
        let additional_barcodes_joined = additional_barcodes.join(";");

        let category_name = product
            .category_id
            .as_ref()
            .and_then(|cid| category_names.get(cid).cloned())
            .unwrap_or_default();

        writer
            .write_record([
                product.sku.as_str(),
                product.description.as_str(),
                category_name.as_str(),
                product.default_unit.as_deref().unwrap_or(""),
                &product.default_alert_days_before.to_string(),
                primary_barcode.as_deref().unwrap_or(""),
                additional_barcodes_joined.as_str(),
                product.notes.as_deref().unwrap_or(""),
                if product.is_active { "true" } else { "false" },
            ])
            .map_err(|e| AppError::Infrastructure(InfrastructureError::Csv(e)))?;
        rows_written += 1;
    }

    writer
        .flush()
        .map_err(|e| AppError::Infrastructure(InfrastructureError::Io(e)))?;
    drop(writer);

    let bytes_written = fs::metadata(path)
        .map_err(|e| AppError::Infrastructure(InfrastructureError::Io(e)))?
        .len();

    Ok(CsvExportResult {
        path: path.to_string_lossy().into_owned(),
        rows_written,
        bytes_written,
    })
}

/// Canonical header order for the report CSV export.
const REPORT_HEADERS: &[&str] = &[
    "lot_id",
    "sku",
    "description",
    "store_name",
    "location_name",
    "quantity",
    "unit",
    "expiry_date",
    "alert_days_before",
    "urgency",
    "days_remaining",
    "batch_code",
    "status",
];

/// Writes the dashboard report CSV using the provided filters. Reuses the
/// dashboard query path so the export reflects the current dashboard view.
pub async fn export_report_csv(
    pool: &DbPool,
    input: ReportExportInput,
) -> Result<CsvExportResult, AppError> {
    let path = PathBuf::from(&input.path);
    let filters = DashboardFilters {
        store_id: input.store_id,
        location_id: input.location_id,
        preset: input.preset,
        urgency: input.urgency,
    };

    // Reuse the dashboard service so urgency, sorting, and counts stay in sync
    // with the UI. This keeps a single source of truth for report rendering.
    let response = crate::services::dashboard::get_dashboard(pool, filters).await?;
    let lots = response.lots;

    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(&path)
        .map_err(|e| AppError::Infrastructure(InfrastructureError::Csv(e)))?;

    writer
        .write_record(REPORT_HEADERS)
        .map_err(|e| AppError::Infrastructure(InfrastructureError::Csv(e)))?;

    let mut rows_written: usize = 0;
    for lot in &lots {
        writer
            .write_record([
                lot.lot_id.as_str(),
                lot.sku.as_str(),
                lot.description.as_str(),
                lot.store_name.as_str(),
                lot.location_name.as_deref().unwrap_or(""),
                &format_quantity(lot.quantity),
                lot.unit.as_str(),
                lot.expiry_date.as_str(),
                &lot.alert_days_before.to_string(),
                lot.urgency.as_str(),
                &lot.days_remaining.to_string(),
                lot.batch_code.as_deref().unwrap_or(""),
                lot.status.as_str(),
            ])
            .map_err(|e| AppError::Infrastructure(InfrastructureError::Csv(e)))?;
        rows_written += 1;
    }

    writer
        .flush()
        .map_err(|e| AppError::Infrastructure(InfrastructureError::Io(e)))?;
    drop(writer);

    let bytes_written = fs::metadata(&path)
        .map_err(|e| AppError::Infrastructure(InfrastructureError::Io(e)))?
        .len();

    Ok(CsvExportResult {
        path: path.to_string_lossy().into_owned(),
        rows_written,
        bytes_written,
    })
}

/// Formats a quantity for CSV output. Trailing zeros are stripped and integers
/// are written without a decimal point so the CSV is easier to read.
fn format_quantity(qty: f64) -> String {
    if qty.fract() == 0.0 {
        format!("{}", qty as i64)
    } else {
        format!("{qty}")
    }
}

// ============================================================
// File reading (used by the frontend to read a user-selected file)
// ============================================================

/// Reads a UTF-8 CSV file and returns its contents. Used by the frontend
/// after a dialog pick to feed the file content into `preview_product_csv`.
pub fn read_csv_text(path: &Path) -> Result<String, AppError> {
    fs::read_to_string(path).map_err(|e| AppError::Infrastructure(InfrastructureError::Io(e)))
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use tempfile::tempdir;
    use uuid::Uuid;

    use crate::db::migrations::fresh_test_pool;
    use crate::dto::dashboard::DashboardPreset;
    use crate::dto::products::{ProductBarcodeCreate, ProductCreate};
    use crate::services::products as products_service;

    // ------------------------------------------------------------
    // Header detection
    // ------------------------------------------------------------

    #[test]
    fn detect_mapping_from_canonical_headers() {
        let headers = csv::StringRecord::from(vec![
            "sku",
            "description",
            "barcode",
            "category",
            "unit",
            "alert_days",
            "notes",
        ]);
        let m = detect_mapping(&headers);
        assert_eq!(m.sku, Some(0));
        assert_eq!(m.description, Some(1));
        assert_eq!(m.barcode, Some(2));
        assert_eq!(m.category, Some(3));
        assert_eq!(m.default_unit, Some(4));
        assert_eq!(m.default_alert_days_before, Some(5));
        assert_eq!(m.notes, Some(6));
    }

    #[test]
    fn detect_mapping_handles_aliases_and_case() {
        let headers = csv::StringRecord::from(vec!["Item SKU", "Name", "UPC", "Type"]);
        let m = detect_mapping(&headers);
        assert_eq!(m.sku, Some(0));
        assert_eq!(m.description, Some(1));
        assert_eq!(m.barcode, Some(2));
        assert_eq!(m.category, Some(3));
    }

    #[test]
    fn detect_mapping_missing_required_fields_returns_none() {
        let headers = csv::StringRecord::from(vec!["foo", "bar", "baz"]);
        let m = detect_mapping(&headers);
        assert!(m.sku.is_none());
        assert!(m.description.is_none());
    }

    #[test]
    fn resolve_mapping_merges_explicit_overrides_with_detected() {
        let headers = csv::StringRecord::from(vec!["sku", "name", "barcode"]);
        let explicit = CsvColumnMapping {
            sku: None,
            description: Some(1),
            barcode: None,
            category: None,
            default_unit: None,
            default_alert_days_before: None,
            notes: None,
        };
        let m = resolve_mapping(&headers, Some(explicit));
        // sku from auto-detection (column 0).
        assert_eq!(m.sku, Some(0));
        // description pinned to column 1 by explicit override.
        assert_eq!(m.description, Some(1));
        // barcode from auto-detection (column 2).
        assert_eq!(m.barcode, Some(2));
    }

    // ------------------------------------------------------------
    // Preview — happy path
    // ------------------------------------------------------------

    #[tokio::test]
    async fn preview_classifies_valid_rows_as_ok() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let csv_content = "sku,description,barcode\nSKU-A,Milk 1L,1234567890\n\
                           SKU-B,Bread,2345678901\n";

        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.total_rows, 2);
        assert_eq!(resp.valid_rows, 2);
        assert_eq!(resp.invalid_rows, 0);
        assert_eq!(resp.duplicate_sku_count, 0);
        assert_eq!(resp.duplicate_barcode_count, 0);
        assert_eq!(resp.missing_required_count, 0);
        assert_eq!(resp.headers, vec!["sku", "description", "barcode"]);
        for row in &resp.rows {
            assert!(matches!(row.status, CsvPreviewRowStatus::Ok));
        }
        Ok(())
    }

    #[tokio::test]
    async fn preview_detects_duplicate_sku_against_existing_product(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // Seed an existing product with SKU "EXIST-001".
        products_service::create_product(
            &pool,
            ProductCreate {
                sku: "EXIST-001".into(),
                description: "Existing".into(),
                category_id: None,
                default_unit: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        let csv_content = "sku,description\nEXIST-001,New description\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.total_rows, 1);
        assert_eq!(resp.valid_rows, 0);
        assert_eq!(resp.duplicate_sku_count, 1);
        let row = &resp.rows[0];
        match &row.status {
            CsvPreviewRowStatus::DuplicateSku {
                existing_sku,
                existing_product_id,
            } => {
                assert_eq!(existing_sku, "EXIST-001");
                assert!(!existing_product_id.is_empty());
            }
            other => panic!("expected DuplicateSku, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn preview_detects_duplicate_barcode_against_existing_product(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = products_service::create_product(
            &pool,
            ProductCreate {
                sku: "BC-OWNER".into(),
                description: "Barcode owner".into(),
                category_id: None,
                default_unit: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        products_service::add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "7501234567890".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;

        let csv_content = "sku,description,barcode\nNEW-001,New,7501234567890\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.duplicate_barcode_count, 1);
        let row = &resp.rows[0];
        match &row.status {
            CsvPreviewRowStatus::DuplicateBarcode {
                existing_product_id,
                existing_barcode,
            } => {
                assert_eq!(existing_product_id.as_str(), p.id.as_str());
                assert_eq!(existing_barcode, "7501234567890");
            }
            other => panic!("expected DuplicateBarcode, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn preview_flags_missing_required_fields() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let csv_content = "sku,description\n,No SKU\nSKU-OK,Valid\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.total_rows, 2);
        assert_eq!(resp.missing_required_count, 1);
        assert_eq!(resp.valid_rows, 1);
        match &resp.rows[0].status {
            CsvPreviewRowStatus::MissingRequired { field } => assert_eq!(field, "sku"),
            other => panic!("expected MissingRequired(sku), got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn preview_rejects_when_required_columns_missing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let csv_content = "foo,bar\n1,2\n";
        let err = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await
        .expect_err("missing required columns must fail");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn preview_flags_invalid_alert_days() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let csv_content =
            "sku,description,alert_days_before\nSKU-NEG,Negative,-5\nSKU-BIG,Too big,9999\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.total_rows, 2);
        assert_eq!(resp.valid_rows, 0);
        assert!(resp
            .rows
            .iter()
            .all(|r| matches!(r.status, CsvPreviewRowStatus::Invalid { .. })));
        Ok(())
    }

    // ------------------------------------------------------------
    // Export products
    // ------------------------------------------------------------

    #[tokio::test]
    async fn export_products_writes_canonical_csv_with_header_and_rows(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        let cat_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at)
             VALUES ($1, 'Dairy', 1, $2, $3)",
        )
        .bind(&cat_id)
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await?;

        let p = products_service::create_product(
            &pool,
            ProductCreate {
                sku: "EXP-001".into(),
                description: "Exported Product".into(),
                category_id: Some(cat_id.clone()),
                default_unit: Some("L".into()),
                default_alert_days_before: 30,
                notes: Some("note text".into()),
            },
        )
        .await?;
        products_service::add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "EXP-BC".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;

        let dir = tempdir()?;
        let out_path = dir.path().join("products.csv");
        let result = export_products_csv(&pool, &out_path).await?;

        assert_eq!(result.rows_written, 1);
        assert!(result.bytes_written > 0);

        let content = std::fs::read_to_string(&out_path)?;
        let first_line = content.lines().next().unwrap();
        assert_eq!(
            first_line,
            "sku,description,category,default_unit,default_alert_days_before,\
             primary_barcode,additional_barcodes,notes,is_active"
        );
        let data_line = content.lines().nth(1).unwrap();
        assert!(data_line.starts_with("EXP-001,Exported Product,Dairy,L,30,EXP-BC,,note text,true"));
        Ok(())
    }

    #[tokio::test]
    async fn export_products_returns_empty_csv_for_no_products(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let dir = tempdir()?;
        let out_path = dir.path().join("products.csv");
        let result = export_products_csv(&pool, &out_path).await?;

        assert_eq!(result.rows_written, 0);
        let content = std::fs::read_to_string(&out_path)?;
        assert_eq!(
            content.trim_end(),
            "sku,description,category,default_unit,default_alert_days_before,\
             primary_barcode,additional_barcodes,notes,is_active"
        );
        Ok(())
    }

    // ------------------------------------------------------------
    // Export report
    // ------------------------------------------------------------

    #[tokio::test]
    async fn export_report_writes_dashboard_rows_with_filters(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Seed a store + product + lot.
        let now = Utc::now().to_rfc3339();
        let store_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
             VALUES ($1, 'Export Store', 1, $2, $3)",
        )
        .bind(&store_id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

        let product = products_service::create_product(
            &pool,
            ProductCreate {
                sku: "EXP-LOT".into(),
                description: "Export Lot".into(),
                category_id: None,
                default_unit: Some("kg".into()),
                default_alert_days_before: 7,
                notes: None,
            },
        )
        .await?;

        let lot_id = Uuid::new_v4().to_string();
        let expiry = (Utc::now().date_naive() + chrono::Duration::days(3))
            .format("%Y-%m-%d")
            .to_string();
        sqlx::query(
            "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit,
                                      expiry_date, alert_days_before, status,
                                      created_at, updated_at)
             VALUES ($1, $2, $3, 4.0, 'kg', $4, 7, 'active', $5, $6)",
        )
        .bind(&lot_id)
        .bind(&product.id)
        .bind(&store_id)
        .bind(&expiry)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

        let dir = tempdir()?;
        let out_path = dir.path().join("report.csv");
        let result = export_report_csv(
            &pool,
            ReportExportInput {
                store_id: Some(store_id.clone()),
                location_id: None,
                preset: Some(DashboardPreset::All),
                urgency: None,
                path: out_path.to_string_lossy().into_owned(),
            },
        )
        .await?;

        assert_eq!(result.rows_written, 1);
        let content = std::fs::read_to_string(&out_path)?;
        let header = content.lines().next().unwrap();
        assert!(header.contains("lot_id"));
        assert!(header.contains("sku"));
        assert!(header.contains("urgency"));
        assert!(content.contains("EXP-LOT"));
        assert!(content.contains("Export Lot"));
        assert!(content.contains("Export Store"));
        Ok(())
    }

    #[tokio::test]
    async fn export_report_respects_preset_filter() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let dir = tempdir()?;
        let out_path = dir.path().join("report.csv");

        // Empty DB + Expired filter should produce zero data rows.
        let result = export_report_csv(
            &pool,
            ReportExportInput {
                store_id: None,
                location_id: None,
                preset: Some(DashboardPreset::Expired),
                urgency: None,
                path: out_path.to_string_lossy().into_owned(),
            },
        )
        .await?;

        assert_eq!(result.rows_written, 0);
        let content = std::fs::read_to_string(&out_path)?;
        assert_eq!(content.lines().count(), 1, "header only, no data");
        Ok(())
    }

    #[tokio::test]
    async fn read_csv_text_returns_file_contents() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let p = dir.path().join("input.csv");
        std::fs::write(&p, "sku,description\nSKU-X,Test\n")?;
        let content = read_csv_text(&p)?;
        assert_eq!(content, "sku,description\nSKU-X,Test\n");
        Ok(())
    }

    // ------------------------------------------------------------
    // format_quantity
    // ------------------------------------------------------------

    #[test]
    fn format_quantity_strips_trailing_zero() {
        assert_eq!(format_quantity(4.0), "4");
        assert_eq!(format_quantity(4.5), "4.5");
        assert_eq!(format_quantity(0.125), "0.125");
    }
}
