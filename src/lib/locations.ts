/**
 * Shared active-store-location loader.
 *
 * Factored out of `ScannerPage`, `DashboardPage`, and `CalendarPage` so that
 * the multi-store location hydration pattern (all active stores, all their
 * active locations, enriched with `store_name`) stays in one place.
 *
 * The returned shape is a flat array of location records, each carrying the
 * owning store's name. This is the canonical form required by `MoveStockModal`
 * and `LotMovementsPanel` for cross-store transfer destination selection.
 */

import { listStores, listStoreLocations } from "./stores.js";
import type { StoreResponse, StoreLocationResponse } from "./stores.js";

/** Shape of a location enriched with its owning store's display name. */
export interface EnrichedLocation {
  readonly id: string;
  readonly name: string;
  readonly store_id: string;
  readonly store_name: string;
}

/**
 * Result of `loadAllActiveStoreLocations`.
 * The `stores` array is the full active-store list (for display purposes);
 * `allLocations` is the flat enriched location list described above.
 */
export interface ActiveStoreLocations {
  readonly stores: StoreResponse[];
  readonly allLocations: EnrichedLocation[];
}

/**
 * Loads all active stores and all active locations across every active store,
 * enriching each location with its owning store's `name`.
 *
 * Used to:
 * - Hydrate the `allLocations` prop of `MoveStockModal` so cross-store
 *   destinations are available without a separate round-trip.
 * - Build the `locationNameById` lookup that powers `resolveLocationDisplay`
 *   in `LotContextPanel` and `ScannerPage`.
 *
 * Errors fall back to empty arrays so callers that need the data handle the
 * error state rather than propagating the exception.
 */
export async function loadAllActiveStoreLocations(): Promise<ActiveStoreLocations> {
  const stores = await listStores();
  const activeStores = stores.filter((s) => s.is_active);

  const results = await Promise.all(
    activeStores.map((s) => listStoreLocations(s.id)),
  );

  const allLocations: EnrichedLocation[] = [];
  for (let i = 0; i < activeStores.length; i++) {
    const store = activeStores[i];
    for (const loc of results[i]) {
      if (loc.is_active) {
        allLocations.push({
          id: loc.id,
          name: loc.name,
          store_id: loc.store_id,
          store_name: store.name,
        });
      }
    }
  }

  return { stores, allLocations };
}
