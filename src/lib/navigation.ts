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

/**
 * Sentinel value that signals ScannerPage to clear its pinned lot context
 * when the user leaves the Scanner tab (ODD task 11).
 * App.svelte sets this when `previousTab === "scanner" && currentTab !== "scanner"`;
 * ScannerPage.svelte watches for it and clears `pinnedContext`.
 */
export const SCANNER_EXIT_SIGNAL = "__scanner_exit__" as const;
export type ScannerExitSignal = typeof SCANNER_EXIT_SIGNAL;
export type ScannerNavigationValue =
  | ScannerNavigationRequest
  | ScannerExitSignal
  | null;

export type ScannerNavigationStore = {
  request: ScannerNavigationValue;
};

/**
 * Shared writable store.
 * - Written by Dashboard with a `ScannerNavigationRequest` to open Scanner.
 * - Written by App.svelte with `SCANNER_EXIT_SIGNAL` when the user leaves Scanner.
 * - Consumed by App.svelte (triggers tab switch) and ScannerPage.svelte
 *   (resolves lot context or clears pinned state).
 */
export const scannerNavigation =
  writable<ScannerNavigationValue>(null);

/**
 * Clears the pending navigation request after Scanner has consumed it.
 * Called from ScannerPage.svelte after resolving a Dashboard-initiated request.
 */
export function clearScannerNavigation(): void {
  scannerNavigation.set(null);
}
