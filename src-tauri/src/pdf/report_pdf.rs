//! Structured PDF rendering for the Caduxo report preview (Slice 11b).
//!
//! This module renders a [`ReportData`] payload to a single multi-page A4
//! landscape PDF on disk. The layout is intentionally fixed:
//!
//! ```text
//!   ┌─────────────────────────────────────────────────────────────┐
//!   │ Caduxo – Report                                              │  title
//!   │ Expired lots                                                 │  type + description
//!   │ Generated: 2025-01-15T10:30:00Z  · Rows: 12                  │  metadata line
//!   │ Filters: store=Main · category=Dairy                         │  filter snapshot
//!   ├─────────────────────────────────────────────────────────────┤
//!   │ SKU | Description | Store / Location | Qty | Expiry | ...    │  table header
//!   │ SKU-001 | Milk 1L | Main / Cold-room | 4 L | 12/01/2025 |…  │  data row
//!   │ ...                                                          │
//!   ├─────────────────────────────────────────────────────────────┤
//!   │ Page 1 of 3                                  Caduxo Report  │  footer
//!   └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! Pagination is row-based. The first page starts at the top of the body
//! band; when the next row would push below the body bottom (above the
//! footer band), a new page is created. Empty reports still render one
//! page with the header and an "No rows" notice.
//!
//! All visible text comes from the report DTO and the locale-aware PDF
//! message table ([`crate::pdf::locale::pdf_messages`]). PDF bytes are
//! written to the path provided by the caller; the function does not log
//! row contents.

use std::borrow::Cow;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use printpdf::{
    BuiltinFont, IndirectFontRef, Line, Mm, PdfDocument, PdfLayerIndex, PdfLayerReference,
    PdfPageIndex, Point,
};
use serde::Serialize;

use crate::dto::dashboard::DashboardLotRow;
use crate::dto::reports::{ReportData, ReportFilters, ReportMetadata, ReportType};
use crate::error::{AppError, InfrastructureError};
use crate::pdf::locale::{self, Locale, PdfMessages};

// ============================================================
// Page geometry (A4 landscape)
// ============================================================

/// A4 landscape width in millimetres.
const PAGE_WIDTH_MM: f32 = 297.0;
/// A4 landscape height in millimetres.
const PAGE_HEIGHT_MM: f32 = 210.0;
const MARGIN_LEFT_MM: f32 = 12.0;
const MARGIN_RIGHT_MM: f32 = 12.0;
const MARGIN_TOP_MM: f32 = 10.0;
const MARGIN_BOTTOM_MM: f32 = 10.0;

// Layout bands.
const HEADER_HEIGHT_MM: f32 = 30.0;
const FOOTER_HEIGHT_MM: f32 = 10.0;
const ROW_HEIGHT_MM: f32 = 6.0;
const TABLE_HEADER_ROW_HEIGHT_MM: f32 = 7.0;

// Derived body band.
const BODY_TOP_Y: f32 = PAGE_HEIGHT_MM - MARGIN_TOP_MM - HEADER_HEIGHT_MM;
const BODY_BOTTOM_Y: f32 = MARGIN_BOTTOM_MM + FOOTER_HEIGHT_MM;
/// Data rows per page (after the table header row). The body band has space
/// for `TABLE_HEADER_ROW_HEIGHT_MM + N * ROW_HEIGHT_MM`.
///
/// Derived from the current A4 landscape body band:
/// `(BODY_TOP_Y - BODY_BOTTOM_Y - TABLE_HEADER_ROW_HEIGHT_MM) / ROW_HEIGHT_MM`.
const DATA_ROWS_PER_PAGE: usize = 23;

// Font sizes (in PDF points).
const FONT_SIZE_TITLE: f32 = 16.0;
const FONT_SIZE_SUBTITLE: f32 = 10.0;
const FONT_SIZE_META: f32 = 8.5;
const FONT_SIZE_TABLE_HEADER: f32 = 8.5;
const FONT_SIZE_BODY: f32 = 8.0;
const FONT_SIZE_FOOTER: f32 = 8.0;

// Column widths (header text comes from PdfMessages.columns).
struct ColumnDef {
    width: f32,
}

const COLUMNS: &[ColumnDef] = &[
    ColumnDef { width: 35.0 },
    ColumnDef { width: 65.0 },
    ColumnDef { width: 55.0 },
    ColumnDef { width: 22.0 },
    ColumnDef { width: 28.0 },
    ColumnDef { width: 20.0 },
    ColumnDef { width: 20.0 },
    ColumnDef { width: 28.0 },
];

fn table_left_x() -> f32 {
    MARGIN_LEFT_MM
}

fn table_right_x() -> f32 {
    MARGIN_LEFT_MM + COLUMNS.iter().map(|c| c.width).sum::<f32>()
}

fn column_x_left(idx: usize) -> f32 {
    let mut x = MARGIN_LEFT_MM;
    for c in &COLUMNS[..idx] {
        x += c.width;
    }
    x
}

// ============================================================
// Rendering
// ============================================================

