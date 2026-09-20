/**
 * TypeScript API wrapper for Caduxo lot movement commands.
 * Thin wrappers around the Rust command layer; no business logic here.
 */

import { invoke } from "@tauri-apps/api/core";
import type { Locales } from "../i18n/i18n-types.js";

// ─── DTOs ────────────────────────────────────────────────────────────────────

/** Direction for inventory_adjustment movements. */
export type MovementDirection = "increase" | "decrease";

/** Movement kind values. */
export type MovementKind =
 | "entry:initial"
 | "transfer"
 | "exit:sale"
 | "exit:waste"
 | "exit:expired"
 | "exit:damaged"
 | "exit:internal_consumption"
 | "exit:return_to_supplier"
 | "exit:inventory_adjustment"
 | "exit:other"
 | "inventory_adjustment";

/** Input for creating a new lot movement. */
export interface LotMovementCreate {
 lot_id: string;
 /** Movement kind (e.g., "transfer", "exit:sale", "inventory_adjustment"). */
 kind: string;
 /** Optional direction for `inventory_adjustment` kind. */
 direction?: MovementDirection | null;
 /** Quantity being moved (always positive magnitude). */
 quantity: number;
 /** Source location for exits and transfers. */
 source_location_id?: string | null;
 /** Destination location for entries and transfers. */
 destination_location_id?: string | null;
 /** Optional notes (required for some kinds). */
 notes?: string | null;
}

/** Full movement response row. */
export interface LotMovementResponse {
 id: string;
 expiry_lot_id: string;
 movement_kind: string;
 direction: string | null;
 quantity: number;
 source_location_id: string | null;
 destination_location_id: string | null;
 reason: string | null;
 notes: string | null;
 actor: string;
 created_at: string;
}

/** Per-location balance for a lot. */
export interface LotLocationBalance {
 location_id: string;
 balance: number;
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/**
 * Creates a new lot movement and updates the lot total atomically.
 *
 * @param input - Movement creation input
 * @param locale - Active BCP-47 locale tag (e.g. `"en"`, `"es"`, `"es-MX"`)
 *   forwarded to the Rust command so user-facing Validation and
 *   BusinessRule messages reach the UI in the active locale. Unknown tags
 *   fall back to English at the backend. Omitting the argument preserves
 *   the previous English-only behaviour.
 * @returns The created movement response
 */
export async function createLotMovement(
 input: LotMovementCreate,
 locale?: Locales,
): Promise<LotMovementResponse> {
 return invoke<LotMovementResponse>("create_lot_movement", { input, locale });
}

/**
 * Lists all movements for a lot, newest first.
 *
 * @param lotId - The lot ID to list movements for
 * @returns Array of movement responses
 */
export async function listLotMovements(
 lotId: string,
): Promise<LotMovementResponse[]> {
 return invoke<LotMovementResponse[]>("list_lot_movements", { lotId });
}

/**
 * Returns per-location balances for a lot.
 *
 * @param lotId - The lot ID to get balances for
 * @returns Array of location balances
 */
export async function getLotLocationBalances(
 lotId: string,
): Promise<LotLocationBalance[]> {
 return invoke<LotLocationBalance[]>("get_lot_location_balances", { lotId });
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/**
 * Formats a movement quantity with sign prefix for inventory_adjustment.
 */
export function formatMovementQuantity(
 quantity: number,
 kind: string,
 direction?: string | null,
): string {
 if (kind === "inventory_adjustment" && direction) {
  return direction === "increase" ? `+${quantity}` : `−${quantity}`;
 }
 return `${quantity}`;
}
