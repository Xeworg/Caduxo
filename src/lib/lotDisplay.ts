/**
 * Pure helpers for displaying lot/location metadata to the user.
 *
 * Sentinel location ids (`loc-sentinel-<store_id>`) are internal markers
 * produced by the backend to represent stock with no assigned location
 * (per `sentinel location for unassigned stock` requirement in the
 * expiry-tracker spec). They MUST NEVER appear raw in the UI. Every
 * display path that surfaces a `location_id` should go through
 * `resolveLocationDisplay` with a caller-supplied localized placeholder
 * for the no-location case.
 *
 * Batch codes are scannable identifiers (see `expiry_lots.batch_code`).
 * When a lot has no batch code we must show a localized placeholder
 * rather than falling back to the location id.
 */

/** Structural location shape — accepts any record with `id` and `name`. */
export interface LocationRef {
    readonly id: string;
    readonly name: string;
}

/** Prefix that identifies a per-store sentinel location id. */
const SENTINEL_LOCATION_ID_PREFIX = "loc-sentinel-";

/**
 * True when the location id is a per-store sentinel. Sentinel ids are
 * reserved by the backend to represent stock without an assigned
 * location and must never be rendered as raw ids in the UI.
 */
export function isSentinelLocationId(locationId: string | null | undefined): boolean {
    if (!locationId) return false;
    return locationId.startsWith(SENTINEL_LOCATION_ID_PREFIX);
}

/**
 * Display label for a location reference.
 *
 * - `null` / `undefined` / empty → `noLocationLabel`.
 * - Sentinel id → `noLocationLabel` (regardless of `locations`).
 * - Known id in `locations` → the matching `name`.
 * - Unknown id → the raw id so a misconfigured row stays inspectable
 *   instead of being silently replaced by the placeholder.
 */
export function resolveLocationDisplay<L extends LocationRef>(
    locationId: string | null | undefined,
    locations: ReadonlyArray<L>,
    noLocationLabel: string,
): string {
    if (!locationId) return noLocationLabel;
    if (isSentinelLocationId(locationId)) return noLocationLabel;
    const match = locations.find((l) => l.id === locationId);
    return match?.name ?? locationId;
}

/**
 * Display label for a batch code.
 *
 * - `null` / `undefined` / empty / whitespace-only → `noBatchCodeLabel`.
 * - Otherwise → the trimmed batch code verbatim (it is a scannable
 *   identifier, so callers must preserve it byte-for-byte).
 */
export function resolveBatchCodeDisplay(
    batchCode: string | null | undefined,
    noBatchCodeLabel: string,
): string {
    if (batchCode === null || batchCode === undefined) return noBatchCodeLabel;
    const trimmed = batchCode.trim();
    if (trimmed === "") return noBatchCodeLabel;
    return trimmed;
}
