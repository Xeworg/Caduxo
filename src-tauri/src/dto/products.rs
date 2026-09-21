//! DTOs for product catalog: categories, products, and barcodes.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::unit_definitions::UnitKind;

// ============================================================
// Categories
// ============================================================

/// Input for creating a category.
#[derive(Debug, Deserialize)]
pub struct CategoryCreate {
    pub name: String,
}

/// Input for updating an existing category.
#[derive(Debug, Deserialize)]
pub struct CategoryUpdate {
    pub id: String,
    pub name: String,
    pub is_active: bool,
}

/// Response shape for a category.
#[derive(Debug, Serialize, FromRow)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================
// Products
// ============================================================

/// Sentinel id used by the frontend picker to represent "products with zero
/// active category relations". Rendered as an `Uncategorized` pseudo-row.
/// Never persisted — only emitted by the picker when the user selects it.
pub const UNCATEGORIZED_SENTINEL: &str = "__uncategorized__";

/// Input for creating a product. SKU must be unique across the database.
///
/// Either `default_unit_id` (catalog FK) or `default_unit` (raw legacy text) may be
/// provided. The service layer resolves whichever is supplied into the FK + `unit_type` pair.
/// `category_ids` replaces the legacy `category_id` field; empty `Vec` or `None` both
/// mean "unassigned".
#[derive(Debug, Deserialize)]
pub struct ProductCreate {
    pub sku: String,
    pub description: String,
    /// Category ids for the new product. Empty `Vec` or `None` means "unassigned".
    /// The legacy `category_id` field is dropped from the runtime model.
    pub category_ids: Option<Vec<String>>,
    /// Raw legacy text for the default unit. Kept as a back-compat echo.
    pub default_unit: Option<String>,
    /// Optional catalog FK. Takes precedence over `default_unit` text.
    pub default_unit_id: Option<String>,
    pub default_alert_days_before: i32,
    pub notes: Option<String>,
}

/// Input for updating an existing product. `category_ids` is the full set
/// on each update; the service layer performs a delete-all + insert-new
/// inside a transaction.
#[derive(Debug, Deserialize)]
pub struct ProductUpdate {
    pub id: String,
    pub sku: String,
    pub description: String,
    /// Full set of category ids on each update; empty `Vec` or `None` means "unassigned".
    pub category_ids: Option<Vec<String>>,
    /// Raw legacy text for the default unit. Clears catalog link when set to None/empty.
    pub default_unit: Option<String>,
    /// Optional catalog FK. Takes precedence over `default_unit` text.
    pub default_unit_id: Option<String>,
    pub default_alert_days_before: i32,
    pub notes: Option<String>,
    pub is_active: bool,
}

/// Response shape for a product.
/// `category_ids` is always present and may be empty. The legacy `category_id`
/// field is dropped from the runtime model; see design §2.2.
#[derive(Debug, Clone, Serialize)]
pub struct ProductResponse {
    pub id: String,
    pub sku: String,
    pub description: String,
    /// Category ids from the `product_categories` junction. Always present; may be empty.
    pub category_ids: Vec<String>,
    /// Echoes the catalog `display_name` when `default_unit_id` is set; raw legacy
    /// text otherwise. Always preserved for compatibility.
    pub default_unit: Option<String>,
    /// Catalog FK. `None` when the product has no catalog link.
    pub default_unit_id: Option<String>,
    /// Unit kind from the catalog, or `None` for legacy products.
    pub unit_type: Option<UnitKind>,
    pub default_alert_days_before: i32,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Detail bundle returned by `get_product`: product, its barcodes, and the
/// resolved categories (zero or more). Replaces the legacy single `category` field.
#[derive(Debug, Serialize)]
pub struct ProductDetailResponse {
    pub product: ProductResponse,
    pub barcodes: Vec<ProductBarcodeResponse>,
    /// Resolved categories from the junction. Empty when the product is unassigned.
    pub categories: Vec<CategoryResponse>,
}

// ============================================================
// Product barcodes
// ============================================================

/// Input for adding a barcode to a product.
#[derive(Debug, Deserialize)]
pub struct ProductBarcodeCreate {
    pub product_id: String,
    pub barcode: String,
    pub barcode_type: Option<String>,
    pub is_primary: bool,
}

/// Response shape for a product barcode.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ProductBarcodeResponse {
    pub id: String,
    pub product_id: String,
    pub barcode: String,
    pub barcode_type: Option<String>,
    pub is_primary: bool,
    pub created_at: String,
}

/// Input for removing a barcode by id.
#[derive(Debug, Deserialize)]
pub struct ProductBarcodeRemoveInput {
    pub id: String,
}

// ============================================================
// Search
// ============================================================

/// Input for the product search command. An empty query lists all products.
#[derive(Debug, Deserialize)]
pub struct ProductSearchQuery {
    pub query: String,
}

/// Search hit shape — minimal fields for scan/search results and listing.
/// `category_ids` replaces the legacy `category_id` field.
#[derive(Debug, Clone, Serialize)]
pub struct ProductSearchResult {
    pub id: String,
    pub sku: String,
    pub description: String,
    pub category_ids: Vec<String>,
    pub primary_barcode: Option<String>,
    pub is_active: bool,
}

// ============================================================
// Category search
// ============================================================

/// Input for the category search command.
#[derive(Debug, Deserialize)]
pub struct CategorySearchInput {
    pub query: String,
    pub limit: Option<usize>,
}

/// Page of category search results.
#[derive(Debug, Serialize)]
pub struct CategorySearchPage {
    pub items: Vec<CategoryResponse>,
    pub total: usize,
    pub has_more: bool,
}
