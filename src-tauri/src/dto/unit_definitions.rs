//! DTOs for the unit definitions catalog.
//!
//! Types here define the Tauri IPC boundary for catalog management:
//! listing preset and custom units, creating custom units inline, renaming,
//! and archiving (with a business-rule guard when referenced).

use serde::{Deserialize, Serialize};

/// Enables SQLx to decode TEXT → UnitKind in SQLite.
impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for UnitKind {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s: &str = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        match s {
            "integer" => Ok(UnitKind::Integer),
            "decimal" => Ok(UnitKind::Decimal),
            other => Err(format!("unknown UnitKind variant: {other}").into()),
        }
    }
}

impl sqlx::Type<sqlx::Sqlite> for UnitKind {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <str as sqlx::Type<sqlx::Sqlite>>::type_info()
    }

    fn compatible(ty: &sqlx::sqlite::SqliteTypeInfo) -> bool {
        <str as sqlx::Type<sqlx::Sqlite>>::compatible(ty)
    }
}

/// Kind of unit — drives LotForm quantity input rules and display formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnitKind {
    /// Count-like units: pieces, boxes, bags, etc.
    Integer,
    /// Mass/volume units: kilogramo, gramo, litro, etc.
    Decimal,
}

/// Full unit definition returned by catalog commands.
#[derive(Debug, Clone, Serialize)]
pub struct UnitDefinitionResponse {
    pub id: String,
    pub key: String,
    pub display_name: String,
    pub kind: UnitKind,
    pub is_preset: bool,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a new custom unit from ProductForm.
#[derive(Debug, Deserialize)]
pub struct UnitDefinitionCreateInput {
    pub key: String,
    pub display_name: String,
    pub kind: UnitKind,
}

/// Input for renaming a unit's display_name (key is immutable in this slice).
#[derive(Debug, Deserialize)]
pub struct UnitDefinitionRenameInput {
    pub id: String,
    pub display_name: String,
}

/// An unrecognized unit group surfaced in the audit banner / review page.
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct UnrecognizedUnitGroup {
    /// The raw text value from `products.default_unit` that has no catalog link.
    pub raw_value: String,
    /// How many products share this raw value.
    pub product_count: usize,
    /// One sample product id for UI linking.
    pub sample_product_id: String,
}

/// Banner visibility state returned by `unit_audit_banner_state`.
#[derive(Debug, Clone, Serialize)]
pub struct UnitAuditBannerState {
    /// True when a non-dismissed unrecognized-value set exists.
    pub visible: bool,
    /// SHA-256 truncated to 16 hex chars of the sorted unrecognized-value list.
    pub signature: String,
    /// Number of distinct unrecognized value groups.
    pub group_count: usize,
}

/// Possible review actions applied from the UnitReviewPage.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UnitReviewAction {
    /// Map all products with `raw_value` to an existing preset unit.
    MapToPreset {
        raw_value: String,
        preset_id: String,
    },
    /// Create a custom unit from the raw value and map all matching products to it.
    KeepAsCustom { raw_value: String },
    /// No-op: leave the products unlinked for later review.
    #[allow(dead_code)]
    LeaveForLater { raw_value: String },
}

/// Result of applying a review action.
#[derive(Debug, Clone, Serialize)]
pub struct UnitReviewActionResult {
    /// Number of products updated by this action.
    pub updated_product_count: usize,
}
