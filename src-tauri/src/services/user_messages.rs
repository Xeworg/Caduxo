//! Locale-aware user-visible message lookup.
//!
//! This module owns the English and Spanish strings for the small set of
//! user-visible backend errors that are surfaced verbatim by the UI over IPC.
//!
//! ## Design
//!
//! All strings are returned via [`user_message`], which takes a [`UserMessage`]
//! variant and a [`Locale`]. The function is the **single source of truth** for
//! the message text — services pass a typed variant, never a raw string, so
//! that the locale lookup happens exactly once at the call site.
//!
//! Unknown message kinds return `None` from [`parse_user_message_kind`] so the
//! caller can fall back to the original English string. Developer-only errors
//! (`DomainError::Internal`, tracing logs) are NEVER routed through this module.
//!
//! ## Scope
//!
//! The following strings are in scope; all other backend messages stay English:
//!
//! | Message | English | Spanish |
//! |---------|---------|---------|
//! | `InvalidDateFormat` | `Invalid {label} format: \`{value}\` (expected YYYY-MM-DD)` | `Formato de {label} no válido: \`{value}\` (se esperaba YYYY-MM-DD)` |
//! | `InvertedDateRange` | `date_from \`{from}\` must be on or before date_to \`{to}\`` | `date_from \`{from}\` debe ser igual o anterior a date_to \`{to}\`` |
//! | `QuantityNonNegative` | `Quantity must be non-negative, got {value}` | `La cantidad debe ser no negativa, se recibió {value}` |
//! | `QuantityPositive` | `Quantity must be positive, got {value}` | `La cantidad debe ser positiva, se recibió {value}` |
//! | `AlertDaysNegative` | `Default alert days before cannot be negative` | `Los días de alerta predeterminados no pueden ser negativos` |
//! | `AlertDaysExceedsMax` | `Default alert days exceeds maximum of {max}` | `Los días de alerta predeterminados exceden el máximo de {max}` |
//! | `SkuColumnNotDetected` | `SKU column not detected. Ensure the CSV has a "sku" or "SKU" header.` | `No se detectó la columna SKU. Asegúrate de que el CSV tenga una cabecera "sku" o "SKU".` |
//! | `DescriptionColumnNotDetected` | `Description column not detected. Ensure the CSV has a "description" or "Descripción" header.` | `No se detectó la columna Descripción. Asegúrate de que el CSV tenga una cabecera "description" o "Descripción".` |
//! | `LocationRequired` | `Please select a location` | `Selecciona una ubicación` |
//! | `SkuEmpty` | `SKU cannot be empty` | `El SKU no puede estar vacío` |
//! | `SkuTooLong { max }` | `SKU exceeds maximum length of {max} characters` | `El SKU excede la longitud máxima de {max} caracteres` |
//! | `DescriptionEmpty` | `Description cannot be empty` | `La descripción no puede estar vacía` |
//! | `DescriptionTooLong { max }` | `Description exceeds maximum length of {max} characters` | `La descripción excede la longitud máxima de {max} caracteres` |
//! | `BarcodeEmpty` | `Barcode cannot be empty` | `El código de barras no puede estar vacío` |
//! | `BarcodeTooLong { max }` | `Barcode exceeds maximum length of {max} characters` | `El código de barras excede la longitud máxima de {max} caracteres` |
//! | `BarcodeInvalidChars` | `Barcode contains invalid characters` | `El código de barras contiene caracteres no válidos` |
//! | `NameEmpty` | `Name cannot be empty` | `El nombre no puede estar vacío` |
//! | `NameTooLong { max }` | `Name exceeds maximum length of {max} characters` | `El nombre excede la longitud máxima de {max} caracteres` |
//! | `ScanValueEmpty` | `Scan value cannot be empty` | `El valor escaneado no puede estar vacío` |
//! | `LanguageNotAllowed { value }` | `language must be one of {en, es}, got \`{value}\`` | `el idioma debe ser uno de {en, es}, se recibió \`{value}\`` |

use crate::error::AppError;
use crate::pdf::locale::Locale;

