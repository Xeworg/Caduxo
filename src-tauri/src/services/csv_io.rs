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
    ConflictStrategy, CsvColumnMapping, CsvExportResult, CsvImportInput, CsvImportResult,
    CsvImportRowOutcome, CsvImportRowResult, CsvPreviewInput, CsvPreviewResponse, CsvPreviewRow,
    CsvPreviewRowStatus, ReportExportInput,
};
use crate::dto::dashboard::DashboardFilters;
use crate::dto::products::ProductLifecycle;
use crate::error::{AppError, DomainError, InfrastructureError};
use crate::pdf::locale::Locale;
use crate::services::user_messages::{user_message, UserMessage};

// ============================================================
// Stable reason codes
// ============================================================
//
// These string codes travel alongside the existing `reason: String` fields on
// CSV preview/import DTOs. The frontend dispatches by code through the
// `csvImport.reasonCodes` i18n namespace and falls back to `reason` when a
// code is missing or unrecognized. New codes can be added; existing ones
// must stay stable because they are persisted as part of the IPC contract.
//
// The constants are kept inline here (and mirrored as string literals in
// `src/components/CsvImportPage.svelte`) so we don't expand the public
// surface of the DTO module just to host code names.

/// Required field `sku` is missing on the row.
pub const REASON_CODE_REQUIRED_FIELD_SKU: &str = "required_field_sku";
/// Required field `description` is missing on the row.
pub const REASON_CODE_REQUIRED_FIELD_DESCRIPTION: &str = "required_field_description";
/// SKU already exists in the database (import skip strategy).
pub const REASON_CODE_SKU_ALREADY_EXISTS: &str = "sku_already_exists";
/// SKU conflict requires manual resolution (import review strategy).
pub const REASON_CODE_SKU_CONFLICT_MANUAL: &str = "sku_conflict_manual";
/// Barcode belongs to another product (import skip/update strategy).
pub const REASON_CODE_BARCODE_BELONGS_TO_OTHER_PRODUCT: &str = "barcode_belongs_to_other_product";
/// Barcode conflict requires manual resolution (import review strategy).
pub const REASON_CODE_BARCODE_CONFLICT_MANUAL: &str = "barcode_conflict_manual";
/// `default_alert_days_before` is negative.
pub const REASON_CODE_ALERT_DAYS_NEGATIVE: &str = "alert_days_negative";
/// `default_alert_days_before` exceeds the upper bound.
pub const REASON_CODE_ALERT_DAYS_TOO_LARGE: &str = "alert_days_too_large";
/// `default_alert_days_before` is outside the inclusive `[0, 3650]` range
/// (import commit path).
pub const REASON_CODE_ALERT_DAYS_RANGE_INVALID: &str = "alert_days_range_invalid";
/// Generic database error (SKU/barcode uniqueness lookup failed).
pub const REASON_CODE_DB_ERROR: &str = "db_error";
/// Generic validation failure (catches domain-rule text from
/// `domain::validation` that does not have its own code).
pub const REASON_CODE_GENERIC_VALIDATION: &str = "generic_validation";

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
    //
    // Both messages are emitted as the canonical English UserMessage text so
    // the command layer can localise them via `localize_validation`. The
    // `starts_with` parser in `user_messages` matches these prefixes, which
    // keeps Spanish UI flows free of English backend validation strings.
    if mapping.sku.is_none() {
        return Err(AppError::Domain(DomainError::Validation {
            message: user_message(UserMessage::SkuColumnNotDetected, Locale::En),
        }));
    }
    if mapping.description.is_none() {
        return Err(AppError::Domain(DomainError::Validation {
            message: user_message(UserMessage::DescriptionColumnNotDetected, Locale::En),
        }));
    }

    let mut rows: Vec<CsvPreviewRow> = Vec::new();
    let mut valid_rows: usize = 0;
    let mut duplicate_sku_count: usize = 0;
    let mut duplicate_barcode_count: usize = 0;
    let mut released_sku_count: usize = 0;
    let mut released_barcode_count: usize = 0;
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
                &default_unit,
                default_alert_days_before,
            )
            .await;

        match &status {
            CsvPreviewRowStatus::Ok => valid_rows += 1,
            CsvPreviewRowStatus::DuplicateSku { .. } => duplicate_sku_count += 1,
            CsvPreviewRowStatus::DuplicateBarcode { .. } => duplicate_barcode_count += 1,
            CsvPreviewRowStatus::ReleasedSku { .. } => {
                // Released SKU counts as valid_rows because the import commits
                // the new product with `lifecycle = active`. The partial unique
                // index excludes the retired row, so no UNIQUE collision fires
                // at the SQL layer (design §6.6 + spec `CSV import preview
                // advisory notice for a released SKU`).
                released_sku_count += 1;
                valid_rows += 1;
            }
            CsvPreviewRowStatus::ReleasedBarcode { .. } => {
                // Same release semantics as `ReleasedSku` (mirror column on
                // `product_barcodes.lifecycle` excludes the retired row from
                // the partial unique index; the new attach succeeds).
                released_barcode_count += 1;
                valid_rows += 1;
            }
            CsvPreviewRowStatus::MissingRequired { .. } => missing_required_count += 1,
            CsvPreviewRowStatus::Invalid { .. } => {}
            // UnknownUnit is non-blocking (warn-and-continue); still counts as valid.
            CsvPreviewRowStatus::UnknownUnit { .. } => valid_rows += 1,
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
        released_sku_count,
        released_barcode_count,
        missing_required_count,
        rows,
    })
}

/// Looks up the effective lifecycle class of the product(s) that own `barcode`.
///
/// A released identifier can exist twice after reuse: once on the retired
/// historical product and once on the newly-created active product. A plain
/// `LIMIT 1`/`fetch_optional` lookup is unsafe in that state because SQLite may
/// return the retired row first, causing import commit to misclassify an active
/// duplicate as reusable and then trip the partial UNIQUE index. Prefer any
/// active/archived owner over retired; return `Retired` only when every owner is
/// retired; return `None` when the barcode does not exist.
async fn lookup_barcode_owner_lifecycle(
    pool: &DbPool,
    barcode: &str,
) -> Result<Option<ProductLifecycle>, AppError> {
    let rows: Vec<(Option<String>,)> = sqlx::query_as(
        r#"
        SELECT products.lifecycle
        FROM products
        INNER JOIN product_barcodes pb ON pb.product_id = products.id
        WHERE pb.barcode = $1
        ORDER BY CASE products.lifecycle
            WHEN 'active' THEN 0
            WHEN 'archived' THEN 1
            WHEN 'retired' THEN 2
            ELSE 3
        END
        "#,
    )
    .bind(barcode)
    .fetch_all(pool)
    .await?;

    for (raw,) in rows {
        match raw.as_deref() {
            Some("active") => return Ok(Some(ProductLifecycle::Active)),
            Some("archived") => return Ok(Some(ProductLifecycle::Archived)),
            Some("retired") => continue,
            _ => return Ok(None),
        }
    }

    let retired_exists: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT 1
        FROM product_barcodes
        WHERE barcode = $1 AND lifecycle = 'retired'
        LIMIT 1
        "#,
    )
    .bind(barcode)
    .fetch_optional(pool)
    .await?;

    Ok(retired_exists.map(|_| ProductLifecycle::Retired))
}

