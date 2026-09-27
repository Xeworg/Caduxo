//! Stock-out reasons domain logic — pure functions with no I/O.
//!
//! All functions here are unit-testable in isolation. They handle:
//! - Movement kind validation (closed enum)
//! - Display name validation
//! - ID collision guard against seeded prefixes

use std::fmt;

use crate::error::DomainError;
use crate::services::user_messages::UserMessage;

/// The seven V20 stock-out movement kinds.
/// `exit:sale` is excluded by design scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StockOutMovementKind {
    Waste,
    Expired,
    Damaged,
    InternalConsumption,
    ReturnToSupplier,
    InventoryAdjustment,
    Other,
}

impl fmt::Display for StockOutMovementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StockOutMovementKind::Waste => write!(f, "exit:waste"),
            StockOutMovementKind::Expired => write!(f, "exit:expired"),
            StockOutMovementKind::Damaged => write!(f, "exit:damaged"),
            StockOutMovementKind::InternalConsumption => write!(f, "exit:internal_consumption"),
            StockOutMovementKind::ReturnToSupplier => write!(f, "exit:return_to_supplier"),
            StockOutMovementKind::InventoryAdjustment => write!(f, "exit:inventory_adjustment"),
            StockOutMovementKind::Other => write!(f, "exit:other"),
        }
    }
}

/// Parses a movement kind string into [`StockOutMovementKind`].
/// Returns `None` for unknown values or `exit:sale`.
pub fn validate_movement_kind(kind: &str) -> Option<StockOutMovementKind> {
    match kind.trim() {
        "exit:waste" => Some(StockOutMovementKind::Waste),
        "exit:expired" => Some(StockOutMovementKind::Expired),
        "exit:damaged" => Some(StockOutMovementKind::Damaged),
        "exit:internal_consumption" => Some(StockOutMovementKind::InternalConsumption),
        "exit:return_to_supplier" => Some(StockOutMovementKind::ReturnToSupplier),
        "exit:inventory_adjustment" => Some(StockOutMovementKind::InventoryAdjustment),
        "exit:other" => Some(StockOutMovementKind::Other),
        _ => None,
    }
}

/// Validates that a movement kind string is a valid stock-out kind.
/// Returns `Ok(())` if valid, or a [`DomainError::Validation`] otherwise.
pub fn validate_movement_kind_strict(kind: &str) -> Result<(), DomainError> {
    if validate_movement_kind(kind).is_some() {
        Ok(())
    } else {
        Err(DomainError::Validation {
            message: crate::services::user_messages::user_message(
                UserMessage::UnknownStockOutMovementKind {
                    kind: kind.to_string(),
                },
                crate::pdf::locale::Locale::En,
            ),
        })
    }
}

/// Validates a display name: non-empty when trimmed.
pub fn validate_display_name(name: &str) -> Result<(), DomainError> {
    if name.trim().is_empty() {
        return Err(DomainError::Validation {
            message: crate::services::user_messages::user_message(
                UserMessage::StockOutReasonDisplayNameEmpty,
                crate::pdf::locale::Locale::En,
            ),
        });
    }
    Ok(())
}

/// Validates a display name maximum length.
pub fn validate_display_name_max_len(name: &str, max: usize) -> Result<(), DomainError> {
    if name.len() > max {
        return Err(DomainError::Validation {
            message: crate::services::user_messages::user_message(
                UserMessage::StockOutReasonDisplayNameTooLong { max },
                crate::pdf::locale::Locale::En,
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Movement kind tests
    // ============================================================

    #[test]
    fn validate_movement_kind_all_seven() {
        let kinds = [
            ("exit:waste", Some(StockOutMovementKind::Waste)),
            ("exit:expired", Some(StockOutMovementKind::Expired)),
            ("exit:damaged", Some(StockOutMovementKind::Damaged)),
            (
                "exit:internal_consumption",
                Some(StockOutMovementKind::InternalConsumption),
            ),
            (
                "exit:return_to_supplier",
                Some(StockOutMovementKind::ReturnToSupplier),
            ),
            (
                "exit:inventory_adjustment",
                Some(StockOutMovementKind::InventoryAdjustment),
            ),
            ("exit:other", Some(StockOutMovementKind::Other)),
        ];

        for (input, expected) in kinds {
            assert_eq!(
                validate_movement_kind(input),
                expected,
                "failed for {input}"
            );
        }
    }

    #[test]
    fn validate_movement_kind_excludes_sale() {
        assert_eq!(validate_movement_kind("exit:sale"), None);
    }

    #[test]
    fn validate_movement_kind_whitespace() {
        assert_eq!(
            validate_movement_kind("  exit:waste  "),
            Some(StockOutMovementKind::Waste)
        );
    }

    #[test]
    fn validate_movement_kind_unknown() {
        assert_eq!(validate_movement_kind("unknown"), None);
        assert_eq!(validate_movement_kind(""), None);
        assert_eq!(validate_movement_kind("EXIT:WASTE"), None); // case sensitive
        assert_eq!(validate_movement_kind("entry:initial"), None);
    }

    #[test]
    fn validate_movement_kind_strict_accepts_valid() {
        assert!(validate_movement_kind_strict("exit:waste").is_ok());
    }

    #[test]
    fn validate_movement_kind_strict_rejects_invalid() {
        let result = validate_movement_kind_strict("exit:sale");
        assert!(result.is_err());
        if let Err(DomainError::Validation { message }) = result {
            assert!(message.contains("Unknown stock-out movement kind"));
        }
    }

    // ============================================================
    // Display name validation tests
    // ============================================================

    #[test]
    fn validate_display_name_accepts_valid() {
        assert!(validate_display_name("Descarte").is_ok());
        assert!(validate_display_name("  Descarte  ").is_ok());
        assert!(validate_display_name("Daño leve").is_ok());
    }

    #[test]
    fn validate_display_name_rejects_empty() {
        assert!(validate_display_name("").is_err());
        assert!(validate_display_name("   ").is_err());
    }

    #[test]
    fn validate_display_name_max_len_accepts_valid() {
        let name = "Descarte";
        assert!(validate_display_name_max_len(name, 50).is_ok());
        assert!(validate_display_name_max_len(name, name.len()).is_ok());
    }

    #[test]
    fn validate_display_name_max_len_rejects_too_long() {
        let name = "Descarte";
        let result = validate_display_name_max_len(name, 3);
        assert!(result.is_err());
    }

    // ============================================================
    // Display tests
    // ============================================================

    #[test]
    fn movement_kind_display() {
        assert_eq!(StockOutMovementKind::Waste.to_string(), "exit:waste");
        assert_eq!(StockOutMovementKind::Expired.to_string(), "exit:expired");
        assert_eq!(
            StockOutMovementKind::InternalConsumption.to_string(),
            "exit:internal_consumption"
        );
        assert_eq!(
            StockOutMovementKind::ReturnToSupplier.to_string(),
            "exit:return_to_supplier"
        );
        assert_eq!(
            StockOutMovementKind::InventoryAdjustment.to_string(),
            "exit:inventory_adjustment"
        );
        assert_eq!(StockOutMovementKind::Other.to_string(), "exit:other");
    }
}
