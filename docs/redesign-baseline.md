# Visual redesign baseline

> PR 1 / Phase 0 of `openspec/changes/caduxo-daisyui-redesign/`.
> Captures the pre-redesign visual state of every main surface so the
> verify pass can compare against this baseline after each phase.

## Method

The baseline is an annotated textual inventory of each main surface,
plus a list of the class names that the redesign will retire. Screenshots
are not captured at this phase because:

- The implementation environment used to author PR 1 is headless
  (no X server, no Tauri WebView). Running `npm run dev` or
  `npm run tauri dev` requires a desktop runtime that is not available
  in this environment.
- The acceptance criterion for the Phase 1 foundation gate is
  *no widespread preflight regression* — the verify report for
  PR 1's final acceptance will capture the actual screenshots in a
  desktop environment after the build is verified to boot.

The per-surface annotations below capture:

1. The route / tab the surface lives under.
2. The viewport widths the surface was inspected at (1440 / 1024 / 720 px).
3. The repeating local CSS class families that the redesign will retire
   (each class maps to a DaisyUI primitive in the design phase).
4. Existing modal shells, OS-chrome leaks, and other items the redesign
   commits to fix.

## Per-surface inventory

### App shell (`src/App.svelte`)

- **Tabs:** Dashboard, Stores, Products, Calendar, Reports, Import,
  Backup, Configuration (`$LL.nav.*`).
- **Visual state today:** Bespoke `.nav` / `.nav-btn` / `.nav-brand`
  markup with hard-coded slate / blue colours (`#1e293b` background,
  `#2563eb` active). 48 px sticky nav with `box-shadow: 0 1px 3px`.
- **Viewport behaviour today:** Static — no responsive collapse. The
  eight tab buttons render in one row at every viewport.
- **OS-chrome leaks today:** None at the shell level (the shell uses
  no native form controls). The shell is fully custom CSS.
- **Redesign targets:** DaisyUI `navbar bg-base-200` with
  `navbar-start` / `navbar-center` / `navbar-end` slots; `dropdown`
  collapse at ≤ 720 px; gradient band behind the brand on Dashboard
  only (PR 5).
- **Class families to retire:** `.app-shell`, `.nav`, `.nav-brand`,
  `.nav-btn`, `.nav-btn.active`.

### Dashboard (`src/components/DashboardPage.svelte`)

- **Surface:** Tab default `stores` in current build (note: app boots
  to `activeTab = "stores"` per `App.svelte`).
- **Visual state today:** Five urgency cards (`.urgency-card`,
  `.urgency-card-expired/today/alert/soon/normal`) with bespoke
  `.status-*` / `.urgency-badge` / `.lot-picker-status` colour
  classes. A `.tab-btn` / `.detail-tabs` / `.tab-content` strip. A
  `.error-banner` block. A `.scan-spinner` icon. An urgency-driven
  table with no zebra / sticky treatment.
- **Viewport behaviour today:** Cards laid out in a horizontal strip
  at every viewport; the lot table is readable but does not wrap
  (overflow at narrow viewports).
- **OS-chrome leaks today:** Two `<select>` controls for store /
  location filtering (render with OS chrome on Windows / macOS /
  GTK).
- **Redesign targets:** `Card.svelte` (urgency cards with `tone`
  prop), `Badge.svelte` (urgency badges with leading dot),
  `Table.svelte` (zebra / sticky), `Tabs.svelte` (bordered),
  `Alert.svelte` (error banner), DaisyUI `loading` (scan spinner),
  `Button.svelte` (action buttons), `Tooltip.svelte` (icon buttons).
  PR 6.
- **Class families to retire:** `.urgency-card*`, `.urgency-badge`,
  `.status-*`, `.lot-picker-status`, `.tab-btn`, `.detail-tabs`,
  `.tab-content`, `.error-banner`, `.scan-spinner`, `.loading-row`,
  `.urgency-card-expired/today/alert/soon/normal`.

### Stores (`src/components/StoresPage.svelte`)