async fn find_non_retired_sku_owner(
    pool: &DbPool,
    sku: &str,
) -> Result<Option<(String, String)>, AppError> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT id, sku
        FROM products
        WHERE sku = $1 AND lifecycle != 'retired'
        ORDER BY CASE lifecycle
            WHEN 'active' THEN 0
            WHEN 'archived' THEN 1
            ELSE 2
        END
        LIMIT 1
        "#,
    )
    .bind(sku)
    .fetch_optional(pool)
    .await?;

    Ok(row)
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
    default_unit: &Option<String>,
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
                reason_code: Some(REASON_CODE_GENERIC_VALIDATION.to_string()),
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
                reason_code: Some(REASON_CODE_GENERIC_VALIDATION.to_string()),
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
                    reason_code: Some(REASON_CODE_GENERIC_VALIDATION.to_string()),
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
                    reason_code: Some(REASON_CODE_ALERT_DAYS_NEGATIVE.to_string()),
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
                    reason_code: Some(REASON_CODE_ALERT_DAYS_TOO_LARGE.to_string()),
                },
                Some(sku),
                barcode.clone(),
                None,
                Some("alert days too large".to_string()),
            );
        }
    }

    // 3. SKU uniqueness check. Three-state classifier:
    //   - `DuplicateSku`       when the matching row is active or archived
    //   - `ReleasedSku`        when the matching row is retired
    //   - `Ok`                 when no row matches
    // The "active-or-archived wins over retired" rule (design §6.6) is
    // enforced by `find_by_sku_exact_with_lifecycle_filter` returning the
    // active/archived match first; the retired-only fallback below emits
    // `ReleasedSku`.
    match crate::db::repositories::products::find_by_sku_exact_including_retired(pool, &sku).await {
        Ok(Some(existing)) => {
            // The lifecycle field rides along on every product row post-V19.
            // Backward-compatibility: when the column is NULL (older
            // backend / pre-V19 fresh database), default to `Active` per
            // design §5.3 dual-read window.
            if existing.lifecycle == Some(ProductLifecycle::Retired) {
                return (
                    CsvPreviewRowStatus::ReleasedSku {
                        existing_product_id: existing.id,
                        existing_sku: existing.sku,
                    },
                    Some(sku),
                    barcode.clone(),
                    None,
                    None,
                );
            }
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
                    reason_code: Some(REASON_CODE_DB_ERROR.to_string()),
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
        match crate::db::repositories::products::find_by_barcode_exact_including_retired(pool, b)
            .await
        {
            Ok(Some(existing)) => {
                // `find_by_barcode_exact` does not project the `lifecycle`
                // column (it is shared with other read paths), so the
                // three-state classifier uses the dedicated lifecycle helper
                // for the barcode owner. The metadata still comes from
                // `find_by_barcode_exact` so `existing_product_id` and
                // `existing_barcode` carry the original (possibly retired)
                // row's identity — important for the "previously associated
                // with product X" tooltip on `ReleasedBarcode`.
                //
                // `classify_row` returns a tuple (not a `Result`), so the
                // helper's error path is surfaced as an `Invalid` row
                // rather than via `?`.
                let owner_lifecycle = match lookup_barcode_owner_lifecycle(pool, b).await {
                    Ok(lc) => lc,
                    Err(e) => {
                        return (
                            CsvPreviewRowStatus::Invalid {
                                reason: format!(
                                    "Database error while checking barcode uniqueness: {e}"
                                ),
                                reason_code: Some(REASON_CODE_DB_ERROR.to_string()),
                            },
                            Some(sku),
                            barcode.clone(),
                            None,
                            Some(e.to_string()),
                        );
                    }
                };
                if owner_lifecycle == Some(ProductLifecycle::Retired) {
                    return (
                        CsvPreviewRowStatus::ReleasedBarcode {
                            existing_product_id: existing.id,
                            existing_barcode: existing
                                .primary_barcode
                                .unwrap_or_else(|| b.to_string()),
                        },
                        Some(sku),
                        barcode.clone(),
                        None,
                        None,
                    );
                }
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
                        reason_code: Some(REASON_CODE_DB_ERROR.to_string()),
                    },
                    Some(sku),
                    barcode.clone(),
                    None,
                    Some(e.to_string()),
                );
            }
        }
    }

    // 5. Unit catalog check — non-blocking warn for unknown units.
    // Only applies when a unit value is present in the CSV row.
    if let Some(raw_unit) = default_unit.as_deref() {
        let unit_key = raw_unit.trim().to_lowercase();
        if crate::db::repositories::unit_definitions::find_by_key(pool, &unit_key)
            .await
            .ok()
            .flatten()
            .is_none()
        {
            // Unit not found in catalog — suggest up to 3 similar keys.
            let suggested: Vec<String> =
                crate::db::repositories::unit_definitions::suggest_similar(pool, &unit_key, 3)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|r| r.key)
                    .collect();
            return (
                CsvPreviewRowStatus::UnknownUnit {
                    raw_value: raw_unit.to_string(),
                    suggested_keys: suggested,
                },
                Some(sku),
                barcode.clone(),
                None,
                None,
            );
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
            .category_ids
            .iter()
            .filter_map(|cid| category_names.get(cid))
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join(", ");

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
        category_ids: None,
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
// Import commit
// ============================================================

/// Imports products and barcodes from a CSV using the given mapping and
/// conflict strategy. Commits changes to the database.
///
/// - `Skip` — only creates products for rows where SKU and barcode are new.
///   Duplicate-SKU rows and duplicate-barcode rows are recorded as `Skipped`.
/// - `Update` — creates new products for non-conflicting rows; for rows where
///   the SKU exists, updates the product fields; for rows where the barcode
///   exists on a different product, the barcode is skipped (silently).
/// - `Review` — returns all rows with their outcomes but makes no database
///   changes. The caller can inspect the list and re-invoke with a resolved
///   strategy.
pub async fn import_product_csv(
    pool: &DbPool,
    input: CsvImportInput,
) -> Result<CsvImportResult, AppError> {
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

    let mapping = resolve_mapping(&headers, Some(input.mapping));

    if mapping.sku.is_none() {
        return Err(AppError::Domain(DomainError::Validation {
            // Same canonical text as `preview_product_csv`; lets a single
            // UserMessage variant cover both auto-detect and explicit-mapping
            // failure paths and keeps round-trip coverage with the parser.
            message: user_message(UserMessage::SkuColumnNotDetected, Locale::En),
        }));
    }
    if mapping.description.is_none() {
        return Err(AppError::Domain(DomainError::Validation {
            message: user_message(UserMessage::DescriptionColumnNotDetected, Locale::En),
        }));
    }

    // Counters are updated by matching &outcome below — no is_review branch needed.

    let mut rows: Vec<CsvImportRowResult> = Vec::new();
    let mut created: usize = 0;
    let mut skipped: usize = 0;
    let mut updated: usize = 0;
    let mut invalid: usize = 0;

    for (idx, record_result) in reader.records().enumerate() {
        let record = record_result.map_err(|e| {
            AppError::Domain(DomainError::Validation {
                message: format!("Failed to read CSV row {}: {e}", idx + 2),
            })
        })?;

        let row_index = idx + 1; // 1-based, excluding header

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
        let category_name = mapping
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

        // Resolve category name to id (no-op on empty/None).
        let category_id = resolve_category_name(pool, category_name.as_deref()).await?;

        let outcome = import_row(
            pool,
            &sku,
            &description,
            barcode.as_deref(),
            category_id.as_deref(),
            default_unit.as_deref(),
            default_alert_days_before,
            notes.as_deref(),
            input.strategy,
        )
        .await?;

        match &outcome {
            CsvImportRowOutcome::Created { .. } => created += 1,
            CsvImportRowOutcome::Skipped { .. } => skipped += 1,
            CsvImportRowOutcome::Updated { .. } => updated += 1,
            CsvImportRowOutcome::Invalid { .. } => invalid += 1,
        }

        rows.push(CsvImportRowResult {
            row_index,
            sku: sku.unwrap_or_default(),
            description: description.unwrap_or_default(),
            barcode: barcode.unwrap_or_default(),
            outcome,
        });
    }

    let total_rows = rows.len();
    Ok(CsvImportResult {
        total_rows,
        created,
        skipped,
        updated,
        invalid,
        rows,
    })
}

/// Imports a single CSV row. Returns the outcome and count deltas.
async fn import_row(
    pool: &DbPool,
    sku: &Option<String>,
    description: &Option<String>,
    barcode: Option<&str>,
    category_name: Option<&str>,
    default_unit: Option<&str>,
    default_alert_days_before: Option<i32>,
    notes: Option<&str>,
    strategy: ConflictStrategy,
) -> Result<CsvImportRowOutcome, AppError> {
    use crate::db::repositories::products as products_repo;
    use crate::domain::validation::{validate_barcode, validate_description, validate_sku};
    use crate::dto::products::{ProductBarcodeCreate, ProductCreate, ProductUpdate};

    // 1. Required field validation.
    let sku = match sku {
        Some(s) if !s.trim().is_empty() => s.clone(),
        _ => {
            return Ok(CsvImportRowOutcome::Invalid {
                reason: "SKU is required".to_string(),
                reason_code: Some(REASON_CODE_REQUIRED_FIELD_SKU.to_string()),
            });
        }
    };
    let description = match description {
        Some(d) if !d.trim().is_empty() => d.clone(),
        _ => {
            return Ok(CsvImportRowOutcome::Invalid {
                reason: "Description is required".to_string(),
                reason_code: Some(REASON_CODE_REQUIRED_FIELD_DESCRIPTION.to_string()),
            });
        }
    };

    // 2. Domain validation.
    if let Err(msg) = validate_sku(&sku) {
        return Ok(CsvImportRowOutcome::Invalid {
            reason: msg,
            reason_code: Some(REASON_CODE_GENERIC_VALIDATION.to_string()),
        });
    }
    if let Err(msg) = validate_description(&description) {
        return Ok(CsvImportRowOutcome::Invalid {
            reason: msg,
            reason_code: Some(REASON_CODE_GENERIC_VALIDATION.to_string()),
        });
    }
    if let Some(b) = barcode {
        if let Err(msg) = validate_barcode(b) {
            return Ok(CsvImportRowOutcome::Invalid {
                reason: msg,
                reason_code: Some(REASON_CODE_GENERIC_VALIDATION.to_string()),
            });
        }
    }
    if let Some(days) = default_alert_days_before {
        if !(0..=3650).contains(&days) {
            return Ok(CsvImportRowOutcome::Invalid {
                reason: "Alert days must be between 0 and 3650".to_string(),
                reason_code: Some(REASON_CODE_ALERT_DAYS_RANGE_INVALID.to_string()),
            });
        }
    }

    let resolved_alert_days = default_alert_days_before.unwrap_or(30);

    // 3. Resolve category name to id (single-category CSV import).
    let resolved_cat_id = resolve_category_name(pool, category_name).await?;

    // 4. Check if SKU exists. A retired product's SKU was released back to
    // the global pool by `apply_lifecycle_transition` (design §5.1 +
    // partial unique index `uq_products_sku_active WHERE lifecycle !=
    // 'retired'`), so a CSV row that matches *only* retired rows is NOT a
    // conflict. Once the identifier has been reused, however, the database may
    // contain both a retired historical row and a new active row with the same
    // SKU. Look specifically for a non-retired owner instead of using a
    // single-row exact lookup that can return the retired row first.
    let existing_by_sku = find_non_retired_sku_owner(pool, &sku).await?;

    match (existing_by_sku, strategy) {
        // ── Skip strategy ──────────────────────────────────────────
        (Some(_existing), ConflictStrategy::Skip) => Ok(CsvImportRowOutcome::Skipped {
            reason: format!("SKU '{}' already exists", sku),
            reason_code: Some(REASON_CODE_SKU_ALREADY_EXISTS.to_string()),
        }),

        // ── Update strategy ────────────────────────────────────────
        (Some(existing), ConflictStrategy::Update) => {
            let update_input = ProductUpdate {
                id: existing.0.clone(),
                sku: existing.1.clone(),
                description: description.clone(),
                category_ids: resolved_cat_id.map(|id| vec![id]),
                default_unit: default_unit.map(String::from),
                default_unit_id: None,
                default_alert_days_before: resolved_alert_days,
                notes: notes.map(String::from),
                is_active: true,
            };
            products_repo::update_product(pool, &update_input, None, None).await?;

            // Try to add barcode; silently skip UNIQUE violations.
            if let Some(b) = barcode {
                let barcode_create = ProductBarcodeCreate {
                    product_id: existing.0.clone(),
                    barcode: b.to_string(),
                    barcode_type: None,
                    is_primary: false,
                };
                if let Err(sqlx_err) = products_repo::insert_barcode(pool, &barcode_create).await {
                    if let sqlx::Error::Database(ref db_err) = sqlx_err {
                        if !db_err.is_unique_violation() {
                            return Err(AppError::from(sqlx_err));
                        }
                    } else {
                        return Err(AppError::from(sqlx_err));
                    }
                }
            }

            Ok(CsvImportRowOutcome::Updated {
                product_id: existing.0,
                sku,
            })
        }

        // ── Review strategy ────────────────────────────────────────
        (Some(_), ConflictStrategy::Review) => Ok(CsvImportRowOutcome::Skipped {
            reason: "SKU conflict requires manual resolution".to_string(),
            reason_code: Some(REASON_CODE_SKU_CONFLICT_MANUAL.to_string()),
        }),

        // ── SKU does not exist ─────────────────────────────────────
        (None, _) => {
            // Check barcode collision when the SKU is new. A retired
            // product's barcode was released by the lifecycle mirror UPDATE
            // (`sync_product_barcodes_lifecycle`), so a barcode that
            // matches only a retired row is NOT a conflict and the new
            // product may attach the value under every strategy. Active
            // or archived duplicates keep the original conflict semantics.
            //
            // `find_by_barcode_exact` does not project the `lifecycle`
            // column on its joined SELECT (the helper is shared with
            // scanner / search read paths that only need the metadata), so
            // the retired-vs-active/archived distinction uses the dedicated
            // `lookup_barcode_owner_lifecycle` helper. Same join, focused
            // SELECT that pulls only the lifecycle column.
            if let Some(b) = barcode {
                let owner_lifecycle = lookup_barcode_owner_lifecycle(pool, b).await?;
                if owner_lifecycle.is_some() && owner_lifecycle != Some(ProductLifecycle::Retired) {
                    let (reason, code) = match strategy {
                        ConflictStrategy::Skip => (
                            format!("Barcode '{}' belongs to another product", b),
                            REASON_CODE_BARCODE_BELONGS_TO_OTHER_PRODUCT,
                        ),
                        ConflictStrategy::Update => (
                            format!("Barcode '{}' belongs to another product", b),
                            REASON_CODE_BARCODE_BELONGS_TO_OTHER_PRODUCT,
                        ),
                        ConflictStrategy::Review => (
                            "Barcode conflict requires manual resolution".to_string(),
                            REASON_CODE_BARCODE_CONFLICT_MANUAL,
                        ),
                    };
                    return Ok(CsvImportRowOutcome::Skipped {
                        reason,
                        reason_code: Some(code.to_string()),
                    });
                }
            }

            // Create the product.
            // CSV import preserves `default_unit` text; `default_unit_id` and
            // `unit_type` are not set here so unrecognized units surface in the
            // audit banner.
            let create_input = ProductCreate {
                sku: sku.clone(),
                description,
                category_ids: resolved_cat_id.map(|id| vec![id]),
                default_unit: default_unit.map(String::from),
                default_unit_id: None,
                default_alert_days_before: resolved_alert_days,
                notes: notes.map(String::from),
            };
            let product = match products_repo::insert_product(pool, &create_input, None, None).await
            {
                Ok(p) => p,
                Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                    return Ok(duplicate_outcome_from_create(&*db_err, &sku, strategy));
                }
                Err(sqlx_err) => return Err(AppError::from(sqlx_err)),
            };

            // Attach the barcode if present; silently skip UNIQUE violations.
            if let Some(b) = barcode {
                let barcode_create = ProductBarcodeCreate {
                    product_id: product.id.clone(),
                    barcode: b.to_string(),
                    barcode_type: None,
                    is_primary: true,
                };
                if let Err(sqlx_err) = products_repo::insert_barcode(pool, &barcode_create).await {
                    if let sqlx::Error::Database(ref db_err) = sqlx_err {
                        if !db_err.is_unique_violation() {
                            return Err(AppError::from(sqlx_err));
                        }
                    } else {
                        return Err(AppError::from(sqlx_err));
                    }
                }
            }

            Ok(CsvImportRowOutcome::Created {
                product_id: product.id,
                sku,
            })
        }
    }
}