/// Renders the given report payload to a PDF file at `path`. The file is
/// truncated if it exists. Returns the absolute path, row count, and byte
/// count written.
///
/// The `locale` parameter controls all user-visible strings inside the PDF.
/// Use [`Locale::parse`] to normalise a BCP-47 tag before calling this
/// function.
///
/// All visible text comes from the report DTO and the locale-aware PDF
/// message table. No product SKU, barcode, description, or notes content
/// is logged by this function.
pub fn render_report(
    data: &ReportData,
    path: &Path,
    locale: Locale,
) -> Result<RenderedReport, AppError> {
    let msgs = locale::pdf_messages(locale);

    let total_rows = data.lots.len();
    let total_pages = if total_rows == 0 {
        1
    } else {
        total_rows.div_ceil(DATA_ROWS_PER_PAGE)
    };

    let (doc, first_page, first_layer) = PdfDocument::new(
        msgs.document_title.as_ref(),
        Mm(PAGE_WIDTH_MM),
        Mm(PAGE_HEIGHT_MM),
        "Layer 1",
    );

    let regular = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(pdf_error)?;
    let bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(pdf_error)?;

    // Create all pages up-front so we know the total page count and can
    // render "Page X of Y" in the footer.
    let mut pages: Vec<(PdfPageIndex, PdfLayerIndex)> = vec![(first_page, first_layer)];
    for _ in 1..total_pages {
        let (p, l) = doc.add_page(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), "Layer 1");
        pages.push((p, l));
    }

    // Distribute rows across pages.
    for (page_idx, (page_idx_ref, layer_idx_ref)) in pages.iter().enumerate() {
        let layer = doc.get_page(*page_idx_ref).get_layer(*layer_idx_ref);
        let page_num = page_idx + 1;

        draw_header(&layer, &data.metadata, &msgs, &bold, &regular, locale);
        draw_table_header(&layer, &msgs, &bold);

        let start_row = page_idx * DATA_ROWS_PER_PAGE;
        let end_row = (start_row + DATA_ROWS_PER_PAGE).min(total_rows);
        let mut y = BODY_TOP_Y - TABLE_HEADER_ROW_HEIGHT_MM;
        for lot in &data.lots[start_row..end_row] {
            draw_table_row(&layer, y, lot, &msgs, &regular, locale);
            y -= ROW_HEIGHT_MM;
        }

        if total_rows == 0 {
            draw_empty_notice(&layer, &msgs, &regular);
        }

        draw_table_grid(&layer, start_row, end_row);
        draw_footer(&layer, page_num, total_pages, &msgs, &regular);
    }

    let file =
        File::create(path).map_err(|e| AppError::Infrastructure(InfrastructureError::Io(e)))?;
    doc.save(&mut BufWriter::new(file)).map_err(pdf_error)?;

    let bytes_written = std::fs::metadata(path)
        .map_err(|e| AppError::Infrastructure(InfrastructureError::Io(e)))?
        .len();

    Ok(RenderedReport {
        path: path.to_string_lossy().into_owned(),
        page_count: total_pages,
        rows_written: total_rows,
        bytes_written,
    })
}

/// Result of a successful PDF render.
#[derive(Debug, Clone, Serialize)]
pub struct RenderedReport {
    pub path: String,
    pub page_count: usize,
    pub rows_written: usize,
    pub bytes_written: u64,
}

// ============================================================
// Header / footer drawing
// ============================================================

fn draw_header(
    layer: &PdfLayerReference,
    metadata: &ReportMetadata,
    msgs: &PdfMessages,
    bold: &IndirectFontRef,
    regular: &IndirectFontRef,
    locale: Locale,
) {
    let title_x = MARGIN_LEFT_MM;
    let title_y = PAGE_HEIGHT_MM - MARGIN_TOP_MM - 8.0;
    layer.use_text(
        msgs.document_title.as_ref(),
        FONT_SIZE_TITLE,
        Mm(title_x),
        Mm(title_y),
        bold,
    );

    // Report type + description on the left, generated_at on the right.
    let subtitle_y = title_y - 6.5;
    let subtitle = report_subtitle(metadata, locale);
    layer.use_text(
        truncate(&subtitle, 90),
        FONT_SIZE_SUBTITLE,
        Mm(title_x),
        Mm(subtitle_y),
        regular,
    );

    let generated_at_human = humanize_generated_at(&metadata.generated_at, locale);
    // Substitute {date} and {n} in the template.
    let meta_line_left = msgs
        .header_generated
        .replace("{date}", &generated_at_human)
        .replace("{n}", &metadata.row_count.to_string());
    layer.use_text(
        truncate(&meta_line_left, 90),
        FONT_SIZE_META,
        Mm(title_x),
        Mm(subtitle_y - 5.0),
        regular,
    );

    let filter_line = format_filters(&metadata.filters_used, msgs);
    if !filter_line.is_empty() {
        layer.use_text(
            format!("{} {}", msgs.header_filters_prefix, filter_line),
            FONT_SIZE_META,
            Mm(title_x),
            Mm(subtitle_y - 9.5),
            regular,
        );
    }

    // Horizontal rule under the header band.
    let rule_y = BODY_TOP_Y + 2.0;
    draw_horizontal_rule(
        layer,
        MARGIN_LEFT_MM,
        PAGE_WIDTH_MM - MARGIN_RIGHT_MM,
        rule_y,
        0.4,
    );
}

