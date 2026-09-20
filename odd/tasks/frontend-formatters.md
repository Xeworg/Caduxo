# Frontend locale formatters

## Goal
Centralize frontend locale-sensitive formatting so report export messages, report tables, and lot-resolution history do not branch directly on `en`/`es`.

## Scope
- Add typesafe-i18n formatters for short dates, date-times, and quantities/numbers.
- Replace `ReportsPage.svelte` row/page plural-word ternaries with dictionary plural keys.
- Replace report date/quantity formatting helpers with locale-aware formatters.
- Replace `ResolveQuantityDialog.svelte` `es-MX`/`en-US` ternary with a locale-aware formatter.

## Non-goals
- Do not change backend/PDF formatting in this slice.
- Do not clean all frontend hardcoded text or urgency label duplication.
- Do not add a third locale.

## Tasks
- [x] Task 1: Implement shared frontend formatters and pluralized export success dictionaries.
- [x] Task 2: Run focused frontend checks and fix regressions within scope.
- [x] Task 3: Commit the verified slice.

## Evidence

### Task 1
- `src/i18n/formatters.ts`: added `shortDate`, `dateTime`, and `quantity` formatters built on `Intl.DateTimeFormat` / `Intl.NumberFormat`, mapping the typesafe-i18n `Locales` (`en` → `en-US`, `es` → `es-MX`) so behaviour matches the previous `toLocaleString(locale.current === "es" ? "es-MX" : "en-US")` ternary.
- `src/i18n/en/index.ts` + `src/i18n/es/index.ts`:
  - `reports.exportSuccess` now uses typesafe-i18n's inline plural parts
    `Exported {{rows:?? row|?? rows}} across {{pages:?? page|?? pages}}`
    (and the Spanish equivalent with `fila|filas` / `página|páginas`).
    Caller no longer composes the singular/plural words.
  - Added wrapper keys `reports.table.shortDate`, `reports.table.qtyFormatted`,
    and `lotMovements.resolution.eventDateTime` that invoke the new
    formatters via `{value|formatter}` syntax.
- `src/components/ReportsPage.svelte`:
  - `exportPdf()` now calls `$LL.reports.exportSuccess({ rows, pages })`
    with no caller-composed singular/plural words.
  - `formatDate` and `formatQty` helpers delegate to `$LL.reports.table.shortDate`
    / `$LL.reports.table.qtyFormatted`.
- `src/components/ResolveQuantityDialog.svelte`:
  - `formatDateTime` now delegates to `$LL.lotMovements.resolution.eventDateTime`,
    removing the `locale.current === "es" ? "es-MX" : "en-US"` ternary.
- `src/i18n/i18n-types.ts`: regenerated via `npm run i18n:generate`;
  `Formatters` now exposes `shortDate`, `dateTime`, `quantity`, and
  `reports.exportSuccess` only takes `{ rows, pages }`.

### Task 2
- `npm run i18n:generate` — observed: regenerated `i18n-types.ts`,
  typesafe-i18n reports `all files are up to date`.
- `npx svelte-check --tsconfig ./tsconfig.json` — observed: 0 errors, 0 warnings.
- `npm run build` — observed: `vite build` succeeds, output
  `dist/assets/index-Wor2cGsX.js 320.63 kB` built in 1.40s.
- `grep -n "locale\.current === \"es\"" src/components/ReportsPage.svelte src/components/ResolveQuantityDialog.svelte`
  — observed: no matches; the scoped ternaries are gone.

### Post-review tighten pass
- `src/i18n/formatters.ts`: bare `YYYY-MM-DD` strings now parse as local calendar dates (`new Date(year, monthIndex, day)`) instead of UTC midnight.
- `src/i18n/formatters.ts`: quantity formatting now allows up to 10 fractional digits to avoid low-precision rounding compared to the previous raw `qty.toString()` behaviour.
- `src/components/ResolveQuantityDialog.svelte`: removed the now-unused `locale` import.
- `npx svelte-check --tsconfig ./tsconfig.json` — observed: 0 errors, 0 warnings.
- `npm run build` — observed: Vite build succeeds.

### Independent verification
- `gentle-ai-verify` PASS.
- `npm run i18n:generate` — observed idempotent; generated files unchanged.
- `npx svelte-check --tsconfig ./tsconfig.json --threshold error` — observed: 0 errors, 0 warnings.
- Verified no scoped `locale.current === "es"` branches remain, generated formatter types match dictionaries, ISO date-only parse is local, and `ResolveQuantityDialog.svelte` has no stale `locale` import.
- Commit: `cc21f99 feat: centralize frontend locale formatters`.
