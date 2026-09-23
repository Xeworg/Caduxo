/**
 * Behavioural contract for `src/lib/lotDisplay.ts`.
 *
 * NOTE: This project does NOT currently configure a TypeScript test
 * runner (no `vitest`, `jest`, `mocha`, or `@testing-library` in
 * `package.json` / `node_modules/.bin`, no `*.test.ts` siblings, no
 * `vitest.config.*`). The task instructions say to add tests "if
 * project test setup supports TS unit tests; otherwise document why
 * skipped" — this file documents the expected behaviour as runnable
 * cases so they can be wired to a runner (recommended: `vitest`) in
 * a follow-up without re-deriving the contract. The assertions are
 * kept framework-agnostic and copy-pasteable.
 *
 * Suggested wiring when a runner is added:
 *
 *   import { describe, it, expect } from "vitest";
 *   import {
 *     isSentinelLocationId,
 *     resolveLocationDisplay,
 *     resolveBatchCodeDisplay,
 *   } from "./lotDisplay.js";
 *
 * The cases below mirror those exact import names.
 */

import type {
    isSentinelLocationId as _isSentinelLocationId,
    resolveLocationDisplay as _resolveLocationDisplay,
    resolveBatchCodeDisplay as _resolveBatchCodeDisplay,
} from "./lotDisplay.js";

// Reference the symbols so the import is not elided when no runner is
// wired up yet. The `void` keeps the symbols out of any runtime side
// effects while keeping the file compileable under
// `svelte-check --threshold error`.
void (null as unknown as typeof _isSentinelLocationId);
void (null as unknown as typeof _resolveLocationDisplay);
void (null as unknown as typeof _resolveBatchCodeDisplay);

// ─── Expected behaviour ─────────────────────────────────────────────────────
//
// describe("isSentinelLocationId", () => {
//   it("returns true for loc-sentinel-<store> ids", () => {
//     expect(isSentinelLocationId("loc-sentinel-store-1")).toBe(true);
//     expect(isSentinelLocationId("loc-sentinel-abc123")).toBe(true);
//   });
//
//   it("returns false for null / undefined / empty", () => {
//     expect(isSentinelLocationId(null)).toBe(false);
//     expect(isSentinelLocationId(undefined)).toBe(false);
//     expect(isSentinelLocationId("")).toBe(false);
//   });
//
//   it("returns false for real location ids", () => {
//     expect(isSentinelLocationId("loc-real-1")).toBe(false);
//     expect(isSentinelLocationId("Fridge A")).toBe(false);
//   });
// });
//
// describe("resolveLocationDisplay", () => {
//   const locations = [
//     { id: "loc-1", name: "Fridge A" },
//     { id: "loc-2", name: "Freezer 1" },
//   ];
//   const NO_LOC = "Sin ubicación";
//
//   it("returns the placeholder for null / undefined", () => {
//     expect(resolveLocationDisplay(null, locations, NO_LOC)).toBe(NO_LOC);
//     expect(resolveLocationDisplay(undefined, locations, NO_LOC)).toBe(NO_LOC);
//     expect(resolveLocationDisplay("", locations, NO_LOC)).toBe(NO_LOC);
//   });
//
//   it("returns the placeholder for sentinel ids even when not in list", () => {
//     expect(
//       resolveLocationDisplay("loc-sentinel-store-1", locations, NO_LOC),
//     ).toBe(NO_LOC);
//   });
//
//   it("returns the matched name for known ids", () => {
//     expect(resolveLocationDisplay("loc-1", locations, NO_LOC)).toBe("Fridge A");
//     expect(resolveLocationDisplay("loc-2", locations, NO_LOC)).toBe("Freezer 1");
//   });
//
//   it("falls back to the raw id for unknown ids (inspectability)", () => {
//     expect(
//       resolveLocationDisplay("loc-unknown", locations, NO_LOC),
//     ).toBe("loc-unknown");
//   });
//
//   it("accepts an empty locations list (scanner balance options)", () => {
//     expect(
//       resolveLocationDisplay("loc-sentinel-store-1", [], NO_LOC),
//     ).toBe(NO_LOC);
//     expect(resolveLocationDisplay("loc-real", [], NO_LOC)).toBe("loc-real");
//   });
// });
//
// describe("resolveBatchCodeDisplay", () => {
//   const NO_BATCH = "Sin código de lote";
//
//   it("returns the placeholder for null / undefined / empty / whitespace", () => {
//     expect(resolveBatchCodeDisplay(null, NO_BATCH)).toBe(NO_BATCH);
//     expect(resolveBatchCodeDisplay(undefined, NO_BATCH)).toBe(NO_BATCH);
//     expect(resolveBatchCodeDisplay("", NO_BATCH)).toBe(NO_BATCH);
//     expect(resolveBatchCodeDisplay("   ", NO_BATCH)).toBe(NO_BATCH);
//   });
//
//   it("returns the batch code verbatim (scannable identifier)", () => {
//     expect(resolveBatchCodeDisplay("B2024-001", NO_BATCH)).toBe("B2024-001");
//   });
//
//   it("trims surrounding whitespace but preserves inner content", () => {
//     expect(resolveBatchCodeDisplay("  B2024-001  ", NO_BATCH)).toBe("B2024-001");
//   });
// });
