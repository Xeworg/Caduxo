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

// ─── Scanner / scan-search workflow ────────────────────────────────────────

/** Discriminates why `findProductByScan` matched. */
export type ScanMatchType = "barcode" | "sku";

/** Successful scan match. */
export interface ScanFoundResult {
   match_type: ScanMatchType;
   product: ProductSearchResult;
   /** True when the product already has at least one active expiry lot. */
   has_lots: boolean;
}

/** Scan value did not match any barcode or SKU. */
export interface ScanNotFoundResult {
   match_type: "not_found";
   scanned_value: string;
}

export type ScanSearchResult = ScanFoundResult | ScanNotFoundResult;

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

/**
 * Adds a barcode to an existing product. Used by the quick-create flow
 * to attach the scanned barcode to a newly created product.
 * Silently ignores `DuplicateField` errors (barcode already attached).
 */
export async function addProductBarcodeIfNew(
   input: ProductBarcodeCreate,
): Promise<ProductBarcodeResponse | null> {
   try {
      return await addProductBarcode(input);
   } catch {
      // Barcode already attached — non-fatal in quick-create.
      return null;
   }
}

// ─── Scanner ────────────────────────────────────────────────────────────────

/**
 * Barcode-first exact lookup, then SKU-second exact lookup.
 *
 * Returns `ScanFoundResult` when a barcode or SKU matches, or
 * `ScanNotFoundResult` when nothing matched.
 * The `has_lots` field tells the caller whether to jump to lot entry
 * directly or show the product detail first.
 */
export async function findProductByScan(
   scannedValue: string,
): Promise<ScanSearchResult> {
   return invoke<ScanSearchResult>("find_product_by_scan", {
      scannedValue,
   });
}
