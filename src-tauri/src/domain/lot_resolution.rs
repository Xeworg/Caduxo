//! Partial lot resolution logic.
//!
//! Pure functions — no I/O.

/// Computes the remaining quantity after a partial resolution.
///
/// Returns `None` if `resolved_qty > current_qty` (invalid).
pub fn compute_remaining_quantity(current_qty: f64, resolved_qty: f64) -> Option<f64> {
    if resolved_qty <= 0.0 || resolved_qty > current_qty {
        return None;
    }
    Some(current_qty - resolved_qty)
}

/// Validates that a quantity value is positive.
pub fn is_valid_quantity(qty: f64) -> bool {
    qty > 0.0
}

/// Returns `true` when remaining quantity is zero (lot fully resolved).
pub fn is_fully_resolved(remaining_qty: f64) -> bool {
    remaining_qty <= 0.0
}

#[cfg(test)]
mod tests {
    // Import the specific functions to avoid shadowing test function names.
    use crate::domain::lot_resolution::{
        compute_remaining_quantity, is_fully_resolved, is_valid_quantity,
    };

    #[test]
    fn compute_remaining_quantity_valid() {
        assert_eq!(compute_remaining_quantity(10.0, 3.0), Some(7.0));
        assert_eq!(compute_remaining_quantity(5.0, 5.0), Some(0.0));
    }

    #[test]
    fn compute_remaining_quantity_invalid() {
        assert!(compute_remaining_quantity(5.0, 6.0).is_none());
        assert!(compute_remaining_quantity(5.0, 0.0).is_none());
        assert!(compute_remaining_quantity(5.0, -1.0).is_none());
    }

    #[test]
    fn t_is_valid_quantity() {
        assert!(is_valid_quantity(1.0));
        assert!(is_valid_quantity(0.001));
        assert!(!is_valid_quantity(0.0));
        assert!(!is_valid_quantity(-1.0));
    }

    #[test]
    fn t_is_fully_resolved() {
        assert!(is_fully_resolved(0.0));
        assert!(!is_fully_resolved(0.5));
        assert!(!is_fully_resolved(5.0));
    }
}