fn draw_footer(
    layer: &PdfLayerReference,
    page_num: usize,
    total_pages: usize,
    msgs: &PdfMessages,
    regular: &IndirectFontRef,
) {
    let y = MARGIN_BOTTOM_MM + 4.0;
    // Substitute {n} and {m} in "Page {n} of {m}".
    let left = msgs
        .footer_page_of
        .replace("{n}", &page_num.to_string())
        .replace("{m}", &total_pages.to_string());
    layer.use_text(&left, FONT_SIZE_FOOTER, Mm(MARGIN_LEFT_MM), Mm(y), regular);

    let right = msgs.brand.as_ref();
    let text_width_mm = approximate_text_width_mm(right, FONT_SIZE_FOOTER);
    let right_x = PAGE_WIDTH_MM - MARGIN_RIGHT_MM - text_width_mm;
    layer.use_text(right, FONT_SIZE_FOOTER, Mm(right_x), Mm(y), regular);

    let rule_y = BODY_BOTTOM_Y + 1.0;
    draw_horizontal_rule(
        layer,
        MARGIN_LEFT_MM,
        PAGE_WIDTH_MM - MARGIN_RIGHT_MM,
        rule_y,
        0.4,
    );
}

fn draw_table_header(layer: &PdfLayerReference, msgs: &PdfMessages, bold: &IndirectFontRef) {
    let y = BODY_TOP_Y - 5.0;
    for (idx, col_header) in msgs.columns.iter().enumerate() {
        let x = column_x_left(idx) + 1.5;
        layer.use_text(
            col_header.as_ref(),
            FONT_SIZE_TABLE_HEADER,
            Mm(x),
            Mm(y),
            bold,
        );
    }
    // Horizontal rule under the table header.
    let rule_y = BODY_TOP_Y - TABLE_HEADER_ROW_HEIGHT_MM;
    draw_horizontal_rule(layer, MARGIN_LEFT_MM, table_right_x(), rule_y, 0.4);
}

fn draw_table_row(
    layer: &PdfLayerReference,
    y: f32,
    lot: &DashboardLotRow,
    msgs: &PdfMessages,
    regular: &IndirectFontRef,
    locale: Locale,
) {
    let baseline = y - 4.0;
    let cells: [String; 8] = [
        truncate(&lot.sku, 18),
        truncate(&lot.description, 36),
        truncate(&format_store_location(lot, msgs), 30),
        format!("{} {}", format_qty(lot.quantity, locale), lot.unit),
        format_iso_date_for_locale(&lot.expiry_date, locale),
        format_days(lot.days_remaining, msgs),
        lot.alert_days_before.to_string(),
        truncate(lot.batch_code.as_deref().unwrap_or(""), 16),
    ];
    for (idx, text) in cells.iter().enumerate() {
        let x = column_x_left(idx) + 1.5;
        layer.use_text(text, FONT_SIZE_BODY, Mm(x), Mm(baseline), regular);
    }
}

fn draw_empty_notice(layer: &PdfLayerReference, msgs: &PdfMessages, regular: &IndirectFontRef) {
    let y = BODY_TOP_Y - TABLE_HEADER_ROW_HEIGHT_MM - 6.0;
    layer.use_text(
        msgs.empty_notice.as_ref(),
        FONT_SIZE_BODY,
        Mm(MARGIN_LEFT_MM + 1.5),
        Mm(y),
        regular,
    );
}

fn draw_table_grid(layer: &PdfLayerReference, start_row: usize, end_row: usize) {
    let row_count = end_row.saturating_sub(start_row);
    let top_y = BODY_TOP_Y;
    let bottom_y = BODY_TOP_Y - TABLE_HEADER_ROW_HEIGHT_MM - (row_count as f32) * ROW_HEIGHT_MM;

    let left = table_left_x();
    let right = table_right_x();

    draw_horizontal_rule(layer, left, right, top_y, 0.4);
    draw_horizontal_rule(layer, left, right, bottom_y, 0.4);
    let mut y = top_y - TABLE_HEADER_ROW_HEIGHT_MM;
    for _ in 0..row_count {
        draw_horizontal_rule(layer, left, right, y, 0.2);
        y -= ROW_HEIGHT_MM;
    }
    for idx in 0..=COLUMNS.len() {
        let x = column_x_left(idx);
        draw_vertical_rule(layer, x, top_y, bottom_y, 0.2);
    }
}

// ============================================================
// Geometry helpers
// ============================================================

fn draw_horizontal_rule(layer: &PdfLayerReference, x_from: f32, x_to: f32, y: f32, weight_pt: f32) {
    let line = Line {
        points: vec![
            (Point::new(Mm(x_from), Mm(y)), false),
            (Point::new(Mm(x_to), Mm(y)), false),
        ],
        is_closed: false,
    };
    layer.set_outline_thickness(weight_pt);
    layer.add_line(line);
}

