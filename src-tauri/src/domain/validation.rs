//! Pure validation rules for domain entities.

/// Validates that a SKU is non-empty and does not exceed a reasonable length.
pub fn validate_sku(sku: &str) -> Result<(), String> {
    if sku.trim().is_empty() {
        return Err("SKU cannot be empty".to_string());
    }
    if sku.len() > 64 {
        return Err("SKU exceeds maximum length of 64 characters".to_string());
    }
    Ok(())
}

/// Validates that a product description is non-empty.
pub fn validate_description(description: &str) -> Result<(), String> {
    if description.trim().is_empty() {
        return Err("Description cannot be empty".to_string());
    }
    if description.len() > 512 {
        return Err("Description exceeds maximum length of 512 characters".to_string());
    }
    Ok(())
}

/// Validates that a barcode string is reasonable.
pub fn validate_barcode(barcode: &str) -> Result<(), String> {
    let trimmed = barcode.trim();
    if trimmed.is_empty() {
        return Err("Barcode cannot be empty".to_string());
    }
    if trimmed.len() > 64 {
        return Err("Barcode exceeds maximum length of 64 characters".to_string());
    }
    // Reject whitespace-only or control characters.
    if trimmed.chars().any(|c| c.is_control()) {
        return Err("Barcode contains invalid characters".to_string());
    }
    Ok(())
}

/// Validates that a store or location name is non-empty.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if name.len() > 128 {
        return Err("Name exceeds maximum length of 128 characters".to_string());
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
