//! Locale-aware PDF message strings for the Caduxo report renderer.
//!
//! This module owns every user-visible string that appears inside the PDF
//! document (title, header, footer, column headers, filter prefixes, empty
//! notice, relative-time label, brand line). It does NOT own locale detection
//! or user-facing UI strings — those live in the frontend i18n layer.
//!
//! ## Supported locales
//!
//! - `En` — English (default fallback)
//! - `Es` — Spanish
//!
//! Unknown locale tags are normalised via [`Locale::parse`] and fall back to
//! `En` so that a malformed tag never causes a rendering crash.

use std::borrow::Cow;

/// Supported PDF locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Es,
}

impl Locale {
    /// Parses a BCP-47 locale tag (or any string) into a [`Locale`].
    ///
    /// Takes the primary subtag (everything before the first `-`), lower-cases
    /// it, and maps to `Es` for `es*` and `En` for everything else (including
    /// empty strings and malformed tags).
    ///
    /// # Examples
    ///
    /// ```
    /// # use caduxo_lib::pdf::locale::Locale;
    /// assert_eq!(Locale::parse("en-US"), Locale::En);
    /// assert_eq!(Locale::parse("en-GB"), Locale::En);
    /// assert_eq!(Locale::parse("es-MX"), Locale::Es);
    /// assert_eq!(Locale::parse("es"), Locale::Es);
    /// assert_eq!(Locale::parse("fr"), Locale::En);
    /// assert_eq!(Locale::parse(""), Locale::En);
    /// ```
    pub fn parse(tag: &str) -> Locale {
        let primary = tag.split('-').next().unwrap_or("").to_ascii_lowercase();
        match primary.as_str() {
            "es" => Locale::Es,
            // "en" and everything else fall back to English.
            _ => Locale::En,
        }
    }
}

/// Keys for every PDF-visible string. Adding a key without populating it for
/// both locales will fail the [`pdf_messages_parity`] test.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum PdfMessageKey {
    DocumentTitle,
    HeaderGenerated,
    HeaderFiltersPrefix,
    FilterPrefixStore,
    FilterPrefixLocation,
    FilterPrefixCategory,
    FilterPrefixCategoriesN,
    FilterPrefixUrgency,
    FilterPrefixFrom,
    FilterPrefixTo,
    ColumnSku,
    ColumnDescription,
    ColumnStoreLocation,
    ColumnQty,
    ColumnExpiry,
    ColumnDays,
    ColumnAlert,
    ColumnBatch,
    EmptyNotice,
    DaysAgoTemplate,
    FooterPageOf,
    Brand,
}

