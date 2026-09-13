/**
 * TypeScript API wrapper for CSV import/export commands.
 *
 * Slice 10a: preview + exports.
 * Slice 10b: import commit with conflict strategies and column mapping UI.
 */

import { invoke } from "@tauri-apps/api/core";
import {
 open as openDialog,
 save as saveDialog,
} from "@tauri-apps/plugin-dialog";

// ─── Conflict strategy ───────────────────────────────────────────────

/** How to handle rows whose SKU or barcode already exists. */
export type ConflictStrategy = "skip" | "update" | "review";

// ─── DTOs (mirror Rust DTOs in src-tauri/src/dto/csv_io.rs) ────────────

/** Optional column index override. Missing fields fall back to auto-detect. */
export interface CsvColumnMapping {
 sku?: number | null;
 description?: number | null;
 barcode?: number | null;
 category?: number | null;
 default_unit?: number | null;
 default_alert_days_before?: number | null;
 notes?: number | null;
}

export interface CsvPreviewInput {
 /** Raw CSV text including the header row. */
 content: string;
 /** Optional explicit column mapping. */
 mapping?: CsvColumnMapping | null;
}

/** Per-row classification produced by `previewProductCsv`. */
export type CsvPreviewRowStatus =
 | { kind: "ok" }
 | {
    kind: "duplicate_sku";
    existing_product_id: string;
    existing_sku: string;
   }
 | {
    kind: "duplicate_barcode";
    existing_product_id: string;
    existing_barcode: string;
   }
 | { kind: "missing_required"; field: string }
 | { kind: "invalid"; reason: string };

export interface CsvPreviewRow {
 row_index: number;
 raw: string[];
 sku?: string | null;
 description?: string | null;
 barcode?: string | null;
 category?: string | null;
 default_unit?: string | null;
 default_alert_days_before?: number | null;
 notes?: string | null;
 status: CsvPreviewRowStatus;
}

export interface CsvPreviewResponse {
 headers: string[];
 mapping_used: CsvColumnMapping;
 total_rows: number;
 valid_rows: number;
 invalid_rows: number;
 duplicate_sku_count: number;
 duplicate_barcode_count: number;
 missing_required_count: number;
 rows: CsvPreviewRow[];
}

export interface ReportExportInput {
 store_id?: string | null;
 location_id?: string | null;
 preset?: DashboardPreset | null;
 urgency?: string | null;
 path: string;
}

export interface CsvExportResult {
 path: string;
 rows_written: number;
 bytes_written: number;
}

// ─── Import DTOs ──────────────────────────────────────────────────────

/** Input for the import commit command. */
export interface CsvImportInput {
 content: string;
 mapping: CsvColumnMapping;
 strategy: ConflictStrategy;
}

/** Per-row import outcome. */
export type CsvImportRowOutcome =
 | { action: "created"; product_id: string; sku: string }
 | { action: "skipped"; reason: string }
 | { action: "updated"; product_id: string; sku: string }
 | { action: "invalid"; reason: string };

/** Summary of a single imported row. */
export interface CsvImportRowResult {
 row_index: number;
 sku: string;
 description: string;
 barcode: string;
 outcome: CsvImportRowOutcome;
}

/** Result returned by `importProductCsv`. */
export interface CsvImportResult {
 total_rows: number;
 created: number;
 skipped: number;
 updated: number;
 invalid: number;
 rows: CsvImportRowResult[];
}

/** Mirror of Rust `DashboardPreset` (snake_case). */
export type DashboardPreset =
 | "expired"
 | "today"
 | "alert_window"
 | "next_7_days"
 | "next_30_days"
 | "all";

// ─── Backend command wrappers ───────────────────────────────────────────

/**
 * Commits a product CSV import using the provided column mapping and conflict
 * strategy. Creates new products, updates existing products (when strategy is
 * `update`), or returns conflict rows for review (when strategy is `review`).
 */
export async function importProductCsv(
 input: CsvImportInput,
): Promise<CsvImportResult> {
 return invoke<CsvImportResult>("import_product_csv", { input });
}

