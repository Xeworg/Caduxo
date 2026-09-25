/**
 * Shared movement-client rules for presentation and validation.
 *
 * Factored out of `MoveStockModal`, `RegisterExitModal`, `AdjustCountModal`,
 * `LotMovementsPanel`, `LotContextPanel`, and `ScannerPage` so that duplicated
 * movement facts (exit-kind labels, note-required exits, integer/decimal unit
 * validation, quantity-input HTML attributes, and movement-kind labels) stay
 * in one place and never drift apart.
 *
 * All exported helpers are pure functions; the module has no side-effects.
 * i18n strings are consumed but not imported — callers pass resolved strings
 * or string-factories from the active `$LL` scope.
 */

import type { UnitKind } from "./products.js";
import type { TranslationFunctions } from "../i18n/i18n-types.js";

// ── Exit reasons ───────────────────────────────────────────────────────────────

/** The eight exit-kind codes recognised by the movement ledger. */
/** The eight exit-kind codes recognised by the movement ledger. */
export const EXIT_KINDS = [
  "exit:sale",
  "exit:waste",
  "exit:expired",
  "exit:damaged",
  "exit:internal_consumption",
  "exit:return_to_supplier",
  "exit:inventory_adjustment",
  "exit:other",
] as const;

export type ExitKind = (typeof EXIT_KINDS)[number];

/**
 * Exit kinds that require a non-empty notes field on submission.
 * Mirrors `RegisterExitModal.REQUIRES_NOTES`.
 */
export const NOTE_REQUIRED_EXITS: readonly ExitKind[] = [
  "exit:inventory_adjustment",
  "exit:other",
] as const;

/** True when the given exit-kind requires notes. */
export function exitRequiresNotes(kind: ExitKind | string): boolean {
  return NOTE_REQUIRED_EXITS.includes(kind as ExitKind);
}

// ── Integer / decimal unit helpers ────────────────────────────────────────────

/**
 * HTML `min` attribute value for a quantity input.
 * Integers start at 1 (no zero-quantity movements); decimals allow fractional.
 */
export function qtyMin(isIntegerUnit: boolean): number {
  return isIntegerUnit ? 1 : 0;
}

/**
 * HTML `step` attribute value for a quantity input.
 * Matches the unit precision so the browser enforces the same constraint.
 */
export function qtyStep(isIntegerUnit: boolean): number {
  return isIntegerUnit ? 1 : 0.01;
}

/**
 * HTML `inputmode` attribute for a quantity input.
 */
export function qtyInputMode(isIntegerUnit: boolean): "numeric" | "decimal" {
  return isIntegerUnit ? "numeric" : "decimal";
}

/**
 * True when `qty` is a fractional value that an integer-unit product
 * cannot represent.
 *
 * Guards the UI before submission so the user gets immediate feedback
 * instead of a round-trip validation error. The backend enforces the
 * same invariant; this avoids the unnecessary call.
 */
export function isFractionalForIntegerUnit(
  qty: number,
  isIntegerUnit: boolean,
): boolean {
  if (!isIntegerUnit) return false;
  if (qty <= 0) return false;
  return !Number.isInteger(qty);
}

/**
 * All unit-aware HTML quantity-input attributes in one call.
 * Suitable for spreading onto `<input type="number">`:
 *
 * ```svelte
 * <input
 *   type="number"
 *   min={qtyAttrs(isIntegerUnit).min}
 *   step={qtyAttrs(isIntegerUnit).step}
 *   inputmode={qtyAttrs(isIntegerUnit).inputmode}
 *   max={maxQuantity}
 *   bind:value={quantity}
 * />
 * ```
 */
export function qtyAttrs(
  isIntegerUnit: boolean,
): {
  min: number;
  step: number;
  inputmode: "numeric" | "decimal";
} {
  return {
    min: qtyMin(isIntegerUnit),
    step: qtyStep(isIntegerUnit),
    inputmode: qtyInputMode(isIntegerUnit),
  };
}

// ── Location helpers ──────────────────────────────────────────────────────────

/** Shape accepted by `locationOption` and `locationLabel`. */
export interface LocationWithStore {
  readonly id: string;
  readonly name: string;
  readonly store_id: string;
  readonly store_name?: string;
}

/**
 * Formatted display label for a location.
 * When `store_name` is present the result is `"Store / Location"`.
 */
export function locationDisplayLabel(loc: LocationWithStore): string {
  return loc.store_name ? `${loc.store_name} / ${loc.name}` : loc.name;
}

// ── Movement kind labels ─────────────────────────────────────────────────────

/**
 * Record mapping movement-kind strings to their display-label factories.
 * Each value is a zero-arg i18n function so the caller provides the active
 * `$LL` scope without coupling this module to the i18n system.
 *
 * Consumed by `LotMovementsPanel.getMovementKindLabel` and
 * `LotContextPanel.getMovementKindLabel`.
 */
export type MovementKindLabelRecord = Record<
  string,
  (() => string) | string | undefined
>;



/**
 * Builds a `MovementKindLabelRecord` from an i18n scope.
 * Each key is the movement kind string; each value is the resolved
 * i18n string or a function that returns it lazily.
 */
export function buildMovementKindLabelRecord(
  LL: TranslationFunctions,
): MovementKindLabelRecord {
  return {
    "entry:initial": LL.lotMovements.movementKinds.initialEntry,
    transfer: LL.lotMovements.movementKinds.transfer,
    "exit:sale": LL.lotMovements.exitReasons.sale,
    "exit:waste": LL.lotMovements.exitReasons.waste,
    "exit:expired": LL.lotMovements.exitReasons.expired,
    "exit:damaged": LL.lotMovements.exitReasons.damaged,
    "exit:internal_consumption": LL.lotMovements.exitReasons.internalConsumption,
    "exit:return_to_supplier": LL.lotMovements.exitReasons.returnToSupplier,
    "exit:inventory_adjustment":
      LL.lotMovements.exitReasons.inventoryAdjustmentExit,
    "exit:other": LL.lotMovements.exitReasons.other,
    inventory_adjustment: LL.lotMovements.movementKinds.inventoryAdjustment,
  };
}

/**
 * Resolves a movement-kind + optional direction to a human-readable label.
 *
 * `inventory_adjustment` with a direction is the only kind that changes
 * label based on direction; all others use the kind label verbatim.
 */
export function getMovementKindLabel(
  kind: string,
  direction: string | null | undefined,
  labels: MovementKindLabelRecord,
): string {
  const raw = labels[kind];
  if (typeof raw === "function") {
    if (kind === "inventory_adjustment" && direction) {
      // Direction-specific labels are caller-provided via the same
      // record under synthetic keys so the generic record stays flat.
      const dirLabel = direction === "increase"
        ? labels["inventory_adjustment:increase"]
        : labels["inventory_adjustment:decrease"];
      if (typeof dirLabel === "function") return dirLabel();
      return raw();
    }
    return raw();
  }
  return raw ?? kind;
}
