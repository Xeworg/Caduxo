/**
 * TypeScript API wrapper for the Scanner operation surface
 * (PR 2 of `scanner-quick-operations`).
 *
 * Thin typed wrapper around the Rust command
 * `resolve_scanner_code` introduced in PR 1. Mirrors the
 * discriminated `ScannerResolveResult` shape from
 * `src-tauri/src/dto/scanner.rs`.
 *
 * Contract (mirrored from the spec):
 *
 * 1. **Lot code** — exact match against `expiry_lots.batch_code`
 *    for active lots in the active store. Returns `LotMatch { lot,
 *    product }`. The matched lot is already selected — FEFO does
 *    NOT apply, and the FEFO policy (even `require_fefo`) MUST
 *    NOT replace the scanned lot with another lot.
 *
 * 2. **Product barcode** — exact match against
 *    `product_barcodes.barcode` for **active** products only
 *    (archived products are intentionally ignored — the Scanner
 *    is a stock-mutation path).
 *
 * 3. **Product SKU** — exact match against `products.sku` for
 *    active products only.
 *
 * 4. **Unknown** — neither step matched; the trimmed scanned
 *    value is preserved verbatim (no upper-casing, lower-casing,
 *    or whitespace normalisation) so the Registration mode can
 *    pre-fill the quick-create form.
 *
 * The dashboard scan/search surface continues to use
 * `find_product_by_scan`; the Scanner tab MUST NOT call that
 * command.
 */

import { invoke } from "@tauri-apps/api/core";
import type { ProductResponse } from "./products.js";
import type { ExpiryLotResponse } from "./expiry_lots.js";
import type { Locales } from "../i18n/i18n-types.js";

// ─── DTOs ────────────────────────────────────────────────────────────────────

/** Input for `resolve_scanner_code`. */
export interface ScannerResolveInput {
  /** Raw scanned value, exactly as the user submitted it. The command
   *  boundary trims leading/trailing whitespace before lookup; the
   *  trimmed value is preserved verbatim in `Unknown` results. */
  scanned_value: string;
}

/** Lot-code match — the scanned value resolved directly to one active lot. */
export interface ScannerLotMatch {
  match_type: "lot_match";
  /** The matched lot. */
  lot: ExpiryLotResponse;
  /** The lot's parent product (resolved in the same command). */
  product: ProductResponse;
}

/** Product match — barcode or SKU resolved to an active product. */
export interface ScannerProductMatch {
  match_type: "product_match";
  /** The matched product. */
  product: ProductResponse;
  /** Active lots for the product in the active store, ordered by stable
   *  FEFO order `(expiry_date ASC, created_at ASC, id ASC)`. The
   *  Scanner tab applies the active FEFO policy to this vector. */
  lots: ExpiryLotResponse[];
}

/** No match — the scanner did not match any lot, barcode, or SKU. */
export interface ScannerUnknown {
  match_type: "unknown";
  /** The trimmed scanned value, preserved verbatim. */
  scanned_value: string;
}

/** Discriminated union returned by `resolve_scanner_code`. */
export type ScannerResolveResult =
  | ScannerLotMatch
  | ScannerProductMatch
  | ScannerUnknown;

// ─── Commands ────────────────────────────────────────────────────────────────

/**
 * Resolves a scanned value against the catalog in strict priority order
 * (lot code → product barcode → product SKU → Unknown).
 *
 * `locale` is forwarded to the Rust command so user-facing Validation
 * rejections (`ScanValueEmpty`, "no active store") reach the UI in the
 * active locale. Omitting the argument preserves the default fallback
 * to English at the backend.
 *
 * The frontend MUST NOT issue this call when no active store exists; the
 * backend still fails safely with a Validation if the precondition slips
 * through.
 */
export async function resolveScannerCode(
  input: ScannerResolveInput,
  locale?: Locales,
): Promise<ScannerResolveResult> {
  return invoke<ScannerResolveResult>("resolve_scanner_code", {
    input,
    locale,
  });
}
