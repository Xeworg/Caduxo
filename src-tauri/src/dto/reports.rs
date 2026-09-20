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

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

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

    /// Returns the short, title-cased display label for the report type in
    /// the given locale. Suitable for compact surfaces like the PDF header
    /// subtitle (`"Expired: Expired lots"`); for the longer, full-sentence
    /// description used in the metadata block, use
    /// [`ReportType::description`].
    ///
    /// This is the **single source of truth** for built-in report-type
    /// labels. Adding a new locale must only require touching this method
    /// (and `description`) — renderers must not duplicate the per-variant
    /// mapping.
    pub fn label(&self, locale: crate::pdf::locale::Locale) -> Cow<'static, str> {
        use crate::pdf::locale::Locale as L;
        match (self, locale) {
            (ReportType::InAlertWindow, L::En) => Cow::Borrowed("In alert window"),
            (ReportType::InAlertWindow, L::Es) => Cow::Borrowed("En ventana de alerta"),
            (ReportType::Expired, L::En) => Cow::Borrowed("Expired"),
            (ReportType::Expired, L::Es) => Cow::Borrowed("Vencidos"),
            (ReportType::Next30Days, L::En) => Cow::Borrowed("Next 30 days"),
            (ReportType::Next30Days, L::Es) => Cow::Borrowed("Próximos 30 días"),
            (ReportType::Custom, L::En) => Cow::Borrowed("Custom"),
            (ReportType::Custom, L::Es) => Cow::Borrowed("Personalizado"),
        }
    }

    /// Returns a human-readable label for the report type in the given locale.
    /// Used in the report metadata so the preview UI / PDF can show a stable
    /// description without having to map snake_case values back to UI labels.
    pub fn description(&self, locale: crate::pdf::locale::Locale) -> Cow<'static, str> {
        use crate::pdf::locale::Locale as L;
        match (self, locale) {
            (ReportType::InAlertWindow, L::En) => Cow::Borrowed("Lots in their alert window"),
            (ReportType::InAlertWindow, L::Es) => Cow::Borrowed("Lotes en ventana de alerta"),
            (ReportType::Expired, L::En) => Cow::Borrowed("Expired lots"),
            (ReportType::Expired, L::Es) => Cow::Borrowed("Lotes vencidos"),
            (ReportType::Next30Days, L::En) => Cow::Borrowed("Lots expiring in the next 30 days"),
            (ReportType::Next30Days, L::Es) => {
                Cow::Borrowed("Lotes que vencen en los próximos 30 días")
            }
            (ReportType::Custom, L::En) => Cow::Borrowed("Custom filtered report"),
            (ReportType::Custom, L::Es) => Cow::Borrowed("Reporte personalizado filtrado"),
        }
    }
}

impl FromStr for ReportType {
    type Err = ReportTypeParseError;

    /// Parses a snake_case report-type discriminator (the same shape used by
    /// [`ReportType::as_str`] and the `report_type` field of
    /// [`ReportMetadata`]) back into a [`ReportType`].
    ///
    /// Round-trip invariant: `s.parse::<ReportType>().ok().map(|r| r.as_str()) == Some(s)`
    /// for every built-in variant.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "in_alert_window" => Ok(ReportType::InAlertWindow),
            "expired" => Ok(ReportType::Expired),
            "next_30_days" => Ok(ReportType::Next30Days),
            "custom" => Ok(ReportType::Custom),
            other => Err(ReportTypeParseError(other.to_string())),
        }
    }
}

/// Error returned when a string cannot be parsed into a [`ReportType`].
/// The wrapped string preserves the original input so callers can surface
/// it in error messages or logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportTypeParseError(pub String);

impl fmt::Display for ReportTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown report type: {:?}", self.0)
    }
}

