//! Lot movement domain logic — pure functions with no I/O.
//!
//! All functions here are unit-testable in isolation. They handle:
//! - Movement kind validation and delta computation
//! - Unit-kind aware quantity validation (integer vs decimal units)
//! - Batch code derivation and collision resolution
//! - Legacy resolution mapping

use std::fmt;

use crate::dto::unit_definitions::UnitKind;

/// Movement kind vocabulary per the spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovementKind {
    /// Initial entry on lot creation (source=null, dest=set).
    EntryInitial,
    /// Transfer between locations (source=set, dest=set).
    Transfer,
    /// Sale exit.
    ExitSale,
    /// Waste / discard exit.
    ExitWaste,
    /// Expired exit.
    ExitExpired,
    /// Damaged exit.
    ExitDamaged,
    /// Internal consumption exit.
    ExitInternalConsumption,
    /// Return to supplier exit.
    ExitReturnToSupplier,
    /// Inventory adjustment exit (requires notes).
    ExitInventoryAdjustment,
    /// Other exit (requires notes).
    ExitOther,
    /// Unified count correction with directional sign.
    InventoryAdjustment,
}

impl fmt::Display for MovementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MovementKind::EntryInitial => write!(f, "entry:initial"),
            MovementKind::Transfer => write!(f, "transfer"),
            MovementKind::ExitSale => write!(f, "exit:sale"),
            MovementKind::ExitWaste => write!(f, "exit:waste"),
            MovementKind::ExitExpired => write!(f, "exit:expired"),
            MovementKind::ExitDamaged => write!(f, "exit:damaged"),
            MovementKind::ExitInternalConsumption => write!(f, "exit:internal_consumption"),
            MovementKind::ExitReturnToSupplier => write!(f, "exit:return_to_supplier"),
            MovementKind::ExitInventoryAdjustment => write!(f, "exit:inventory_adjustment"),
            MovementKind::ExitOther => write!(f, "exit:other"),
            MovementKind::InventoryAdjustment => write!(f, "inventory_adjustment"),
        }
    }
}

/// Direction for inventory_adjustment movements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Increase,
    Decrease,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::Increase => write!(f, "increase"),
            Direction::Decrease => write!(f, "decrease"),
        }
    }
}

// ============================================================
// Movement kind parsing
// ============================================================

/// Parses a movement kind string. Returns `None` for unknown values.
pub fn validate_movement_kind(kind: &str) -> Option<MovementKind> {
    match kind.trim() {
        "entry:initial" => Some(MovementKind::EntryInitial),
        "transfer" => Some(MovementKind::Transfer),
        "exit:sale" => Some(MovementKind::ExitSale),
        "exit:waste" => Some(MovementKind::ExitWaste),
        "exit:expired" => Some(MovementKind::ExitExpired),
        "exit:damaged" => Some(MovementKind::ExitDamaged),
        "exit:internal_consumption" => Some(MovementKind::ExitInternalConsumption),
        "exit:return_to_supplier" => Some(MovementKind::ExitReturnToSupplier),
        "exit:inventory_adjustment" => Some(MovementKind::ExitInventoryAdjustment),
        "exit:other" => Some(MovementKind::ExitOther),
        "inventory_adjustment" => Some(MovementKind::InventoryAdjustment),
        _ => None,
    }
}

// ============================================================
// Delta computation
// ============================================================

/// Computes the signed delta multiplier for a movement kind.
/// - `+1` means the movement increases the lot total.
/// - `-1` means the movement decreases the lot total.
/// - `0` means the movement preserves the lot total (e.g., transfer).
///
/// Note: For `inventory_adjustment`, the caller must multiply by the `direction`
/// sign: increase → +1, decrease → -1.
pub fn compute_movement_delta(kind: &MovementKind) -> i32 {
    match kind {
        MovementKind::EntryInitial => 1,
        MovementKind::Transfer => 0,
        MovementKind::InventoryAdjustment => 1, // caller must apply direction sign
        // All exit kinds decrease the lot total
        MovementKind::ExitSale
        | MovementKind::ExitWaste
        | MovementKind::ExitExpired
        | MovementKind::ExitDamaged
        | MovementKind::ExitInternalConsumption
        | MovementKind::ExitReturnToSupplier
        | MovementKind::ExitInventoryAdjustment
        | MovementKind::ExitOther => -1,
    }
}

