/**
 * TypeScript API wrapper for Caduxo Tauri commands (Slice 11a/11b — reports).
 *
 * Thin wrappers around the Rust `preview_report` and `export_report_pdf`
 * commands. No business logic here. Reuses `DashboardLotRow` from
 * `lib/dashboard.js` so the preview payload matches the dashboard column
 * vocabulary.
 */

import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import type { DashboardLotRow } from "./dashboard.js";
import type { SupportedLocale } from "../i18n/locale.svelte.js";

// ─── DTOs (mirror Rust DTOs in src-tauri/src/dto/reports.rs) ──────────────────

/** Built-in report types. */
export type ReportType =
 | "in_alert_window"
 | "expired"
 | "next_30_days"
 | "custom";

/**
 * Filters supported by reports. Built-in reports honour `store_id` and
 * `location_id`; every report type accepts the remaining fields as
 * post-filters layered on top of the dashboard query.
 */
export interface ReportFilters {
 /** Restrict to a specific store. */
 store_id?: string | null;
 /** Restrict to a specific internal location within the selected store. */
 location_id?: string | null;
 /** Restrict to products assigned to the given categories (any-of via `product_categories` junction, V4).
  * May include `UNCATEGORIZED_SENTINEL` from `lib/categories.ts`. */
 category_ids?: string[] | null;
 /** Restrict to a specific urgency bucket (snake_case string). */
 urgency?: string | null;
 /** Inclusive lower bound on `expiry_date` (YYYY-MM-DD). */
 date_from?: string | null;
 /** Inclusive upper bound on `expiry_date` (YYYY-MM-DD). */
 date_to?: string | null;
}

/** A request to build a report preview payload. */
export interface ReportRequest {
 /** Report type discriminator. The field is named `kind` in the JSON shape. */
 kind: ReportType;
 /** Optional filters. `null` / `undefined` means "no filters". */
 filters?: ReportFilters | null;
}

/** Metadata describing a generated report. */
export interface ReportMetadata {
 /** Report type discriminator (snake_case string). */
 report_type: ReportType;
 /** Human-readable description for the chosen type. */
 description: string;
 /** Snapshot of the effective filters that produced this report. */
 filters_used: ReportFilters;
 /** Generation timestamp in RFC3339 / ISO-8601 form (UTC). */
 generated_at: string;
 /** Number of lots in `ReportData.lots` after all filters are applied. */
 row_count: number;
}

/** Report preview payload: metadata + sorted, classified lot rows. */
export interface ReportData {
 metadata: ReportMetadata;
 lots: DashboardLotRow[];
}

// ─── Command wrappers ────────────────────────────────────────────────────────

/**
 * Builds a preview payload (metadata + sorted lot rows) for a report
 * request. The backend reuses `services::dashboard::get_dashboard` so urgency
 * classification, sort order, and active-lot filtering match the dashboard
 * UI and any future CSV / PDF consumer.
 *
 * Reports supported:
 *   - `in_alert_window` — lots in their per-lot alert window
 *   - `expired` — lots past their expiry date
 *   - `next_30_days` — lots expiring within 30 calendar days
 *   - `custom` — arbitrary combination of store / location / category /
 *     urgency / date-range filters
 *
 * @throws User-safe `CommandError` for malformed dates or inverted ranges.
 */
export async function previewReport(
 request: ReportRequest,
 locale: SupportedLocale,
): Promise<ReportData> {
 return invoke<ReportData>("preview_report", { request, locale });
}

// ─── PDF export (Slice 11b) ───────────────────────────────────────────────

/** Result of a successful PDF render on the Rust side. */
export interface PdfExportResult {
 /** Absolute path of the file written to disk. */
 path: string;
 /** Total pages in the generated PDF. */
 page_count: number;
 /** Number of lot rows in the report body. */
 rows_written: number;
 /** File size in bytes on disk. */
 bytes_written: number;
}

/**
 * Renders the given report request to a PDF file at `file_path`. The PDF
 * rows match `previewReport` exactly because both paths go through the same
 * `services::reports` data layer. A4 landscape, pagination, and metadata
 * headers are handled by the Rust PDF generator.
 */
export async function exportReportPdf(
 request: ReportRequest,
 file_path: string,
 locale: SupportedLocale,
): Promise<PdfExportResult> {
 return invoke<PdfExportResult>("export_report_pdf", {
  request,
  filePath: file_path,
  locale,
 });
}

// ─── Tauri dialog helpers (Slice 11b) ────────────────────────────────────

/** Default file filters for PDF report files. */
const PDF_FILTERS = [
 { name: "PDF files", extensions: ["pdf"] },
 { name: "All files", extensions: ["*"] },
];

/**
 * Opens a native "save file" dialog scoped to PDF files and returns the
 * chosen destination path, or `null` when the user cancels. The dialog
 * suggests the given default filename (without extension) and adds a `.pdf`
 * extension automatically.
 *
 * Lives in `lib/reports.ts` rather than `lib/csv.ts` because `lib/csv.ts`
 * is outside Slice 11b’s allowed edit surfaces.
 */
export async function pickPdfSavePath(
 defaultName: string = "caduxo-report",
 title: string = "Save PDF report",
): Promise<string | null> {
 const path = await saveDialog({
  defaultPath: `${defaultName}.pdf`,
  filters: PDF_FILTERS,
  title,
 });
 return path ?? null;
}

/**
 * High-level "export PDF" flow: opens a native save dialog and writes the
 * report PDF. Returns `null` when the user cancels the dialog.
 */
export async function exportReportPdfWithDialog(
 request: ReportRequest,
 locale: SupportedLocale,
): Promise<PdfExportResult | null> {
 const path = await pickPdfSavePath(`caduxo-${request.kind}`);
 if (!path) return null;
 return exportReportPdf(request, path, locale);
}
