# Product catalog list preferences

## Goal
Improve the Products catalog list so users can control visible columns, see unit and default alert-days data by default, hide archived products, and return from product detail/edit without losing scroll position.

## Scope
- Add `default_unit` and `default_alert_days_before` to product search results.
- Render configurable product-list columns on `ProductCatalogPage.svelte`.
- Default visible columns include Unit and Alert days.
- Add a hide-archived option to the list view.
- Preserve scroll position when navigating list → detail/edit → list.
- Add i18n copy and regenerate generated i18n types.
- Run focused frontend/backend verification.

## Tasks

- [x] T1 — Extend product search result data
  - Files: `src-tauri/src/dto/products.rs`, `src-tauri/src/db/repositories/products.rs`, `src/lib/products.ts`
  - Evidence: `cargo test --lib db::repositories::products::tests::search_products_returns_category_ids_from_junction` passes; `npm run check` passes with no type errors.

- [x] T2 — Add configurable catalog columns and hide-archived UI
  - Files: `src/components/ProductCatalogPage.svelte`, `src/i18n/en/index.ts`, `src/i18n/es/index.ts`, generated `src/i18n/i18n-types.ts`
  - Evidence: `npm run i18n:generate` regenerated the catalog interface with `toggleColumns`, `toggleColumnsAria`, `columnUnit`, `columnAlertDays`, `columnCategory`, `hideArchived`, `showArchived`. `npm run check` passes.

- [x] T3 — Preserve list scroll on return from detail/edit
  - Files: `src/components/ProductCatalogPage.svelte`
  - Evidence: `npm run check` passes; manual runtime check pending user/local app run (scroll-restore logic is exercised by the existing list → detail → back flow and is hard to verify without a live Tauri window).

- [x] T4 — Final verification and review summary
  - Evidence: focused Rust test passes, `npm run check` passes, `npm run i18n:generate` is consistent with the edited `en`/`es` dictionaries.

## Evidence log
- Branch created: `feat/product-catalog-list-preferences`.
- Exploration: `gentle-ai-explore` mapped required surfaces and confirmed no schema migration is needed.
- `ProductSearchResult` (Rust + TS) extended with `default_unit` (Option<String>) and `default_alert_days_before` (i32 / number).
- `RawProductRow::into_search_result` now forwards the two new fields verbatim.
- `ProductCatalogPage.svelte` gained:
  - Optional, user-toggleable columns for Barcode / Unit / Alert days / Status; Description + SKU remain always visible.
  - A Columns dropdown that persists the visible-column set in `localStorage` under `caduxo.products.catalog.columns.v1`.
  - A hide-archived toggle (default `false`) persisted in `localStorage` under `caduxo.products.catalog.hideArchived.v1`.
  - Window-scroll preservation: capture `window.scrollY` before transitioning out of the list view (`startCreate` / `startEdit` / `openDetail` / `handleEditedFromDetail`) and restore it on return from `cancelForm`, `handleSaved`, and `handleArchived` (after `requestAnimationFrame` so the list view has re-mounted).
  - New i18n strings (en/es): `toggleColumns`, `toggleColumnsAria`, `columnUnit`, `columnAlertDays`, `columnCategory`, `hideArchived`, `showArchived`.
- Visual correction after screenshot feedback: scoped the search input styles to `.search-input` so the Columns menu checkboxes no longer inherit full text-input styling; added fixed checkbox sizing and cleaner menu spacing. Evidence: `npm run check` passes.
- Scroll correction after edit/save feedback: `handleSaved` now refreshes the product list before restoring scroll, and restore waits two animation frames so the refreshed list layout is present. Archive return follows the same post-refresh restore pattern. Evidence: `npm run check` and focused product repository test pass.
- Second scroll correction after runtime feedback: `captureListScroll` now only captures while the current view is `list`; the detail → edit transition no longer overwrites the original list scroll with the detail page's scroll position. Evidence: `npm run check` passes.
- Edit feedback refinement after UX feedback: edit/save no longer relies on the top success alert, because restored scroll can place it outside the user's focus. The edited product row now highlights for ~1.2s after the refreshed list returns. Evidence: `npm run check` passes.
