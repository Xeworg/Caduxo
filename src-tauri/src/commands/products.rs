//! Tauri commands for product catalog management (categories, products,
//! barcodes) and product search.

use tauri::State;

use crate::dto::products::{
    CategoryCreate, CategoryResponse, CategoryUpdate, ProductBarcodeCreate,
    ProductBarcodeRemoveInput, ProductBarcodeResponse, ProductCreate, ProductDetailResponse,
    ProductResponse, ProductSearchQuery, ProductSearchResult, ProductUpdate,
};
use crate::dto::scanner::ScanSearchResult;
use crate::error::{AppError, CommandError};
use crate::services::products as service;
use crate::state::AppState;

// ============================================================
// Categories
// ============================================================

/// Lists active categories, ordered alphabetically.
#[tauri::command]
pub async fn list_categories(
    state: State<'_, AppState>,
) -> Result<Vec<CategoryResponse>, CommandError> {
    service::list_categories(&state.pool)
        .await
        .map_err(AppError::into)
}

/// Creates a new category.
#[tauri::command]
pub async fn create_category(
    state: State<'_, AppState>,
    input: CategoryCreate,
) -> Result<CategoryResponse, CommandError> {
    service::create_category(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Updates an existing category (rename and/or archive).
#[tauri::command]
pub async fn update_category(
    state: State<'_, AppState>,
    input: CategoryUpdate,
) -> Result<CategoryResponse, CommandError> {
    service::update_category(&state.pool, input)
        .await
        .map_err(AppError::into)
}

// ============================================================
// Products
// ============================================================

/// Creates a new active product.
#[tauri::command]
pub async fn create_product(
    state: State<'_, AppState>,
    input: ProductCreate,
) -> Result<ProductResponse, CommandError> {
    service::create_product(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Updates an existing product.
#[tauri::command]
pub async fn update_product(
    state: State<'_, AppState>,
    input: ProductUpdate,
) -> Result<ProductResponse, CommandError> {
    service::update_product(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Soft-archives a product.
#[tauri::command]
pub async fn archive_product(state: State<'_, AppState>, id: String) -> Result<(), CommandError> {
    service::archive_product(&state.pool, id)
        .await
        .map_err(AppError::into)
}

/// Returns the product detail bundle (product + barcodes + category).
#[tauri::command]
pub async fn get_product(
    state: State<'_, AppState>,
    id: String,
) -> Result<ProductDetailResponse, CommandError> {
    service::get_product(&state.pool, id)
        .await
        .map_err(AppError::into)
}

/// Searches products by description, SKU, or barcode.
#[tauri::command]
pub async fn search_products(
    state: State<'_, AppState>,
    query: ProductSearchQuery,
) -> Result<Vec<ProductSearchResult>, CommandError> {
    service::search_products(&state.pool, query)
        .await
        .map_err(AppError::into)
}

/// Scanner workflow: barcode-first exact lookup, then SKU-second exact lookup.
///
/// Returns `Found { product, has_lots }` when a barcode or SKU matches, or
/// `NotFound { scanned_value }` when nothing matched. The `has_lots` field
/// tells the frontend whether to jump directly to lot entry or show the
/// product detail first.
#[tauri::command]
pub async fn find_product_by_scan(
    state: State<'_, AppState>,
    scanned_value: String,
) -> Result<ScanSearchResult, CommandError> {
    service::find_product_by_scan(&state.pool, &scanned_value)
        .await
        .map_err(AppError::into)
}

/// Returns the software-suggested default alert-days value for new products.
#[tauri::command]
pub fn suggested_product_alert_days() -> i32 {
    service::suggested_alert_days()
}

// ============================================================
// Barcodes
// ============================================================

/// Adds a barcode to a product.
#[tauri::command]
pub async fn add_product_barcode(
    state: State<'_, AppState>,
    input: ProductBarcodeCreate,
) -> Result<ProductBarcodeResponse, CommandError> {
    service::add_barcode(&state.pool, input)
        .await
        .map_err(AppError::into)
}

/// Lists all barcodes for a product.
#[tauri::command]
pub async fn list_product_barcodes(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<Vec<ProductBarcodeResponse>, CommandError> {
    service::list_barcodes(&state.pool, product_id)
        .await
        .map_err(AppError::into)
}

/// Removes a barcode by id.
#[tauri::command]
pub async fn remove_product_barcode(
    state: State<'_, AppState>,
    input: ProductBarcodeRemoveInput,
) -> Result<(), CommandError> {
    service::remove_barcode(&state.pool, input)
        .await
        .map_err(AppError::into)
}