fn draw_vertical_rule(
    layer: &PdfLayerReference,
    x: f32,
    y_top: f32,
    y_bottom: f32,
    weight_pt: f32,
) {
    let line = Line {
        points: vec![
            (Point::new(Mm(x), Mm(y_top)), false),
            (Point::new(Mm(x), Mm(y_bottom)), false),
        ],
        is_closed: false,
    };
    layer.set_outline_thickness(weight_pt);
    layer.add_line(line);
}

fn pdf_error(e: printpdf::Error) -> AppError {
    AppError::Infrastructure(InfrastructureError::Pdf(e))
}

// ============================================================
// String helpers
// ============================================================

/// Formats a quantity value with locale-appropriate decimal separator.
///
/// - Integer-valued inputs are always stripped of the trailing `.0`, in both
///   locales. This keeps existing `format!("{qty}")` behaviour for whole
///   numbers.
/// - Fractional values use `.` for English and `,` for Spanish. The number of
///   fractional digits is preserved (no rounding) by reusing Rust's default
///   `Display` formatting for `f64`.
fn format_qty(qty: f64, locale: Locale) -> String {
    if qty.fract() == 0.0 {
        format!("{}", qty as i64)
    } else {
        let rendered = format!("{qty}");
        match locale {
            Locale::En => rendered,
            Locale::Es => rendered.replace('.', ","),
        }
    }
}

/// Formats an ISO-8601 `YYYY-MM-DD` date string for the given locale.
///
/// - English (US convention): `MM/DD/YYYY`.
/// - Spanish (most common European/LatAm convention): `DD/MM/YYYY`.
///
/// Anything that does not match the `YYYY-MM-DD` shape is returned unchanged
/// so that malformed inputs never crash the renderer.
fn format_iso_date_for_locale(date_str: &str, locale: Locale) -> String {
    if date_str.len() == 10 && date_str.as_bytes()[4] == b'-' && date_str.as_bytes()[7] == b'-' {
        let bytes = date_str.as_bytes();
        let day = std::str::from_utf8(&bytes[8..10]).unwrap_or("");
        let month = std::str::from_utf8(&bytes[5..7]).unwrap_or("");
        let year = std::str::from_utf8(&bytes[0..4]).unwrap_or("");
        match locale {
            Locale::En => format!("{month}/{day}/{year}"),
            Locale::Es => format!("{day}/{month}/{year}"),
        }
    } else {
        date_str.to_string()
    }
}

/// Formats the relative-day cell shown in the "Days" column.
///
/// Negative values mean "expired by N days" and use the locale's singular
/// (`days_ago_one`) form for `-1` and plural (`days_ago_other`) form for any
/// other negative value. Non-negative values are rendered as the bare number
/// (`"0"`, `"7"`).
fn format_days(days: i64, msgs: &PdfMessages) -> String {
    if days < 0 {
        let template = if days == -1 {
            msgs.days_ago_one.as_ref()
        } else {
            msgs.days_ago_other.as_ref()
        };
        template.replace("{n}", &(-days).to_string())
    } else {
        days.to_string()
    }
}

fn format_store_location(lot: &DashboardLotRow, msgs: &PdfMessages) -> String {
    match &lot.location_name {
        Some(loc) if !loc.is_empty() => {
            format!("{}{}{}", lot.store_name, msgs.store_location_separator, loc)
        }
        _ => lot.store_name.clone(),
    }
}

/// Converts an RFC3339 timestamp into the human-readable line shown in the
/// PDF header, with the date portion rendered in the requested locale.
///
/// The time portion is rendered as `HH:MM:SS` (timezone offset and fractional
/// seconds are stripped). The date portion goes through
/// [`format_iso_date_for_locale`] so English shows `MM/DD/YYYY` and Spanish
/// shows `DD/MM/YYYY`.
///
/// Inputs that do not contain an RFC3339 `T` separator are returned unchanged
/// so that malformed timestamps never crash the renderer.
fn humanize_generated_at(rfc3339: &str, locale: Locale) -> String {
    if let Some(t_pos) = rfc3339.find('T') {
        let date = &rfc3339[..t_pos];
        let rest = &rfc3339[t_pos + 1..];
        let time = rest.split(['+', 'Z', '-']).next().unwrap_or(rest);
        let time = time.split('.').next().unwrap_or(time);
        format!("{} {}", format_iso_date_for_locale(date, locale), time)
    } else {
        rfc3339.to_string()
    }
}

