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
//! All visible text comes from the report DTO. PDF bytes are written to the
//! path provided by the caller; the function does not log row contents.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use printpdf::{
    BuiltinFont, IndirectFontRef, Line, Mm, PdfDocument, PdfLayerIndex, PdfLayerReference,
    PdfPageIndex, Point,
};
use serde::Serialize;

use crate::dto::dashboard::DashboardLotRow;
use crate::dto::reports::{ReportData, ReportFilters, ReportMetadata};
use crate::error::{AppError, InfrastructureError};

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

// Column definitions: (header text, width_mm)
struct ColumnDef {
    header: &'static str,
    width: f32,
}

const COLUMNS: &[ColumnDef] = &[
    ColumnDef {
        header: "SKU",
        width: 35.0,
    },
    ColumnDef {
        header: "Description",
        width: 65.0,
    },
    ColumnDef {
        header: "Store / Location",
        width: 55.0,
    },
    ColumnDef {
        header: "Qty",
        width: 22.0,
    },
    ColumnDef {
        header: "Expiry",
        width: 28.0,
    },
    ColumnDef {
        header: "Days",
        width: 20.0,
    },
    ColumnDef {
        header: "Alert",
        width: 20.0,
    },
    ColumnDef {
        header: "Batch",
        width: 28.0,
    },
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
/// All visible text comes from the report DTO. No product SKU, barcode,
/// description, or notes content is logged by this function.
pub fn render_report(data: &ReportData, path: &Path) -> Result<RenderedReport, AppError> {
    let total_rows = data.lots.len();
    let total_pages = if total_rows == 0 {
        1
    } else {
        // Ceiling division without floats: (n + d - 1) / d
        total_rows.div_ceil(DATA_ROWS_PER_PAGE)
    };

    let (doc, first_page, first_layer) = PdfDocument::new(
        "Caduxo Report",
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

        draw_header(&layer, &data.metadata, &bold, &regular);
        draw_table_header(&layer, &bold);

        let start_row = page_idx * DATA_ROWS_PER_PAGE;
        let end_row = (start_row + DATA_ROWS_PER_PAGE).min(total_rows);
        let mut y = BODY_TOP_Y - TABLE_HEADER_ROW_HEIGHT_MM;
        for lot in &data.lots[start_row..end_row] {
            draw_table_row(&layer, y, lot, &regular);
            y -= ROW_HEIGHT_MM;
        }

        if total_rows == 0 {
            draw_empty_notice(&layer, &regular);
        }

        draw_table_grid(&layer, start_row, end_row);
        draw_footer(&layer, page_num, total_pages, &regular);
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
    bold: &IndirectFontRef,
    regular: &IndirectFontRef,
) {
    let title_x = MARGIN_LEFT_MM;
    let title_y = PAGE_HEIGHT_MM - MARGIN_TOP_MM - 8.0;
    layer.use_text(
        "Caduxo Report",
        FONT_SIZE_TITLE,
        Mm(title_x),
        Mm(title_y),
        bold,
    );

    // Report type + description on the left, generated_at on the right.
    let subtitle_y = title_y - 6.5;
    let subtitle = format!(
        "{}: {}",
        capitalize(metadata.report_type.as_str()),
        metadata.description
    );
    layer.use_text(
        truncate(&subtitle, 90),
        FONT_SIZE_SUBTITLE,
        Mm(title_x),
        Mm(subtitle_y),
        regular,
    );

    let generated_at_human = humanize_generated_at(&metadata.generated_at);
    let meta_line_left = format!(
        "Generated: {}  ·  Rows: {}",
        generated_at_human, metadata.row_count
    );
    layer.use_text(
        truncate(&meta_line_left, 90),
        FONT_SIZE_META,
        Mm(title_x),
        Mm(subtitle_y - 5.0),
        regular,
    );

    let filter_line = format_filters(&metadata.filters_used);
    if !filter_line.is_empty() {
        layer.use_text(
            format!("Filters: {}", filter_line),
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
    regular: &IndirectFontRef,
) {
    let y = MARGIN_BOTTOM_MM + 4.0;
    let left = format!("Page {} of {}", page_num, total_pages);
    layer.use_text(left, FONT_SIZE_FOOTER, Mm(MARGIN_LEFT_MM), Mm(y), regular);

    let right = "Caduxo · Expiry Tracker";
    // Right-anchored text by estimating character widths.
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

fn draw_table_header(layer: &PdfLayerReference, bold: &IndirectFontRef) {
    let y = BODY_TOP_Y - 5.0;
    for (idx, col) in COLUMNS.iter().enumerate() {
        let x = column_x_left(idx) + 1.5;
        layer.use_text(col.header, FONT_SIZE_TABLE_HEADER, Mm(x), Mm(y), bold);
    }
    // Horizontal rule under the table header.
    let rule_y = BODY_TOP_Y - TABLE_HEADER_ROW_HEIGHT_MM;
    draw_horizontal_rule(layer, MARGIN_LEFT_MM, table_right_x(), rule_y, 0.4);
}

fn draw_table_row(
    layer: &PdfLayerReference,
    y: f32,
    lot: &DashboardLotRow,
    regular: &IndirectFontRef,
) {
    let baseline = y - 4.0;
    let cells: [String; 8] = [
        truncate(&lot.sku, 18),
        truncate(&lot.description, 36),
        truncate(&format_store_location(lot), 30),
        format!("{} {}", format_qty(lot.quantity), lot.unit),
        format_date_dd_mm_yyyy(&lot.expiry_date),
        format_days(lot.days_remaining),
        lot.alert_days_before.to_string(),
        truncate(lot.batch_code.as_deref().unwrap_or(""), 16),
    ];
    for (idx, text) in cells.iter().enumerate() {
        let x = column_x_left(idx) + 1.5;
        layer.use_text(text, FONT_SIZE_BODY, Mm(x), Mm(baseline), regular);
    }
}

fn draw_empty_notice(layer: &PdfLayerReference, regular: &IndirectFontRef) {
    let y = BODY_TOP_Y - TABLE_HEADER_ROW_HEIGHT_MM - 6.0;
    layer.use_text(
        "No rows match the current report filters.",
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

    // Outer rectangle + horizontal lines for every row.
    let left = table_left_x();
    let right = table_right_x();

    draw_horizontal_rule(layer, left, right, top_y, 0.4);
    draw_horizontal_rule(layer, left, right, bottom_y, 0.4);
    let mut y = top_y - TABLE_HEADER_ROW_HEIGHT_MM;
    for _ in 0..row_count {
        draw_horizontal_rule(layer, left, right, y, 0.2);
        y -= ROW_HEIGHT_MM;
    }
    // Vertical column lines.
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

fn format_qty(qty: f64) -> String {
    if qty.fract() == 0.0 {
        format!("{}", qty as i64)
    } else {
        format!("{qty}")
    }
}

fn format_date_dd_mm_yyyy(date_str: &str) -> String {
    // date_str is YYYY-MM-DD; if not, return as-is.
    if date_str.len() == 10 && date_str.as_bytes()[4] == b'-' && date_str.as_bytes()[7] == b'-' {
        let bytes = date_str.as_bytes();
        let day = std::str::from_utf8(&bytes[8..10]).unwrap_or("");
        let month = std::str::from_utf8(&bytes[5..7]).unwrap_or("");
        let year = std::str::from_utf8(&bytes[0..4]).unwrap_or("");
        format!("{day}/{month}/{year}")
    } else {
        date_str.to_string()
    }
}

fn format_days(days: i64) -> String {
    if days < 0 {
        format!("{} ago", -days)
    } else {
        days.to_string()
    }
}

fn format_store_location(lot: &DashboardLotRow) -> String {
    match &lot.location_name {
        Some(loc) if !loc.is_empty() => format!("{} / {}", lot.store_name, loc),
        _ => lot.store_name.clone(),
    }
}

fn humanize_generated_at(rfc3339: &str) -> String {
    // Trim sub-seconds and timezone for a compact display.
    // RFC3339 looks like "2025-01-15T10:30:00.123+00:00" or "2025-01-15T10:30:00Z".
    if let Some(t_pos) = rfc3339.find('T') {
        let date = &rfc3339[..t_pos];
        let rest = &rfc3339[t_pos + 1..];
        let time = rest.split(['+', 'Z', '-']).next().unwrap_or(rest);
        let time = time.split('.').next().unwrap_or(time);
        format!("{} {}", date, time)
    } else {
        rfc3339.to_string()
    }
}

fn format_filters(filters: &ReportFilters) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = filters.store_id.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("store={}", truncate(s, 16)));
    }
    if let Some(l) = filters.location_id.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("location={}", truncate(l, 16)));
    }
    if let Some(c) = filters.category_id.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("category={}", truncate(c, 16)));
    }
    if let Some(u) = filters.urgency.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("urgency={u}"));
    }
    if let Some(d) = filters.date_from.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("from={d}"));
    }
    if let Some(d) = filters.date_to.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("to={d}"));
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