/**
 * Backend preview for a product CSV. Detects headers, validates the column
 * mapping, classifies each row against existing products/barcodes, and
 * returns a row-by-row summary plus aggregate counts.
 *
 * Does NOT modify the database — preview only.
 */
export async function previewProductCsv(
 input: CsvPreviewInput,
): Promise<CsvPreviewResponse> {
 return invoke<CsvPreviewResponse>("preview_product_csv", { input });
}

/**
 * Writes the canonical products CSV (every product, active + archived) to
 * the given destination path. Use `pickCsvSavePath` to let the user choose
 * the destination.
 */
export async function exportProductsCsv(
 path: string,
): Promise<CsvExportResult> {
 return invoke<CsvExportResult>("export_products_csv", { path });
}

/**
 * Writes the dashboard report CSV to the given destination path using the
 * same filter shape as `listDashboardLots`. The export reflects the current
 * dashboard view (urgency classification, sorting, counts).
 */
export async function exportReportCsv(
 input: ReportExportInput,
): Promise<CsvExportResult> {
 return invoke<CsvExportResult>("export_report_csv", { input });
}

/** Reads a UTF-8 CSV file from disk and returns its contents. */
export async function readCsvText(path: string): Promise<string> {
 return invoke<string>("read_csv_text", { path });
}

// ─── Tauri dialog helpers ──────────────────────────────────────────────

/** Default file filters for product CSV files. */
const CSV_FILTERS = [
 { name: "CSV files", extensions: ["csv"] },
 { name: "All files", extensions: ["*"] },
];

/**
 * Opens a native "open file" dialog scoped to CSV files. Returns the selected
 * file path, or `null` when the user cancels.
 */
export async function pickCsvFile(): Promise<string | null> {
 const selected = await openDialog({
  multiple: false,
  directory: false,
  filters: CSV_FILTERS,
  title: "Select a CSV file",
 });
 if (selected === null) return null;
 return Array.isArray(selected) ? (selected[0] ?? null) : selected;
}

/**
 * Opens a native "save file" dialog and returns the chosen destination path,
 * or `null` when the user cancels. The dialog suggests the given default
 * filename (without extension) and adds a `.csv` extension automatically.
 */
export async function pickCsvSavePath(
 defaultName: string,
 title: string = "Save CSV file",
): Promise<string | null> {
 const path = await saveDialog({
  defaultPath: `${defaultName}.csv`,
  filters: CSV_FILTERS,
  title,
 });
 return path ?? null;
}

// ─── High-level flows ──────────────────────────────────────────────────

/**
 * High-level "import CSV" preview flow:
 * 1. Open a native file picker (`pickCsvFile`).
 * 2. Read the file contents via the `read_csv_text` backend command.
 * 3. Run `preview_product_csv` and return the preview summary.
 *
 * Returns `null` when the user cancels the file dialog. The preview never
 * modifies the database.
 */
export async function importProductCsvPreview(): Promise<{
 path: string;
 preview: CsvPreviewResponse;
} | null> {
 const path = await pickCsvFile();
 if (!path) return null;
 const content = await readCsvText(path);
 const preview = await previewProductCsv({ content });
 return { path, preview };
}

/**
 * High-level "export products" flow:
 * 1. Open a native save dialog with a sensible default filename.
 * 2. Call `export_products_csv` with the chosen destination.
 *
 * Returns `null` when the user cancels.
 */
export async function exportProductsWithDialog(): Promise<CsvExportResult | null> {
 const path = await pickCsvSavePath("caduxo-products");
 if (!path) return null;
 return exportProductsCsv(path);
}

/**
 * High-level "export report" flow:
 * 1. Open a native save dialog.
 * 2. Call `export_report_csv` with the current dashboard filters and the
 * chosen destination.
 *
 * Returns `null` when the user cancels.
 */
export async function exportReportWithDialog(
 filters: Omit<ReportExportInput, "path">,
): Promise<CsvExportResult | null> {
 const path = await pickCsvSavePath("caduxo-report");
 if (!path) return null;
 return exportReportCsv({ ...filters, path });
}