- **Visual state today:** `.page-header` + `.store-form` + `.store-list` /
  `.store-detail` layout. `.btn-primary` / `.btn-secondary` / `.btn-small`
  button families. `.badge-inactive` for inactive stores. `.alert-error`
  / `.alert-success` for inline feedback.
- **Viewport behaviour today:** Two-column layout (`store-list` aside +
  `store-detail` main); collapses awkwardly at 720 px.
- **OS-chrome leaks today:** Bare `<input>` (text / checkbox),
  `<textarea>`, native `<select>` (not in stores page directly, but
  present in filters), and a checkbox `<label>` wrapper.
- **Redesign targets:** Form fields via `Input.svelte` /
  `Select.svelte` / `Toggle.svelte`. Store list to `Table.svelte` or
  DaisyUI `list` (PR 9). Inline alerts via `Alert.svelte`. Action
  buttons via `Button.svelte`. PR 8.
- **Class families to retire:** `.page-header`, `.btn-primary`,
  `.btn-secondary`, `.btn-small`, `.alert-error`, `.alert-success`,
  `.badge-inactive`, `.store-form`, `.form-actions`,
  `.checkbox-label`, `.location-list`, `.location-item`,
  `.section-header`.

### Products (`src/components/ProductCatalogPage.svelte`,
`src/components/ProductDetailPage.svelte`,
`src/components/ProductForm.svelte`)

- **Visual state today:** Catalog has an inline filter bar and a list
  of products (table-like surface). Product detail shows a tabbed
  layout. Form uses `ProductForm.svelte` with bespoke
  `.form-group` / `.field-label` / `.input` / `.error-msg` markup and
  `<datalist>` autocomplete for barcode type + unit definitions.
- **OS-chrome leaks today:** Bare `<input type="text">` (multiple),
  `<input type="checkbox">`, `<textarea>`, `<datalist>`. The
  datalist suggestions render with browser default styling.
- **Redesign targets:** Form migration in PR 8. Catalog list in
  PR 9 (`Table.svelte`). Product detail's `.detail-tabs` in
  `Tabs.svelte` (PR 6 alongside dashboard tabs).
- **Class families to retire:** `.form-group`, `.field-label`,
  `.small-label`, `.inline-error`, `.field-error`, `.saving-msg`,
  `.action-btn`, `.chip-clear`, `.banner-btn`, `.link-btn`, `.caret`.

### Calendar (`src/components/CalendarPage.svelte`,
`src/components/CalendarMonth.svelte`,
`src/components/DatePicker.svelte`)

- **Visual state today:** Month grid with day cells, today ring,
  badge dots for expirations, prev/next chevron navigation. Day-detail
  panel listing active lots for the selected day. DatePicker popover
  for expiry dates with year picker / decade navigation / manual
  text input / `clearable` semantics.
- **OS-chrome leaks today:** Bare `<input type="text">` in DatePicker
  (date entry). The popover is hand-rolled with absolute positioning
  + outside-click listener (not DaisyUI).
- **Redesign targets:** Calendar restyle (PR 10) — visual only,
  behaviour preserved (year range, keyboard nav, ISO bind). DatePicker
  popover acquires `class="dropdown dropdown-content"` while
  preserving keyboard contracts. CategoryPicker (in
  `inputs/CategoryPicker.svelte`) same treatment.
- **Class families to retire:** (none explicitly listed by the
  redesign gate; the redesign retains the bespoke calendar widgets
  but restyles them).

### Reports (`src/components/ReportsPage.svelte`)

- **Visual state today:** Filters at the top (store / location /
  urgency), results table below. Bespoke `.reports-table` /
  `.reports-empty` styling.
- **OS-chrome leaks today:** Native `<select>` for store / location /
  urgency filters; bare `<input>` for date range / search.
- **Redesign targets:** Filters migrated in PR 8
  (`Input.svelte` / `Select.svelte` / radio group via DaisyUI
  `radio`). Data table in PR 9 (`Table.svelte`).
- **Class families to retire:** `.reports-table`, `.reports-empty`,
  `.form-group` / `.field-label` / `.inline-error` (shared form
  utilities).

### CSV import (`src/components/CsvImportPage.svelte`,
`src/components/ColumnMapper.svelte`)

