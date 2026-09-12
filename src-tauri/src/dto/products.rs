//! DTOs for product catalog: categories, products, and barcodes.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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

/// Input for creating a product. SKU must be unique across the database.
#[derive(Debug, Deserialize)]
pub struct ProductCreate {
    pub sku: String,
    pub description: String,
    pub category_id: Option<String>,
    pub default_unit: Option<String>,
    pub default_alert_days_before: i32,
    pub notes: Option<String>,
}

/// Input for updating an existing product.
#[derive(Debug, Deserialize)]
pub struct ProductUpdate {
    pub id: String,
    pub sku: String,
    pub description: String,
    pub category_id: Option<String>,
    pub default_unit: Option<String>,
    pub default_alert_days_before: i32,
    pub notes: Option<String>,
    pub is_active: bool,
}

/// Response shape for a product.
#[derive(Debug, Serialize, FromRow)]
pub struct ProductResponse {
    pub id: String,
    pub sku: String,
    pub description: String,
    pub category_id: Option<String>,
    pub default_unit: Option<String>,
    pub default_alert_days_before: i32,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Detail bundle returned by `get_product`: product, its barcodes, and the
/// resolved category (when one is assigned).
#[derive(Debug, Serialize)]
pub struct ProductDetailResponse {
    pub product: ProductResponse,
    pub barcodes: Vec<ProductBarcodeResponse>,
    pub category: Option<CategoryResponse>,
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
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ProductSearchResult {
    pub id: String,
    pub sku: String,
    pub description: String,
    pub category_id: Option<String>,
    pub primary_barcode: Option<String>,
    pub is_active: bool,
}