/// Looks up a category by name and returns its id, or `None` if no such
/// category exists. Category names are matched case-insensitively.
async fn resolve_category_name(
    pool: &DbPool,
    name: Option<&str>,
) -> Result<Option<String>, AppError> {
    use crate::db::repositories::products as products_repo;

    let name = match name {
        Some(n) if !n.trim().is_empty() => n.trim(),
        _ => return Ok(None),
    };

    // Case-insensitive lookup: SQLite LIKE is case-insensitive for ASCII.
    let categories = products_repo::list_all_categories(pool).await?;
    Ok(categories
        .into_iter()
        .find(|c| c.name.to_lowercase() == name.to_lowercase())
        .map(|c| c.id))
}

/// Maps a `UNIQUE constraint failed` raised while creating a new product to a
/// row-level import outcome. The partial unique indexes (`uq_products_sku_active
/// WHERE lifecycle != 'retired'` and `uq_product_barcodes_barcode_active WHERE
/// lifecycle != 'retired'`) only fire for non-retired duplicates, so reaching
/// this path means a *real* duplicate slipped through the pre-check (e.g. a
/// parallel writer committed between `find_non_retired_sku_owner` and the
/// `INSERT`, or the same SKU appears twice in the CSV with one row committing
/// before the second). The CSV must keep its full row outcome so a single
/// transient race never aborts the whole import.
fn duplicate_outcome_from_create(
    db_err: &dyn sqlx::error::DatabaseError,
    sku: &str,
    strategy: ConflictStrategy,
) -> CsvImportRowOutcome {
    let message = db_err.message();
    let field = if message.contains("product_barcodes") {
        "barcode"
    } else {
        "sku"
    };
    let value = if field == "sku" {
        sku.to_string()
    } else {
        // Best-effort value extraction; SQLite message format is `…:
        // …products.<column>` so we fall back to an empty placeholder when the
        // concrete value is not recoverable.
        extract_unique_value(message).unwrap_or_default()
    };

    match (field, strategy) {
        ("sku", ConflictStrategy::Skip) => CsvImportRowOutcome::Skipped {
            reason: format!("SKU '{}' already exists", sku),
            reason_code: Some(REASON_CODE_SKU_ALREADY_EXISTS.to_string()),
        },
        ("sku", ConflictStrategy::Update) | ("sku", ConflictStrategy::Review) => {
            CsvImportRowOutcome::Skipped {
                reason: format!("SKU '{}' already exists", sku),
                reason_code: Some(REASON_CODE_SKU_ALREADY_EXISTS.to_string()),
            }
        }
        ("barcode", _) => CsvImportRowOutcome::Skipped {
            reason: format!("Barcode '{}' belongs to another product", value),
            reason_code: Some(REASON_CODE_BARCODE_BELONGS_TO_OTHER_PRODUCT.to_string()),
        },
        _ => CsvImportRowOutcome::Skipped {
            reason: format!("Duplicate field: {}", message),
            reason_code: Some(REASON_CODE_GENERIC_VALIDATION.to_string()),
        },
    }
}