/// Computes the signed delta for a movement with optional direction.
/// Returns the delta to apply to `expiry_lots.quantity`.
pub fn compute_signed_delta(
    kind: &MovementKind,
    direction: Option<&Direction>,
    quantity: f64,
) -> f64 {
    let sign = match kind {
        MovementKind::InventoryAdjustment => {
            match direction {
                Some(Direction::Increase) => 1.0,
                Some(Direction::Decrease) => -1.0,
                None => return 0.0, // no direction = invalid, caller should reject
            }
        }
        _ => {
            if direction.is_some() {
                // Non-inventory_adjustment kinds must not have a direction
                return 0.0;
            }
            compute_movement_delta(kind) as f64
        }
    };
    sign * quantity
}

// ============================================================
// Notes requirement
// ============================================================

/// Returns `true` when a notes value is required for the given movement kind.
pub fn notes_required(kind: &MovementKind) -> bool {
    matches!(
        kind,
        MovementKind::ExitOther
            | MovementKind::ExitInventoryAdjustment
            | MovementKind::InventoryAdjustment
    )
}

/// Validates that notes are present for kinds that require them.
/// Returns `Ok(())` if valid, or an error message if notes are required but missing.
pub fn validate_notes(kind: &MovementKind, notes: Option<&str>) -> Result<(), String> {
    let has_notes = notes.map(|n| !n.trim().is_empty()).unwrap_or(false);
    if notes_required(kind) && !has_notes {
        return Err(format!("Notes are required for movement kind `{}`", kind));
    }
    Ok(())
}

// ============================================================
// Batch code derivation
// ============================================================

/// Derives a 3–6 character alphanumeric prefix from a SKU.
/// - SKUs with ≥3 alphanumeric chars: take first 6, uppercase, alphanumeric only.
/// - SKUs with <3 alphanumeric chars: pad with 'X' to reach 3 chars.
/// - Empty or None SKU: return "LOT".
pub fn derive_batch_prefix(sku: Option<&str>) -> String {
    let sku = match sku {
        Some(s) => s,
        None => return "LOT".to_string(),
    };

    let alnum: String = sku
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .take(6)
        .collect();

    if alnum.len() >= 3 {
        return alnum;
    }

    // Pad with X to reach 3 chars
    let mut padded = alnum;
    while padded.len() < 3 {
        padded.push('X');
    }
    padded
}

/// Generates the next batch candidate given the current max NNN.
/// Handles collision suffix `-M` format when NNN > 999.
pub fn next_batch_candidate(prefix: &str, date: &str, last_nnn: u32) -> String {
    let nnn = last_nnn.saturating_add(1);
    if nnn <= 999 {
        format!("{}-{}-{:03}", prefix, date, nnn)
    } else {
        // Fall back to collision suffix format
        format!("{}-{}-{:03}-2", prefix, date, 999)
    }
}

// ============================================================
// Legacy resolution mapping
// ============================================================

// ============================================================
// Unit-kind aware quantity validation
// ============================================================

