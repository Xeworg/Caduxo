# Explore — caduxo-daisyui-redesign

> Read-only exploration. No code edits. Persists the current UI architecture,
> pain points, DaisyUI/theme implications, native UI replacements, visual &
> effects opportunities, scope boundaries, risks, and the recommended next
> SDD phase for the new change `caduxo-daisyui-redesign`.
>
> Source-of-truth branch: `feat/daisyui-redesign-plan` (currently carries an
> uncommitted planning doc at `docs/daisyui-redesign-plan.md`). The explore
> phase builds on that doc but does not replace it.

## 1. Current UI architecture

### Stack and entry points

- **Framework**: Svelte 5 + Vite 6 + Tauri 2 (`@tauri-apps/api` ^2, plugins
  `dialog`, `notification`, `os`).
- **Styling**: zero design-system. Plain CSS, **no Tailwind, no DaisyUI**.
- **i18n**: `typesafe-i18n` (en + es) with Svelte adapter, `$LL.*` stores.
- **Locale bootstrap**: `src/i18n/locale.svelte.ts` initialises the rune in
  `src/main.ts` ahead of mount (top-level await avoided for WebKitGTK on
  Tauri compatibility).
- **Type checking**: `svelte-check` + `tsc`; prebuild/predev run
  `npm run i18n:generate`.

### Global stylesheet

- `src/style.css` is a 33-line hero/landing stylesheet (`.shell`, `.hero`,
  `.eyebrow`, `.cards`). It is imported from `src/main.ts` but no production
  component references its classes — appears vestigial from the original
  landing page. Safe to retire or repurpose.

### Component inventory (24 files)

```
src/App.svelte                                  app shell + top nav (tabs)
src/components/StoresPage.svelte                stores + locations CRUD
src/components/ProductCatalogPage.svelte        product list
src/components/ProductDetailPage.svelte         product detail tabs + lots
src/components/ProductForm.svelte               create/edit product + inline unit form
src/components/LotForm.svelte                   create/edit lot
src/components/LotMovementsPanel.svelte         ledger panel inside ProductDetail
src/components/DashboardPage.svelte             urgency cards, lot table, modals
src/components/ScanSearchBox.svelte             scan + text search bar
src/components/CalendarPage.svelte              month view wrapper
src/components/CalendarMonth.svelte             calendar grid + year picker
src/components/DatePicker.svelte                custom popover date picker
src/components/ReportsPage.svelte               report type cards + filters + table
src/components/CsvImportPage.svelte             3-stage CSV import flow
src/components/ColumnMapper.svelte              CSV column mapping editor
src/components/BackupRestorePage.svelte         export + validate + restore
src/components/ConfigurationPage.svelte         language + lots toggles
src/components/UnitReviewPage.svelte            unrecognized unit triage
src/components/UnitReviewBanner.svelte          yellow warning banner
src/components/MoveStockModal.svelte            modal: move stock between locations
src/components/RegisterExitModal.svelte         modal: register exit
src/components/AdjustCountModal.svelte          modal: adjust counted quantity
src/components/ArchiveLotDialog.svelte          modal: archive lot
src/components/ResolveQuantityDialog.svelte     modal: resolve discrepancy
src/components/inputs/CategoryPicker.svelte     combobox popover with chips
```

### Shared "design vocabulary" — duplicated, not centralized

Every component file defines its own subset of the same patterns. Representative
classes (all local to each component):