fn format_filters(filters: &ReportFilters, msgs: &PdfMessages) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = filters.store_id.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("{}{}", msgs.filter_prefix_store, truncate(s, 16)));
    }
    if let Some(l) = filters.location_id.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!(
            "{}{}",
            msgs.filter_prefix_location,
            truncate(l, 16)
        ));
    }
    if let Some(ids) = filters.category_ids.as_ref() {
        if !ids.is_empty() {
            if ids.len() == 1 {
                // Single category: render the actual category id with the
                // singular key=value prefix (`category=` / `categoría=`).
                parts.push(format!(
                    "{}{}",
                    msgs.filter_prefix_category,
                    truncate(&ids[0], 16)
                ));
            } else {
                // Multiple categories: render the count form
                // (`categories ({n})` / `categorías ({n})`). CLDR plural form
                // is always "other" for n >= 2 in both supported locales.
                let template = msgs
                    .filter_prefix_categories_other
                    .replace("{n}", &ids.len().to_string());
                parts.push(template);
            }
        }
    }
    if let Some(u) = filters.urgency.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("{}{u}", msgs.filter_prefix_urgency));
    }
    if let Some(d) = filters.date_from.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("{}{d}", msgs.filter_prefix_from));
    }
    if let Some(d) = filters.date_to.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("{}{d}", msgs.filter_prefix_to));
    }
    parts.join(" · ")
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Resolves the `report_type` metadata string to a short localized display
/// label. Uses [`ReportType::label`] as the single source of truth for the
/// built-in variants so adding a new locale only requires updating the
/// domain type, not the renderer.
///
/// When the metadata string is not a known report-type code (e.g. a future
/// enum variant arrives before the renderer is updated), we fall back to a
/// capitalised version of the raw metadata so the header still shows
/// something sensible.
fn localized_report_type_label(raw: &str, locale: Locale) -> Cow<'static, str> {
    match raw.parse::<ReportType>() {
        Ok(rt) => rt.label(locale),
        Err(_) => Cow::Owned(capitalize(raw)),
    }
}

/// Builds the localized `Label: description` line that the PDF header shows
/// under the document title.
fn report_subtitle(metadata: &ReportMetadata, locale: Locale) -> String {
    let label = localized_report_type_label(metadata.report_type.as_str(), locale);
    format!("{label}: {}", metadata.description)
}

/// Truncates `s` to at most `max_chars` characters, appending `…` when truncated.
fn truncate(s: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(s.len());
    for (count, c) in s.chars().enumerate() {
        if count >= max_chars {
            out.push('…');
            return out;
        }
        out.push(c);
    }
    out
}