/// Validates a movement quantity against the product's unit kind.
///
/// - `UnitKind::Integer`: quantity must be a whole number (no fractional part).
/// - `UnitKind::Decimal`: any positive magnitude is allowed.
/// - `None` (legacy / uncatalogued product): treated as decimal — preserves
///   back-compat with products that have no catalog link.
///
/// `entry:initial` is allowed to use `quantity = 0` for every unit kind
/// (the lot's quantity already exists on the `expiry_lots` row, so the
/// initial movement is a history marker rather than a real delta).
///
/// Returns `Ok(())` when valid, or a Spanish-language error string explaining
/// the rejection when invalid.
pub fn validate_quantity_for_unit_kind(
    quantity: f64,
    kind: &MovementKind,
    unit_kind: Option<UnitKind>,
) -> Result<(), String> {
    // entry:initial keeps its special-case: qty == 0 is allowed even for
    // integer-unit products (history marker; lot quantity is already on file).
    if matches!(kind, MovementKind::EntryInitial) && quantity == 0.0 {
        return Ok(());
    }

    match unit_kind {
        Some(UnitKind::Integer) => {
            // Negative or zero quantity is rejected for non-entry movements
            // (the existing service-level rule).
            if quantity <= 0.0 {
                return Err(format!(
                    "La cantidad debe ser mayor a 0 (recibido {})",
                    quantity
                ));
            }
            // Fractional component must be zero — reject 1.5, 0.25, etc.
            if quantity.fract() != 0.0 {
                return Err(format!(
                    "La unidad del producto es de tipo entero; no se permiten cantidades fraccionarias ({})",
                    quantity
                ));
            }
            Ok(())
        }
        Some(UnitKind::Decimal) | None => {
            // Decimal units (and legacy/null units) accept any positive magnitude.
            // entry:initial still allows 0; other kinds reject non-positive.
            if quantity <= 0.0 && !matches!(kind, MovementKind::EntryInitial) {
                return Err(format!(
                    "La cantidad debe ser mayor a 0 (recibido {})",
                    quantity
                ));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Movement kind tests
    // ============================================================

    #[test]
    fn validate_movement_kind_valid() {
        assert_eq!(
            validate_movement_kind("entry:initial"),
            Some(MovementKind::EntryInitial)
        );
        assert_eq!(
            validate_movement_kind("transfer"),
            Some(MovementKind::Transfer)
        );
        assert_eq!(
            validate_movement_kind("exit:sale"),
            Some(MovementKind::ExitSale)
        );
        assert_eq!(
            validate_movement_kind("exit:waste"),
            Some(MovementKind::ExitWaste)
        );
        assert_eq!(
            validate_movement_kind("exit:expired"),
            Some(MovementKind::ExitExpired)
        );
        assert_eq!(
            validate_movement_kind("exit:damaged"),
            Some(MovementKind::ExitDamaged)
        );
        assert_eq!(
            validate_movement_kind("exit:internal_consumption"),
            Some(MovementKind::ExitInternalConsumption)
        );
        assert_eq!(
            validate_movement_kind("exit:return_to_supplier"),
            Some(MovementKind::ExitReturnToSupplier)
        );
        assert_eq!(
            validate_movement_kind("exit:inventory_adjustment"),
            Some(MovementKind::ExitInventoryAdjustment)
        );
        assert_eq!(
            validate_movement_kind("exit:other"),
            Some(MovementKind::ExitOther)
        );
        assert_eq!(
            validate_movement_kind("inventory_adjustment"),
            Some(MovementKind::InventoryAdjustment)
        );
    }

    #[test]
    fn validate_movement_kind_whitespace() {
        assert_eq!(
            validate_movement_kind("  entry:initial  "),
            Some(MovementKind::EntryInitial)
        );
    }

    #[test]
    fn validate_movement_kind_unknown() {
        assert_eq!(validate_movement_kind("unknown"), None);
        assert_eq!(validate_movement_kind(""), None);
        assert_eq!(validate_movement_kind("EXIT:SALE"), None); // case sensitive
    }

    // ============================================================
    // Delta computation tests
    // ============================================================

    #[test]
    fn compute_movement_delta_entry() {
        assert_eq!(compute_movement_delta(&MovementKind::EntryInitial), 1);
    }

    #[test]
    fn compute_movement_delta_transfer() {
        assert_eq!(compute_movement_delta(&MovementKind::Transfer), 0);
    }

    #[test]
    fn compute_movement_delta_inventory_adjustment() {
        // Base delta is +1; caller must apply direction sign
        assert_eq!(
            compute_movement_delta(&MovementKind::InventoryAdjustment),
            1
        );
    }

    #[test]
    fn compute_movement_delta_all_exits() {
        let exits = [
            MovementKind::ExitSale,
            MovementKind::ExitWaste,
            MovementKind::ExitExpired,
            MovementKind::ExitDamaged,
            MovementKind::ExitInternalConsumption,
            MovementKind::ExitReturnToSupplier,
            MovementKind::ExitInventoryAdjustment,
            MovementKind::ExitOther,
        ];
        for exit in exits {
            assert_eq!(
                compute_movement_delta(&exit),
                -1,
                "exit kind {:?} should have delta -1",
                exit
            );
        }
    }

    #[test]
    fn compute_signed_delta_increase() {
        let delta = compute_signed_delta(
            &MovementKind::InventoryAdjustment,
            Some(&Direction::Increase),
            5.0,
        );
        assert_eq!(delta, 5.0);
    }

    #[test]
    fn compute_signed_delta_decrease() {
        let delta = compute_signed_delta(
            &MovementKind::InventoryAdjustment,
            Some(&Direction::Decrease),
            3.0,
        );
        assert_eq!(delta, -3.0);
    }

    #[test]
    fn compute_signed_delta_inventory_adjustment_no_direction() {
        let delta = compute_signed_delta(&MovementKind::InventoryAdjustment, None, 5.0);
        assert_eq!(delta, 0.0);
    }

    #[test]
    fn compute_signed_delta_entry() {
        let delta = compute_signed_delta(&MovementKind::EntryInitial, None, 10.0);
        assert_eq!(delta, 10.0);
    }

    #[test]
    fn compute_signed_delta_exit_sale() {
        let delta = compute_signed_delta(&MovementKind::ExitSale, None, 2.0);
        assert_eq!(delta, -2.0);
    }

    #[test]
    fn compute_signed_delta_transfer() {
        let delta = compute_signed_delta(&MovementKind::Transfer, None, 5.0);
        assert_eq!(delta, 0.0);
    }

    // ============================================================
    // Notes requirement tests
    // ============================================================

    #[test]
    fn notes_required_for_exit_other() {
        assert!(notes_required(&MovementKind::ExitOther));
    }

    #[test]
    fn notes_required_for_exit_inventory_adjustment() {
        assert!(notes_required(&MovementKind::ExitInventoryAdjustment));
    }

    #[test]
    fn notes_required_for_inventory_adjustment() {
        assert!(notes_required(&MovementKind::InventoryAdjustment));
    }

    #[test]
    fn notes_not_required_for_exit_sale() {
        assert!(!notes_required(&MovementKind::ExitSale));
    }

    #[test]
    fn notes_not_required_for_transfer() {
        assert!(!notes_required(&MovementKind::Transfer));
    }

    #[test]
    fn notes_not_required_for_entry_initial() {
        assert!(!notes_required(&MovementKind::EntryInitial));
    }

    #[test]
    fn validate_notes_ok_when_present() {
        assert!(validate_notes(&MovementKind::ExitOther, Some("Reason: damaged")).is_ok());
    }

    #[test]
    fn validate_notes_ok_when_not_required() {
        assert!(validate_notes(&MovementKind::ExitSale, None).is_ok());
    }

    #[test]
    fn validate_notes_fails_when_required_but_blank() {
        assert!(validate_notes(&MovementKind::ExitOther, None).is_err());
        assert!(validate_notes(&MovementKind::ExitOther, Some("")).is_err());
        assert!(validate_notes(&MovementKind::ExitOther, Some("   ")).is_err());
    }

    // ============================================================
    // Batch prefix derivation tests
    // ============================================================

    #[test]
    fn derive_batch_prefix_standard_sku() {
        assert_eq!(derive_batch_prefix(Some("SKU-001")), "SKU001");
        assert_eq!(derive_batch_prefix(Some("ABC123")), "ABC123");
        assert_eq!(derive_batch_prefix(Some("YOG-001")), "YOG001");
    }

    #[test]
    fn derive_batch_prefix_short_sku() {
        // 2 chars: "AB" → "ABX" (padded to 3)
        assert_eq!(derive_batch_prefix(Some("AB")), "ABX");
        // 1 char: "A" → "AXX" (padded to 3)
        assert_eq!(derive_batch_prefix(Some("A")), "AXX");
    }

    #[test]
    fn derive_batch_prefix_none() {
        assert_eq!(derive_batch_prefix(None), "LOT");
    }

    #[test]
    fn derive_batch_prefix_mixed_case() {
        assert_eq!(derive_batch_prefix(Some("sku-001")), "SKU001");
        assert_eq!(derive_batch_prefix(Some("Yog-123")), "YOG123");
    }

    #[test]
    fn derive_batch_prefix_non_alphanumeric() {
        assert_eq!(derive_batch_prefix(Some("SKU-001")), "SKU001");
        assert_eq!(derive_batch_prefix(Some("A-B-C")), "ABC");
    }

    #[test]
    fn derive_batch_prefix_empty_sku() {
        // Empty string is treated as SKU (not None), so padded to 3 chars
        assert_eq!(derive_batch_prefix(Some("")), "XXX");
        // Spaces also treated as SKU
        assert_eq!(derive_batch_prefix(Some("   ")), "XXX");
    }

    #[test]
    fn derive_batch_prefix_long_sku() {
        // Takes first 6 alphanumeric chars
        assert_eq!(derive_batch_prefix(Some("LONG-SKU-12345")), "LONGSK");
    }

    // ============================================================
    // Next batch candidate tests
    // ============================================================

    #[test]
    fn next_batch_candidate_basic() {
        assert_eq!(
            next_batch_candidate("SKU001", "20261015", 0),
            "SKU001-20261015-001"
        );
        assert_eq!(
            next_batch_candidate("SKU001", "20261015", 1),
            "SKU001-20261015-002"
        );
    }

    #[test]
    fn next_batch_candidate_at_999() {
        // At 999, saturates and falls back to collision suffix
        assert_eq!(
            next_batch_candidate("SKU001", "20261015", 999),
            "SKU001-20261015-999-2"
        );
    }

    #[test]
    fn next_batch_candidate_collision_fallback() {
        // When last_nnn > 999, falls back to collision suffix
        assert_eq!(
            next_batch_candidate("SKU001", "20261015", 1000),
            "SKU001-20261015-999-2"
        );
    }

    // ============================================================
    // MovementKind display tests
    // ============================================================

    #[test]
    fn movement_kind_display() {
        assert_eq!(MovementKind::EntryInitial.to_string(), "entry:initial");
        assert_eq!(MovementKind::Transfer.to_string(), "transfer");
        assert_eq!(MovementKind::ExitSale.to_string(), "exit:sale");
        assert_eq!(
            MovementKind::InventoryAdjustment.to_string(),
            "inventory_adjustment"
        );
    }

    // ============================================================
    // Direction display tests
    // ============================================================

    #[test]
    fn direction_display() {
        assert_eq!(Direction::Increase.to_string(), "increase");
        assert_eq!(Direction::Decrease.to_string(), "decrease");
    }

    // ============================================================
    // Unit-kind aware quantity validation tests
    // ============================================================

    #[test]
    fn validate_qty_integer_accepts_whole_numbers() {
        // Integer unit kinds accept positive whole quantities (1, 2, 100).
        assert!(validate_quantity_for_unit_kind(
            1.0,
            &MovementKind::ExitSale,
            Some(UnitKind::Integer)
        )
        .is_ok());
        assert!(validate_quantity_for_unit_kind(
            5.0,
            &MovementKind::InventoryAdjustment,
            Some(UnitKind::Integer)
        )
        .is_ok());
        assert!(validate_quantity_for_unit_kind(
            100.0,
            &MovementKind::Transfer,
            Some(UnitKind::Integer)
        )
        .is_ok());
    }

    #[test]
    fn validate_qty_integer_rejects_fractional() {
        // Fractional quantities are rejected for integer-unit products.
        let err =
            validate_quantity_for_unit_kind(1.5, &MovementKind::ExitSale, Some(UnitKind::Integer));
        assert!(err.is_err());
        let msg = err.unwrap_err();
        assert!(
            msg.contains("entero") && msg.contains("fraccionarias"),
            "error message should mention integer-unit and fractional rejection, got: {msg}"
        );
    }

    #[test]
    fn validate_qty_integer_rejects_small_fractional() {
        // 0.25 — small fractional quantity also rejected.
        assert!(validate_quantity_for_unit_kind(
            0.25,
            &MovementKind::InventoryAdjustment,
            Some(UnitKind::Integer)
        )
        .is_err());
        // 0.99 — almost-whole but still fractional.
        assert!(validate_quantity_for_unit_kind(
            0.99,
            &MovementKind::ExitSale,
            Some(UnitKind::Integer)
        )
        .is_err());
    }

    #[test]
    fn validate_qty_integer_rejects_zero_and_negative_for_non_entry() {
        // Zero is rejected for non-entry movement kinds.
        assert!(validate_quantity_for_unit_kind(
            0.0,
            &MovementKind::ExitSale,
            Some(UnitKind::Integer)
        )
        .is_err());
        // Negative quantities are rejected too.
        assert!(validate_quantity_for_unit_kind(
            -1.0,
            &MovementKind::ExitSale,
            Some(UnitKind::Integer)
        )
        .is_err());
    }

    #[test]
    fn validate_qty_integer_allows_zero_for_entry_initial() {
        // entry:initial is a history marker; qty == 0 is allowed even
        // when the product uses integer units (the lot's quantity is
        // already on the expiry_lots row).
        assert!(validate_quantity_for_unit_kind(
            0.0,
            &MovementKind::EntryInitial,
            Some(UnitKind::Integer)
        )
        .is_ok());
    }

    #[test]
    fn validate_qty_integer_rejects_fractional_for_entry_initial() {
        // Even entry:initial rejects fractional quantities for integer units.
        assert!(validate_quantity_for_unit_kind(
            0.5,
            &MovementKind::EntryInitial,
            Some(UnitKind::Integer)
        )
        .is_err());
    }

    #[test]
    fn validate_qty_decimal_accepts_fractional() {
        // Decimal unit kinds accept any positive magnitude, including
        // fractional values.
        assert!(validate_quantity_for_unit_kind(
            0.25,
            &MovementKind::ExitSale,
            Some(UnitKind::Decimal)
        )
        .is_ok());
        assert!(validate_quantity_for_unit_kind(
            1.5,
            &MovementKind::InventoryAdjustment,
            Some(UnitKind::Decimal)
        )
        .is_ok());
        assert!(validate_quantity_for_unit_kind(
            3.14159,
            &MovementKind::Transfer,
            Some(UnitKind::Decimal)
        )
        .is_ok());
    }

    #[test]
    fn validate_qty_decimal_rejects_zero_and_negative_for_non_entry() {
        // Decimal units still reject qty == 0 for non-entry movements.
        assert!(validate_quantity_for_unit_kind(
            0.0,
            &MovementKind::ExitSale,
            Some(UnitKind::Decimal)
        )
        .is_err());
        assert!(validate_quantity_for_unit_kind(
            -2.0,
            &MovementKind::ExitSale,
            Some(UnitKind::Decimal)
        )
        .is_err());
    }

    #[test]
    fn validate_qty_none_unit_kind_treated_as_decimal() {
        // Legacy / uncatalogued products (unit_kind = None) accept
        // fractional quantities to preserve back-compat.
        assert!(validate_quantity_for_unit_kind(1.5, &MovementKind::ExitSale, None).is_ok());
        assert!(
            validate_quantity_for_unit_kind(0.75, &MovementKind::InventoryAdjustment, None).is_ok()
        );
        // Whole numbers are also accepted.
        assert!(validate_quantity_for_unit_kind(4.0, &MovementKind::Transfer, None).is_ok());
    }

    #[test]
    fn validate_qty_inventory_adjustment_increase_with_integer_unit_whole() {
        // Inventory adjustment (increase) with integer unit + whole qty.
        assert!(validate_quantity_for_unit_kind(
            7.0,
            &MovementKind::InventoryAdjustment,
            Some(UnitKind::Integer)
        )
        .is_ok());
    }

    #[test]
    fn validate_qty_inventory_adjustment_decrease_with_integer_unit_whole() {
        // Inventory adjustment (decrease) with integer unit + whole qty.
        assert!(validate_quantity_for_unit_kind(
            3.0,
            &MovementKind::InventoryAdjustment,
            Some(UnitKind::Integer)
        )
        .is_ok());
    }

    #[test]
    fn validate_qty_error_messages_in_spanish() {
        // Verify all rejection branches produce Spanish-language messages.
        let err_integer_frac =
            validate_quantity_for_unit_kind(1.5, &MovementKind::ExitSale, Some(UnitKind::Integer))
                .unwrap_err();
        assert!(err_integer_frac.contains("entero"));

        let err_integer_zero =
            validate_quantity_for_unit_kind(0.0, &MovementKind::ExitSale, Some(UnitKind::Integer))
                .unwrap_err();
        assert!(err_integer_zero.contains("mayor a 0"));

        let err_decimal_zero =
            validate_quantity_for_unit_kind(0.0, &MovementKind::ExitSale, Some(UnitKind::Decimal))
                .unwrap_err();
        assert!(err_decimal_zero.contains("mayor a 0"));
    }
}
