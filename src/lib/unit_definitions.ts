/**
 * TypeScript wrappers for the unit definitions catalog Tauri commands
 * (caduxo-measurement-unit-options).
 */
import { invoke } from "@tauri-apps/api/core";
import type { UnitKind } from "./products.js";

// ─── DTOs ─────────────────────────────────────────────────────────────────────

/** A row from the `unit_definitions` catalog. */
export interface UnitDefinitionResponse {
 id: string;
 key: string;
 display_name: string;
 kind: UnitKind;
 is_preset: boolean;
 archived_at: string | null;
 created_at: string;
 updated_at: string;
}

/** Input for creating a new custom unit from ProductForm. */
export interface UnitDefinitionCreateInput {
 key: string;
 display_name: string;
 kind: UnitKind;
}

/** Input for renaming a unit's display_name (key is immutable in this slice). */
export interface UnitDefinitionRenameInput {
 id: string;
 display_name: string;
}

/** An unrecognized unit group surfaced in the audit banner / review page. */
export interface UnrecognizedUnitGroup {
 /** The raw text value from `products.default_unit` with no catalog link. */
 raw_value: string;
 /** How many products share this raw value. */
 product_count: number;
 /** One sample product id for UI linking. */
 sample_product_id: string;
}

/** Whether the unit-audit banner is currently visible. */
export interface UnitAuditBannerState {
 /** True when at least one unrecognized unit group exists and the stored
  *  signature does not match the current state. */
 show_banner: boolean;
 /** SHA-256 (truncated to 16 hex) of the sorted raw-value list. */
 current_signature: string;
 /** Number of distinct unrecognized raw values. */
 unrecognized_count: number;
}

// ─── Review actions ───────────────────────────────────────────────────────────

/** Discriminated review action from UnitReviewPage. */
export type UnitReviewAction =
 | { MapToPreset: { raw_value: string; preset_id: string } }
 | { KeepAsCustom: { raw_value: string; display_name: string; kind: UnitKind } }
 | { LeaveForLater: { raw_value: string } };

/** Result of applying a review action. */
export interface UnitReviewActionResult {
 /** How many products were updated by this action. */
 updated_count: number;
 /** The unit that was assigned (for MapToPreset / KeepAsCustom). */
 assigned_unit: UnitDefinitionResponse | null;
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/** Lists all active (non-archived) unit definitions ordered by kind then display_name. */
export async function listUnitDefinitions(): Promise<UnitDefinitionResponse[]> {
 return invoke<UnitDefinitionResponse[]>("list_unit_definitions");
}

/** Creates a new custom unit. Rejects duplicate keys (case-insensitive). */
export async function createUnitDefinition(
 input: UnitDefinitionCreateInput,
): Promise<UnitDefinitionResponse> {
 return invoke<UnitDefinitionResponse>("create_unit_definition", { input });
}

/** Renames a unit's display_name (key is immutable in this slice). */
export async function renameUnitDefinition(
 input: UnitDefinitionRenameInput,
): Promise<UnitDefinitionResponse> {
 return invoke<UnitDefinitionResponse>("rename_unit_definition", { input });
}

/** Returns all unrecognized unit groups: products with no catalog link but a
 *  non-empty `default_unit` text value. */
export async function listUnrecognizedUnits(): Promise<
 UnrecognizedUnitGroup[]
> {
 return invoke<UnrecognizedUnitGroup[]>("list_unrecognized_units");
}

/** Returns the banner visibility state. */
export async function unitAuditBannerState(): Promise<UnitAuditBannerState> {
 return invoke<UnitAuditBannerState>("unit_audit_banner_state");
}

/** Dismisses the audit banner by persisting the current signature. */
export async function dismissUnitAuditBanner(): Promise<void> {
 return invoke<void>("dismiss_unit_audit_banner");
}

/** Applies a review action from UnitReviewPage. */
export async function applyUnitReviewAction(
 action: UnitReviewAction,
): Promise<UnitReviewActionResult> {
 return invoke<UnitReviewActionResult>("apply_unit_review_action", { action });
}