| Pattern        | Variant classes seen across files                                            |
|----------------|-----------------------------------------------------------------------------|
| Buttons        | `btn-primary`, `btn-secondary`, `btn-ghost`, `btn-outline`, `btn-danger`, `btn-link`, `btn-sm`, `btn-small`, `action-btn`, `chip-clear`, `banner-btn`, `inline-unit-close`, `link-btn` |
| Alerts / msgs  | `alert-error`, `alert-success`, `alert-info`, `alert`, `message`, `message error`, `message success`, `error-msg`, `saving-msg`, `error-banner`, `empty-hint`, `inline-error`, `field-error` |
| Modals         | `modal-overlay`, `modal-box`, `modal-box-wide`, `modal-header`, `modal-body`, `modal-footer`, `modal-close`, `modal-loading` |
| Cards          | `urgency-card`, `urgency-card-expired|today|alert|soon`, `settings-section`, `card`, `group-card` |
| Tables         | `lot-table`, `table-wrapper`, `reports-table`, `reports-empty`, `lot-picker`, `lot-picker-item`, `lot-picker-status`, `info-list`, `checks-list`, `confirm-box` |
| Badges/pills   | `urgency-badge`, `urgencyClass(lot.urgency)`, `status-*`, `store-chip`, `barcode-chip`, `cp-chip`, `cp-chip--uncat` |
| Tabs           | `tab-btn`, `detail-tabs`, `tab-content`                                       |
| Inputs         | `form-group`, `field-label`, `small-label`, `input`, `checkbox-label`, `radio-label`, `kind-radios`, `inline-unit-form`, `grid-2`, `checkbox-row` |
| Toggle         | `toggle-wrap`, `toggle-input`, `toggle-track`, `toggle-thumb` (custom switch) |
| Loading        | `loading`, `loading-msg`, `loading-row`, `scan-spinner` + `@keyframes spin`   |
| Empty state    | `empty-state`, `empty-hint` (text only, no shared illustration)               |
| Banner         | `unit-banner`, `banner-icon`, `banner-text`, `warning-banner`, `confirm-box` |

Color tokens repeated everywhere (representative, not exhaustive):
`#2563eb` (primary blue), `#1e293b` (slate-800 nav), `#f6f8fb` (base bg),
`#fee2e2 / #fca5a5 / #991b1b` (error red), `#dcfce7 / #86efac / #166534`
(success green), `#fef9c3 / #fde047 / #713f12` (warning amber),
`#fff / #e5e7eb / #d1d5db / #9ca3af / #6b7280 / #374151 / #111827` (neutrals).

Spacing/typography is also per-component: `font-size: 0.85rem`, `0.9rem`,
`0.95rem`, `1rem`, `1.2rem`, `1.5rem`, all uncoordinated.

### Existing interactivity patterns

- **Native `<select>`**: one occurrence only — `ConfigurationPage.svelte`
  locale picker (`.locale-select`). Renders with OS-native chrome and breaks
  the visual cohesion of the rest of the app.
- **Native `<datalist>` autocomplete**: `ProductForm.svelte` uses two
  `<datalist>` blocks for barcode types (`EAN13`, `EAN8`, `UPC`, `CODE128`,
  `CODE39`, `QR`) and unit definitions.
- **Native checkbox**: `ConfigurationPage` builds a custom switch from
  `<input type="checkbox">` + custom track/thumb. Several forms use plain
  `<input type="checkbox">` and radios directly.
- **Custom popovers (hand-rolled)**:
  - `DatePicker.svelte` — outside-click, scroll/resize listeners, fixed
    positioning, custom keyboard interaction.
  - `inputs/CategoryPicker.svelte` — same hand-rolled popover, with
    listbox semantics, arrow-key navigation, inline create row.
- **`<details>/<summary>` dropdowns** in `UnitReviewPage.svelte` for
  "Map to preset" and "Keep as custom" panels. Accessible but limited and
  hard to style cohesively.
- **Modals**: five dedicated modal components (`MoveStockModal`,
  `RegisterExitModal`, `AdjustCountModal`, `ArchiveLotDialog`,
  `ResolveQuantityDialog`) plus three inline modal overlays inside
  `DashboardPage` (product detail, lot detail, quick product create). Each
  rebuilds `.modal-overlay / .modal-box / .modal-header / .modal-body /
  .modal-footer / .modal-close` with local styles. No focus trap, no shared
  Escape handling beyond per-dialog `onClose`.

### i18n and copy hygiene

All current visible copy flows through `$LL.*`; the prior i18n-support
change closed the remaining hardcoded `aria-label` residues. The redesign
must keep this discipline — every new empty/loading/toast string enters
both `src/i18n/en/index.ts` and `src/i18n/es/index.ts` and is regenerated
by `npm run i18n:generate`.

### Motion / a11y baseline

- 20+ ad-hoc `transition: ... 0.1s–0.2s` declarations across components.
- One `@keyframes spin` in `ScanSearchBox.svelte` (no `prefers-reduced-motion`
  guard).
