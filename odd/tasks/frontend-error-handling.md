# Frontend error handling

## Goal

Centralize frontend error display so structured backend errors and thrown objects do not leak as raw `String(e)` / `[object Object]` in user-facing toasts, alerts, or status text.

## Scope

- Add one shared frontend error humanization helper.
- Replace user-facing `String(e)` / `String(err)` call-sites in Svelte components with the helper.
- Fix the existing `String("Failed")` catch bug in configuration save handling.
- Keep existing silent-swallow catch blocks silent.

## Non-goals

- Do not rewrite duplicate-field regex classification in this slice.
- Do not change backend error shapes or IPC contracts.
- Do not add or alter i18n dictionary keys unless required by checks.
- Do not touch `.codegraph/`.

## Tasks

- [x] Task 1: Implement shared frontend error humanization and wire user-facing component call-sites.
- [x] Task 2: Run focused frontend checks and fix regressions within scope.
- [x] Task 3: Commit the verified slice.

## Evidence

### Task 1
- `src/lib/errors.ts`: new shared helper `humanizeError(e: unknown): string` with two internal helpers (`extractFromString` walks a JSON-shaped string, `messageFromObject` walks a structured value). Behaviour:
  - `null` / `undefined` → `""`.
  - `Error` instance → unwrap `e.message` through the JSON heuristic (so `Error: {"kind":"validation","detail":{"message":"…"}}` collapses to the inner message) and fall back to the raw `e.message` when the heuristic returns empty.
  - `string` → fast-path returns the raw value unless it starts with `{` / `[`, in which case it tries `JSON.parse` to lift `detail.message` or top-level `message`. JSON-shaped strings that fail to parse, or parse to a value with no recognizable message, fall back to the raw string. Normal `Error.message` text (e.g. `"uniqueness violation: …"`) is preserved verbatim.
  - Direct object (Tauri sometimes throws the structured `CommandError` instead of a string) → `detail.message`, then top-level `message`. Unknown object shapes return `""` rather than `"[object Object]"` so the caller never has to defend against that surface.
- `src/components/ReportsPage.svelte`:
  - Dropped the local `humanizeError(raw: string)` and its comment block.
  - Added `import { humanizeError } from "../lib/errors.js";`.
  - All three call-sites (`onMount` load, `runPreview` catch, `exportPdf` catch) now call `humanizeError(e)` directly on the caught value instead of `humanizeError(String(e))`.
- `src/components/ConfigurationPage.svelte`:
  - Added the shared import.
  - The `handleLocaleChange` catch was a `} catch {` that assigned `String("Failed")` (literal — the actual thrown value was discarded). Fixed to `} catch (e)` and now appends `humanizeError(e)`, so structured Tauri errors surface their real message instead of the hard-coded literal.
  - `onMount` (language load) and `handleToggle` (location toggle) catches now use `humanizeError(e)` in place of `String(e)`.
- `src/components/StoresPage.svelte`: replaced `errorMsg = String(e)` and the two `flash(String(e), "error")` sites with `humanizeError(...)`. Silent-swallow catches (`loadLocations`, `saveLastSelected`) left as-is.
- `src/components/CalendarPage.svelte`:
  - The `loadLots` catch no longer uses `e instanceof Error ? e.message : String(e)` — it now uses `humanizeError(e)`, so a thrown `CommandError` object unwraps to `detail.message` instead of falling back to the JS object stringification.
  - `openLot` catch also converted.
  - Per-stage diagnostic `loadStage = \`failed: ${message}\`;` is preserved with the cleaned message.
- `src/components/DashboardPage.svelte`: all eight user-facing `errorMsg = String(e)` call-sites (loadStores, loadLocationsForStore, loadDashboard, exportReport, viewProduct, loadProductDetailLots, refreshSelectedLot, editLot) now use `humanizeError(e)`. The silent `loadUnitCatalog` swallow (`.catch(() => …)` is implicit through a `try { … } catch { }` block — kept silent.
- `src/components/ProductCatalogPage.svelte`: `onMount`, `runSearch`, and `exportProducts` catches now use `humanizeError(e)`.
- `src/components/ProductDetailPage.svelte`: `load`, `submitBarcode`, `removeBarcode`, `confirmArchive`, and `openLotDetail` catches now use `humanizeError(e)`. `refreshDetailLot`'s silent-swallow catch is left as-is.
- `src/components/ProductForm.svelte`:
  - The duplicate-field regex (`/duplicate|already exists|unique/i.test(msg)`) is preserved verbatim per the slice scope, but the raw string extraction now uses `humanizeError(e)` so structured `CommandError` objects unwrap before the regex match.
  - The outer `submit` catch also uses `humanizeError(e)`.
- `src/components/LotForm.svelte`: `init` and `submit` catches now use `humanizeError(e)`. Silent `getSettings` swallows (default location requirement) are kept silent.
- `src/components/LotMovementsPanel.svelte`: `load` catch uses `humanizeError(e)`.
- `src/components/MoveStockModal.svelte`, `RegisterExitModal.svelte`, `AdjustCountModal.svelte`: submit catches use `humanizeError(e)`.
- `src/components/ArchiveLotDialog.svelte`: `submit` catch uses `humanizeError(e)`.
- `src/components/CsvImportPage.svelte`: `selectAndPreview`, `handleMappingConfirm`, `handleImport` catches now use `humanizeError(err)`.
- `src/components/BackupRestorePage.svelte`: `handleExport`, `handleValidate`, `handleRestore` catches now use `humanizeError(e)`.
- `src/components/ResolveQuantityDialog.svelte`: `submit` catch uses `humanizeError(e)`. Silent `loadHistory` catch left as-is.
- `src/components/ScanSearchBox.svelte`: `handleScan` catch now passes `humanizeError(e)` into the `$LL.scan.searchError({ msg: … })` dictionary call.
- `src/components/inputs/CategoryPicker.svelte`: the duplicate-key regex classification (`/duplicate|case|already|exists/i.test(msg)`) is preserved, but the raw string extraction now uses `humanizeError(e)`. Import uses the nested `../../lib/errors.js` path.
- `src/components/UnitReviewPage.svelte`: `init`, `mapToPreset`, `keepAsCustom`, `leaveForLater` catches all use `humanizeError(e)`.

### Task 2
- `npm run i18n:generate` — observed idempotent; generated files unchanged.
- `npx svelte-check --tsconfig ./tsconfig.json --threshold error` — observed: 0 errors, 0 warnings.
- `npm run build` — observed: Vite build succeeds; output `dist/assets/index-BQPBsJrK.js` built in 1.36s.
- `grep -RIn 'String(e)\|String(err)\|String("Failed")' src/components` — observed: no matches.

### Task 3
- Commit: `7f232b6 fix: humanize frontend error messages`.

