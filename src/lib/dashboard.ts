/**
 * TypeScript API wrapper for Caduxo Tauri commands (Slice 6a — dashboard).
 * Thin wrappers around the Rust command layer; no business logic here.
 */

import { invoke } from "@tauri-apps/api/core";
import type { UnitKind } from "./products.js";

// ─── DTOs ─────────────────────────────────────────────────────────────────────

/** Quick-filter preset for the dashboard lot table. */
export type DashboardPreset =
    | "expired"
    | "today"
    | "alert_window"
    | "next_7_days"
    | "next_30_days"
    | "all";

/** Optional filters for the dashboard lot query. */
export interface DashboardFilters {
    /** Return only lots in this store. */
    store_id?: string | null;
    /** Return only lots in this location. */
    location_id?: string | null;
    /** Quick-filter preset. */
    preset?: DashboardPreset | null;
    /** Explicit urgency filter (overrides preset when set). */
    urgency?: string | null;
    /** Filter by category — any-of semantics via the `product_categories` junction (V4).
     * `None` or empty list = no filter. May include `UNCATEGORIZED_SENTINEL` to include
     * products with zero active category memberships. */
    category_ids?: string[] | null;
}

/** Urgency bucket counts shown on the urgency cards. */
export interface UrgencyCounts {
    expired: number;
    today: number;
    alert_window: number;
    next_30_days: number;
}

/** A single row in the dashboard lot table. */
export interface DashboardLotRow {
    lot_id: string;
    product_id: string;
    sku: string;
    description: string;
    store_id: string;
    store_name: string;
    location_id: string | null;
    location_name: string | null;
    quantity: number;
    unit: string;
    expiry_date: string;
    alert_days_before: number;
    batch_code: string | null;
    status: string;
    /** Urgency classification: "expired" | "today" | "alert_window" | "next_30_days" | "future" */
    urgency: string;
    /** Days remaining until expiry (negative for expired lots). */
    days_remaining: number;
    /** FK into `unit_definitions`; present when the product has a catalog link. */
    default_unit_id: string | null;
    /** Kind from the linked unit; drives LotForm quantity input rules. */
    unit_type: UnitKind | null;
}

/** Full dashboard response bundle. */
export interface DashboardResponse {
    counts: UrgencyCounts;
    lots: DashboardLotRow[];
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/**
 * Returns urgency counts and a sorted, urgency-filtered lot table.
 *
 * Filters:
 *   - store_id / location_id → SQL filter
 *   - preset or urgency → in-memory urgency filter
 *
 * Sorting: urgency-first (expired → today → alert_window → next_30_days →
 *          future), then by expiry_date ASC within each group.
 */
export async function listDashboardLots(
    filters: DashboardFilters = {},
): Promise<DashboardResponse> {
    return invoke<DashboardResponse>("list_dashboard_lots", { filters });
}
