//! DTOs for backend report preview and metadata (Slice 11a).
//!
//! These types define the Tauri IPC boundary for the `preview_report` command.
//! The actual rendering (PDF, A4 layout, pagination, page numbers, CSV wiring)
//! is intentionally deferred to later slices. The backend in this slice only
//! produces a structured preview payload: metadata + sorted lot rows.
//!
//! Reports reuse the dashboard's urgency classification and sort order by
//! delegating to `services::dashboard::get_dashboard`. That keeps a single
//! source of truth for "what does an alert-window / expired / next-30-days
//! report mean" between the dashboard UI, the report preview, and any future
//! PDF/CSV consumer.

use serde::{Deserialize, Serialize};

// ============================================================
// Report type
// ============================================================

/// Built-in report types that map to dashboard urgency buckets.
///
/// `Custom` does NOT map to a dashboard preset — it lets the caller pick an
/// arbitrary urgency (or none) and combine it with category / date-range
/// filters via `ReportFilters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    /// Lots whose expiry falls inside their per-lot alert window
    /// (`alert_days_before > 0`, `0 < diff <= alert_days_before`).
    InAlertWindow,
    /// Lots whose expiry date is strictly in the past.
    Expired,
    /// Lots expiring within the next 30 calendar days.
    #[serde(rename = "next_30_days")]
    Next30Days,
    /// Ad-hoc report driven entirely by `ReportFilters`.
    Custom,
}

impl ReportType {
    /// Returns the snake_case string used in metadata / JSON output.
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportType::InAlertWindow => "in_alert_window",
            ReportType::Expired => "expired",
            ReportType::Next30Days => "next_30_days",
            ReportType::Custom => "custom",
        }
    }

    /// Returns a human-readable label for the report type. Used in the report
    /// metadata so the preview UI / PDF can show a stable description without
    /// having to map snake_case values back to UI labels.
    pub fn description(&self) -> &'static str {
        match self {
            ReportType::InAlertWindow => "Lots in their alert window",
            ReportType::Expired => "Expired lots",
            ReportType::Next30Days => "Lots expiring in the next 30 days",
            ReportType::Custom => "Custom filtered report",
        }
    }
}

// ============================================================
// Filters
// ============================================================

/// Filters supported by reports. Built-in reports honour `store_id` and
/// `location_id`; `Custom` reports honour every field below.
///
/// `category_ids`, `urgency`, `date_from`, and `date_to` are applied as
/// post-filters on top of the dashboard query, so they can be combined with
/// any built-in report type without forking the SQL.
/// `category_ids` replaces the legacy `category_id` field.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ReportFilters {
    /// Restrict to a specific store. `None` means every store.
    pub store_id: Option<String>,
    /// Restrict to a specific internal location. `None` means every location.
    pub location_id: Option<String>,
    /// Restrict to products in the given categories (ANY-of semantics). May
    /// include `UNCATEGORIZED_SENTINEL` to also include products with zero
    /// active category relations. `None` or empty list means no filter.
    pub category_ids: Option<Vec<String>>,
    /// Restrict to a specific urgency bucket (snake_case string from the
    /// dashboard preset vocabulary). For `Custom` reports this becomes the
    /// dashboard urgency filter; for built-in reports it is overridden by
    /// the report type's preset.
    pub urgency: Option<String>,
    /// Inclusive lower bound on `expiry_date` (YYYY-MM-DD). Blank treated as
    /// "no lower bound".
    pub date_from: Option<String>,
    /// Inclusive upper bound on `expiry_date` (YYYY-MM-DD). Blank treated as
    /// "no upper bound".
    pub date_to: Option<String>,
}

// ============================================================
// Request
// ============================================================

/// A request to build a report preview payload.
///
/// The field is named `kind` (not `type`) to avoid the Rust reserved keyword
/// while keeping the JSON shape obvious for callers.
#[derive(Debug, Deserialize)]
pub struct ReportRequest {
    /// Report type to build.
    pub kind: ReportType,
    /// Optional filters. `None` and an empty struct both mean "no filters".
    pub filters: Option<ReportFilters>,
}

// ============================================================
// Metadata
// ============================================================

/// Metadata describing a generated report. Returned alongside the lot rows so
/// the preview UI / PDF header can render type, effective filters, and
/// generation timestamp without a second round-trip.
#[derive(Debug, Clone, Serialize)]
pub struct ReportMetadata {
    /// Report type discriminator (snake_case string).
    pub report_type: String,
    /// Human-readable description for the chosen type.
    pub description: String,
    /// Snapshot of the effective filters that produced this report. When a
    /// filter was ignored (e.g. `urgency` overridden by a built-in preset), the
    /// field still reflects the user-supplied value so the metadata is a
    /// faithful record of the request.
    pub filters_used: ReportFilters,
    /// Generation timestamp in RFC3339 / ISO-8601 form (UTC).
    pub generated_at: String,
    /// Number of lots in `ReportData::lots` after all filters are applied.
    pub row_count: usize,
}

// ============================================================
// Response
// ============================================================

/// Report preview payload: metadata + sorted, classified lot rows.
///
/// Lot rows reuse the existing `DashboardLotRow` shape so the report preview,
/// dashboard, and future CSV/PDF consumers all share the same column
/// vocabulary. Built-in reports return rows in dashboard urgency order
/// (expired first). Custom reports inherit that ordering from the dashboard
/// service unless an `urgency` filter narrows the bucket.
#[derive(Debug, Serialize)]
pub struct ReportData {
    pub metadata: ReportMetadata,
    pub lots: Vec<crate::dto::dashboard::DashboardLotRow>,
}