fn extract_unique_value(message: &str) -> Option<String> {
    let (_, tail) = message.split_once('`')?;
    let value = tail.split_once('`')?.0;
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
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
    use crate::dto::products::{
        ProductBarcodeCreate, ProductCreate, ProductLifecycle, ProductSearchQuery,
        RetireProductInput,
    };
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
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
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
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
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
    // Preview — unit catalog awareness (Section E)
    // ------------------------------------------------------------

    /// RED: preview must flag unknown unit values with suggested keys.
    #[tokio::test]
    async fn preview_flags_unknown_unit_with_suggestions() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        // "litro" is not in the preset catalog but suggests "L" (Litro) via
        // lexicographic prefix match: 'L' comes before 'litro' in ORDER BY key.
        let csv_content = "sku,description,default_unit\nSKU-UKN,Unknown Unit,litro\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.total_rows, 1);
        let row = &resp.rows[0];
        match &row.status {
            CsvPreviewRowStatus::UnknownUnit {
                raw_value,
                suggested_keys,
            } => {
                assert_eq!(raw_value, "litro");
                // "L" is the closest suggestion (lexicographic prefix match).
                assert!(
                    suggested_keys.iter().any(|k| k == "L"),
                    "expected 'L' in suggestions, got {suggested_keys:?}"
                );
                // The row is still classified as valid (non-blocking).
                assert_eq!(resp.valid_rows, 1);
            }
            other => panic!("expected UnknownUnit, got {other:?}"),
        }
        Ok(())
    }

    /// RED: a row with an unknown unit must still count as valid (warn-and-continue).
    #[tokio::test]
    async fn preview_does_not_block_unknown_unit() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let csv_content =
            "sku,description,default_unit\nSKU-OK,Good product,kg\nSKU-UKN,Bad unit,nonexistent\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.total_rows, 2);
        // "kg" is a preset — valid row.
        assert_eq!(resp.valid_rows, 2, "unknown_unit is non-blocking");
        assert_eq!(resp.invalid_rows, 0);
        assert_eq!(resp.missing_required_count, 0);
        assert_eq!(resp.duplicate_sku_count, 0);
        Ok(())
    }

    /// RED: import of an unknown-unit row creates a product with text preserved
    /// and no catalog link; the product surfaces in the audit banner.
    #[tokio::test]
    async fn import_unknown_unit_creates_product_with_text_only(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let csv_content = "sku,description,default_unit\nSKU-UKN,Custom unit product,misunit\n";

        let import_resp = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await?;

        assert_eq!(import_resp.total_rows, 1);
        assert_eq!(import_resp.created, 1);
        assert_eq!(import_resp.invalid, 0);

        // Verify the product was created with text preserved but no catalog link.
        let product_id = match &import_resp.rows[0].outcome {
            CsvImportRowOutcome::Created { product_id, .. } => product_id.clone(),
            other => panic!("expected Created, got {other:?}"),
        };
        let product_detail = crate::services::products::get_product(&pool, product_id).await?;
        let product = &product_detail.product;
        assert_eq!(
            product.default_unit.as_deref(),
            Some("misunit"),
            "unknown unit text must be preserved"
        );
        assert!(
            product.default_unit_id.is_none(),
            "unknown unit must not get a catalog FK"
        );
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
                category_ids: Some(vec![cat_id.clone()]),
                default_unit: Some("L".into()),
                default_unit_id: None,
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
                category_ids: None,
                default_unit: Some("kg".into()),
                default_unit_id: None,
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

    // ------------------------------------------------------------
    // Import — skip strategy
    // ------------------------------------------------------------

    #[tokio::test]
    async fn import_skip_creates_new_products() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let csv_content =
            "sku,description,barcode\nSKU-NEW-1,New milk,1234567890\nSKU-NEW-2,New bread,\n";

        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await?;

        assert_eq!(result.total_rows, 2);
        assert_eq!(result.created, 2);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.invalid, 0);

        for row in &result.rows {
            assert!(matches!(
                &row.outcome,
                CsvImportRowOutcome::Created { sku, .. } if !sku.is_empty()
            ));
        }
        Ok(())
    }

    #[tokio::test]
    async fn import_skip_skips_existing_sku() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        products_service::create_product(
            &pool,
            ProductCreate {
                sku: "EXIST-SKU".into(),
                description: "Existing product".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        let csv_content = "sku,description\nEXIST-SKU,New desc\nSKU-FRESH,Fresh product\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await?;

        assert_eq!(result.total_rows, 2);
        assert_eq!(result.created, 1);
        assert_eq!(result.skipped, 1);

        let skipped_row = result.rows.iter().find(|r| r.sku == "EXIST-SKU").unwrap();
        match &skipped_row.outcome {
            CsvImportRowOutcome::Skipped {
                reason,
                reason_code,
            } => {
                assert!(reason.contains("EXIST-SKU"));
                assert_eq!(
                    reason_code.as_deref(),
                    Some(REASON_CODE_SKU_ALREADY_EXISTS),
                    "expected sku_already_exists code for skip-strategy duplicate SKU"
                );
            }
            other => panic!("expected Skipped, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn import_skip_skips_barcode_owned_by_another_product(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let p = products_service::create_product(
            &pool,
            ProductCreate {
                sku: "BC-OWNER".into(),
                description: "Barcode owner".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        products_service::add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: p.id.clone(),
                barcode: "7500000000001".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;

        let csv_content = "sku,description,barcode\nSKU-FRESH,Fresh product,7500000000001\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await?;

        assert_eq!(result.total_rows, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.created, 0);

        let row = &result.rows[0];
        match &row.outcome {
            CsvImportRowOutcome::Skipped {
                reason,
                reason_code,
            } => {
                assert!(reason.contains("another product"));
                assert_eq!(
                    reason_code.as_deref(),
                    Some(REASON_CODE_BARCODE_BELONGS_TO_OTHER_PRODUCT),
                    "expected barcode_belongs_to_other_product code when barcode is owned by another product"
                );
            }
            other => panic!("expected Skipped, got {other:?}"),
        }
        Ok(())
    }

    // ------------------------------------------------------------
    // Import — update strategy
    // ------------------------------------------------------------

    #[tokio::test]
    async fn import_update_updates_existing_product() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let existing = products_service::create_product(
            &pool,
            ProductCreate {
                sku: "UPDATE-ME".into(),
                description: "Old description".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        let csv_content = "sku,description\nUPDATE-ME,Updated description\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Update,
            },
        )
        .await?;

        assert_eq!(result.total_rows, 1);
        assert_eq!(result.updated, 1);
        assert_eq!(result.created, 0);

        let row = &result.rows[0];
        match &row.outcome {
            CsvImportRowOutcome::Updated { sku, product_id } => {
                assert_eq!(sku, "UPDATE-ME");
                assert_eq!(product_id, &existing.id);
            }
            other => panic!("expected Updated, got {other:?}"),
        }

        let updated = products_service::get_product(&pool, existing.id.clone()).await?;
        assert_eq!(updated.product.description, "Updated description");
        Ok(())
    }

    #[tokio::test]
    async fn import_update_creates_new_products() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let csv_content = "sku,description\nNEW-A,New A\nNEW-B,New B\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Update,
            },
        )
        .await?;

        assert_eq!(result.total_rows, 2);
        assert_eq!(result.created, 2);
        assert_eq!(result.updated, 0);
        Ok(())
    }

    #[tokio::test]
    async fn import_update_adds_barcode_to_existing_product(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let existing = products_service::create_product(
            &pool,
            ProductCreate {
                sku: "BC-TARGET".into(),
                description: "Barcode target".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        let csv_content = "sku,description,barcode\nBC-TARGET,Barcode target,9900001234567\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Update,
            },
        )
        .await?;

        assert_eq!(result.updated, 1);
        assert_eq!(result.created, 0);

        let barcodes = products_service::list_barcodes(&pool, existing.id.clone()).await?;
        assert!(barcodes.iter().any(|b| b.barcode == "9900001234567"));
        Ok(())
    }

    // ------------------------------------------------------------
    // Import — review strategy
    // ------------------------------------------------------------

    #[tokio::test]
    async fn import_review_returns_conflicts_without_changes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        products_service::create_product(
            &pool,
            ProductCreate {
                sku: "REVIEW-SKU".into(),
                description: "Existing".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        let csv_content = "sku,description\nREVIEW-SKU,Conflicting\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Review,
            },
        )
        .await?;

        assert_eq!(result.total_rows, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.created, 0);
        assert_eq!(result.updated, 0);

        let row = &result.rows[0];
        match &row.outcome {
            CsvImportRowOutcome::Skipped {
                reason,
                reason_code,
            } => {
                assert!(reason.contains("manual resolution"));
                assert_eq!(
                    reason_code.as_deref(),
                    Some(REASON_CODE_SKU_CONFLICT_MANUAL),
                    "expected sku_conflict_manual code for review-strategy SKU conflict"
                );
            }
            other => panic!("expected Skipped, got {other:?}"),
        }

        let found = products_service::search_products(
            &pool,
            ProductSearchQuery {
                query: "REVIEW-SKU".into(),
            },
        )
        .await?;
        assert!(!found.is_empty(), "product should still exist");
        assert_eq!(found[0].description, "Existing");
        Ok(())
    }

    // ------------------------------------------------------------
    // Import — validation
    // ------------------------------------------------------------

    #[tokio::test]
    async fn import_rejects_missing_sku() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let csv_content = "sku,description\n,No SKU\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await?;

        assert_eq!(result.total_rows, 1);
        assert_eq!(result.invalid, 1);

        let row = &result.rows[0];
        match &row.outcome {
            CsvImportRowOutcome::Invalid {
                reason,
                reason_code,
            } => {
                assert!(reason.contains("SKU"));
                assert_eq!(
                    reason_code.as_deref(),
                    Some(REASON_CODE_REQUIRED_FIELD_SKU),
                    "expected required_field_sku code for missing SKU"
                );
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn import_rejects_missing_description() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let csv_content = "sku,description\nSKU-OK,\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await?;

        assert_eq!(result.total_rows, 1);
        assert_eq!(result.invalid, 1);

        let row = &result.rows[0];
        match &row.outcome {
            CsvImportRowOutcome::Invalid {
                reason,
                reason_code,
            } => {
                assert!(reason.contains("Description"));
                assert_eq!(
                    reason_code.as_deref(),
                    Some(REASON_CODE_REQUIRED_FIELD_DESCRIPTION),
                    "expected required_field_description code for missing description"
                );
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn import_requires_sku_and_description_columns() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let csv_content = "name\nJust a name\n";
        let err = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await
        .expect_err("missing SKU/description columns must fail");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    // ------------------------------------------------------------
    // Released SKU / barcode — preview + import regression suite
    //
    // These tests pin the contract from design §6.6: a retired product's
    // SKU and barcode are released back to the global pool, so a CSV row
    // whose identifier matches *only* a retired product must preview as
    // `ReleasedSku` / `ReleasedBarcode` (counted as `valid_rows`) and must
    // commit as `Created` under every conflict strategy. An active or
    // archived duplicate must keep blocking under the original conflict
    // semantics.
    // ------------------------------------------------------------

    /// Helper: creates a product, then retires it. Returns the retired row
    /// so individual tests can assert against `id` / `lifecycle`.
    async fn seed_retired_product(
        pool: &DbPool,
        sku: &str,
        description: &str,
        barcode: Option<&str>,
        retire_reason: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let product = products_service::create_product(
            pool,
            ProductCreate {
                sku: sku.to_string(),
                description: description.to_string(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        if let Some(bc) = barcode {
            products_service::add_barcode(
                pool,
                ProductBarcodeCreate {
                    product_id: product.id.clone(),
                    barcode: bc.to_string(),
                    barcode_type: None,
                    is_primary: true,
                },
            )
            .await?;
        }
        products_service::retire_product(
            pool,
            RetireProductInput {
                id: product.id.clone(),
                reason: retire_reason.to_string(),
                actor: None,
            },
        )
        .await?;
        Ok(product.id)
    }

    /// Preview: a SKU matching only a retired product emits `ReleasedSku`
    /// and counts as `valid_rows` (import will commit the new product).
    #[tokio::test]
    async fn preview_emits_released_sku_for_retired_only_match(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        seed_retired_product(
            &pool,
            "RETIRED-SKU",
            "Retired predecessor",
            None,
            "Discontinued",
        )
        .await?;

        let csv_content = "sku,description\nRETIRED-SKU,Fresh successor\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.total_rows, 1);
        assert_eq!(
            resp.released_sku_count, 1,
            "released SKU counter must increment"
        );
        assert_eq!(
            resp.duplicate_sku_count, 0,
            "released SKU must NOT count as a duplicate"
        );
        assert_eq!(
            resp.valid_rows, 1,
            "released SKU counts as valid so the import commits it"
        );
        assert_eq!(resp.invalid_rows, 0);
        match &resp.rows[0].status {
            CsvPreviewRowStatus::ReleasedSku {
                existing_sku,
                existing_product_id,
            } => {
                assert_eq!(existing_sku, "RETIRED-SKU");
                assert!(!existing_product_id.is_empty());
            }
            other => panic!("expected ReleasedSku, got {other:?}"),
        }
        Ok(())
    }

    /// Preview: a barcode matching only a retired product emits
    /// `ReleasedBarcode` and counts as `valid_rows`.
    #[tokio::test]
    async fn preview_emits_released_barcode_for_retired_only_match(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        seed_retired_product(
            &pool,
            "RETIRED-BC",
            "Retired barcode owner",
            Some("7509999999999"),
            "Discontinued",
        )
        .await?;

        let csv_content = "sku,description,barcode\nFRESH-BC,Fresh product,7509999999999\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.released_barcode_count, 1);
        assert_eq!(resp.duplicate_barcode_count, 0);
        assert_eq!(resp.valid_rows, 1);
        assert_eq!(resp.invalid_rows, 0);
        match &resp.rows[0].status {
            CsvPreviewRowStatus::ReleasedBarcode {
                existing_barcode,
                existing_product_id,
            } => {
                assert_eq!(existing_barcode, "7509999999999");
                assert!(!existing_product_id.is_empty());
            }
            other => panic!("expected ReleasedBarcode, got {other:?}"),
        }
        Ok(())
    }

    /// Preview: an active (non-retired) duplicate SKU must still emit
    /// `DuplicateSku` — the released-vs-duplicate split is gated on
    /// `lifecycle`, not on "is there a row".
    #[tokio::test]
    async fn preview_active_duplicate_sku_still_emits_duplicate_sku(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        products_service::create_product(
            &pool,
            ProductCreate {
                sku: "ACTIVE-DUP".into(),
                description: "Active row".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        let csv_content = "sku,description\nACTIVE-DUP,Fresh description\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.duplicate_sku_count, 1);
        assert_eq!(resp.released_sku_count, 0);
        assert_eq!(
            resp.valid_rows, 0,
            "active duplicate must NOT count as valid"
        );
        match &resp.rows[0].status {
            CsvPreviewRowStatus::DuplicateSku {
                existing_sku,
                existing_product_id,
            } => {
                assert_eq!(existing_sku, "ACTIVE-DUP");
                assert!(!existing_product_id.is_empty());
            }
            other => panic!("expected DuplicateSku, got {other:?}"),
        }
        Ok(())
    }

    /// Preview: an active (non-retired) duplicate barcode must still emit
    /// `DuplicateBarcode`. Mirror of the SKU case.
    #[tokio::test]
    async fn preview_active_duplicate_barcode_still_emits_duplicate_barcode(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let owner = products_service::create_product(
            &pool,
            ProductCreate {
                sku: "BC-OWNER".into(),
                description: "Barcode owner".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;
        products_service::add_barcode(
            &pool,
            ProductBarcodeCreate {
                product_id: owner.id.clone(),
                barcode: "7508888888888".into(),
                barcode_type: None,
                is_primary: true,
            },
        )
        .await?;

        let csv_content = "sku,description,barcode\nFRESH-OWNER,Fresh,7508888888888\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.duplicate_barcode_count, 1);
        assert_eq!(resp.released_barcode_count, 0);
        assert_eq!(resp.valid_rows, 0);
        match &resp.rows[0].status {
            CsvPreviewRowStatus::DuplicateBarcode {
                existing_barcode,
                existing_product_id,
            } => {
                assert_eq!(existing_barcode, "7508888888888");
                assert_eq!(existing_product_id.as_str(), owner.id.as_str());
            }
            other => panic!("expected DuplicateBarcode, got {other:?}"),
        }
        Ok(())
    }

    /// Preview summary: when a single CSV mixes a released SKU, a released
    /// barcode, and an active duplicate SKU, the three counters partition
    /// the rows correctly and `valid_rows` includes both released counts.
    #[tokio::test]
    async fn preview_summary_partitions_released_and_active_duplicates(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // Released SKU source.
        seed_retired_product(
            &pool,
            "REL-SKU",
            "Retired SKU predecessor",
            None,
            "Discontinued",
        )
        .await?;
        // Released barcode source (different product, distinct SKU).
        seed_retired_product(
            &pool,
            "REL-BC",
            "Retired barcode predecessor",
            Some("7507777777777"),
            "Discontinued",
        )
        .await?;
        // Active duplicate SKU source.
        products_service::create_product(
            &pool,
            ProductCreate {
                sku: "DUP-SKU".into(),
                description: "Active duplicate".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        let csv_content = "sku,description,barcode\n\
                           REL-SKU,Fresh A,\n\
                           FRESH-SKU,Fresh B,7507777777777\n\
                           DUP-SKU,Fresh C,\n\
                           BRAND-NEW,Brand new,\n";
        let resp = preview_product_csv(
            &pool,
            CsvPreviewInput {
                content: csv_content.to_string(),
                mapping: None,
            },
        )
        .await?;

        assert_eq!(resp.total_rows, 4);
        assert_eq!(resp.released_sku_count, 1);
        assert_eq!(resp.released_barcode_count, 1);
        assert_eq!(resp.duplicate_sku_count, 1);
        assert_eq!(resp.duplicate_barcode_count, 0);
        assert_eq!(
            resp.valid_rows, 3,
            "Ok + ReleasedSku + ReleasedBarcode are valid; DuplicateSku is not"
        );
        assert_eq!(resp.invalid_rows, 1);
        Ok(())
    }

    /// Import (`Skip`): a SKU matching only a retired product must commit
    /// as `Created`, not `Skipped`. The retired row's identifier was
    /// released by `apply_lifecycle_transition` so the new product may
    /// reuse the value (partial unique index excludes the retired row).
    #[tokio::test]
    async fn import_skip_creates_when_sku_matches_only_retired(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        seed_retired_product(
            &pool,
            "RETIRED-SKU",
            "Retired predecessor",
            None,
            "Discontinued",
        )
        .await?;

        let csv_content = "sku,description\nRETIRED-SKU,Fresh successor\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await?;

        assert_eq!(result.total_rows, 1);
        assert_eq!(result.created, 1);
        assert_eq!(result.skipped, 0);
        match &result.rows[0].outcome {
            CsvImportRowOutcome::Created { sku, .. } => {
                assert_eq!(sku, "RETIRED-SKU");
            }
            other => panic!("expected Created for retired SKU, got {other:?}"),
        }

        // Verify the new product is active and independent of the retired row.
        let new_id = match &result.rows[0].outcome {
            CsvImportRowOutcome::Created { product_id, .. } => product_id.clone(),
            _ => unreachable!(),
        };
        let detail = products_service::get_product(&pool, new_id).await?;
        assert_eq!(detail.product.lifecycle, Some(ProductLifecycle::Active));
        Ok(())
    }

    /// Import (`Update`): a SKU matching only a retired product must commit
    /// as `Created`, not `Updated` against the retired row. Updates against
    /// retired rows are forbidden by `apply_lifecycle_transition` and would
    /// be wrong semantically.
    #[tokio::test]
    async fn import_update_creates_when_sku_matches_only_retired(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        seed_retired_product(
            &pool,
            "RETIRED-SKU",
            "Retired predecessor",
            None,
            "Discontinued",
        )
        .await?;

        let csv_content = "sku,description\nRETIRED-SKU,Fresh successor\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Update,
            },
        )
        .await?;

        assert_eq!(result.created, 1);
        assert_eq!(result.updated, 0);
        match &result.rows[0].outcome {
            CsvImportRowOutcome::Created { .. } => {}
            other => panic!("expected Created for retired SKU, got {other:?}"),
        }
        Ok(())
    }

    /// Import (`Review`): a SKU matching only a retired product must commit
    /// as `Created`, not `Skipped` with a manual-resolution reason. The
    /// retired row is not a conflict for the import commit.
    #[tokio::test]
    async fn import_review_creates_when_sku_matches_only_retired(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        seed_retired_product(
            &pool,
            "RETIRED-SKU",
            "Retired predecessor",
            None,
            "Discontinued",
        )
        .await?;

        let csv_content = "sku,description\nRETIRED-SKU,Fresh successor\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Review,
            },
        )
        .await?;

        assert_eq!(result.created, 1);
        assert_eq!(result.skipped, 0);
        match &result.rows[0].outcome {
            CsvImportRowOutcome::Created { .. } => {}
            other => panic!("expected Created for retired SKU, got {other:?}"),
        }
        Ok(())
    }

    /// Import (`Skip`): a barcode matching only a retired product must
    /// commit as `Created` and attach the barcode to the new product.
    /// Mirrors the SKU release contract.
    #[tokio::test]
    async fn import_skip_creates_and_attaches_when_barcode_matches_only_retired(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        seed_retired_product(
            &pool,
            "RETIRED-BC",
            "Retired barcode owner",
            Some("7506666666666"),
            "Discontinued",
        )
        .await?;

        let csv_content = "sku,description,barcode\nFRESH-BC,Fresh product,7506666666666\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await?;

        assert_eq!(result.created, 1);
        assert_eq!(result.skipped, 0);
        match &result.rows[0].outcome {
            CsvImportRowOutcome::Created { sku, .. } => assert_eq!(sku, "FRESH-BC"),
            other => panic!("expected Created for released barcode, got {other:?}"),
        }

        // Verify the barcode was attached to the new product.
        let new_id = match &result.rows[0].outcome {
            CsvImportRowOutcome::Created { product_id, .. } => product_id.clone(),
            _ => unreachable!(),
        };
        let barcodes = products_service::list_barcodes(&pool, new_id).await?;
        assert!(
            barcodes.iter().any(|b| b.barcode == "7506666666666"),
            "released barcode must attach to the newly created product"
        );
        Ok(())
    }

    /// Regression guard: an active duplicate SKU still skips/updates/reviews
    /// under the original conflict semantics. Released vs. duplicate split
    /// is gated on `lifecycle`, not on the existence of any row.
    #[tokio::test]
    async fn import_skip_still_skips_active_duplicate_sku() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        products_service::create_product(
            &pool,
            ProductCreate {
                sku: "ACTIVE-DUP".into(),
                description: "Active duplicate".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: None,
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        let csv_content = "sku,description\nACTIVE-DUP,Fresh description\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await?;

        assert_eq!(result.created, 0);
        assert_eq!(result.skipped, 1);
        match &result.rows[0].outcome {
            CsvImportRowOutcome::Skipped { reason_code, .. } => {
                assert_eq!(
                    reason_code.as_deref(),
                    Some(REASON_CODE_SKU_ALREADY_EXISTS),
                    "active duplicate SKU must keep the existing skip semantics"
                );
            }
            other => panic!("expected Skipped for active duplicate, got {other:?}"),
        }
        Ok(())
    }

    /// Regression guard for the manual reproduction case
    /// (`/home/xeworg/Documentos/testimportacion.csv`). When a CSV row's SKU
    /// matches both a retired historical row AND a freshly-committed active
    /// row created earlier in the same import, the `INSERT` will trip the
    /// partial UNIQUE index `uq_products_sku_active WHERE lifecycle !=
    /// 'retired'`. The import must convert that infrastructure error into a
    /// row-level `Skipped` outcome (`reason_code = sku_already_exists`) rather
    /// than bubbling it up as a generic "An internal error occurred" command
    /// failure that aborts the whole import and leaves behind partial state.
    ///
    /// Reproduction recipe:
    /// 1. Seed a retired product for SKU `RETIRED-RACE`.
    /// 2. Drive the import with two rows: one reuses `RETIRED-RACE` (becomes
    ///    a fresh `Created` row), one reuses the same SKU a second time.
    /// 3. The second `INSERT` triggers the UNIQUE violation that used to
    ///    become an internal error.
    #[tokio::test]
    async fn import_sku_race_returns_row_skip_not_internal_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        seed_retired_product(
            &pool,
            "RETIRED-RACE",
            "Retired predecessor",
            None,
            "Discontinued",
        )
        .await?;

        let csv_content = "sku,description\n\
                           RETIRED-RACE,Fresh successor A\n\
                           RETIRED-RACE,Fresh successor B\n";
        let result = import_product_csv(
            &pool,
            CsvImportInput {
                content: csv_content.to_string(),
                mapping: CsvColumnMapping::default(),
                strategy: ConflictStrategy::Skip,
            },
        )
        .await?;

        assert_eq!(result.total_rows, 2);
        assert_eq!(
            result.created, 1,
            "first row should commit by reusing the released identifier"
        );
        assert_eq!(
            result.skipped, 1,
            "second row must be Skipped with reason_code=sku_already_exists, not crash"
        );
        assert_eq!(result.invalid, 0);

        match &result.rows[0].outcome {
            CsvImportRowOutcome::Created { sku, .. } => assert_eq!(sku, "RETIRED-RACE"),
            other => panic!("expected Created for first row, got {other:?}"),
        }
        match &result.rows[1].outcome {
            CsvImportRowOutcome::Skipped { reason_code, .. } => {
                assert_eq!(
                    reason_code.as_deref(),
                    Some(REASON_CODE_SKU_ALREADY_EXISTS),
                    "duplicate SKU must surface a row-level Skip instead of an internal error"
                );
            }
            other => panic!("expected Skipped for second row, got {other:?}"),
        }
        Ok(())
    }
}