/// Approximate text width in millimetres for right-anchored text.
fn approximate_text_width_mm(text: &str, font_size_pt: f32) -> f32 {
    let avg_advance_pt = 0.5 * font_size_pt;
    let chars = text.chars().count() as f32;
    chars * avg_advance_pt * 0.3527778
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::locale::pdf_messages;

    fn make_metadata(locale: Locale) -> ReportMetadata {
        ReportMetadata {
            report_type: "expired".to_string(),
            description: ReportType::Expired.description(locale).into_owned(),
            filters_used: ReportFilters {
                store_id: Some("store-a".to_string()),
                location_id: None,
                category_ids: Some(vec!["dairy".to_string()]),
                urgency: None,
                date_from: None,
                date_to: None,
            },
            generated_at: "2025-01-15T10:30:00Z".to_string(),
            row_count: 7,
        }
    }

    fn make_lot(sku: &str, desc: &str, expiry: &str, days_remaining: i64) -> DashboardLotRow {
        DashboardLotRow {
            lot_id: format!("lot-{sku}"),
            product_id: format!("prod-{sku}"),
            sku: sku.to_string(),
            description: desc.to_string(),
            store_id: "store-a".to_string(),
            store_name: "Main".to_string(),
            location_id: None,
            location_name: None,
            quantity: 4.0,
            unit: "L".to_string(),
            expiry_date: expiry.to_string(),
            alert_days_before: 14,
            batch_code: Some("BC-001".to_string()),
            status: "active".to_string(),
            urgency: "expired".to_string(),
            days_remaining,
            default_unit_id: None,
            unit_type: None,
        }
    }

    fn empty_report(locale: Locale) -> ReportData {
        ReportData {
            metadata: make_metadata(locale),
            lots: vec![],
        }
    }

    #[test]
    fn table_width_matches_available_width() {
        let available = PAGE_WIDTH_MM - MARGIN_LEFT_MM - MARGIN_RIGHT_MM;
        let sum: f32 = COLUMNS.iter().map(|c| c.width).sum();
        assert!(
            sum <= available,
            "table width {sum} exceeds available {available}"
        );
    }

    #[test]
    fn column_x_left_is_monotonic() {
        let mut prev = MARGIN_LEFT_MM;
        for idx in 0..=COLUMNS.len() {
            let x = column_x_left(idx);
            assert!(x >= prev, "column {idx} starts at {x} before {prev}");
            prev = x;
        }
        assert!((column_x_left(COLUMNS.len()) - table_right_x()).abs() < 0.01);
    }

    #[test]
    fn format_qty_strips_trailing_zero_for_integer_values() {
        // Integer-valued quantities must be rendered without a decimal
        // separator in both locales — this matches the pre-locale behaviour
        // and avoids surprising users with `4,0` for whole units.
        assert_eq!(format_qty(4.0, Locale::En), "4");
        assert_eq!(format_qty(4.0, Locale::Es), "4");
        assert_eq!(format_qty(0.0, Locale::En), "0");
        assert_eq!(format_qty(0.0, Locale::Es), "0");
    }

    #[test]
    fn format_qty_uses_dot_decimal_separator_in_english() {
        assert_eq!(format_qty(4.5, Locale::En), "4.5");
        assert_eq!(format_qty(0.125, Locale::En), "0.125");
    }

    #[test]
    fn format_qty_uses_comma_decimal_separator_in_spanish() {
        // Regression: Spanish PDF output used to render quantities with a
        // `.` decimal separator (e.g. `4.5 L`) which is not idiomatic in
        // Spanish-speaking regions.
        assert_eq!(format_qty(4.5, Locale::Es), "4,5");
        assert_eq!(format_qty(0.125, Locale::Es), "0,125");
    }

    #[test]
    fn format_iso_date_for_locale_uses_mm_dd_yyyy_in_english() {
        // US English convention.
        assert_eq!(
            format_iso_date_for_locale("2025-01-15", Locale::En),
            "01/15/2025"
        );
        assert_eq!(
            format_iso_date_for_locale("2099-12-31", Locale::En),
            "12/31/2099"
        );
    }

    #[test]
    fn format_iso_date_for_locale_uses_dd_mm_yyyy_in_spanish() {
        // Spanish convention (matches most of Europe and Latin America).
        assert_eq!(
            format_iso_date_for_locale("2025-01-15", Locale::Es),
            "15/01/2025"
        );
        assert_eq!(
            format_iso_date_for_locale("2099-12-31", Locale::Es),
            "31/12/2099"
        );
    }

    #[test]
    fn format_iso_date_for_locale_passes_through_malformed_input() {
        // Malformed inputs must not crash the renderer.
        assert_eq!(
            format_iso_date_for_locale("not-a-date", Locale::En),
            "not-a-date"
        );
        assert_eq!(
            format_iso_date_for_locale("not-a-date", Locale::Es),
            "not-a-date"
        );
        assert_eq!(
            format_iso_date_for_locale("2025/01/15", Locale::En),
            "2025/01/15"
        );
    }

    #[test]
    fn format_days_uses_english_singular_template_for_one_expired_day() {
        // Regression: English PDF used to render "1 ago" (the prior Spanish
        // singular bug also affected English in this codebase). Singular for
        // n == 1 must use the `day` form.
        let msgs = pdf_messages(Locale::En);
        assert_eq!(format_days(-1, &msgs), "1 day ago");
    }

    #[test]
    fn format_days_uses_english_plural_template_for_many_expired_days() {
        let msgs = pdf_messages(Locale::En);
        assert_eq!(format_days(-2, &msgs), "2 days ago");
        assert_eq!(format_days(-30, &msgs), "30 days ago");
    }

    #[test]
    fn format_days_uses_spanish_singular_template_for_one_expired_day() {
        // Regression: Spanish PDF used to render `hace 1 días` for the
        // singular case. The pluralization rules must select the singular
        // template for n == 1.
        let msgs = pdf_messages(Locale::Es);
        assert_eq!(format_days(-1, &msgs), "hace 1 día");
    }

    #[test]
    fn format_days_uses_spanish_plural_template_for_many_expired_days() {
        let msgs = pdf_messages(Locale::Es);
        assert_eq!(format_days(-2, &msgs), "hace 2 días");
        assert_eq!(format_days(-30, &msgs), "hace 30 días");
    }

    #[test]
    fn format_days_renders_non_negative_as_bare_number() {
        let msgs = pdf_messages(Locale::En);
        assert_eq!(format_days(0, &msgs), "0");
        assert_eq!(format_days(7, &msgs), "7");
        let msgs = pdf_messages(Locale::Es);
        assert_eq!(format_days(0, &msgs), "0");
        assert_eq!(format_days(7, &msgs), "7");
    }

    #[test]
    fn format_store_location_uses_locale_separator() {
        // The separator must come from `PdfMessages.store_location_separator`
        // for both locales; in this codebase both happen to be `" / "`, but
        // the value must be sourced from the message table rather than
        // hardcoded.
        let mut lot = make_lot("S1", "Milk", "2025-01-01", -5);

        let en = pdf_messages(Locale::En);
        assert_eq!(format_store_location(&lot, &en), "Main");
        lot.location_name = Some("Cold-room".to_string());
        assert_eq!(format_store_location(&lot, &en), "Main / Cold-room");

        let es = pdf_messages(Locale::Es);
        lot.location_name = Some("Cámara fría".to_string());
        assert_eq!(format_store_location(&lot, &es), "Main / Cámara fría");
    }

    #[test]
    fn format_store_location_drops_empty_location_name() {
        // A location row may be present but empty (e.g. the lot lives in the
        // store itself, with no named sub-location). The renderer must not
        // emit a dangling separator in that case.
        let msgs = pdf_messages(Locale::En);
        let mut lot = make_lot("S1", "Milk", "2025-01-01", -5);
        lot.location_name = Some(String::new());
        assert_eq!(format_store_location(&lot, &msgs), "Main");
    }

    #[test]
    fn truncate_short_string_returns_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string_appends_ellipsis() {
        let out = truncate("a long description", 8);
        assert_eq!(out.chars().count(), 9);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn format_filters_uses_locale_prefixes_in_english() {
        let msgs = pdf_messages(Locale::En);
        let f = ReportFilters {
            store_id: Some("main".to_string()),
            location_id: Some("".to_string()),
            category_ids: Some(vec!["dairy".to_string()]),
            urgency: Some("expired".to_string()),
            date_from: Some("2025-01-01".to_string()),
            date_to: None,
        };
        let s = format_filters(&f, &msgs);
        assert!(s.contains("store=main"));
        assert!(s.contains("category=dairy"));
        assert!(s.contains("urgency=expired"));
        assert!(s.contains("from=2025-01-01"));
        assert!(!s.contains("location="));
        assert!(!s.contains("to="));
    }

    #[test]
    fn format_filters_uses_locale_prefixes_in_spanish() {
        let msgs = pdf_messages(Locale::Es);
        let f = ReportFilters {
            store_id: Some("Bodega".to_string()),
            category_ids: Some(vec!["dairy".to_string(), "bakery".to_string()]),
            ..Default::default()
        };
        let s = format_filters(&f, &msgs);
        assert!(s.contains("tienda=Bodega"), "got: {s}");
        assert!(s.contains("categorías (2)"), "got: {s}");
        assert!(!s.contains("store="), "got: {s}");
        assert!(!s.contains("category="), "got: {s}");
    }

    #[test]
    fn format_filters_empty_when_all_blank() {
        let msgs = pdf_messages(Locale::En);
        let f = ReportFilters::default();
        assert!(format_filters(&f, &msgs).is_empty());
    }

    #[test]
    fn humanize_generated_at_strips_timezone_and_subseconds_in_english() {
        // English convention renders the date portion as `MM/DD/YYYY`.
        assert_eq!(
            humanize_generated_at("2025-01-15T10:30:00Z", Locale::En),
            "01/15/2025 10:30:00"
        );
        assert_eq!(
            humanize_generated_at("2025-01-15T10:30:00.123+00:00", Locale::En),
            "01/15/2025 10:30:00"
        );
        assert_eq!(
            humanize_generated_at("not-a-date", Locale::En),
            "not-a-date"
        );
    }

    #[test]
    fn humanize_generated_at_uses_spanish_date_order() {
        // Spanish convention renders the date portion as `DD/MM/YYYY`.
        assert_eq!(
            humanize_generated_at("2025-01-15T10:30:00Z", Locale::Es),
            "15/01/2025 10:30:00"
        );
        assert_eq!(
            humanize_generated_at("2025-01-15T10:30:00.123+00:00", Locale::Es),
            "15/01/2025 10:30:00"
        );
        assert_eq!(
            humanize_generated_at("not-a-date", Locale::Es),
            "not-a-date"
        );
    }

    #[test]
    fn capitalize_uppercases_first_character_only() {
        assert_eq!(capitalize("expired"), "Expired");
        assert_eq!(capitalize("next_30_days"), "Next_30_days");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn approximate_text_width_scales_with_chars() {
        let one = approximate_text_width_mm("a", FONT_SIZE_FOOTER);
        let ten = approximate_text_width_mm("aaaaaaaaaa", FONT_SIZE_FOOTER);
        assert!(ten > one * 5.0);
    }

    // --- Pagination / render integration tests -----------------------------

    #[test]
    fn render_report_writes_pdf_file_with_empty_data_en() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("empty_en.pdf");
        let result = render_report(&empty_report(Locale::En), &out, Locale::En).expect("render");
        assert_eq!(result.rows_written, 0);
        assert_eq!(result.page_count, 1);
        assert!(result.bytes_written > 0);
        let bytes = std::fs::read(&out).expect("read");
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn render_report_writes_pdf_file_with_empty_data_es() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("empty_es.pdf");
        let result = render_report(&empty_report(Locale::Es), &out, Locale::Es).expect("render");
        assert_eq!(result.rows_written, 0);
        assert_eq!(result.page_count, 1);
        assert!(result.bytes_written > 0);
        let bytes = std::fs::read(&out).expect("read");
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn render_report_writes_pdf_file_with_many_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("many.pdf");
        let mut data = empty_report(Locale::En);
        for i in 0..60 {
            data.lots.push(make_lot(
                &format!("SKU-{i:03}"),
                &format!("Product {i}"),
                "2025-12-31",
                30,
            ));
        }
        data.metadata.row_count = data.lots.len();
        let result = render_report(&data, &out, Locale::En).expect("render");
        assert_eq!(result.rows_written, 60);
        assert!(result.page_count >= 2);
        assert!(result.bytes_written > 0);
        let bytes = std::fs::read(&out).expect("read");
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn render_report_pages_contain_page_marker_object() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("markers.pdf");
        let mut data = empty_report(Locale::En);
        for i in 0..(DATA_ROWS_PER_PAGE + 5) {
            data.lots.push(make_lot(
                &format!("S-{i}"),
                &format!("Desc {i}"),
                "2025-06-01",
                7,
            ));
        }
        data.metadata.row_count = data.lots.len();
        let result = render_report(&data, &out, Locale::En).expect("render");
        assert_eq!(result.page_count, 2, "fixture should force exactly 2 pages");

        let bytes = std::fs::read(&out).expect("read");
        let content = String::from_utf8_lossy(&bytes);
        assert!(
            content.contains("/Type/Pages/Count 2"),
            "expected /Type/Pages/Count 2 in PDF metadata, got snippet: {:?}",
            content
                .split('/')
                .find(|s| s.starts_with("Type/Pages"))
                .unwrap_or("<not found>")
        );
    }

    #[test]
    fn render_report_pagination_count_matches_total_pages() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("pag.pdf");
        let mut data = empty_report(Locale::En);
        for i in 0..60 {
            data.lots
                .push(make_lot(&format!("S-{i}"), "x", "2025-06-01", 5));
        }
        data.metadata.row_count = data.lots.len();
        let result = render_report(&data, &out, Locale::En).expect("render");
        let expected = 60_usize.div_ceil(DATA_ROWS_PER_PAGE);
        assert_eq!(result.page_count, expected);
    }

    // --- Localized subtitle line --------------------------------------

    fn metadata_with_type(report_type: &str, description: &str, locale: Locale) -> ReportMetadata {
        let mut m = make_metadata(locale);
        m.report_type = report_type.to_string();
        m.description = description.to_string();
        m
    }

    #[test]
    fn report_subtitle_uses_localized_label_in_english() {
        let m = metadata_with_type(
            "in_alert_window",
            ReportType::InAlertWindow.description(Locale::En).as_ref(),
            Locale::En,
        );
        let line = report_subtitle(&m, Locale::En);
        assert!(line.starts_with("In alert window:"), "got: {line}");
        assert!(
            line.ends_with(": Lots in their alert window"),
            "description must be preserved after the colon, got: {line}"
        );
    }

    #[test]
    fn report_subtitle_uses_localized_label_in_spanish() {
        let m = metadata_with_type(
            "in_alert_window",
            ReportType::InAlertWindow.description(Locale::Es).as_ref(),
            Locale::Es,
        );
        let line = report_subtitle(&m, Locale::Es);
        assert!(line.starts_with("En ventana de alerta:"), "got: {line}");
        assert!(
            line.ends_with(": Lotes en ventana de alerta"),
            "description must be preserved after the colon, got: {line}"
        );
    }

    #[test]
    fn report_subtitle_does_not_leak_snake_case_for_any_type() {
        // Regression: every built-in report type used to render the raw
        // snake_case code (e.g. `In_alert_window:`) in the PDF header.
        for (raw_type, expected_en_label) in [
            ("in_alert_window", "In alert window"),
            ("expired", "Expired"),
            ("next_30_days", "Next 30 days"),
            ("custom", "Custom"),
        ] {
            let m = metadata_with_type(raw_type, "any description", Locale::En);
            let line = report_subtitle(&m, Locale::En);
            assert!(
                line.starts_with(&format!("{expected_en_label}:")),
                "expected localized label `{expected_en_label}` for `{raw_type}`, got `{line}`"
            );
            assert!(
                !line.contains('_'),
                "subtitle `{line}` must not contain underscores (would leak snake_case)"
            );
        }
    }

    #[test]
    fn report_subtitle_falls_back_to_capitalised_metadata_for_unknown_type() {
        // Forward-compat: an unknown report_type (e.g. a future enum variant
        // that hasn't been wired into ReportType::label yet) must not panic;
        // the renderer should fall back to a capitalised version of the
        // metadata string so the header is still informative.
        let m = metadata_with_type("future_type", "any description", Locale::En);
        let line = report_subtitle(&m, Locale::En);
        assert!(
            line.starts_with("Future_type:"),
            "fallback label should be capitalised metadata, got `{line}`"
        );
        // The caller-supplied description is preserved after the colon.
        assert!(line.ends_with(": any description"), "got: {line}");
    }

    #[test]
    fn report_subtitle_label_source_matches_report_type_label_for_all_builtins() {
        // Centralization guard: for every built-in variant and every
        // supported locale, the renderer's subtitle label must equal the
        // value returned by `ReportType::label`. This prevents the renderer
        // from re-introducing a duplicate hardcoded label map; adding a new
        // locale should only require editing the domain type.
        let builtins = [
            ("in_alert_window", ReportType::InAlertWindow),
            ("expired", ReportType::Expired),
            ("next_30_days", ReportType::Next30Days),
            ("custom", ReportType::Custom),
        ];
        for (raw_type, rt) in builtins {
            for locale in [Locale::En, Locale::Es] {
                let m = metadata_with_type(raw_type, "any description", locale);
                let line = report_subtitle(&m, locale);
                let expected_prefix = format!("{}:", rt.label(locale).as_ref());
                assert!(
                    line.starts_with(&expected_prefix),
                    "subtitle `{line}` should start with `{expected_prefix}` \
                     for {rt:?}/{locale:?} (single-source-of-truth invariant)"
                );
            }
        }
    }
}
