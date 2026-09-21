//! Pure validation rules for domain entities.

use crate::pdf::locale::Locale;
use crate::services::user_messages::{user_message, UserMessage};

/// Maximum length for a SKU string.
const SKU_MAX_LEN: usize = 64;
/// Maximum length for a product description.
const DESCRIPTION_MAX_LEN: usize = 512;
/// Maximum length for a barcode string.
const BARCODE_MAX_LEN: usize = 64;
/// Maximum length for a store or location name.
const NAME_MAX_LEN: usize = 128;

/// Validates that a SKU is non-empty and does not exceed a reasonable length.
///
/// Emits canonical English via [`UserMessage`] so the command layer can
/// translate the error with [`crate::services::user_messages::localize_validation`].
pub fn validate_sku(sku: &str) -> Result<(), String> {
    if sku.trim().is_empty() {
        return Err(user_message(UserMessage::SkuEmpty, Locale::En));
    }
    if sku.len() > SKU_MAX_LEN {
        return Err(user_message(
            UserMessage::SkuTooLong { max: SKU_MAX_LEN },
            Locale::En,
        ));
    }
    Ok(())
}

/// Validates that a product description is non-empty and within the length cap.
///
/// Emits canonical English via [`UserMessage`] so the command layer can
/// translate the error with [`crate::services::user_messages::localize_validation`].
pub fn validate_description(description: &str) -> Result<(), String> {
    if description.trim().is_empty() {
        return Err(user_message(UserMessage::DescriptionEmpty, Locale::En));
    }
    if description.len() > DESCRIPTION_MAX_LEN {
        return Err(user_message(
            UserMessage::DescriptionTooLong {
                max: DESCRIPTION_MAX_LEN,
            },
            Locale::En,
        ));
    }
    Ok(())
}

/// Validates that a barcode string is reasonable.
///
/// Emits canonical English via [`UserMessage`] so the command layer can
/// translate the error with [`crate::services::user_messages::localize_validation`].
pub fn validate_barcode(barcode: &str) -> Result<(), String> {
    let trimmed = barcode.trim();
    if trimmed.is_empty() {
        return Err(user_message(UserMessage::BarcodeEmpty, Locale::En));
    }
    if trimmed.len() > BARCODE_MAX_LEN {
        return Err(user_message(
            UserMessage::BarcodeTooLong {
                max: BARCODE_MAX_LEN,
            },
            Locale::En,
        ));
    }
    // Reject whitespace-only or control characters.
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(user_message(UserMessage::BarcodeInvalidChars, Locale::En));
    }
    Ok(())
}

/// Validates that a store or location name is non-empty and within the
/// length cap.
///
/// Emits canonical English via [`UserMessage`] so the command layer can
/// translate the error with [`crate::services::user_messages::localize_validation`].
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err(user_message(UserMessage::NameEmpty, Locale::En));
    }
    if name.len() > NAME_MAX_LEN {
        return Err(user_message(
            UserMessage::NameTooLong { max: NAME_MAX_LEN },
            Locale::En,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::domain::validation::{
        validate_barcode, validate_description, validate_name, validate_sku,
    };

    #[test]
    fn t_validate_sku() {
        assert!(validate_sku("ABC-123").is_ok());
        assert!(validate_sku("").is_err());
        assert!(validate_sku("   ").is_err());
        assert!(validate_sku(&"x".repeat(65)).is_err());
    }

    #[test]
    fn t_validate_description() {
        assert!(validate_description("Milk 1L").is_ok());
        assert!(validate_description("").is_err());
        assert!(validate_description("   ").is_err());
        assert!(validate_description(&"x".repeat(513)).is_err());
    }

    #[test]
    fn t_validate_barcode() {
        assert!(validate_barcode("1234567890123").is_ok());
        assert!(validate_barcode("").is_err());
        assert!(validate_barcode("   ").is_err());
        assert!(validate_barcode(&"1".repeat(65)).is_err());
        // Control characters
        assert!(validate_barcode("123\x00456").is_err());
    }

    #[test]
    fn t_validate_name() {
        assert!(validate_name("Main Store").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name(&"x".repeat(129)).is_err());
    }
}