- **Visual state today:** Three-stage flow (upload → map columns →
  preview). Strategy radios, preview table, detail editor.
- **OS-chrome leaks today:** Native `<select>` for column mapping,
  bare `<input>`, `<input type="radio">`, native `<datalist>` is
  not used here.
- **Redesign targets:** Stage buttons / strategy radios / detail
  editor fields in PR 8. Preview table in PR 9 (`Table.svelte`).
- **Class families to retire:** `.form-group` / `.field-label` /
  `.inline-error` / `.action-btn`.

### Backup / restore (`src/components/BackupRestorePage.svelte`)

- **Visual state today:** Export / validate / restore sections with
  `.info-list` / `.checks-list` / `.confirm-box` surfaces.
- **OS-chrome leaks today:** Bare `<input type="file">` (file
  picker) and bare `<button>` controls.
- **Redesign targets:** Section actions + restore confirmation in
  PR 8. Info lists in PR 9 (`Table.svelte` or DaisyUI `list`).
- **Class families to retire:** `.info-list`, `.checks-list`,
  `.confirm-box`, `.action-btn`, `.banner-btn`.

### Configuration (`src/components/ConfigurationPage.svelte`)

- **Visual state today:** Locale selector (`<select>` with OS chrome),
  "Ubicación inicial obligatoria" toggle (bespoke `.toggle-wrap` /
  `.toggle-track` / `.toggle-thumb` CSS with `transition: left
  0.2s`).
- **OS-chrome leaks today:** Native `<select>` for language selector
  — the **single most visible** OS-chrome leak in the app. The
  toggle's bespoke animation has no reduced-motion gate.
- **Redesign targets:** Locale selector to `Select.svelte`. Bespoke
  toggle to `Toggle.svelte`. New "Tema / Theme" section in PR 5
  with the theme switcher (lists `AVAILABLE_THEMES`, shows active
  source, surfaces IPC failure).
- **Class families to retire:** `.toggle-wrap`, `.toggle-track`,
  `.toggle-thumb`, native `<select>` markup, `.select-styled`
  (if present).

### Unit review (`src/components/UnitReviewPage.svelte`,
`src/components/UnitReviewBanner.svelte`)

- **Visual state today:** `.review-page` / `.review-header` /
  `.group-list` / `.group-card` layout. `<details>` / `<summary>`
  panels for "Map to preset" / "Keep as custom" with `.btn-outline
  btn-sm` summaries. Banner with `.banner-btn` dismiss button.
- **OS-chrome leaks today:** Bare `<input>`, `<input type="radio">`,
  `<details>` (which renders with default browser styling on some
  platforms).
- **Redesign targets:** `<details>` panels migrate to DaisyUI
  `collapse collapse-arrow` (PR 10). Banner dismiss button to
  `Button.svelte` with `Tooltip.svelte` for the icon close.
  `.review-header` action buttons to `Button.svelte`.
- **Class families to retire:** `.preset-dropdown`, `.custom-form`,
  `.dropdown-panel`, `.dropdown-hint`, `.dropdown-item`, `.caret`,
  `.btn-outline`, `.btn-sm` (locally redefined), `.banner-btn`.

### Modals (`src/components/MoveStockModal.svelte`,
`RegisterExitModal.svelte`, `AdjustCountModal.svelte`,
`ArchiveLotDialog.svelte`, `ResolveQuantityDialog.svelte`, and the
inline overlays inside `DashboardPage.svelte`)

- **Visual state today:** Bespoke `.modal-overlay` / `.modal-box` /
  `.modal-box-wide` / `.modal-header` / `.modal-body` / `.modal-footer`
  / `.modal-close` / `.modal-loading` markup. No native `<dialog>`
  usage — focus trap and Escape handling are hand-rolled (or absent).
- **OS-chrome leaks today:** Native `<select>`, bare `<input>`,
  `<textarea>` (inside the modals). The modal shells themselves
  are not OS-chrome but neither are they accessible (no native
  focus trap).
- **Redesign targets:** Native `<dialog class="modal">` driven by
  `showModal()`. `Modal.svelte` primitive (PR 4) handles focus
  trap, Escape, backdrop click, focus restoration. Migration
  scheduled in PR 7 (split 7a + 7b if apply-time exceeds the
  400-line budget).