/// A typed description of a user-visible message.
#[derive(Debug, Clone)]
pub enum UserMessage {
    InvalidDateFormat {
        label: String,
        value: String,
    },
    InvertedDateRange {
        from: String,
        to: String,
    },
    QuantityNonNegative {
        value: f64,
    },
    QuantityPositive {
        value: f64,
    },
    AlertDaysNegative,
    AlertDaysExceedsMax {
        max: i32,
    },
    SkuColumnNotDetected,
    DescriptionColumnNotDetected,
    /// Emitted when the create-lot flow requires a location but the input
    /// has none. The English canonical used to live as a hardcoded Spanish
    /// string in `services::expiry_lots::create_expiry_lot`; aligning it
    /// lets `localize_validation` translate it at the IPC boundary while
    /// preserving a stable persisted database sentinel (`Sin ubicacion`)
    /// elsewhere.
    LocationRequired,
    /// Product catalog validation: SKU is blank.
    SkuEmpty,
    /// Product catalog validation: SKU exceeds the maximum allowed length.
    SkuTooLong {
        max: usize,
    },
    /// Product catalog validation: product description is blank.
    DescriptionEmpty,
    /// Product catalog validation: product description exceeds the maximum
    /// allowed length.
    DescriptionTooLong {
        max: usize,
    },
    /// Product catalog validation: barcode is blank.
    BarcodeEmpty,
    /// Product catalog validation: barcode exceeds the maximum allowed length.
    BarcodeTooLong {
        max: usize,
    },
    /// Product catalog validation: barcode contains control characters.
    BarcodeInvalidChars,
    /// Store / location / category validation: name is blank.
    NameEmpty,
    /// Store / location / category validation: name exceeds the maximum
    /// allowed length.
    NameTooLong {
        max: usize,
    },
    /// Scanner workflow: scanned value is blank.
    ScanValueEmpty,
    /// Settings validation: language is not one of the allowed values
    /// (`en`, `es`).
    LanguageNotAllowed {
        value: String,
    },
}