impl std::error::Error for ReportTypeParseError {}

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

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::locale::Locale;
    use std::str::FromStr;

    #[test]
    fn label_is_localized_for_every_variant() {
        // Every (variant, locale) pair must produce a stable label so the
        // PDF header subtitle and any future surface share one source of
        // truth. Adding a new locale must only require editing
        // `ReportType::label` and `ReportType::description`.
        let cases = [
            (ReportType::InAlertWindow, Locale::En, "In alert window"),
            (
                ReportType::InAlertWindow,
                Locale::Es,
                "En ventana de alerta",
            ),
            (ReportType::Expired, Locale::En, "Expired"),
            (ReportType::Expired, Locale::Es, "Vencidos"),
            (ReportType::Next30Days, Locale::En, "Next 30 days"),
            (ReportType::Next30Days, Locale::Es, "Próximos 30 días"),
            (ReportType::Custom, Locale::En, "Custom"),
            (ReportType::Custom, Locale::Es, "Personalizado"),
        ];
        for (rt, locale, expected) in cases {
            assert_eq!(
                rt.label(locale).as_ref(),
                expected,
                "wrong label for {rt:?} in {locale:?}"
            );
        }
    }

    #[test]
    fn label_is_shorter_than_or_equal_to_description() {
        // The label is the title-cased short form used in compact surfaces
        // (PDF header subtitle). The description is the full-sentence
        // version. Both must be non-empty and the label must not be longer
        // than the description — that is the actual contract that lets the
        // renderer substitute the label in tight layouts without overflow.
        // We intentionally do NOT assert that the description starts with
        // the label, because some locales surface the description as a
        // sentence that rephrases the topic rather than appending to it
        // (e.g. English `In alert window` / `Lots in their alert window`).
        for rt in [
            ReportType::InAlertWindow,
            ReportType::Expired,
            ReportType::Next30Days,
            ReportType::Custom,
        ] {
            for locale in [Locale::En, Locale::Es] {
                let label = rt.label(locale);
                let description = rt.description(locale);
                assert!(
                    !label.is_empty(),
                    "label for {rt:?}/{locale:?} must not be empty"
                );
                assert!(
                    !description.is_empty(),
                    "description for {rt:?}/{locale:?} must not be empty"
                );
                assert!(
                    label.len() <= description.len(),
                    "label `{label}` should not be longer than description `{description}` for {rt:?}/{locale:?}"
                );
            }
        }
    }

    #[test]
    fn label_returns_borrowed_for_known_variants() {
        // Returning `Cow::Borrowed` for built-in variants keeps the per-render
        // allocation profile flat; this guards against an accidental `format!`
        // regression that would allocate a new `String` per call.
        for rt in [
            ReportType::InAlertWindow,
            ReportType::Expired,
            ReportType::Next30Days,
            ReportType::Custom,
        ] {
            for locale in [Locale::En, Locale::Es] {
                assert!(
                    matches!(rt.label(locale), Cow::Borrowed(_)),
                    "{rt:?}/{locale:?} label should be Cow::Borrowed, got {:?}",
                    rt.label(locale)
                );
            }
        }
    }

    #[test]
    fn from_str_roundtrips_through_as_str_for_every_variant() {
        // The round-trip invariant — `as_str` → `from_str` → original
        // variant — is what lets the PDF renderer rely on the metadata
        // string to recover a `ReportType`.
        for rt in [
            ReportType::InAlertWindow,
            ReportType::Expired,
            ReportType::Next30Days,
            ReportType::Custom,
        ] {
            let parsed = ReportType::from_str(rt.as_str())
                .unwrap_or_else(|e| panic!("from_str({:?}) failed: {e}", rt.as_str()));
            assert_eq!(parsed, rt);
        }
    }

    #[test]
    fn from_str_rejects_unknown_strings_and_preserves_input() {
        let err = ReportType::from_str("future_type").expect_err("must reject unknown input");
        assert_eq!(err.0, "future_type");

        let err = ReportType::from_str("").expect_err("must reject empty input");
        assert_eq!(err.0, "");
    }

    #[test]
    fn parse_error_display_includes_input_for_diagnostics() {
        let err = ReportType::from_str("not-a-type").expect_err("must reject");
        let msg = err.to_string();
        assert!(
            msg.contains("not-a-type"),
            "error message should include the offending input, got: {msg}"
        );
    }
}
