//! Tauri commands for product catalog management (categories, products,
//! barcodes) and product search.

use tauri::State;

use crate::dto::products::{
    CategoryCreate, CategoryResponse, CategorySearchInput, CategorySearchPage, CategoryUpdate,
    ProductBarcodeCreate, ProductBarcodeRemoveInput, ProductBarcodeResponse, ProductCreate,
    ProductDetailResponse, ProductResponse, ProductSearchQuery, ProductSearchResult, ProductUpdate,
};
use crate::dto::scanner::ScanSearchResult;
use crate::error::{AppError, CommandError};
use crate::pdf::locale::Locale;
use crate::services::categories as categories_service;
use crate::services::products as service;
use crate::services::user_messages::localize_validation;
use crate::state::AppState;

/// Resolves an optional BCP-47 locale tag into a [`Locale`], falling back to
/// English when the frontend does not supply one. Used at every product
/// command boundary so the catalog validation messages stay
/// frontend-compatible (no required caller-side argument).
fn resolve_locale(locale: Option<String>) -> Locale {
    locale
        .as_deref()
        .map(Locale::parse)
        .unwrap_or(Locale::En)
}

// ============================================================
// Categories
// ============================================================

/// Lists active categories, ordered alphabetically.
#[tauri::command]
pub async fn list_categories(
    state: State<'_, AppState>,
) -> Result<Vec<CategoryResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_categories(&pool)
        .await
        .map_err(AppError::into)
}

/// Creates a new category.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` so the
/// category `Name` validation surfaced by the service reaches the UI in the
/// active locale. Unknown tags fall back to English via `Locale::parse`.
#[tauri::command]
pub async fn create_category(
    state: State<'_, AppState>,
    input: CategoryCreate,
    locale: Option<String>,
) -> Result<CategoryResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::create_category(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(AppError::into)
}

/// Updates an existing category (rename and/or archive).
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` for the same
/// reason as `create_category`.
#[tauri::command]
pub async fn update_category(
    state: State<'_, AppState>,
    input: CategoryUpdate,
    locale: Option<String>,
) -> Result<CategoryResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::update_category(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(AppError::into)
}

/// Paginated category search (prefix-first, substring fallback).
///
/// Backs the CategoryPicker's async search path when `onSearch` is wired up.
/// Empty query returns every active category, ordered alphabetically.
#[tauri::command]
pub async fn list_categories_search(
    state: State<'_, AppState>,
    input: CategorySearchInput,
) -> Result<CategorySearchPage, CommandError> {
    let pool = state.pool().await;
    categories_service::search(&pool, input)
        .await
        .map_err(AppError::into)
}

// ============================================================
// Products
// ============================================================

/// Creates a new active product.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` so the
/// SKU / description / alert-days validation surfaced by the service reaches
/// the UI in the active locale. Unknown tags fall back to English via
/// `Locale::parse`.
#[tauri::command]
pub async fn create_product(
    state: State<'_, AppState>,
    input: ProductCreate,
    locale: Option<String>,
) -> Result<ProductResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::create_product(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(AppError::into)
}

/// Updates an existing product.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` for the same
/// reason as `create_product`.
#[tauri::command]
pub async fn update_product(
    state: State<'_, AppState>,
    input: ProductUpdate,
    locale: Option<String>,
) -> Result<ProductResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::update_product(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(AppError::into)
}

/// Soft-archives a product.
#[tauri::command]
pub async fn archive_product(state: State<'_, AppState>, id: String) -> Result<(), CommandError> {
    let pool = state.pool().await;
    service::archive_product(&pool, id)
        .await
        .map_err(AppError::into)
}

/// Returns the product detail bundle (product + barcodes + category).
#[tauri::command]
pub async fn get_product(
    state: State<'_, AppState>,
    id: String,
) -> Result<ProductDetailResponse, CommandError> {
    let pool = state.pool().await;
    service::get_product(&pool, id)
        .await
        .map_err(AppError::into)
}

/// Searches products by description, SKU, or barcode.
#[tauri::command]
pub async fn search_products(
    state: State<'_, AppState>,
    query: ProductSearchQuery,
) -> Result<Vec<ProductSearchResult>, CommandError> {
    let pool = state.pool().await;
    service::search_products(&pool, query)
        .await
        .map_err(AppError::into)
}

/// Scanner workflow: barcode-first exact lookup, then SKU-second exact lookup.
///
/// Returns `Found { product, has_lots }` when a barcode or SKU matches, or
/// `NotFound { scanned_value }` when nothing matched. The `has_lots` field
/// tells the frontend whether to jump directly to lot entry or show the
/// product detail first.
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` so the
/// `ScanValueEmpty` validation surfaced by the service reaches the UI in the
/// active locale. Unknown tags fall back to English via `Locale::parse`.
#[tauri::command]
pub async fn find_product_by_scan(
    state: State<'_, AppState>,
    scanned_value: String,
    locale: Option<String>,
) -> Result<ScanSearchResult, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::find_product_by_scan(&pool, &scanned_value)
        .await
        .map_err(|e| localize_validation(e, loc))
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
///
/// `locale` (BCP-47 tag) is forwarded to `localize_validation` so the
/// barcode validation surfaced by the service reaches the UI in the active
/// locale. Unknown tags fall back to English via `Locale::parse`.
#[tauri::command]
pub async fn add_product_barcode(
    state: State<'_, AppState>,
    input: ProductBarcodeCreate,
    locale: Option<String>,
) -> Result<ProductBarcodeResponse, CommandError> {
    let pool = state.pool().await;
    let loc = resolve_locale(locale);
    service::add_barcode(&pool, input)
        .await
        .map_err(|e| localize_validation(e, loc))
        .map_err(AppError::into)
}

/// Lists all barcodes for a product.
#[tauri::command]
pub async fn list_product_barcodes(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<Vec<ProductBarcodeResponse>, CommandError> {
    let pool = state.pool().await;
    service::list_barcodes(&pool, product_id)
        .await
        .map_err(AppError::into)
}

/// Removes a barcode by id.
#[tauri::command]
pub async fn remove_product_barcode(
    state: State<'_, AppState>,
    input: ProductBarcodeRemoveInput,
) -> Result<(), CommandError> {
    let pool = state.pool().await;
    service::remove_barcode(&pool, input)
        .await
        .map_err(AppError::into)
}
