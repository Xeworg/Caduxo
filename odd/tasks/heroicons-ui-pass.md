# Heroicons UI pass

## Goal
Add local/offline Heroicons to Caduxo UI surfaces where icons improve recognition, scanning speed, or action affordance, without requiring internet access at runtime.

## Constraints
- Use Heroicons through an npm/package-bundled local dependency or checked-in local SVG components; no CDN or remote icon fonts.
- Keep icons semantic and restrained; do not decorate every label.
- Preserve accessibility: icons that are decorative must be `aria-hidden`; icon-only buttons need accessible labels.
- Preserve existing behavior, i18n, keyboard navigation, and theme compatibility.
- Prefer a small shared wrapper/mapping if it reduces coupling to raw Heroicons imports.

## Tasks
- [x] Analyze current UI surfaces and recommend specific icon placements and names.
- [x] Add Heroicons dependency or local components for offline use.
- [x] Implement selected icon placements across navigation, actions, status/empty states, and high-value controls.
- [x] Verify with `npm run check` and `npm run build`.

## Allowed edit surfaces
- `src/components/ui/Icon.svelte`
- `src/components/DashboardPage.svelte`
- `src/components/StoresPage.svelte`
- `src/components/ProductDetailPage.svelte`
- `src/components/CsvImportPage.svelte`
- `src/components/LotMovementsPanel.svelte`
- `src/components/BackupRestorePage.svelte`
- `src/components/ReportsPage.svelte`
- `src/components/CalendarPage.svelte`
- `src/components/DatePicker.svelte`
- `src/components/UnitReviewBanner.svelte`
- `src/components/inputs/CategoryPicker.svelte`
- `odd/tasks/heroicons-ui-pass.md`

## Analysis summary
Use a local inline Heroicons wrapper instead of an npm dependency, matching the existing design decision to avoid icon dependencies. Prioritize replacing emoji/unicode action icons and adding restrained `Button` icon slots to high-value actions. Icons must use `currentColor`, semantic DaisyUI colors only, and no hardcoded fill/stroke colors.

## Implementation summary
Single `src/components/ui/Icon.svelte` primitive — closed `IconName` union + `Record<IconName, string>` path map, no npm install, no CDN, no raw Heroicons imports anywhere else. The component renders a 24x24 SVG with `fill="none"` and `stroke="currentColor"`, so it inherits text colour through all seven DaisyUI themes (`caduxo-light`, `dark`, `dracula`, `valentine`, `luxury`, `sunset`, `nord`). Stroke-width scales with size: xs=2, sm=1.75, md=1.5, lg=1.5. Sizing is via Tailwind `h-N w-N` classes only — no raw palette. Decorative default (`aria-hidden="true"`, `focusable="false"`); non-decorative usage exposes a `label` prop and flips `role="img"`.

The path map ships a top-level attribution comment noting Heroicons is by the Tailwind Labs team and is MIT licensed (license retained per MIT terms). No raw `fill="#"` or `stroke="#"` attributes — the only colours come from `currentColor`.

## Icon catalog (closed map)
20 icons added to the `ICONS` record:

| Icon | Used in |
| --- | --- |
| `x-mark` | Dashboard clear-chip, DatePicker clear, ProductDetail barcode remove, CategoryPicker chip remove, CategoryPicker popover close |
| `pencil` | Dashboard row edit, StoresPage location edit, ProductDetail lot edit |
| `arrow-down-tray` | Dashboard "view product and movements" |
| `clipboard-document-list` | ProductDetail movement history |
| `archive-box-arrow-down` | ProductDetail archive lot |
| `arrow-down-on-square-stack` | ProductDetail resolve quantity |
| `document-arrow-up` | CsvImport select-file card |
| `information-circle` | CsvImport expected-columns card |
| `arrow-path` | CalendarPage refresh |
| `calendar` | DatePicker open-calendar trigger |
| `exclamation-triangle` | UnitReviewBanner, BackupRestorePage destructive restore |
| `arrows-right-left` | LotMovementsPanel move stock |
| `arrow-right-on-rectangle` | LotMovementsPanel register exit |
| `adjustments-horizontal` | LotMovementsPanel adjust count |
| `document-arrow-down` | BackupRestorePage export, ReportsPage export PDF |
| `document-magnifying-glass` | BackupRestorePage select / choose-different |
| `arrow-uturn-left` | BackupRestorePage restore |
| `eye` | ReportsPage preview |
| `pencil-square` | ReportsPage edit filters |
| `check` | CategoryPicker selected-option indicator |

