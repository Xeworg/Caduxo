/**
 * TypeScript API wrapper for Caduxo Tauri commands (Slice 5a — expiry lots).
 * Thin wrappers around the Rust command layer; no business logic here.
 */

import { invoke } from "@tauri-apps/api/core";
import type { UnitKind } from "./products.js";
import {
   DEFAULT_LOCALE,
   locale as activeLocale,
   type SupportedLocale,
} from "../i18n/locale.svelte.js";

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

/**
 * Allow-listed archive reason codes. Mirrors the backend
 * `VALID_ARCHIVE_REASONS` constant in `src-tauri/src/services/expiry_lots.rs`.
 * Keep both in sync when adding new reasons.
 */
export interface ArchiveReasonOption {
  value: string;
  label: string;
}

export const ARCHIVE_REASONS: readonly ArchiveReasonOption[] = [
  { value: "expired_unsold", label: "Expired (unsold)" },
  { value: "damaged", label: "Damaged" },
  { value: "returned_to_supplier", label: "Returned to supplier" },
  { value: "recall", label: "Manufacturer recall" },
  { value: "lost", label: "Lost / unaccounted" },
  { value: "internal_use", label: "Internal use" },
  { value: "administrative", label: "Administrative cleanup" },
  { value: "other", label: "Other" },
] as const;

/** Minimum trimmed length required for archive `notes` (server-side rule). */
export const ARCHIVE_NOTES_MIN_CHARS = 5;
/** Maximum trimmed length allowed for archive `notes` (server-side rule). */
export const ARCHIVE_NOTES_MAX_CHARS = 1000;

/** Input for archiving an active expiry lot with a required justification. */
export interface ArchiveLotInput {
  id: string;
  reason: string;
  notes: string;
}

/**
 * One (location_id, quantity) pair within a distributed lot creation.
 *
 * Allocations are applied atomically by `createExpiryLotDistributed`.
 * The backend rejects empty arrays, duplicate `location_id`s,
 * quantities `<= 0`, fractional quantities for integer-unit products,
 * and locations that don't belong to the lot's `store_id` or are
 * inactive. The frontend should mirror these rules in the form layer
 * so the user gets inline feedback before the IPC round-trip.
 */
export interface ExpiryLotAllocationInput {
  location_id: string;
  quantity: number;
}

/**
 * Input for creating one expiry lot distributed across multiple active
 * locations in the same store. Mirrors
 * `dto::expiry_lots::ExpiryLotDistributedCreate` in the Rust service.
 *
 * `total_quantity` is the explicit lot total and must equal the sum of
 * allocation quantities (within a tiny float tolerance); a mismatch is
 * rejected at the IPC boundary with `DistributionTotalMismatch`.
 */
export interface ExpiryLotDistributedCreate {
  product_id: string;
  store_id: string;
  total_quantity: number;
  allocations: ExpiryLotAllocationInput[];
  unit?: string | null;
  expiry_date: string;
  alert_days_before?: number | null;
  batch_code?: string | null;
  notes?: string | null;
}

/**
 * Result of a distributed lot creation. `lot` is the persisted expiry
 * lot; `anchor_location_id` is the location that received the single
 * `entry:initial` movement (i.e. the first allocation); the per-location
 * ledger can be re-read through `getLotLocationBalances`.
 */
export interface ExpiryLotDistributedCreateResult {
  lot: ExpiryLotResponse;
  anchor_location_id: string;
  allocation_count: number;
}

export interface ExpiryLotResponse {
  id: string;
  product_id: string;
  store_id: string;
  location_id: string | null;
  /**
   * Human-readable location name projected from `store_locations.name` via
   * the backend `LEFT JOIN` (see `odd/tasks/lot-location-name.md`).
   * `null` when the lot has no `location_id` or the row is missing;
   * inactive locations are still included so historic lots stay
   * interpretable. UI display helpers consume this to render the name
   * instead of the raw `location_id`; treat `location_id` as the
   * authoritative relationship key and `location_name` as a presentational
   * projection that may be absent on legacy rows.
   */
  location_name: string | null;
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

/**
 * Returns the active UI locale, used as the implicit default for the
 * `locale` parameter accepted by every backend-call wrapper in this file.
 *
 * Callers that already hold a `SupportedLocale` can pass it explicitly to
 * avoid the rune read at call time; otherwise the wrapper reads
 * `activeLocale.current` and falls back to `DEFAULT_LOCALE` while the rune
 * is still uninitialised.
 */
function resolveLocale(locale?: SupportedLocale): SupportedLocale {
   if (locale) return locale;
   const current = activeLocale?.current;
   return (current ?? DEFAULT_LOCALE) as SupportedLocale;
}

/**
 * Creates a new expiry lot.
 *
 * `locale` is forwarded to the Rust command so the backend can localise
 * validation errors (notably `LocationRequired`) before they reach the UI.
 * When omitted, the wrapper uses the active UI locale.
 */
export async function createExpiryLot(
  input: ExpiryLotCreate,
  locale?: SupportedLocale,
): Promise<ExpiryLotResponse> {
  return invoke<ExpiryLotResponse>("create_expiry_lot", {
    input,
    locale: resolveLocale(locale),
  });
}

/**
 * Creates one expiry lot with the initial quantity distributed across
 * multiple active locations in the same store, atomically.
 *
 * The backend writes one `entry:initial` movement for the full lot total
 * to the first allocation's location, then one `transfer` per remaining
 * allocation, all in a single DB transaction. Either every write
 * commits or every write rolls back — there is no partial commit path.
 *
 * `locale` is forwarded to the Rust command so the backend can localise
 * the new distributed-creation Validation / BusinessRule messages
 * (empty distribution, duplicate location, total mismatch, non-store or
 * inactive location, fractional-quantity rejection for integer-unit
 * products) before they reach the UI. When omitted, the wrapper uses
 * the active UI locale.
 */
export async function createExpiryLotDistributed(
  input: ExpiryLotDistributedCreate,
  locale?: SupportedLocale,
): Promise<ExpiryLotDistributedCreateResult> {
  return invoke<ExpiryLotDistributedCreateResult>("create_expiry_lot_distributed", {
    input,
    locale: resolveLocale(locale),
  });
}

/** Updates an existing active expiry lot. */
export async function updateExpiryLot(
  input: ExpiryLotUpdate,
): Promise<ExpiryLotResponse> {
  return invoke<ExpiryLotResponse>("update_expiry_lot", { input });
}

/**
 * Soft-archives an active expiry lot with a required justification.
 *
 * The Rust command persists the archive reason + notes in `lot_movements`
 * as an `exit:other` marker in the same transaction as flipping the lot
 * status to `archived`. Pass `notes` as trimmed text — the backend will
 * also enforce trimming, a minimum of 5 characters, and a cap of 1000.
 */
export async function archiveExpiryLot(input: ArchiveLotInput): Promise<void> {
  return invoke<void>("archive_expiry_lot", { input });
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