/// Returns the user-visible message string for the given kind in the given locale.
/// Developer-only errors are NOT routed through this function.
pub fn user_message(kind: UserMessage, locale: Locale) -> String {
    use Locale as L;
    match (kind, locale) {
        (UserMessage::InvalidDateFormat { label, value }, L::En) => {
            format!("Invalid {label} format: `{value}` (expected YYYY-MM-DD)")
        }
        (UserMessage::InvalidDateFormat { label, value }, L::Es) => {
            format!("Formato de {label} no válido: `{value}` (se esperaba YYYY-MM-DD)")
        }
        (UserMessage::InvertedDateRange { from, to }, L::En) => {
            format!("date_from `{from}` must be on or before date_to `{to}`")
        }
        (UserMessage::InvertedDateRange { from, to }, L::Es) => {
            format!("date_from `{from}` debe ser igual o anterior a date_to `{to}`")
        }
        (UserMessage::QuantityNonNegative { value }, L::En) => {
            format!("Quantity must be non-negative, got {value}")
        }
        (UserMessage::QuantityNonNegative { value }, L::Es) => {
            format!("La cantidad debe ser no negativa, se recibió {value}")
        }
        (UserMessage::QuantityPositive { value }, L::En) => {
            format!("Quantity must be positive, got {value}")
        }
        (UserMessage::QuantityPositive { value }, L::Es) => {
            format!("La cantidad debe ser positiva, se recibió {value}")
        }
        (UserMessage::AlertDaysNegative, L::En) => {
            "Default alert days before cannot be negative".to_string()
        }
        (UserMessage::AlertDaysNegative, L::Es) => {
            "Los días de alerta predeterminados no pueden ser negativos".to_string()
        }
        (UserMessage::AlertDaysExceedsMax { max }, L::En) => {
            format!("Default alert days exceeds maximum of {max}")
        }
        (UserMessage::AlertDaysExceedsMax { max }, L::Es) => {
            format!("Los días de alerta predeterminados exceden el máximo de {max}")
        }
        (UserMessage::SkuColumnNotDetected, L::En) => {
            "SKU column not detected. Ensure the CSV has a \"sku\" or \"SKU\" header.".to_string()
        }
        (UserMessage::SkuColumnNotDetected, L::Es) => {
            "No se detectó la columna SKU. Asegúrate de que el CSV tenga una cabecera \"sku\" o \"SKU\".".to_string()
        }
        (UserMessage::DescriptionColumnNotDetected, L::En) => {
            "Description column not detected. Ensure the CSV has a \"description\" or \"Descripción\" header.".to_string()
        }
        (UserMessage::DescriptionColumnNotDetected, L::Es) => {
            "No se detectó la columna Descripción. Asegúrate de que el CSV tenga una cabecera \"description\" o \"Descripción\".".to_string()
        }
        (UserMessage::LocationRequired, L::En) => "Please select a location".to_string(),
        (UserMessage::LocationRequired, L::Es) => "Selecciona una ubicación".to_string(),
        (UserMessage::SkuEmpty, L::En) => "SKU cannot be empty".to_string(),
        (UserMessage::SkuEmpty, L::Es) => "El SKU no puede estar vacío".to_string(),
        (UserMessage::SkuTooLong { max }, L::En) => {
            format!("SKU exceeds maximum length of {max} characters")
        }
        (UserMessage::SkuTooLong { max }, L::Es) => {
            format!("El SKU excede la longitud máxima de {max} caracteres")
        }
        (UserMessage::DescriptionEmpty, L::En) => "Description cannot be empty".to_string(),
        (UserMessage::DescriptionEmpty, L::Es) => {
            "La descripción no puede estar vacía".to_string()
        }
        (UserMessage::DescriptionTooLong { max }, L::En) => {
            format!("Description exceeds maximum length of {max} characters")
        }
        (UserMessage::DescriptionTooLong { max }, L::Es) => {
            format!("La descripción excede la longitud máxima de {max} caracteres")
        }
        (UserMessage::BarcodeEmpty, L::En) => "Barcode cannot be empty".to_string(),
        (UserMessage::BarcodeEmpty, L::Es) => {
            "El código de barras no puede estar vacío".to_string()
        }
        (UserMessage::BarcodeTooLong { max }, L::En) => {
            format!("Barcode exceeds maximum length of {max} characters")
        }
        (UserMessage::BarcodeTooLong { max }, L::Es) => {
            format!("El código de barras excede la longitud máxima de {max} caracteres")
        }
        (UserMessage::BarcodeInvalidChars, L::En) => {
            "Barcode contains invalid characters".to_string()
        }
        (UserMessage::BarcodeInvalidChars, L::Es) => {
            "El código de barras contiene caracteres no válidos".to_string()
        }
        (UserMessage::NameEmpty, L::En) => "Name cannot be empty".to_string(),
        (UserMessage::NameEmpty, L::Es) => "El nombre no puede estar vacío".to_string(),
        (UserMessage::NameTooLong { max }, L::En) => {
            format!("Name exceeds maximum length of {max} characters")
        }
        (UserMessage::NameTooLong { max }, L::Es) => {
            format!("El nombre excede la longitud máxima de {max} caracteres")
        }
        (UserMessage::ScanValueEmpty, L::En) => "Scan value cannot be empty".to_string(),
        (UserMessage::ScanValueEmpty, L::Es) => {
            "El valor escaneado no puede estar vacío".to_string()
        }
        (UserMessage::LanguageNotAllowed { value }, L::En) => {
            format!("language must be one of {{en, es}}, got `{value}`")
        }
        (UserMessage::LanguageNotAllowed { value }, L::Es) => {
            format!("el idioma debe ser uno de {{en, es}}, se recibió `{value}`")
        }
    }
}