## Files changed
- `src/components/ui/Icon.svelte` — created. Closed `IconName` + `IconSize` union, size→class/stroke-width map, `ICONS` record with verbatim Heroicons 24 outline paths, attribution comment.
- `src/components/DashboardPage.svelte` — clear-chip, row edit, row view replaced with `Button` + `iconStart` snippets; `Icon` import added.
- `src/components/StoresPage.svelte` — location edit button now uses `Button` + `iconStart`.
- `src/components/ProductDetailPage.svelte` — barcode remove and all four lot actions migrated to `Button` + `iconStart`; ad-hoc `.btn-icon` left in place only for non-action styling hooks.
- `src/components/CsvImportPage.svelte` — primary + info action cards now render `Icon` inside `.card-icon` (decorative, `aria-hidden`); `.card-icon` wrapper styled as inline-flex so SVG sits at the expected pixel size.
- `src/components/CalendarPage.svelte` — refresh button gets an `iconStart` `Icon`.
- `src/components/DatePicker.svelte` — calendar trigger and clear button each get `iconStart` Icons.
- `src/components/UnitReviewBanner.svelte` — banner prefix and Review button get an `iconStart` Icon; banner-icon wrapper coloured via `--color-warning` mix to avoid raw palette.
- `src/components/inputs/CategoryPicker.svelte` — chip-remove buttons render `<Icon>` directly (not full `Button`, preserves chip styling); popover close button uses `Button` + `iconStart`; option check indicator replaced with `<Icon name="check">`; `.cp-check` and `.cp-chip-remove` styled as inline-flex.
- `src/components/LotMovementsPanel.svelte` — move-stock / register-exit / adjust-count buttons each get an `iconStart` Icon.
- `src/components/BackupRestorePage.svelte` — export, select-backup, restore, restore-data, choose-different buttons each get an `iconStart` Icon.
- `src/components/ReportsPage.svelte` — edit-filters, export-PDF, preview buttons each get an `iconStart` Icon.

## Behaviour preserved
- All existing aria-labels on icon-only buttons kept verbatim. The new `Icon` defaults to `aria-hidden="true"` so the surrounding `Button` aria-label still owns the action.
- The existing `Tooltip` wrappers and titles remain on every icon-only action.
- DaisyUI button variants, sizes, loading/disabled states, and onclick handlers untouched.
- The `icon` variant's `aria-label` requirement (already enforced by `Button.svelte`) covers every icon-only case.
- The existing i18n copy (move stock / register exit / adjust count / export / etc.) is preserved in every button — the `Icon` is added as an `iconStart` slot, never as a replacement for the visible label.

## Validation evidence
- `npm run check` — `svelte-check found 0 errors and 0 warnings` (TypeScript and Svelte diagnostics).
- `npm run build` — Vite production build succeeded: `225 modules transformed`, `dist/assets/index-*.css 234.58 kB`, `dist/assets/index-*.js 385.36 kB`, built in 1.97 s. No Tailwind v4 scanner warnings reported by the build.
- Grep 1 (hardcoded colors in `Icon.svelte`): `grep -nE 'fill="#|stroke="#' src/components/ui/Icon.svelte` returned no matches. PASS.
- Grep 2 (raw palette classes in modified files): `grep -nE 'text-(red|blue|green|...|gray)-\d+'` against every modified file returned no matches. PASS.
- Grep 3 (remaining emoji hits in modified files): single residual hit at `src/components/DashboardPage.svelte:801` — the `★` "primary barcode" indicator rendered as `{bc.barcode}{bc.is_primary ? " ★" : ""}` inside a `.barcode-chip` content chip. This is a content-data decoration (not an action affordance or button icon) and was outside the task's listed replacement targets; left untouched deliberately so the task stays scoped to action/icon affordances.

## Risks / follow-ups
- `ProductDetailPage` still has a `.btn-icon` style block (used for legacy `.btn-danger-icon` colour tint). The local class is no longer attached to any visible element after this migration, but the CSS was retained to avoid touching unrelated styling hooks. A future cleanup PR can drop it.
- The `★` primary-barcode marker on `DashboardPage` and the same marker on `ProductDetailPage` (rendered as `<span class="badge-primary">` text) remain as text indicators. Not action icons, so not in this task's scope; a future pass can replace them with `star`/`star-solid` icons if the project wants consistent shape vocabulary.
- No themes outside the curated seven were tested visually for the new Icons — verification was type + build + grep based per the parent-supplied task scope.
