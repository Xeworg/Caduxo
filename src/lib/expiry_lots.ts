/**
 * TypeScript API wrapper for Caduxo Tauri commands (Slice 5a — expiry lots).
 * Thin wrappers around the Rust command layer; no business logic here.
 */

import { invoke } from "@tauri-apps/api/core";
import type { UnitKind } from "./products.js";

// ─── DTOs ────────────────────────────────────────────────────────────────────

export interface ExpiryLotCreate {
  product_id: string;
  store_id: string;
  location_id?: string | null;
  quantity: number;
  unit?: string | null;
  expiry_date: string;
  alert_days_before?: number | null;
  batch_code?: string | null;
  notes?: string | null;
}

export interface ExpiryLotUpdate {
  id: string;
  location_id?: string | null;
  quantity: number;
  unit: string;
  expiry_date: string;
  alert_days_before: number;
  batch_code?: string | null;
  notes?: string | null;
}

export interface ExpiryLotResolve {
  lot_id: string;
  quantity: number;
  resolution: string;
  notes?: string | null;
}

export interface ExpiryLotResponse {
  id: string;
  product_id: string;
  store_id: string;
  location_id: string | null;
  quantity: number;
  unit: string;
  expiry_date: string;
  alert_days_before: number;
  batch_code: string | null;
  status: string;
  resolution: string | null;
  resolved_at: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
  /**
   * Unit kind resolved from the product's catalog link.
   * `null` for legacy / uncatalogued products — treated as decimal by the UI.
   * Drives dynamic min/step/validation rules in movement modals.
   */
  unit_type: UnitKind | null;
}

export interface LotResolutionEventResponse {
  id: string;
  expiry_lot_id: string;
  quantity: number;
  resolution: string;
  notes: string | null;
  created_at: string;
}

export interface ExpiryLotResolveResult {
  lot_id: string;
  resolved_quantity: number;
  remaining_quantity: number;
  is_fully_resolved: boolean;
  resolution_event_id: string;
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/** Returns all active expiry lots across all stores, ordered by expiry date. */
export async function listExpiryLots(): Promise<ExpiryLotResponse[]> {
  return invoke<ExpiryLotResponse[]>("list_expiry_lots");
}

/** Returns all active expiry lots for a given store. */
export async function listExpiryLotsByStore(
  storeId: string,
): Promise<ExpiryLotResponse[]> {
  return invoke<ExpiryLotResponse[]>("list_expiry_lots_by_store", {
    storeId,
  });
}

/** Returns all active expiry lots for a given product, ordered by expiry date. */
export async function listExpiryLotsByProduct(
  productId: string,
): Promise<ExpiryLotResponse[]> {
  return invoke<ExpiryLotResponse[]>("list_expiry_lots_by_product", {
    productId,
  });
}

/** Returns a single expiry lot by id. */
export async function getExpiryLot(id: string): Promise<ExpiryLotResponse> {
  return invoke<ExpiryLotResponse>("get_expiry_lot", { id });
}

/** Creates a new expiry lot. */
export async function createExpiryLot(
  input: ExpiryLotCreate,
): Promise<ExpiryLotResponse> {
  return invoke<ExpiryLotResponse>("create_expiry_lot", { input });
}

/** Updates an existing active expiry lot. */
export async function updateExpiryLot(
  input: ExpiryLotUpdate,
): Promise<ExpiryLotResponse> {
  return invoke<ExpiryLotResponse>("update_expiry_lot", { input });
}

/** Soft-archives an expiry lot (status = 'archived'). */
export async function archiveExpiryLot(id: string): Promise<void> {
  return invoke<void>("archive_expiry_lot", { id });
}

/**
 * Resolves (consumes, discards, or transfers) a quantity from a lot.
 * Records a resolution event and marks the lot as fully resolved when
 * remaining quantity reaches zero.
 */
export async function resolveExpiryLot(
  input: ExpiryLotResolve,
): Promise<ExpiryLotResolveResult> {
  return invoke<ExpiryLotResolveResult>("resolve_expiry_lot", { input });
}

/** Returns all resolution events for a given lot, newest first. */
export async function listLotResolutionEvents(
  lotId: string,
): Promise<LotResolutionEventResponse[]> {
  return invoke<LotResolutionEventResponse[]>("list_lot_resolution_events", {
    lotId,
  });
}