/// Tries to parse a known English message string back into a [`UserMessage`] variant.
/// Returns `None` for messages we don't own, allowing the caller to fall back to
/// the original string.
pub fn parse_user_message_kind(message: &str) -> Option<UserMessage> {
    // We implement a simple prefix-based parser for the messages we own.
    // This is the inverse of `user_message` for known English variants.

    if message.starts_with("Invalid ")
        && message.contains(" format: `")
        && message.contains("` (expected YYYY-MM-DD)")
    {
        // Format: "Invalid {label} format: `{value}` (expected YYYY-MM-DD)"
        if let Some(rest) = message.strip_prefix("Invalid ") {
            if let Some(format_pos) = rest.find(" format: `") {
                let label = rest[..format_pos].to_string();
                if let Some(value_start) = rest.find("` (expected YYYY-MM-DD)") {
                    let value = rest[format_pos + 10..value_start].to_string();
                    return Some(UserMessage::InvalidDateFormat { label, value });
                }
            }
        }
    }

    if message.starts_with("date_from `") && message.contains("` must be on or before date_to `") {
        if let Some(from_end) = message.find("` must be on or before date_to `") {
            let from = message[10..from_end].to_string();
            let to = message[from_end + 31..message.len() - 1].to_string();
            return Some(UserMessage::InvertedDateRange { from, to });
        }
    }

    if message == "Quantity must be non-negative, got "
        || message.starts_with("Quantity must be non-negative, got ")
    {
        if let Some(val_str) = message.strip_prefix("Quantity must be non-negative, got ") {
            if let Ok(v) = val_str.parse::<f64>() {
                return Some(UserMessage::QuantityNonNegative { value: v });
            }
        }
    }

    if message.starts_with("Quantity must be positive, got ") {
        if let Some(val_str) = message.strip_prefix("Quantity must be positive, got ") {
            if let Ok(v) = val_str.parse::<f64>() {
                return Some(UserMessage::QuantityPositive { value: v });
            }
        }
    }

    if message == "Default alert days before cannot be negative" {
        return Some(UserMessage::AlertDaysNegative);
    }

    if message.starts_with("Default alert days exceeds maximum of ") {
        if let Some(max_str) = message.strip_prefix("Default alert days exceeds maximum of ") {
            if let Ok(max) = max_str.parse::<i32>() {
                return Some(UserMessage::AlertDaysExceedsMax { max });
            }
        }
    }

    if message.starts_with("SKU column not detected") {
        return Some(UserMessage::SkuColumnNotDetected);
    }

    if message.starts_with("Description column not detected") {
        return Some(UserMessage::DescriptionColumnNotDetected);
    }

    if message == "Please select a location" {
        return Some(UserMessage::LocationRequired);
    }

    if message == "SKU cannot be empty" {
        return Some(UserMessage::SkuEmpty);
    }

    if let Some(max) = parse_too_long_suffix(message, "SKU exceeds maximum length of ") {
        return Some(UserMessage::SkuTooLong { max });
    }

    if message == "Description cannot be empty" {
        return Some(UserMessage::DescriptionEmpty);
    }

    if let Some(max) = parse_too_long_suffix(message, "Description exceeds maximum length of ") {
        return Some(UserMessage::DescriptionTooLong { max });
    }

    if message == "Barcode cannot be empty" {
        return Some(UserMessage::BarcodeEmpty);
    }

    if let Some(max) = parse_too_long_suffix(message, "Barcode exceeds maximum length of ") {
        return Some(UserMessage::BarcodeTooLong { max });
    }

    if message == "Barcode contains invalid characters" {
        return Some(UserMessage::BarcodeInvalidChars);
    }

    if message == "Name cannot be empty" {
        return Some(UserMessage::NameEmpty);
    }

    if let Some(max) = parse_too_long_suffix(message, "Name exceeds maximum length of ") {
        return Some(UserMessage::NameTooLong { max });
    }

    if message == "Scan value cannot be empty" {
        return Some(UserMessage::ScanValueEmpty);
    }

    if let Some(value) = parse_language_not_allowed(message) {
        return Some(UserMessage::LanguageNotAllowed { value });
    }

    None
}

/// Parses the numeric `max` out of a `{prefix}{N} characters` message.
/// Returns `None` if the message doesn't match the prefix or the inner
/// value doesn't parse as `usize`.
fn parse_too_long_suffix(message: &str, prefix: &str) -> Option<usize> {
    if !message.starts_with(prefix) || !message.ends_with(" characters") {
        return None;
    }
    let inner = message.strip_prefix(prefix)?.strip_suffix(" characters")?;
    inner.parse::<usize>().ok()
}

/// Parses `language must be one of {en, es}, got \`{value}\`` back into the
/// captured value. Returns `None` for malformed input.
fn parse_language_not_allowed(message: &str) -> Option<String> {
    const PREFIX: &str = "language must be one of {en, es}, got `";
    if !message.starts_with(PREFIX) || !message.ends_with('`') {
        return None;
    }
    let inner = message.strip_prefix(PREFIX)?.strip_suffix('`')?;
    Some(inner.to_string())
}

