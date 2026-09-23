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
///
/// `lifecycle` rides along on every product detail so the frontend can
/// render the lifecycle badge without an extra round-trip; the value is
/// `None` for pre-V19 backends (dual-read window per design §5.3) and
/// the frontend derives `lifecycle ?? (is_active ? "active" : "archived")`
/// to keep the badge correct in either case.
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
    /// Lifecycle state of the product. `None` when the backend is on a
    /// pre-V19 schema (rolling-deploy compatibility).
    pub lifecycle: Option<ProductLifecycle>,
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
// Lifecycle
// ============================================================

/// Product lifecycle state. The wire shape is `snake_case` to match the
/// stored `lifecycle` TEXT column on `products` (and the sibling mirror
/// column on `product_barcodes`). The default for newly created
/// products is `Active`; the terminal state is `Retired`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductLifecycle {
    Active,
    Archived,
    Retired,
}

/// Product lifecycle event type. The wire shape is `snake_case`:
/// `archived`, `unarchived`, `retired`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductLifecycleEventType {
    Archived,
    Unarchived,
    Retired,
}

/// Row shape returned by `list_product_lifecycle_events` and surfaced to
/// the per-product history pane on `ProductDetailPage.svelte`. The
/// `actor` and `reason` fields are optional because the service-layer
/// contract allows both to be `NULL` (system-emitted archive / unarchive
/// events have no actor; only retire events carry a reason).
#[derive(Debug, Clone, Serialize)]
pub struct ProductLifecycleEventResponse {
    pub id: String,
    pub product_id: String,
    pub event_type: ProductLifecycleEventType,
    pub from_state: ProductLifecycle,
    pub to_state: ProductLifecycle,
    pub actor: Option<String>,
    pub reason: Option<String>,
    pub created_at: String,
}

/// Input for the `retire_product` IPC. `reason` is REQUIRED and the
/// service layer enforces non-blank-after-trim (design constraint 5).
/// `actor` is optional and may be supplied by the frontend when the
/// operator is identified; the current Caduxo UX passes `None` and the
/// service layer treats `actor` as `NULL` (system-emitted).
#[derive(Debug, Deserialize)]
pub struct RetireProductInput {
    pub id: String,
    pub reason: String,
    pub actor: Option<String>,
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
///
/// `lifecycle` rides along on every search hit so the frontend can render
/// the lifecycle badge without an extra round-trip; the value is `None`
/// for pre-V19 backends (dual-read window per design §5.3) and defaults
/// to `Active` for `is_active = true` / `Archived` for `is_active = false`
/// in the frontend compatibility helper.
#[derive(Debug, Clone, Serialize)]
pub struct ProductSearchResult {
    pub id: String,
    pub sku: String,
    pub description: String,
    pub category_ids: Vec<String>,
    pub default_unit: Option<String>,
    pub default_alert_days_before: i32,
    pub primary_barcode: Option<String>,
    pub is_active: bool,
    /// Lifecycle state of the product. `None` when the backend is on a
    /// pre-V19 schema (older binary during rolling deploy); the frontend
    /// derives `lifecycle ?? (is_active ? "active" : "archived")` to keep
    /// the badge correct in either case.
    pub lifecycle: Option<ProductLifecycle>,
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
