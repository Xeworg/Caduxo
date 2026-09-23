# Lot display sentinel cleanup

Branch: `feat/product-lifecycle-reusable-identifiers`

## Goal

Prevent internal sentinel location ids (`loc-sentinel-*`) from appearing in the UI, while showing real lot batch codes clearly.

## Decisions

- Show sentinel stock as localized `No location` / `Sin ubicación`.
- Keep sentinel locations selectable when they hold stock.
- Lot scan codes come from `expiry_lots.batch_code` only.
- If a lot has no `batch_code`, show localized `No batch code` / `Sin código de lote`.

## Tasks

- [x] Add reusable lot display helper and tests if supported.
- [x] Apply helper in product detail, calendar detail, and scanner location options.
- [x] Add i18n placeholder for missing batch code and regenerate types.
- [x] Run checks and commit work unit.

## Evidence

- `src/lib/lotDisplay.ts` — pure helpers `isSentinelLocationId`,
  `resolveLocationDisplay`, `resolveBatchCodeDisplay`. Sentinel prefix
  `loc-sentinel-` detected by string match; unknown ids fall back to the
  raw id so misconfigured rows stay inspectable.
- `src/lib/lotDisplay.test.ts` — documents expected behaviour as
  framework-agnostic `describe` / `it` / `expect` cases. Project has
  no TS test runner configured (no `vitest` / `jest` / `mocha` in
  `package.json`, no `*.test.ts` siblings, no `vitest.config.*`), per
  task instruction "if project test setup supports TS unit tests;
  otherwise document why skipped" — ready to run once a runner is added.
- `src/components/ProductDetailPage.svelte` — lot list row uses
  `lotBatchCodeLabel` / `lotLocationLabel` (always renders batch code
  via the helper, renders location only when `location_id` is present,
  sentinel ids show `No location`); detail modal `lotDetailPanel`
  uses `lotBatchCodeLabel` / `detailLotLocationLabel` (passes
  `detailLotLocations` for name lookup).
- `src/components/CalendarPage.svelte` — detail modal location cell
  routes through `resolveLocationDisplay(detailLot.location_id,
  lotDetailLocations, $LL.common.noLocation())` instead of falling
  back to the raw id.
- `src/components/ScannerPage.svelte` — `locationOptions` for Sale /
  Stock-out labels each balance with `resolveLocationDisplay(...,
  [], $LL.common.noLocation())` so sentinel balances render
  `No location (N)` but stay selectable (raw `value: b.location_id`
  preserved).
- `src/i18n/en/index.ts` + `src/i18n/es/index.ts` — new key
  `lotsDetail.noBatchCode` = `No batch code` / `Sin código de lote`,
  regenerated via `npm run i18n:generate`.
- `src/i18n/i18n-types.ts` — regenerated; `noBatchCode` appears in
  both the structural-type declaration and the runtime-typed
  translator function table.

## Validation

- `npm run i18n:generate` — generated `i18n-types.ts` cleanly; new
  `noBatchCode` key present in both structural types and translator
  function table.
- `npx svelte-check --workspace . --threshold error` —
  `svelte-check found 0 errors and 0 warnings`.
- `npm run build` — `✓ 233 modules transformed`, `✓ built in 2.38s`,
  no warnings.
- No TS test runner wired in this project — see
  `src/lib/lotDisplay.test.ts` for the rationale and the runnable
  behavioural contract.
- Commit: `19af849 fix(lots): hide sentinel location ids`.