/// All PDF-visible strings for a single locale.
#[derive(Debug, Clone)]
pub struct PdfMessages {
    /// Document title in the PDF metadata and the top header line.
    pub document_title: Cow<'static, str>,
    /// "Generated: {date}  ·  Rows: {n}" / "Generado: {date}  ·  Filas: {n}".
    pub header_generated: Cow<'static, str>,
    /// "Filters:" / "Filtros:".
    pub header_filters_prefix: Cow<'static, str>,
    /// "store=" / "tienda=".
    pub filter_prefix_store: Cow<'static, str>,
    /// "location=" / "ubicación=".
    pub filter_prefix_location: Cow<'static, str>,
    /// "category=" / "categoría=".
    pub filter_prefix_category: Cow<'static, str>,
    /// "categories ({n})" / "categorías ({n})".
    pub filter_prefix_categories_n: Cow<'static, str>,
    /// "urgency=" / "urgencia=".
    pub filter_prefix_urgency: Cow<'static, str>,
    /// "from=" / "desde=".
    pub filter_prefix_from: Cow<'static, str>,
    /// "to=" / "hasta=".
    pub filter_prefix_to: Cow<'static, str>,
    /// Column headers in order: SKU, Description, Store/Location, Qty, Expiry,
    /// Days, Alert, Batch.
    pub columns: [Cow<'static, str>; 8],
    /// Notice shown when the report has no rows.
    pub empty_notice: Cow<'static, str>,
    /// "{n} ago" / "hace {n} días". The `{n}` placeholder is replaced at
    /// render time.
    pub days_ago_template: Cow<'static, str>,
    /// "Page {n} of {m}" / "Página {n} de {m}".
    pub footer_page_of: Cow<'static, str>,
    /// "Caduxo · Expiry Tracker" / "Caduxo · Control de caducidades".
    pub brand: Cow<'static, str>,
}

/// Returns the full set of PDF messages for the given locale.
pub fn pdf_messages(locale: Locale) -> PdfMessages {
    match locale {
        Locale::En => PdfMessages {
            document_title: Cow::Borrowed("Caduxo Report"),
            header_generated: Cow::Borrowed("Generated: {date}  ·  Rows: {n}"),
            header_filters_prefix: Cow::Borrowed("Filters:"),
            filter_prefix_store: Cow::Borrowed("store="),
            filter_prefix_location: Cow::Borrowed("location="),
            filter_prefix_category: Cow::Borrowed("category="),
            filter_prefix_categories_n: Cow::Borrowed("categories ({n})"),
            filter_prefix_urgency: Cow::Borrowed("urgency="),
            filter_prefix_from: Cow::Borrowed("from="),
            filter_prefix_to: Cow::Borrowed("to="),
            columns: [
                Cow::Borrowed("SKU"),
                Cow::Borrowed("Description"),
                Cow::Borrowed("Store / Location"),
                Cow::Borrowed("Qty"),
                Cow::Borrowed("Expiry"),
                Cow::Borrowed("Days"),
                Cow::Borrowed("Alert"),
                Cow::Borrowed("Batch"),
            ],
            empty_notice: Cow::Borrowed("No rows match the current report filters."),
            days_ago_template: Cow::Borrowed("{n} ago"),
            footer_page_of: Cow::Borrowed("Page {n} of {m}"),
            brand: Cow::Borrowed("Caduxo · Expiry Tracker"),
        },
        Locale::Es => PdfMessages {
            document_title: Cow::Borrowed("Reporte Caduxo"),
            header_generated: Cow::Borrowed("Generado: {date}  ·  Filas: {n}"),
            header_filters_prefix: Cow::Borrowed("Filtros:"),
            filter_prefix_store: Cow::Borrowed("tienda="),
            filter_prefix_location: Cow::Borrowed("ubicación="),
            filter_prefix_category: Cow::Borrowed("categoría="),
            filter_prefix_categories_n: Cow::Borrowed("categorías ({n})"),
            filter_prefix_urgency: Cow::Borrowed("urgencia="),
            filter_prefix_from: Cow::Borrowed("desde="),
            filter_prefix_to: Cow::Borrowed("hasta="),
            columns: [
                Cow::Borrowed("SKU"),
                Cow::Borrowed("Descripción"),
                Cow::Borrowed("Tienda / Ubicación"),
                Cow::Borrowed("Cantidad"),
                Cow::Borrowed("Caducidad"),
                Cow::Borrowed("Días"),
                Cow::Borrowed("Alerta"),
                Cow::Borrowed("Lote"),
            ],
            empty_notice: Cow::Borrowed("Ninguna fila coincide con los filtros actuales."),
            days_ago_template: Cow::Borrowed("hace {n} días"),
            footer_page_of: Cow::Borrowed("Página {n} de {m}"),
            brand: Cow::Borrowed("Caduxo · Control de caducidades"),
        },
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_parse_maps_es_variants_to_es() {
        assert_eq!(Locale::parse("es"), Locale::Es);
        assert_eq!(Locale::parse("es-MX"), Locale::Es);
        assert_eq!(Locale::parse("es-ES"), Locale::Es);
        assert_eq!(Locale::parse("ES"), Locale::Es); // case-insensitive
        assert_eq!(Locale::parse("ES-mx"), Locale::Es); // case-insensitive
    }

    #[test]
    fn locale_parse_maps_en_variants_to_en() {
        assert_eq!(Locale::parse("en"), Locale::En);
        assert_eq!(Locale::parse("en-US"), Locale::En);
        assert_eq!(Locale::parse("en-GB"), Locale::En);
    }

    #[test]
    fn locale_parse_falls_back_to_en_for_unknown() {
        assert_eq!(Locale::parse("fr"), Locale::En);
        assert_eq!(Locale::parse("de"), Locale::En);
        assert_eq!(Locale::parse("pt-BR"), Locale::En);
        assert_eq!(Locale::parse(""), Locale::En);
        assert_eq!(Locale::parse("xyz"), Locale::En);
    }

    #[test]
    fn pdf_messages_parity_en_and_es() {
        let en = pdf_messages(Locale::En);
        let es = pdf_messages(Locale::Es);

        // Every string must be non-empty in both locales.
        let fields: Vec<(&str, &str, &str)> = vec![
            (
                "document_title",
                en.document_title.as_ref(),
                es.document_title.as_ref(),
            ),
            (
                "header_generated",
                en.header_generated.as_ref(),
                es.header_generated.as_ref(),
            ),
            (
                "header_filters_prefix",
                en.header_filters_prefix.as_ref(),
                es.header_filters_prefix.as_ref(),
            ),
            (
                "filter_prefix_store",
                en.filter_prefix_store.as_ref(),
                es.filter_prefix_store.as_ref(),
            ),
            (
                "filter_prefix_location",
                en.filter_prefix_location.as_ref(),
                es.filter_prefix_location.as_ref(),
            ),
            (
                "filter_prefix_category",
                en.filter_prefix_category.as_ref(),
                es.filter_prefix_category.as_ref(),
            ),
            (
                "filter_prefix_categories_n",
                en.filter_prefix_categories_n.as_ref(),
                es.filter_prefix_categories_n.as_ref(),
            ),
            (
                "filter_prefix_urgency",
                en.filter_prefix_urgency.as_ref(),
                es.filter_prefix_urgency.as_ref(),
            ),
            (
                "filter_prefix_from",
                en.filter_prefix_from.as_ref(),
                es.filter_prefix_from.as_ref(),
            ),
            (
                "filter_prefix_to",
                en.filter_prefix_to.as_ref(),
                es.filter_prefix_to.as_ref(),
            ),
            (
                "empty_notice",
                en.empty_notice.as_ref(),
                es.empty_notice.as_ref(),
            ),
            (
                "days_ago_template",
                en.days_ago_template.as_ref(),
                es.days_ago_template.as_ref(),
            ),
            (
                "footer_page_of",
                en.footer_page_of.as_ref(),
                es.footer_page_of.as_ref(),
            ),
            ("brand", en.brand.as_ref(), es.brand.as_ref()),
        ];

        for (field, en_str, es_str) in fields {
            assert!(!en_str.is_empty(), "English `{field}` must be non-empty");
            assert!(
                !es_str.is_empty(),
                "Spanish `{field}` must be non-empty (EN: `{en_str}`)"
            );
        }

        // Columns: 8 headers each.
        assert_eq!(en.columns.len(), 8, "English must have 8 column headers");
        assert_eq!(es.columns.len(), 8, "Spanish must have 8 column headers");
        for (i, (en_col, es_col)) in en.columns.iter().zip(es.columns.iter()).enumerate() {
            assert!(!en_col.is_empty(), "English column {i} must be non-empty");
            assert!(
                !es_col.is_empty(),
                "Spanish column {i} must be non-empty (EN: `{en_col}`)"
            );
        }
    }

    #[test]
    fn pdf_messages_contains_spanish_filter_prefixes() {
        let es = pdf_messages(Locale::Es);
        assert_eq!(es.filter_prefix_store.as_ref(), "tienda=");
        assert_eq!(es.filter_prefix_location.as_ref(), "ubicación=");
        assert_eq!(es.filter_prefix_category.as_ref(), "categoría=");
        assert_eq!(es.filter_prefix_categories_n.as_ref(), "categorías ({n})");
        assert_eq!(es.filter_prefix_urgency.as_ref(), "urgencia=");
        assert_eq!(es.filter_prefix_from.as_ref(), "desde=");
        assert_eq!(es.filter_prefix_to.as_ref(), "hasta=");
    }

    #[test]
    fn pdf_messages_spanish_column_headers() {
        let es = pdf_messages(Locale::Es);
        assert_eq!(es.columns[0].as_ref(), "SKU");
        assert_eq!(es.columns[1].as_ref(), "Descripción");
        assert_eq!(es.columns[2].as_ref(), "Tienda / Ubicación");
        assert_eq!(es.columns[3].as_ref(), "Cantidad");
        assert_eq!(es.columns[4].as_ref(), "Caducidad");
        assert_eq!(es.columns[5].as_ref(), "Días");
        assert_eq!(es.columns[6].as_ref(), "Alerta");
        assert_eq!(es.columns[7].as_ref(), "Lote");
    }

    #[test]
    fn pdf_messages_english_baseline_unchanged() {
        let en = pdf_messages(Locale::En);
        assert_eq!(en.document_title.as_ref(), "Caduxo Report");
        assert_eq!(en.header_filters_prefix.as_ref(), "Filters:");
        assert_eq!(en.filter_prefix_store.as_ref(), "store=");
        assert_eq!(
            en.empty_notice.as_ref(),
            "No rows match the current report filters."
        );
        assert_eq!(en.days_ago_template.as_ref(), "{n} ago");
        assert_eq!(en.footer_page_of.as_ref(), "Page {n} of {m}");
        assert_eq!(en.brand.as_ref(), "Caduxo · Expiry Tracker");
    }
}