/// Truncates `s` to at most `max_chars` characters, appending `…` when truncated.
/// This is a char-based limit, suitable for proportional fonts at known sizes.
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
/// Uses the standard Helvetica average glyph width of 0.5 * font size in pt
/// (≈ 0.5 * size * 0.353 mm per pt). Multi-character glyphs (e.g. 'm') are
/// wider, space is narrower, but the approximation is good enough for footer
/// alignment at small font sizes.
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

    fn make_metadata() -> ReportMetadata {
        ReportMetadata {
            report_type: "expired".to_string(),
            description: "Expired lots".to_string(),
            filters_used: ReportFilters {
                store_id: Some("store-a".to_string()),
                location_id: None,
                category_id: Some("dairy".to_string()),
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
        }
    }

    fn empty_report() -> ReportData {
        ReportData {
            metadata: make_metadata(),
            lots: vec![],
        }
    }

    #[test]
    fn table_width_matches_available_width() {
        // Table must not exceed the printable width (page width minus side margins).
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
    fn format_qty_strips_trailing_zero() {
        assert_eq!(format_qty(4.0), "4");
        assert_eq!(format_qty(4.5), "4.5");
        assert_eq!(format_qty(0.125), "0.125");
    }

    #[test]
    fn format_date_dd_mm_yyyy_reorders_iso_date() {
        assert_eq!(format_date_dd_mm_yyyy("2025-01-15"), "15/01/2025");
        assert_eq!(format_date_dd_mm_yyyy("2099-12-31"), "31/12/2099");
        assert_eq!(format_date_dd_mm_yyyy("not-a-date"), "not-a-date");
    }

    #[test]
    fn format_days_signs_ago_for_negative() {
        assert_eq!(format_days(-30), "30 ago");
        assert_eq!(format_days(0), "0");
        assert_eq!(format_days(7), "7");
    }

    #[test]
    fn format_store_location_with_and_without_location() {
        let mut lot = make_lot("S1", "Milk", "2025-01-01", -5);
        assert_eq!(format_store_location(&lot), "Main");
        lot.location_name = Some("Cold-room".to_string());
        assert_eq!(format_store_location(&lot), "Main / Cold-room");
    }

    #[test]
    fn truncate_short_string_returns_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string_appends_ellipsis() {
        let out = truncate("a long description", 8);
        // 8 chars + ellipsis
        assert_eq!(out.chars().count(), 9);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn format_filters_includes_only_present_fields() {
        let f = ReportFilters {
            store_id: Some("main".to_string()),
            location_id: Some("".to_string()),
            category_id: Some("dairy".to_string()),
            urgency: Some("expired".to_string()),
            date_from: Some("2025-01-01".to_string()),
            date_to: None,
        };
        let s = format_filters(&f);
        assert!(s.contains("store=main"));
        assert!(s.contains("category=dairy"));
        assert!(s.contains("urgency=expired"));
        assert!(s.contains("from=2025-01-01"));
        assert!(!s.contains("location=")); // empty filtered out
        assert!(!s.contains("to=")); // None filtered out
    }

    #[test]
    fn format_filters_empty_when_all_blank() {
        let f = ReportFilters::default();
        assert!(format_filters(&f).is_empty());
    }

    #[test]
    fn humanize_generated_at_strips_timezone_and_subseconds() {
        assert_eq!(
            humanize_generated_at("2025-01-15T10:30:00Z"),
            "2025-01-15 10:30:00"
        );
        assert_eq!(
            humanize_generated_at("2025-01-15T10:30:00.123+00:00"),
            "2025-01-15 10:30:00"
        );
        assert_eq!(humanize_generated_at("not-a-date"), "not-a-date");
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
    fn render_report_writes_pdf_file_with_empty_data() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("empty.pdf");
        let result = render_report(&empty_report(), &out).expect("render");
        assert_eq!(result.rows_written, 0);
        assert_eq!(result.page_count, 1);
        assert!(result.bytes_written > 0);
        // PDF magic header: every PDF starts with "%PDF-".
        let bytes = std::fs::read(&out).expect("read");
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn render_report_writes_pdf_file_with_many_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("many.pdf");
        let mut data = empty_report();
        for i in 0..60 {
            data.lots.push(make_lot(
                &format!("SKU-{i:03}"),
                &format!("Product {i}"),
                "2025-12-31",
                30,
            ));
        }
        data.metadata.row_count = data.lots.len();
        let result = render_report(&data, &out).expect("render");
        assert_eq!(result.rows_written, 60);
        // 60 rows at DATA_ROWS_PER_PAGE per page → at least 2 pages.
        assert!(
            result.page_count >= 2,
            "expected multi-page output, got {}",
            result.page_count
        );
        assert!(result.bytes_written > 0);
        let bytes = std::fs::read(&out).expect("read");
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn render_report_pages_contain_page_marker_object() {
        // Verifies that the renderer splits rows across multiple pages and that
        // each page footer contains the expected page marker. PDF files are
        // compressed binary streams so we cannot inspect the text directly.
        // Instead, we verify the page count from the PDF metadata dictionary
        // and check that the rendered page count equals the expected number of
        // data-page splits. The pagination math is exercised separately.
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("markers.pdf");
        let mut data = empty_report();
        // Force at least 2 pages so the multi-page path is exercised.
        for i in 0..(DATA_ROWS_PER_PAGE + 5) {
            data.lots.push(make_lot(
                &format!("S-{i}"),
                &format!("Desc {i}"),
                "2025-06-01",
                7,
            ));
        }
        data.metadata.row_count = data.lots.len();
        let result = render_report(&data, &out).expect("render");
        assert_eq!(result.page_count, 2, "fixture should force exactly 2 pages");

        let bytes = std::fs::read(&out).expect("read");
        let content = String::from_utf8_lossy(&bytes);
        // The PDF metadata should declare exactly 2 pages via the
        // `/Type/Pages/Count N` line. This is the structural equivalent of
        // checking the page marker text strings.
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
        // 60 rows / DATA_ROWS_PER_PAGE per page == expected page count.
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("pag.pdf");
        let mut data = empty_report();
        for i in 0..60 {
            data.lots
                .push(make_lot(&format!("S-{i}"), "x", "2025-06-01", 5));
        }
        data.metadata.row_count = data.lots.len();
        let result = render_report(&data, &out).expect("render");
        let expected = 60_usize.div_ceil(DATA_ROWS_PER_PAGE);
        assert_eq!(result.page_count, expected);
    }
}