/// Wraps a validation error with locale-aware text if the message is one of the
/// known user-facing strings. Otherwise returns the original error unchanged.
/// Developer-only errors (`DomainError::Internal`) are returned as-is.
pub fn localize_validation(err: AppError, locale: Locale) -> AppError {
    let crate::error::AppError::Domain(crate::error::DomainError::Validation { message }) = err
    else {
        return err;
    };
    if let Some(kind) = parse_user_message_kind(&message) {
        return crate::error::AppError::Domain(crate::error::DomainError::Validation {
            message: user_message(kind, locale),
        });
    }
    crate::error::AppError::Domain(crate::error::DomainError::Validation { message })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn en(msg: UserMessage) -> String {
        user_message(msg, Locale::En)
    }

    fn es(msg: UserMessage) -> String {
        user_message(msg, Locale::Es)
    }

    #[test]
    fn invalid_date_format_en() {
        let got = en(UserMessage::InvalidDateFormat {
            label: "notification_date".into(),
            value: "not-a-date".into(),
        });
        assert!(got.starts_with("Invalid notification_date format: `not-a-date`"));
        assert!(got.contains("expected YYYY-MM-DD"));
    }

    #[test]
    fn invalid_date_format_es() {
        let got = es(UserMessage::InvalidDateFormat {
            label: "notification_date".into(),
            value: "x".into(),
        });
        assert!(got.starts_with("Formato de notification_date no válido"));
        assert!(got.contains("se esperaba YYYY-MM-DD"));
    }

    #[test]
    fn inverted_date_range_en() {
        let got = en(UserMessage::InvertedDateRange {
            from: "2025-12-31".into(),
            to: "2025-01-01".into(),
        });
        assert!(got.contains("date_from `2025-12-31`"));
        assert!(got.contains("must be on or before"));
        assert!(got.contains("date_to `2025-01-01`"));
    }

    #[test]
    fn inverted_date_range_es() {
        let got = es(UserMessage::InvertedDateRange {
            from: "2025-12-31".into(),
            to: "2025-01-01".into(),
        });
        assert!(got.contains("date_from `2025-12-31`"));
        assert!(got.contains("debe ser igual o anterior a"));
        assert!(got.contains("date_to `2025-01-01`"));
    }

    #[test]
    fn quantity_positive_en() {
        let got = en(UserMessage::QuantityPositive { value: -1.0 });
        assert_eq!(got, "Quantity must be positive, got -1");
    }

    #[test]
    fn quantity_positive_es() {
        let got = es(UserMessage::QuantityPositive { value: 0.5 });
        assert_eq!(got, "La cantidad debe ser positiva, se recibió 0.5");
    }

    #[test]
    fn alert_days_negative_en() {
        let got = en(UserMessage::AlertDaysNegative);
        assert_eq!(got, "Default alert days before cannot be negative");
    }

    #[test]
    fn alert_days_negative_es() {
        let got = es(UserMessage::AlertDaysNegative);
        assert_eq!(
            got,
            "Los días de alerta predeterminados no pueden ser negativos"
        );
    }

    #[test]
    fn alert_days_exceeds_max_en() {
        let got = en(UserMessage::AlertDaysExceedsMax { max: 365 });
        assert_eq!(got, "Default alert days exceeds maximum of 365");
    }

    #[test]
    fn alert_days_exceeds_max_es() {
        let got = es(UserMessage::AlertDaysExceedsMax { max: 365 });
        assert_eq!(
            got,
            "Los días de alerta predeterminados exceden el máximo de 365"
        );
    }

    #[test]
    fn sku_column_not_detected_en() {
        let got = en(UserMessage::SkuColumnNotDetected);
        assert!(got.contains("SKU column not detected"));
    }

    #[test]
    fn sku_column_not_detected_es() {
        let got = es(UserMessage::SkuColumnNotDetected);
        assert!(got.contains("No se detectó la columna SKU"));
    }

    #[test]
    fn location_required_en() {
        let got = en(UserMessage::LocationRequired);
        assert_eq!(got, "Please select a location");
    }

    #[test]
    fn location_required_es() {
        let got = es(UserMessage::LocationRequired);
        assert_eq!(got, "Selecciona una ubicación");
    }

    #[test]
    fn parse_location_required_roundtrips() {
        let en_msg = en(UserMessage::LocationRequired);
        let parsed = parse_user_message_kind(&en_msg);
        assert!(matches!(parsed, Some(UserMessage::LocationRequired)));
        assert_eq!(en_msg, en(parsed.unwrap()));
    }

    #[test]
    fn localize_validation_translates_location_required() {
        use crate::error::{AppError, DomainError};
        let err = AppError::Domain(DomainError::Validation {
            message: "Please select a location".into(),
        });
        let localized = localize_validation(err, Locale::Es);
        let AppError::Domain(DomainError::Validation { message }) = localized else {
            panic!("expected Validation");
        };
        assert_eq!(message, "Selecciona una ubicación");
    }

    #[test]
    fn parse_user_message_kind_roundtrips() {
        let en_msg = en(UserMessage::InvalidDateFormat {
            label: "date_to".into(),
            value: "abc".into(),
        });
        let parsed = parse_user_message_kind(&en_msg);
        assert!(parsed.is_some());
        assert_eq!(en_msg, en(parsed.unwrap()));
    }

    #[test]
    fn parse_user_message_kind_unknown_returns_none() {
        let got = parse_user_message_kind("some developer-only error message");
        assert!(got.is_none());
    }

    #[test]
    fn localize_validation_translates_known_message() {
        use crate::error::{AppError, DomainError};
        let err = AppError::Domain(DomainError::Validation {
            message: "Invalid notification_date format: `bad` (expected YYYY-MM-DD)".into(),
        });
        let localized = localize_validation(err, Locale::Es);
        let AppError::Domain(DomainError::Validation { message }) = localized else {
            panic!("expected Validation");
        };
        assert!(message.starts_with("Formato de"));
    }

    #[test]
    fn localize_validation_preserves_unknown_message() {
        use crate::error::{AppError, DomainError};
        let err = AppError::Domain(DomainError::Validation {
            message: "some unknown message".into(),
        });
        let localized = localize_validation(err, Locale::Es);
        let AppError::Domain(DomainError::Validation { message }) = localized else {
            panic!("expected Validation");
        };
        assert_eq!(message, "some unknown message");
    }

    #[test]
    fn localize_validation_ignores_non_validation_error() {
        // `DomainError::Internal` was removed from the error model; we now
        // exercise the pass-through branch with a non-Validation variant
        // (NotFound) to assert that `localize_validation` returns any
        // non-Validation domain error untouched instead of mapping it to a
        // Validation surface.
        use crate::error::{AppError, DomainError};
        let err = AppError::Domain(DomainError::NotFound {
            resource: "expiry_lot",
            id: "missing".into(),
        });
        let result = localize_validation(err, Locale::Es);
        match result {
            AppError::Domain(DomainError::NotFound { resource, id }) => {
                assert_eq!(resource, "expiry_lot");
                assert_eq!(id, "missing");
            }
            other => panic!("expected NotFound unchanged, got {other:?}"),
        }
    }

    // ─── Product catalog validation messages ────────────────────────────────

    #[test]
    fn sku_empty_en() {
        let got = en(UserMessage::SkuEmpty);
        assert_eq!(got, "SKU cannot be empty");
    }

    #[test]
    fn sku_empty_es() {
        let got = es(UserMessage::SkuEmpty);
        assert_eq!(got, "El SKU no puede estar vacío");
    }

    #[test]
    fn sku_too_long_en() {
        let got = en(UserMessage::SkuTooLong { max: 64 });
        assert_eq!(got, "SKU exceeds maximum length of 64 characters");
    }

    #[test]
    fn sku_too_long_es() {
        let got = es(UserMessage::SkuTooLong { max: 64 });
        assert_eq!(got, "El SKU excede la longitud máxima de 64 caracteres");
    }

    #[test]
    fn description_empty_en() {
        let got = en(UserMessage::DescriptionEmpty);
        assert_eq!(got, "Description cannot be empty");
    }

    #[test]
    fn description_empty_es() {
        let got = es(UserMessage::DescriptionEmpty);
        assert_eq!(got, "La descripción no puede estar vacía");
    }

    #[test]
    fn description_too_long_en() {
        let got = en(UserMessage::DescriptionTooLong { max: 512 });
        assert_eq!(got, "Description exceeds maximum length of 512 characters");
    }

    #[test]
    fn description_too_long_es() {
        let got = es(UserMessage::DescriptionTooLong { max: 512 });
        assert_eq!(
            got,
            "La descripción excede la longitud máxima de 512 caracteres"
        );
    }

    #[test]
    fn barcode_empty_en() {
        let got = en(UserMessage::BarcodeEmpty);
        assert_eq!(got, "Barcode cannot be empty");
    }

    #[test]
    fn barcode_empty_es() {
        let got = es(UserMessage::BarcodeEmpty);
        assert_eq!(got, "El código de barras no puede estar vacío");
    }

    #[test]
    fn barcode_too_long_en() {
        let got = en(UserMessage::BarcodeTooLong { max: 64 });
        assert_eq!(got, "Barcode exceeds maximum length of 64 characters");
    }

    #[test]
    fn barcode_too_long_es() {
        let got = es(UserMessage::BarcodeTooLong { max: 64 });
        assert_eq!(
            got,
            "El código de barras excede la longitud máxima de 64 caracteres"
        );
    }

    #[test]
    fn barcode_invalid_chars_en() {
        let got = en(UserMessage::BarcodeInvalidChars);
        assert_eq!(got, "Barcode contains invalid characters");
    }

    #[test]
    fn barcode_invalid_chars_es() {
        let got = es(UserMessage::BarcodeInvalidChars);
        assert_eq!(got, "El código de barras contiene caracteres no válidos");
    }

    #[test]
    fn name_empty_en() {
        let got = en(UserMessage::NameEmpty);
        assert_eq!(got, "Name cannot be empty");
    }

    #[test]
    fn name_empty_es() {
        let got = es(UserMessage::NameEmpty);
        assert_eq!(got, "El nombre no puede estar vacío");
    }

    #[test]
    fn name_too_long_en() {
        let got = en(UserMessage::NameTooLong { max: 128 });
        assert_eq!(got, "Name exceeds maximum length of 128 characters");
    }

    #[test]
    fn name_too_long_es() {
        let got = es(UserMessage::NameTooLong { max: 128 });
        assert_eq!(got, "El nombre excede la longitud máxima de 128 caracteres");
    }

    #[test]
    fn scan_value_empty_en() {
        let got = en(UserMessage::ScanValueEmpty);
        assert_eq!(got, "Scan value cannot be empty");
    }

    #[test]
    fn scan_value_empty_es() {
        let got = es(UserMessage::ScanValueEmpty);
        assert_eq!(got, "El valor escaneado no puede estar vacío");
    }

    #[test]
    fn language_not_allowed_en() {
        let got = en(UserMessage::LanguageNotAllowed { value: "fr".into() });
        assert_eq!(got, "language must be one of {en, es}, got `fr`");
    }

    #[test]
    fn language_not_allowed_es() {
        let got = es(UserMessage::LanguageNotAllowed { value: "fr".into() });
        assert_eq!(got, "el idioma debe ser uno de {en, es}, se recibió `fr`");
    }

    // ─── Parser roundtrips for product/store catalog ─────────────────────────

    #[test]
    fn parse_sku_empty_roundtrips() {
        let en_msg = en(UserMessage::SkuEmpty);
        let parsed = parse_user_message_kind(&en_msg);
        assert!(matches!(parsed, Some(UserMessage::SkuEmpty)));
    }

    #[test]
    fn parse_sku_too_long_roundtrips() {
        let en_msg = en(UserMessage::SkuTooLong { max: 64 });
        let parsed = parse_user_message_kind(&en_msg);
        match parsed {
            Some(UserMessage::SkuTooLong { max }) => assert_eq!(max, 64),
            other => panic!("expected SkuTooLong, got {other:?}"),
        }
    }

    #[test]
    fn parse_description_empty_roundtrips() {
        let en_msg = en(UserMessage::DescriptionEmpty);
        assert!(matches!(
            parse_user_message_kind(&en_msg),
            Some(UserMessage::DescriptionEmpty)
        ));
    }

    #[test]
    fn parse_description_too_long_roundtrips() {
        let en_msg = en(UserMessage::DescriptionTooLong { max: 512 });
        match parse_user_message_kind(&en_msg) {
            Some(UserMessage::DescriptionTooLong { max }) => assert_eq!(max, 512),
            other => panic!("expected DescriptionTooLong, got {other:?}"),
        }
    }

    #[test]
    fn parse_barcode_empty_roundtrips() {
        let en_msg = en(UserMessage::BarcodeEmpty);
        assert!(matches!(
            parse_user_message_kind(&en_msg),
            Some(UserMessage::BarcodeEmpty)
        ));
    }

    #[test]
    fn parse_barcode_too_long_roundtrips() {
        let en_msg = en(UserMessage::BarcodeTooLong { max: 64 });
        match parse_user_message_kind(&en_msg) {
            Some(UserMessage::BarcodeTooLong { max }) => assert_eq!(max, 64),
            other => panic!("expected BarcodeTooLong, got {other:?}"),
        }
    }

    #[test]
    fn parse_barcode_invalid_chars_roundtrips() {
        let en_msg = en(UserMessage::BarcodeInvalidChars);
        assert!(matches!(
            parse_user_message_kind(&en_msg),
            Some(UserMessage::BarcodeInvalidChars)
        ));
    }

    #[test]
    fn parse_name_empty_roundtrips() {
        let en_msg = en(UserMessage::NameEmpty);
        assert!(matches!(
            parse_user_message_kind(&en_msg),
            Some(UserMessage::NameEmpty)
        ));
    }

    #[test]
    fn parse_name_too_long_roundtrips() {
        let en_msg = en(UserMessage::NameTooLong { max: 128 });
        match parse_user_message_kind(&en_msg) {
            Some(UserMessage::NameTooLong { max }) => assert_eq!(max, 128),
            other => panic!("expected NameTooLong, got {other:?}"),
        }
    }

    #[test]
    fn parse_scan_value_empty_roundtrips() {
        let en_msg = en(UserMessage::ScanValueEmpty);
        assert!(matches!(
            parse_user_message_kind(&en_msg),
            Some(UserMessage::ScanValueEmpty)
        ));
    }

    #[test]
    fn parse_language_not_allowed_roundtrips() {
        let en_msg = en(UserMessage::LanguageNotAllowed { value: "fr".into() });
        match parse_user_message_kind(&en_msg) {
            Some(UserMessage::LanguageNotAllowed { value }) => assert_eq!(value, "fr"),
            other => panic!("expected LanguageNotAllowed, got {other:?}"),
        }
    }

    // ─── Parser: prefix collision guards ────────────────────────────────────

    #[test]
    fn parse_too_long_disambiguates_sku_vs_description() {
        // Same suffix ` characters` but different prefix → different variant.
        let sku_msg = en(UserMessage::SkuTooLong { max: 64 });
        let desc_msg = en(UserMessage::DescriptionTooLong { max: 512 });
        assert!(matches!(
            parse_user_message_kind(&sku_msg),
            Some(UserMessage::SkuTooLong { .. })
        ));
        assert!(matches!(
            parse_user_message_kind(&desc_msg),
            Some(UserMessage::DescriptionTooLong { .. })
        ));
    }

    #[test]
    fn parse_too_long_rejects_unknown_prefix() {
        // A truncated or unrelated string must not parse as any TooLong variant.
        let msg = "Something exceeds maximum length of 10 characters";
        let parsed = parse_user_message_kind(msg);
        assert!(
            !matches!(
                parsed,
                Some(
                    UserMessage::SkuTooLong { .. }
                        | UserMessage::DescriptionTooLong { .. }
                        | UserMessage::BarcodeTooLong { .. }
                        | UserMessage::NameTooLong { .. },
                )
            ),
            "unknown prefix must not match any TooLong variant: {parsed:?}"
        );
    }

    #[test]
    fn parse_too_long_rejects_non_numeric_max() {
        let msg = "SKU exceeds maximum length of many characters";
        assert!(parse_user_message_kind(msg).is_none());
    }

    #[test]
    fn parse_language_not_allowed_rejects_wrong_closing() {
        // The ` characters` suffix from TooLong must not be confused with the
        // backtick close on LanguageNotAllowed.
        let msg = "language must be one of {en, es}, got `fr characters";
        assert!(parse_user_message_kind(msg).is_none());
    }

    // ─── localize_validation integration for the new catalog ─────────────────

    #[test]
    fn localize_validation_translates_sku_too_long() {
        use crate::error::{AppError, DomainError};
        let err = AppError::Domain(DomainError::Validation {
            message: "SKU exceeds maximum length of 64 characters".into(),
        });
        let localized = localize_validation(err, Locale::Es);
        let AppError::Domain(DomainError::Validation { message }) = localized else {
            panic!("expected Validation");
        };
        assert_eq!(message, "El SKU excede la longitud máxima de 64 caracteres");
    }

    #[test]
    fn localize_validation_translates_language_not_allowed() {
        use crate::error::{AppError, DomainError};
        let err = AppError::Domain(DomainError::Validation {
            message: "language must be one of {en, es}, got `fr`".into(),
        });
        let localized = localize_validation(err, Locale::Es);
        let AppError::Domain(DomainError::Validation { message }) = localized else {
            panic!("expected Validation");
        };
        assert_eq!(
            message,
            "el idioma debe ser uno de {en, es}, se recibió `fr`"
        );
    }
}