- **Class families to retire:** `.modal-overlay`, `.modal-box`,
  `.modal-box-wide`, `.modal-header`, `.modal-body`,
  `.modal-footer`, `.modal-close`, `.modal-loading`.

## Cross-cutting observations

- **Native OS controls leak in 8+ distinct surfaces:** every native
  `<select>` and bare `<input>` is an OS-chrome leak. PR 8 retires
  the form surfaces; the locale selector on Configuration is the
  single most visible offender and migrates in PR 5.
- **No reduced-motion gate today.** No component in `src/` reads
  `prefers-reduced-motion`. PR 1 introduces the global reset in
  `src/app.css`; subsequent phases wire per-component opt-ins.
- **No theme support today.** Every colour is a hard-coded hex
  literal. PR 1 introduces the `caduxo-light` custom theme + DaisyUI
  built-in `dark`; PR 5 introduces the theme switcher; PR 3+
  migrates components to DaisyUI theme tokens.
- **Twenty-four component files carry their own subset of the same
  visual primitives** — buttons, cards, badges, modals, tables,
  alerts, popovers, toggle controls. PR 3 + PR 4 extract shared
  primitives (`Button`, `Card`, `Badge`, `Alert`, `EmptyState`,
  `LoadingState`, `Toggle`, `Tooltip`, `Modal`, `Table`, `Tabs`,
  `Select`, `Input`).
- **Motion today is ad-hoc:** twenty-plus `transition: ... 0.1s–0.2s`
  declarations, one `@keyframes spin`. PR 12 centralises motion
  through the foundation tokens added in PR 1.

## Files / discovery targets for the migration

```text
src/App.svelte                            → PR 5
src/components/DashboardPage.svelte       → PR 6 + PR 7 (overlays)
src/components/MoveStockModal.svelte      → PR 7a
src/components/RegisterExitModal.svelte   → PR 7a/b
src/components/AdjustCountModal.svelte    → PR 7a
src/components/ArchiveLotDialog.svelte    → PR 7a
src/components/ResolveQuantityDialog.svelte → PR 7b
src/components/StoresPage.svelte          → PR 8 + PR 9
src/components/ProductCatalogPage.svelte  → PR 9
src/components/ProductDetailPage.svelte   → PR 6 + PR 8
src/components/ProductForm.svelte         → PR 8
src/components/LotForm.svelte             → PR 8
src/components/CalendarPage.svelte        → PR 10
src/components/CalendarMonth.svelte       → PR 10
src/components/DatePicker.svelte          → PR 10
src/components/inputs/CategoryPicker.svelte → PR 10
src/components/ReportsPage.svelte         → PR 8 + PR 9
src/components/BackupRestorePage.svelte   → PR 8 + PR 9
src/components/CsvImportPage.svelte       → PR 8 + PR 9
src/components/ColumnMapper.svelte        → PR 8
src/components/ConfigurationPage.svelte   → PR 5 + PR 8
src/components/UnitReviewPage.svelte      → PR 10
src/components/UnitReviewBanner.svelte    → PR 10
src/components/LotMovementsPanel.svelte   → PR 9
src/components/ScanSearchBox.svelte       → PR 6
src/components/inputs/                    → PR 10
```

## Acceptance baseline (post-PR 1)

After PR 1 lands, the unmigrated UI should still render with no
widespread breakage. Tailwind preflight MAY shift default heading /
list / button / input / table styling. Every main surface remains
usable.

The verify report for PR 1's final acceptance will:

- Capture screenshots of every main surface in both `caduxo-light`
  and `dark` themes (toggle via DevTools: `document.documentElement
  .setAttribute("data-theme", "dark")`).
- Confirm no widespread preflight regression.
- Confirm the locale selector on Configuration still renders with
  OS chrome (will be retired in PR 5).
- Confirm the bespoke toggle's `transition: left 0.2s` is not gated
  on `prefers-reduced-motion` (will be retired in PR 5).
- Confirm no console errors on `npm run dev` boot.