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

use crate::error::AppError;
use crate::pdf::locale::Locale;

/// A typed description of a user-visible message.
#[derive(Debug, Clone)]
pub enum UserMessage {
    InvalidDateFormat { label: String, value: String },
    InvertedDateRange { from: String, to: String },
    QuantityNonNegative { value: f64 },
    QuantityPositive { value: f64 },
    AlertDaysNegative,
    AlertDaysExceedsMax { max: i32 },
    SkuColumnNotDetected,
    DescriptionColumnNotDetected,
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
    }
}

/// Tries to parse a known English message string back into a [`UserMessage`] variant.
/// Returns `None` for messages we don't own, allowing the caller to fall back to
/// the original string.
pub fn parse_user_message_kind(message: &str) -> Option<UserMessage> {
    // We implement a simple prefix-based parser for the messages we own.
    // This is the inverse of `user_message` for known English variants.

    if message.starts_with("Invalid ") && message.contains(" format: `") && message.contains("` (expected YYYY-MM-DD)") {
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

    if message == "Quantity must be non-negative, got " || message.starts_with("Quantity must be non-negative, got ") {
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

    None
}

/// Wraps a validation error with locale-aware text if the message is one of the
/// known user-facing strings. Otherwise returns the original error unchanged.
/// Developer-only errors (`DomainError::Internal`) are returned as-is.
pub fn localize_validation(err: AppError, locale: Locale) -> AppError {
    let crate::error::AppError::Domain(crate::error::DomainError::Validation { message }) = err else {
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
        assert_eq!(got, "Los días de alerta predeterminados no pueden ser negativos");
    }

    #[test]
    fn alert_days_exceeds_max_en() {
        let got = en(UserMessage::AlertDaysExceedsMax { max: 365 });
        assert_eq!(got, "Default alert days exceeds maximum of 365");
    }

    #[test]
    fn alert_days_exceeds_max_es() {
        let got = es(UserMessage::AlertDaysExceedsMax { max: 365 });
        assert_eq!(got, "Los días de alerta predeterminados exceden el máximo de 365");
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
    fn localize_validation_ignores_internal_error() {
        use crate::error::{AppError, DomainError};
        let err = AppError::Domain(DomainError::Internal {
            message: "internal error".into(),
        });
        let result = localize_validation(err, Locale::Es);
        // Should return the original error unchanged (Internal, not Validation)
        matches!(result, AppError::Domain(DomainError::Internal { .. }));
    }
}
