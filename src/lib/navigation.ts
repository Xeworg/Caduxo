/**
 * In-memory shell-navigation channel for the Dashboard → Scanner flow
 * (ODD task 7 of `feat/scanner-first-inventory`).
 *
 * Dashboard writes a `ScannerNavigationRequest` here; App.svelte reacts
 * by switching the shell to the Scanner tab; ScannerPage.svelte consumes
 * the request, resolves the lot and product, populates `pinnedContext`,
 * and clears the request.
 *
 * No URL router, no backend, no persisted state. The request is cleared
 * as soon as Scanner consumes it.
 */

import { writable } from "svelte/store";
import type { UnitKind } from "./products.js";

export type ScannerNavigationRequest = {
  /** Lot to open in Scanner lot context. Set for known products with lots. */
  lotId: string;
  /** Product context for lot-context or product-only navigation. */
  productId: string;
  /** Unit type from the lot or product. */
  unitType: UnitKind | null;
  /**
   * Scanned value to seed the product form when opening quick-create.
   * Used for product-only navigation (no lot) and unknown scans.
   * Omit when navigating with a lot context (quick-create is not opened).
   */
  scannedValue?: string;
};

export type ScannerNavigationStore = {
  request: ScannerNavigationRequest | null;
};

/** Shared writable store — written by Dashboard, consumed by App + Scanner. */
export const scannerNavigation = writable<ScannerNavigationRequest | null>(null);

/**
 * Clears the pending navigation request after Scanner has consumed it.
 * Called from ScannerPage.svelte.
 */
export function clearScannerNavigation(): void {
  scannerNavigation.set(null);
}
