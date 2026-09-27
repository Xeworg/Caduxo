/**
 * TypeScript API wrapper for stock-out reasons (catalog-driven exit reasons).
 *
 * These reasons are stored in a database catalog and support soft-delete
 * (archival) rather than hard deletion. The frontend displays active reasons
 * in the stock-out dropdown and stores an immutable snapshot of the selected
 * reason's display name in each movement record for audit trail integrity.
 *
 * Rust commands (Tauri IPC):
 *   list_stock_out_reasons       → active reasons only
 *   list_all_stock_out_reasons   → all including archived
 *   get_stock_out_reason         → single by id
 *   create_stock_out_reason      → creates with sort_order
 *   rename_stock_out_reason      → renames display_name (movement_kind is immutable)
 *   archive_stock_out_reason     → archives (soft-delete)
 *   unarchive_stock_out_reason   → restores archived reason
 */

import { invoke } from "@tauri-apps/api/core";

// ─── DTOs ───────────────────────────────────────────────────────────────────

/**
 * The seven V20 stock-out movement kinds (exit:sale excluded by design scope).
 * These are the valid values for StockOutReason.movement_kind.
 */
export type ExitReasonMovementKind =
  | "exit:waste"
  | "exit:expired"
  | "exit:damaged"
  | "exit:internal_consumption"
  | "exit:return_to_supplier"
  | "exit:inventory_adjustment"
  | "exit:other";

/** A stock-out reason catalog entry. */
export interface StockOutReason {
  /** Stable identifier (seeded sor-* or custom). */
  id: string;
  /** Human-readable display name (e.g., "Vencido", "Dañado"). */
  display_name: string;
  /** The V20 stock-out movement kind (e.g. exit:waste). */
  movement_kind: string;
  /** Sort order within movement kind group. */
  sort_order: number;
  /**
   * Null when active; timestamp when archived.
   * Use `archived_at === null` to determine active status (NOT is_active).
   */
  archived_at: string | null;
  /** Creation timestamp. */
  created_at: string;
  /** Last update timestamp. */
  updated_at: string;
}

/** Input for creating a new stock-out reason. */
export interface StockOutReasonCreate {
  display_name: string;
  movement_kind: ExitReasonMovementKind;
  /** Optional sort order override; defaults to 0. */
  sort_order?: number;
}

/** Input for renaming a stock-out reason's display_name.
 *  The movement_kind is immutable and cannot be changed after creation.
 */
export interface StockOutReasonRename {
  id: string;
  display_name: string;
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/**
 * Lists all active (non-archived) stock-out reasons ordered by sort_order.
 * Use this for dropdowns where only selectable reasons are needed.
 *
 * @returns Array of active stock-out reason catalog entries
 */
export async function listStockOutReasons(): Promise<StockOutReason[]> {
  return invoke<StockOutReason[]>("list_stock_out_reasons");
}

/**
 * Lists all stock-out reasons including archived ones, ordered by sort_order.
 * Use this when the UI needs to display archived reasons as immutable
 * snapshots in historical records.
 *
 * @returns Array of all stock-out reason catalog entries
 */
export async function listAllStockOutReasons(): Promise<StockOutReason[]> {
  return invoke<StockOutReason[]>("list_all_stock_out_reasons");
}

/**
 * Gets a single stock-out reason by its ID.
 *
 * @param id - Reason ID
 * @returns The stock-out reason or null if not found
 */
export async function getStockOutReason(id: string): Promise<StockOutReason | null> {
  return invoke<StockOutReason | null>("get_stock_out_reason", { id });
}

/**
 * Creates a new stock-out reason.
 *
 * @param input - Reason creation input (display_name, movement_kind, optional sort_order)
 * @returns The created stock-out reason
 */
export async function createStockOutReason(
  input: StockOutReasonCreate,
): Promise<StockOutReason> {
  return invoke<StockOutReason>("create_stock_out_reason", { input });
}

/**
 * Renames a stock-out reason's display_name.
 * The movement_kind is immutable and cannot be changed after creation.
 *
 * @param input - Rename input (id, display_name)
 * @returns The updated stock-out reason
 */
export async function renameStockOutReason(
  input: StockOutReasonRename,
): Promise<StockOutReason> {
  return invoke<StockOutReason>("rename_stock_out_reason", { input });
}

/**
 * Archives (soft-deletes) a stock-out reason.
 * Archived reasons are no longer selectable but remain visible as
 * immutable snapshots in historical movement records.
 *
 * @param id - Reason ID to archive
 */
export async function archiveStockOutReason(id: string): Promise<void> {
  return invoke<void>("archive_stock_out_reason", { id });
}

/**
 * Restores an archived stock-out reason to active status.
 *
 * @param id - Reason ID to unarchive
 * @returns The restored stock-out reason
 */
export async function unarchiveStockOutReason(id: string): Promise<StockOutReason> {
  return invoke<StockOutReason>("unarchive_stock_out_reason", { id });
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Determines if a StockOutReason is active (not archived).
 * An active reason can be selected in the UI dropdown.
 * An archived reason (archived_at !== null) is shown as immutable
 * snapshot text in movement history but is not offered in dropdowns.
 */
export function isReasonActive(reason: StockOutReason): boolean {
  return reason.archived_at === null;
}

/**
 * Returns active reasons filtered from a list of all reasons.
 * Use this when the UI needs only selectable reasons.
 */
export function filterActiveReasons(reasons: StockOutReason[]): StockOutReason[] {
  return reasons.filter(isReasonActive);
}

/**
 * Returns archived reasons filtered from a list of all reasons.
 * Use this for displaying archived reasons as disabled options when
 * they are selected in historical records.
 */
export function filterArchivedReasons(reasons: StockOutReason[]): StockOutReason[] {
  return reasons.filter((r) => !isReasonActive(r));
}
