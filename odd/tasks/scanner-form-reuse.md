# Scanner form reuse

Branch: `feat/product-lifecycle-reusable-identifiers`

## Goal

Reuse the canonical product and lot forms from Scanner flows so quick product registration and new lot creation behave consistently with Product pages.

## Tasks

- [x] Replace Scanner inline quick product creation with `ProductForm` using `prefillUpc` for unknown scanned values.
- [x] Add optional product context props to `LotForm` and pass description/SKU from Scanner new-lot flow.
- [x] Run frontend checks and commit the work unit.

## Evidence

- `npx svelte-check --workspace . --threshold error`: 0 errors, 0 warnings.
- `npm run build`: vite build OK (232 modules, 2.42s); i18n generation reported `all files are up to date`.
- Commit: `278c5ab fix(scanner): reuse product and lot forms`.
