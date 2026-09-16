//! Lot movement domain logic — pure functions with no I/O.
//!
//! All functions here are unit-testable in isolation. They handle:
//! - Movement kind validation and delta computation
//! - Batch code derivation and collision resolution
//! - Legacy resolution mapping

use std::fmt;

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

impl MovementKind {
    /// All valid movement kind string values.
    pub const ALL_KINDS: &'static [&'static str] = &[
        "entry:initial",
        "transfer",
        "exit:sale",
        "exit:waste",
        "exit:expired",
        "exit:damaged",
        "exit:internal_consumption",
        "exit:return_to_supplier",
        "exit:inventory_adjustment",
        "exit:other",
        "inventory_adjustment",
    ];
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

/// Extracts the NNN counter from a batch code matching `PREFIX-YYYYMMDD-NNN` format.
/// Returns `None` if the format doesn't match.
pub fn extract_batch_nnn(batch_code: &str, prefix: &str, date: &str) -> Option<u32> {
    let expected_prefix = format!("{}-{}", prefix, date);
    if !batch_code.starts_with(&expected_prefix) {
        return None;
    }

    let suffix = &batch_code[expected_prefix.len()..];
    // Strip leading dash if present (for collision suffix format)
    let suffix = suffix.strip_prefix('-').unwrap_or(suffix);

    // Try NN or NNN format
    let nnn_str = suffix
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    nnn_str.parse::<u32>().ok()
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

/// Maps a legacy resolution text to the corresponding movement kind.
/// The lookup is case-insensitive.
pub fn legacy_resolution_to_kind(text: &str) -> MovementKind {
    match text.trim().to_ascii_lowercase().as_str() {
        "consumed" => MovementKind::ExitInternalConsumption,
        "sold" => MovementKind::ExitSale,
        "discarded" => MovementKind::ExitWaste,
        // Unknown resolutions map to exit:other
        _ => MovementKind::ExitOther,
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
    // Batch NNN extraction tests
    // ============================================================

    #[test]
    fn extract_batch_nnn_valid() {
        assert_eq!(
            extract_batch_nnn("SKU001-20261015-001", "SKU001", "20261015"),
            Some(1)
        );
        assert_eq!(
            extract_batch_nnn("SKU001-20261015-999", "SKU001", "20261015"),
            Some(999)
        );
    }

    #[test]
    fn extract_batch_nnn_with_collision_suffix() {
        assert_eq!(
            extract_batch_nnn("SKU001-20261015-999-2", "SKU001", "20261015"),
            Some(999)
        );
    }

    #[test]
    fn extract_batch_nnn_mismatch() {
        assert_eq!(
            extract_batch_nnn("OTHER-20261015-001", "SKU001", "20261015"),
            None
        );
        assert_eq!(
            extract_batch_nnn("SKU001-20261231-001", "SKU001", "20261015"),
            None
        );
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
    // Legacy resolution mapping tests
    // ============================================================

    #[test]
    fn legacy_resolution_known_values() {
        assert_eq!(
            legacy_resolution_to_kind("consumed"),
            MovementKind::ExitInternalConsumption
        );
        assert_eq!(
            legacy_resolution_to_kind("CONSUMED"),
            MovementKind::ExitInternalConsumption
        );
        assert_eq!(legacy_resolution_to_kind("sold"), MovementKind::ExitSale);
        assert_eq!(
            legacy_resolution_to_kind("discarded"),
            MovementKind::ExitWaste
        );
    }

    #[test]
    fn legacy_resolution_unknown() {
        assert_eq!(
            legacy_resolution_to_kind("donated"),
            MovementKind::ExitOther
        );
        assert_eq!(
            legacy_resolution_to_kind("transferred"),
            MovementKind::ExitOther
        );
        assert_eq!(legacy_resolution_to_kind("other"), MovementKind::ExitOther);
        assert_eq!(
            legacy_resolution_to_kind("unknown-value"),
            MovementKind::ExitOther
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
}