- No `prefers-reduced-motion` media query anywhere in `src/`.
- No `prefers-color-scheme` handling.
- No theme switching of any kind.

## 2. Pain points

1. **No design system.** Visual primitives are duplicated across 24 files,
   with subtle drift (border radii 4/5/6/8/10 px, padding 4/6/8/10/14/20 px,
   font sizes 0.85–1.5rem, blue #2563eb vs #1d4ed8 vs #93c5fd).
2. **Inconsistent button hierarchy.** `btn-primary / btn-secondary /
   btn-ghost / btn-outline / btn-danger / btn-link / btn-sm / btn-small` plus
   ad-hoc `action-btn`, `chip-clear`, `banner-btn`, `inline-unit-close`,
   `link-btn`, `caret`. There is no canonical list of when each is right.
3. **Native OS UI leaks into the chrome.** The locale `<select>` is the
   single biggest visible inconsistency: it renders with the OS dropdown
   chrome on Windows, macOS, and Linux/GTK. All form inputs are also unstyled
   bare browser controls.
4. **Custom popovers reinvent accessibility.** DatePicker and CategoryPicker
   reimplement outside-click, positioning, scroll/resize handling, and
   keyboard interaction. DaisyUI/Tailwind v4 + DaisyUI's modal/dropdown
   primitives can replace most of this hand-rolled code while improving
   focus trap and `prefers-reduced-motion` support.
5. **Modal pattern repeated 8+ times.** Same overlay/header/close/body/footer
   structure with local CSS each time. No shared escape semantics, focus
   restoration, or `prefers-reduced-motion` handling.
6. **Alert/message vocabulary is fragmented.** `alert-error / alert-success /
   alert-info / alert / message / error-msg / saving-msg / error-banner /
   field-error / empty-hint / inline-error` — no shared feedback component.
7. **No spacing/typography tokens.** Hard-coded `0.85rem / 0.9rem / 14px /
   20px` everywhere. DaisyUI's base scale + Tailwind spacing utility classes
   would normalise this.
8. **No theme support.** The app reads color tokens directly from inline
   values. The only "switch" surface is language. Adding dark mode or
   accent themes is currently infeasible without a wholesale rewrite.
9. **Motion is ad-hoc and inconsistent.** Various 100–200ms `background` and
   `color` transitions, the one `spin` keyframe, and a 0.2s `left` transition
   on the toggle thumb — no shared easing or duration tokens, no reduced-
   motion gate.
10. **Empty/loading states inconsistent.** `<p class="loading">` vs
    `<p class="loading-row">` vs `<span class="loading-msg">` vs inline text
    — same idea, three or four presentations.
11. **Tables are bespoke.** Dashboard `lot-table`, LotMovementsPanel,
    ReportsPage, CsvImportPage preview, StoresPage store list, backup info-
    list, check-list, confirm-box all define different table/row/header
    styles. Sticky headers, zebra rows, numeric alignment, hover state —
    none shared.
12. **Urgency badges live everywhere.** `.urgency-badge`, `.status-*`,
    `.lot-picker-status`, yellow banner, amber chip — no central
    expired/today/alert/soon/normal vocabulary.
13. **Forms lack visual consistency.** `LotForm`, `ProductForm`,
    `StoresPage`, `ConfigurationPage` each define their own `form-group`,
    label, input, required marker, help/error text rendering.
14. **Tabs are unstyled.** `DashboardPage.svelte`'s `.tab-btn` is the only
    tab UI in the app and it is minimal.
15. **No app-shell typography/colour hierarchy.** The nav uses Inter via
    `src/style.css` fallback; body components do not inherit it. There is no
    visible "brand" beyond the wordmark.

## 3. DaisyUI / theme implications

### Dependencies to introduce

- `tailwindcss` (v4)
- `@tailwindcss/vite`
- `daisyui`

Per `docs/daisyui-redesign-plan.md`, the install path is:

- `vite.config.ts`: add the Tailwind Vite plugin alongside `svelte()`.
- New `src/app.css`: `@import "tailwindcss"; @plugin "daisyui";` plus the
  project's custom theme block (DaisyUI v5 syntax).
- `src/main.ts`: import `src/app.css` instead of the legacy `./style.css`.
- `src/style.css`: remove after the migration confirms no remaining
  references.

### DaisyUI component classes that map cleanly to Caduxo

| Current pain point                | DaisyUI primitive                                          |
|-----------------------------------|------------------------------------------------------------|
| 8+ ad-hoc button classes          | `btn`, `btn-primary`, `btn-secondary`, `btn-ghost`, `btn-outline`, `btn-error`, `btn-warning`, `btn-success`, `btn-circle`, `btn-sm`, `btn-xs`, plus icon slot |
| Bespoke modals × 8                | `<dialog class="modal">` with `showModal()` API (native focus trap, Escape, backdrop) |
| Custom alert vocabulary           | `alert`, `alert-success`, `alert-error`, `alert-warning`, `alert-info` |
| Bespoke cards (urgency, settings) | `card`, `card-body`, `card-title`, `stats`, `stat`         |
| Bespoke badges                    | `badge`, `badge-error`, `badge-warning`, `badge-info`, `badge-success`, `badge-neutral`, plus `indicator`/`status` dot for urgency |
| Bespoke tables                    | `table`, `table-zebra`, `table-pin-rows`, alignment helpers |
| Custom switch checkbox            | `toggle`, `toggle-primary`, `toggle-sm`                    |
| Native checkbox/radio             | `checkbox`, `radio`                                        |
| Native `<select>`                 | `select`, `select-bordered`, with custom caret             |
| Native `<input>`                  | `input`, `input-bordered`, `input-error`, `input-success`, plus `label`/`label-text`/`label-text-alt` |
| Hand-rolled popovers              | `dropdown`, `dropdown-content`, plus a shared `Popover` primitive that preserves DatePicker/CategoryPicker keyboard a11y |
| Unstyled tab strip                 | `tabs`, `tab`, `tab-active`, `tabs-bordered` / `tabs-lifted` |
| Hand-rolled spinner               | `loading`, `loading-spinner`, `loading-dots`, plus `skeleton` for table rows |
| Inline `successMsg / errorMsg`    | `toast` + `alert` (deferred until page migration is stable) |
| Ad-hoc navbar                     | `navbar`, `navbar-start`, `navbar-center`, `navbar-end`, `menu`, plus `drawer` for narrow viewports |
| `title=` tooltips                 | `tooltip`, `tooltip-open`, keyboard-reachable               |

### Built-in DaisyUI themes

DaisyUI ships with two default themes (`light`, `dark`) and a large set of
named themes (`cupcake`, `corporate`, `business`, `emerald`, `valentine`,
`dracula`, `night`, `winter`, `dim`, `nord`, `sunset`, etc.). The user wants
built-in theme support — the simplest approach is to ship the default
`light` + `dark` pair as v1, persist the choice in `app_settings` (the
existing settings row used by `locale.svelte.ts`), and add a theme switcher
in `ConfigurationPage` next to the language picker. The existing OS-language
detection pattern can be mirrored for the theme: fall back to
`prefers-color-scheme` when no `app_settings.theme` row exists.

### Custom light theme

The Caduxo brand colour `#2563eb` (current primary blue) and base
`#f6f8fb` background can become a `@plugin "daisyui/theme"` block named
`caduxo-light` with semantic overrides for `primary`, `secondary`, `accent`,
`neutral`, `base-100`, `base-200`, `base-300`, `info`, `success`, `warning`,
`error`. This becomes the default theme; `dark` (or `caduxo-dark`) is the
opt-in alternative.

### Caution: Tailwind v4 preflight

Preflight resets heading sizes, list styles, button defaults, input
borders, table spacing, etc. Phase 1 of the implementation plan must detect
whether the existing buttons, headings, inputs, and tables shift visually
**before** any page migration begins — otherwise the migration will chase
ghost regressions. This is already captured in `docs/daisyui-redesign-plan.md`
as Phase 1 acceptance criteria.

## 4. Native / system UI elements to replace or restyle

(Each item: where it lives today → what it becomes in the redesign.)

1. **`<select>` in `ConfigurationPage.svelte`** (locale picker, line ~147)
   → DaisyUI `select select-bordered select-sm` (or `select-md`) with
   consistent caret, focus ring, and theme-aware hover/active states. Long
   locales list → consider also adding `select-ghost` variant for less
   weight, or a custom combobox if i18n locale count grows.

2. **`<datalist>` in `ProductForm.svelte`** (barcode type + unit definitions)
   → keep the `<datalist>` for native accessibility (keyboard, screen
   reader announcements) but wrap the input in DaisyUI `input` styling.
   Option to upgrade to a DaisyUI `dropdown` + listbox when the catalog is
   large or when the user wants richer rendering.

3. **`<input type="text">` (all forms)** — `ProductForm`, `LotForm`,
   `StoresPage`, `BackupRestorePage`, `ConfigurationPage`,
   `ResolveQuantityDialog`, etc. → DaisyUI `input input-bordered`,
   `input-error` for invalid state, `label` + `label-text` for label
   rendering, `label-text-alt` for required marker and helper text.

4. **`<input type="checkbox">` plain checkboxes** in `ProductForm`
   ("set as primary barcode", `LotForm`, `ReportsPage`) → DaisyUI
   `checkbox checkbox-primary checkbox-sm`.

5. **`<input type="radio">` in `UnitReviewPage`** (kind integer/decimal)
   → DaisyUI `radio radio-primary radio-sm`.

6. **Custom toggle in `ConfigurationPage`** (`toggle-wrap / toggle-track /
   toggle-thumb`) → DaisyUI `toggle toggle-primary`. Eliminates ~25 lines
   of CSS.

7. **`<details>/<summary>` dropdowns in `UnitReviewPage.svelte`** (Map to
   preset, Keep as custom) → migrate to DaisyUI `dropdown` +
   `dropdown-content` + `dropdown-end` for the floating panel, or to the
   shared `Popover` primitive if Phase 8 extracts one. Either way, the
   `details/summary` semantics (which have known a11y gaps with screen
   readers) are replaced with proper ARIA roles.

8. **Custom popovers in `DatePicker.svelte`** and
   **`inputs/CategoryPicker.svelte`** → keep the existing keyboard
   contracts (arrow keys, Enter, Escape, Backspace on CategoryPicker) but
   restyle with DaisyUI `dropdown`/`dropdown-content` and the `popover`
   class. Phase 8 may extract a `Popover` primitive that both share.

9. **All modal dialogs** — `MoveStockModal`, `RegisterExitModal`,
   `AdjustCountModal`, `ArchiveLotDialog`, `ResolveQuantityDialog`, plus
   the inline product/lot/quick-create overlays in `DashboardPage` →
   DaisyUI `<dialog class="modal modal-open">` driven by `dialog.showModal()`
   for native focus trap and Escape-to-close, with `modal-box`, `modal-box-
   wide` size variants, `modal-action` for footer buttons. The current
   `✕ modal-close` becomes a `btn btn-circle btn-ghost btn-sm absolute top-2
   end-2`. Keeps existing Escape / click-outside / focus-restoration
   behaviour but standardises the shell.

10. **Custom buttons everywhere** — every `class="btn-primary"`,
    `class="btn-secondary"`, `class="btn-ghost"`, `class="btn-outline"`,
    `class="btn-danger"`, `class="action-btn"`, `class="chip-clear"`,
    `class="banner-btn"`, `class="link-btn"` → the unified
    `src/components/ui/Button.svelte` primitive that wraps DaisyUI
    variants.

11. **Tables (DashboardPage lot-table, LotMovementsPanel, ReportsPage,
    CsvImportPage preview, StoresPage, BackupRestorePage, ConfigurationPage
    info-list)** → wrap with DaisyUI `table table-zebra table-pin-rows`,
    alignment utility classes for numeric columns, `table-sm` for dense
    rows, shared empty/loading slots.

12. **Urgency badges** in `DashboardPage` (`.urgency-badge`, `.status-*`,
    `.lot-picker-status`) → DaisyUI `badge` + semantic modifier
    (`badge-error`, `badge-warning`, `badge-info`, `badge-success`,
    `badge-neutral`) with a leading `status status-error status-warning
    status-info` dot where appropriate.

13. **Banner / alert strips** (`UnitReviewBanner` yellow warning,
    `DashboardPage` `.error-banner`) → DaisyUI `alert alert-warning` /
    `alert-error` with consistent icon + text + action button slot.

14. **Tabs in `DashboardPage`** (`.tab-btn`, `.detail-tabs`,
    `.tab-content`) → DaisyUI `tabs tabs-bordered` (or `tabs-lifted`) with
    `tab tab-active`. Becomes a shared `src/components/ui/Tabs.svelte`
    primitive.

15. **Spinner in `ScanSearchBox`** → DaisyUI `loading loading-spinner
    loading-sm`. Hooks up `prefers-reduced-motion` for free.

16. **Empty states** — currently plain text rows in `LotMovementsPanel`,
    `DashboardPage`, `StoresPage`, etc. → DaisyUI heroicon-style SVG + text
    + optional action button, behind a shared
    `src/components/ui/EmptyState.svelte` primitive.

17. **`title="..."` tooltips** on icon-only buttons
    (`UnitReviewBanner.dismiss`, dashboard action buttons, calendar nav
    arrows) → DaisyUI `tooltip tooltip-left` (or `tooltip-bottom`).
    Tooltips are now keyboard-reachable, not hover-only.

18. **App shell nav** (`App.svelte` `.nav / .nav-btn / .nav-brand`) →
    DaisyUI `navbar bg-base-200` with `navbar-start` (brand),
    `navbar-center` (tab buttons), `navbar-end` (theme switcher, future
    overflow menu). Responsive collapse via `dropdown` for narrow viewports.

19. **Color-tinted surfaces** for hero/dashboard header (none today —
    opportunity to introduce a subtle `bg-gradient-to-r from-primary/5
    to-base-100` band above the urgency cards).

20. **OS scrollbars** → keep default scrollbars but add
    `scrollbar-thin scrollbar-thumb-base-300` (CSS variable, theme-aware)
    so wide tables and the dashboard lot list feel deliberate.

## 5. Visual / effects opportunities (called out explicitly)

Effects are part of the redesign, not an afterthought. The plan explicitly
asks for striking/appealing visuals. The following effects are in scope
and will be specified in the Design phase:

1. **Switchable themes.** DaisyUI built-in `light` and `dark` plus a custom
   `caduxo-light` brand theme. Choice persisted in `app_settings.theme`,
   defaulting to `prefers-color-scheme`. Theme switcher in
   `ConfigurationPage`.

2. **Subtle default transitions.** DaisyUI's defaults (~150–200ms ease-out)
   for hover, focus, dropdown open/close, modal show/hide. Standardised
   via a `transition-base` utility group rather than per-component
   hand-tuning.

3. **Modal fade + scale.** DaisyUI modal already animates opacity + scale
   (~150ms). Verify `prefers-reduced-motion: reduce` disables it (turns
   into instant show/hide).

4. **Dropdown / popover slide-in.** DaisyUI `dropdown-content` does a
   subtle slide-down (~150ms). Same reduced-motion guard.

5. **Card hover lift.** `transition: transform 200ms ease, box-shadow
   200ms ease` with `hover:-translate-y-0.5 hover:shadow-lg` on urgency
   cards and report type cards. Reduced-motion: no translation.

6. **Button press feedback.** `active:translate-y-px` or
   `active:scale-95` for tactile confirmation. Reduced-motion: skip.

7. **Urgency pulse — opt-in.** Slow pulse (1.5–2s) on the "expired"
   urgency badge and on a small dot inside the `.urgency-card-expired`.
   Gated on `@media (prefers-reduced-motion: no-preference)`. Designed
   to draw the eye to genuinely expired lots, not to flash decoratively.
   This is the *only* deliberately animated indicator; everything else
   stays still unless the user is animating.

8. **Skeleton shimmer.** DaisyUI `skeleton` for table rows during initial
   load on Dashboard, Reports, Stores, Products. Reduced-motion: static
   grey, no shimmer.

9. **Toast notifications.** DaisyUI `toast` + `alert` for ephemeral
   success/error feedback (replaces repeated `successMsg / errorMsg`
   strings). Deferred to Phase 9 per the planning doc — do not introduce
   the toast primitive until the core migration is stable.

10. **Focus rings.** DaisyUI's default outline rings render on every
    interactive primitive. Verify they remain visible against the new
    theme tokens.

11. **Calendar day-cell transitions.** Selection state, today ring,
    multi-day badge dot transitions (`transition-colors duration-150`).

12. **Category picker popover.** Fade + 4px y-translate on open, same
    reduced-motion guard.

13. **Sidebar / drawer on narrow viewports.** DaisyUI `drawer` for the
    navigation, replacing the current wrap-or-scroll behaviour on small
    widths.

14. **Backdrop blur for modals.** DaisyUI modal supports a soft backdrop
    via `::backdrop`; with a subtle `backdrop-blur-sm` on capable
    platforms for a "depth" effect. Reduced-motion: no blur animation.

15. **App-shell gradient band.** Subtle gradient header behind the
    dashboard brand area (`bg-gradient-to-br from-primary/5 to-base-100`)
    for visual interest above the urgency cards.

## 6. Scope boundaries and risks

### In scope

- Add Tailwind v4 + DaisyUI dependency stack and config.
- Create shared UI primitives in `src/components/ui/` (`Button`, `Card`,
  `Modal`, `Table`, `Badge`, `Alert`, `EmptyState`, `LoadingState`,
  `Tabs`, `Select`, `Input`, `Toggle`, optional `Popover`).
- Add custom light theme + DaisyUI built-in dark/light themes + theme
  switcher in Configuration page + persistence in `app_settings`.
- Migrate all 24 components to use the primitives and DaisyUI classes,
  page by page, deleting local styles as each surface converts.
- Define and apply shared motion tokens + `prefers-reduced-motion`
  compliance.
- Add new `i18n` entries for new visible strings (empty states, toast
  copy, theme labels) in both EN and ES.

### Out of scope

- Backend / Tauri command changes. No DB schema, Rust API, or service
  contract changes unless the migration exposes a required bug fix.
- Business logic rewrites (unit-review flow, scan/search, stock
  movements keep their behaviour).
- Replacing `DatePicker`, `CalendarMonth`, or `CategoryPicker` with
  off-the-shelf widgets. Visual restyle only; behaviour preserved.
- Router-based refactor (the tab-based shell stays).
- New product features (bulk actions, advanced filters, dashboards 2.0,
  multi-user, etc.).
- Dark mode first-class polish (ship `dark` theme, but don't pre-tune
  every surface for it beyond what DaisyUI gives for free).
- Tauri-specific work (icons, bundling, installer) — out of scope.
- Auto-switching theme based on time of day — only OS preference drives
  the default for v1.

### Risks

| # | Risk | Mitigation |
|---|------|-----------|
| 1 | **Review-budget overflow.** A full visual redesign will exceed the 400-line review budget many times over. | Migrate phase by phase (the 13 phases in `docs/daisyui-redesign-plan.md`). If a phase still overshoots, split by page or by primitive. Use chained PRs. |
| 2 | **Tailwind preflight resets existing CSS.** Headings, buttons, inputs, lists may shift unexpectedly. | Phase 1 foundation gate: install Tailwind/DaisyUI without migrating any component, run `npm run check` + `npm run build`, manually screenshot the main screens, stop and fix if widespread breakage. |
| 3 | **Half-migrated UI.** Mixed local styles + DaisyUI primitives look worse than a fully consistent system. | Create primitives first, migrate by full screen, delete local styles as each surface converts. The plan's "no half-migration" rule is enforced by the phase ordering. |
| 4 | **Accessibility regressions in custom popovers.** DatePicker and CategoryPicker have hand-rolled keyboard a11y; replacing them with a shared primitive must preserve all interactions. | Phase 8 explicitly keeps behaviour; manual keyboard pass on every primitive; non-regression manual smoke after each phase. |
| 5 | **Custom calendar widget regressions.** CalendarMonth/DatePicker carry domain logic (year picker, decade navigation, badge dots). | Restyle only; do not rewrite; manual smoke test keyboard nav, year picker, day selection, and out-of-range guards after each Phase 8 change. |
| 6 | **i18n catalog drift.** New empty/loading/toast strings must land in EN + ES. | Every visible string goes through `src/i18n/en/index.ts` and `src/i18n/es/index.ts`. `predev` and `prebuild` already run `npm run i18n:generate`; CI must include `npm run i18n:generate && npm run check`. |
| 7 | **Theme persistence requires a new settings row.** | Extend `app_settings` row(s) with `theme` field; reuse the same read/write pattern as `language`. If a new Tauri command is needed, add it under the existing `settings` command surface, not a new domain. |
| 8 | **CSS bundle size growth.** Tailwind v4 + DaisyUI adds CSS weight. | DaisyUI is tree-shakeable per-theme; ship `light` + `dark` only by default. Verify `npm run build` final CSS size before merging each phase; gate PR if size regresses >20%. |
| 9 | **WebKitGTK quirk in Tauri.** Per `src/main.ts` comment, older WebKitGTK builds can fail on top-level await. Tailwind/DaisyUI shouldn't re-introduce this, but Phase 1 must verify a Tauri dev launch (or document a browser fallback if Tauri unavailable). | Add a Tauri/manual smoke to Phase 1 acceptance. |
| 10 | **Plugin ordering.** `@tailwindcss/vite` must be evaluated correctly by Vite alongside `@sveltejs/vite-plugin-svelte`. | Phase 1 verification: vite dev + build both pass; Svelte's HMR works; no console errors. |
| 11 | **Font inconsistency.** Inter is currently declared in `src/style.css`; DaisyUI uses its own font stack. | Decide the canonical font in the Design phase (Inter vs DaisyUI default) and apply it via `font-sans` or a custom theme token. |
| 12 | **Urgency pulse risk.** A pulsing badge could trigger vestibular sensitivity. | Gate the animation on `@media (prefers-reduced-motion: no-preference)`; never pulse on hover; document the rationale in the Design phase. |
| 13 | **Chained PR review attention.** A 4000–8000-line change split into 13 chained PRs is still heavy. | Use the `auto-forecast` chain strategy from `openspec/config.yaml`; ask the user before any `size:exception`; respect `ask-on-risk` pause for any oversized phase. |

## 7. Recommended next phase

**Proposal (interactive).** This exploration confirms the redesign is
feasible inside the existing stack and surfaces the high-impact decisions
that should be settled before the user reviews a detailed spec. The
interactive preflight requires a proposal-question round *before* writing
the proposal document, focused on product/business tradeoffs — not on
harness mechanics or test commands.

Concrete questions to raise in the next round:

1. **Which theme set ships at GA?** Default `light` only; `light` + `dark`
   (OS-aware default, switchable in Configuration); or `light` + `dark` +
   a small curated accent set (e.g. `corporate`, `dim`)?
2. **Is theme switching in v1, or deferred until dark-mode polish lands?**
   v1 ships the theme row + switcher but only `light` + `dark` are tuned.
3. **Is the `Popover` / shared combobox primitive in scope for the same
   milestone as DatePicker/CategoryPicker restyle, or later?** Restyling
   both pickers on top of their hand-rolled implementations is cheaper;
   extracting a shared primitive is more correct but adds work.
4. **Is the deliberate urgency-pulse animation approved for "expired"
   lots/cards**, or should everything be still? (Affects only the expired
   urgency variant; everything else stays still.)
5. **Does the toast host land in v1 or post-stable?** Toast host requires
   threading ephemeral feedback through every page; deferring it keeps the
   first visual cuts smaller.

Once those are answered, the SDD flow continues with:

- **Spec** — concrete primitives API (`Button`, `Card`, `Modal`, `Table`,
  `Badge`, `Alert`, `EmptyState`, `LoadingState`, `Tabs`, `Select`,
  `Input`, `Toggle`, optional `Popover`), tailwind/daisyui config block,
  theme tokens, and per-phase migration order.
- **Design** — visual mock references, motion tokens, the urgency-pulse
  rationale, and the per-screen migration table.
- **Tasks** — work units aligned with the 13 phases in
  `docs/daisyui-redesign-plan.md`, each with its own commit and chained
  PR if needed.
- **Apply / Verify / Archive** — phase-by-phase, with the verify report
  listing `npm run check`, `npm run build`, manual smoke, and a manual
  a11y pass per phase.

No code edits, commits, or pushes happen in this explore phase. The plan
document at `docs/daisyui-redesign-plan.md` is preserved as the
controlling checklist; this file documents the architectural baseline
and decision points the proposal phase will resolve.
