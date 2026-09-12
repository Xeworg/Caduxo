/**
 * TypeScript API wrapper for Caduxo Tauri commands (Slice 4 — product catalog).
 * Thin wrappers around the Rust command layer; no business logic here.
 */

import { invoke } from "@tauri-apps/api/core";

// ─── Categories ─────────────────────────────────────────────────────────────

export interface CategoryResponse {
 id: string;
 name: string;
 is_active: boolean;
 created_at: string;
 updated_at: string;
}

export interface CategoryCreate {
 name: string;
}

export interface CategoryUpdate {
 id: string;
 name: string;
 is_active: boolean;
}

// ─── Products ───────────────────────────────────────────────────────────────

export interface ProductResponse {
 id: string;
 sku: string;
 description: string;
 category_id: string | null;
 default_unit: string | null;
 default_alert_days_before: number;
 notes: string | null;
 is_active: boolean;
 created_at: string;
 updated_at: string;
}

export interface ProductCreate {
 sku: string;
 description: string;
 category_id?: string | null;
 default_unit?: string | null;
 default_alert_days_before: number;
 notes?: string | null;
}

export interface ProductUpdate {
 id: string;
 sku: string;
 description: string;
 category_id?: string | null;
 default_unit?: string | null;
 default_alert_days_before: number;
 notes?: string | null;
 is_active: boolean;
}

export interface ProductDetailResponse {
 product: ProductResponse;
 barcodes: ProductBarcodeResponse[];
 category: CategoryResponse | null;
}

// ─── Product barcodes ───────────────────────────────────────────────────────

export interface ProductBarcodeResponse {
 id: string;
 product_id: string;
 barcode: string;
 barcode_type: string | null;
 is_primary: boolean;
 created_at: string;
}

export interface ProductBarcodeCreate {
 product_id: string;
 barcode: string;
 barcode_type?: string | null;
 is_primary: boolean;
}

export interface ProductBarcodeRemoveInput {
 id: string;
}

// ─── Search ─────────────────────────────────────────────────────────────────

export interface ProductSearchQuery {
 query: string;
}

export interface ProductSearchResult {
 id: string;
 sku: string;
 description: string;
 category_id: string | null;
 primary_barcode: string | null;
 is_active: boolean;
}

// ─── Categories CRUD ────────────────────────────────────────────────────────

export async function listCategories(): Promise<CategoryResponse[]> {
 return invoke<CategoryResponse[]>("list_categories");
}

export async function createCategory(
 input: CategoryCreate,
): Promise<CategoryResponse> {
 return invoke<CategoryResponse>("create_category", { input });
}

export async function updateCategory(
 input: CategoryUpdate,
): Promise<CategoryResponse> {
 return invoke<CategoryResponse>("update_category", { input });
}

// ─── Products CRUD ──────────────────────────────────────────────────────────

export async function createProduct(
 input: ProductCreate,
): Promise<ProductResponse> {
 return invoke<ProductResponse>("create_product", { input });
}

export async function updateProduct(
 input: ProductUpdate,
): Promise<ProductResponse> {
 return invoke<ProductResponse>("update_product", { input });
}

export async function archiveProduct(id: string): Promise<void> {
 return invoke<void>("archive_product", { id });
}

export async function getProduct(id: string): Promise<ProductDetailResponse> {
 return invoke<ProductDetailResponse>("get_product", { id });
}

export async function searchProducts(
 query: ProductSearchQuery,
): Promise<ProductSearchResult[]> {
 return invoke<ProductSearchResult[]>("search_products", { query });
}

/**
 * Returns the software-suggested default alert-days value for new products.
 * Sync command — no pool call.
 */
export async function suggestedProductAlertDays(): Promise<number> {
 return invoke<number>("suggested_product_alert_days");
}

// ─── Barcodes ───────────────────────────────────────────────────────────────

export async function addProductBarcode(
 input: ProductBarcodeCreate,
): Promise<ProductBarcodeResponse> {
 return invoke<ProductBarcodeResponse>("add_product_barcode", { input });
}

export async function listProductBarcodes(
 productId: string,
): Promise<ProductBarcodeResponse[]> {
 return invoke<ProductBarcodeResponse[]>("list_product_barcodes", {
  productId,
 });
}

export async function removeProductBarcode(
 input: ProductBarcodeRemoveInput,
): Promise<void> {
 return invoke<void>("remove_product_barcode", { input });
}
