# DaisyUI redesign plan

## Status

Planning branch: `feat/daisyui-redesign-plan`

This document defines the full visual redesign plan before any UI implementation. The decision for this plan is explicit: **Caduxo will adopt Tailwind CSS + DaisyUI** as the design-system foundation, instead of doing another partial ad-hoc CSS cleanup.

## Why a complete plan first

The previous i18n work grew complicated because many small improvements touched connected surfaces over time. The UI redesign has the same risk: navigation, cards, forms, modals, tables, responsive behavior, accessibility, and i18n copy all overlap.

The goal is to avoid a half-migrated interface by defining:

- the target visual system,
- the migration order,
- the components that must become shared primitives,
- the screens that must be converted,
- the verification gates for every slice,
- and the boundaries that prevent polish work from becoming uncontrolled redesign.

## Product goal

Make Caduxo feel like a cohesive desktop inventory application:

- clearer visual hierarchy,
- calmer spacing and typography,
- consistent buttons, cards, forms, tables, modals, badges, and alerts,
- better empty/loading/error states,
- visually engaging motion and effects that make the product feel modern without becoming distracting,
- redesigned native/system-looking controls, especially selects/dropdowns, so the app no longer feels like browser-default forms,
- support for DaisyUI built-in themes plus a Caduxo-branded default theme,
- responsive layouts that do not collapse awkwardly,
- preserved accessibility and existing i18n behavior.

This is a visual/system redesign, not a feature rewrite.

## Technical direction

### Adopted stack

- Tailwind CSS
- DaisyUI
- Existing Svelte 5 components
- Existing Tauri/Vite build
- Existing `typesafe-i18n` catalog

### Non-goals

- Do not rewrite the app into a router-based architecture in the first pass.
- Do not replace working business logic.
- Do not change database, backend services, command contracts, or report semantics unless a UI migration exposes a required bug fix.
- Do not replace custom complex widgets just because DaisyUI has nearby primitives.
- Do not rewrite complex custom widgets unless the redesign requires it; restyle first and replace logic only in a dedicated phase.
- Do not make dark mode a separate future-only concern: theme support is in scope, but the first implementation must keep theme support bounded to DaisyUI themes and persisted selection.

### Preserve carefully

These areas already carry behavior and accessibility detail and should be restyled without logic rewrites unless there is a dedicated follow-up:

- `src/components/DatePicker.svelte`
- `src/components/CalendarMonth.svelte`
- `src/components/inputs/CategoryPicker.svelte`
- stock movement modals and quantity flows
- scan/search flow
- existing locale detection and selector behavior

## Current UI architecture summary

Caduxo currently uses component-scoped CSS with repeated local style blocks.

Repeated patterns that should become shared design primitives:

- buttons: primary, secondary, danger, ghost, icon, small, action
- cards: urgency cards, report cards, settings cards, result cards
- tables: dashboard, reports, calendar, CSV preview, lot movement ledger
- modals: dashboard detail, calendar day detail, product detail, stock movement dialogs
- tabs: product/detail sections and dashboard/calendar detail panels
- badges: urgency/status/category/unit labels
- alerts: success, error, info, warning
- loading states: plain text rows/messages
- empty states: currently inconsistent or absent

## Target design system

### DaisyUI themes

Theme support is part of the redesign, not a later afterthought. The implementation should ship with:

- a Caduxo-branded default theme, e.g. `caduxo-light`,
- DaisyUI built-in `light`,
- DaisyUI built-in `dark`,
- a small curated set of optional built-in DaisyUI themes that fit the app tone, such as `cupcake`, `corporate`, `business`, `emerald`, or `night`.

Do not expose every DaisyUI theme if it makes the product feel random. The UI should feel curated, not like a theme playground.

Theme selection requirements:

- theme selector in Configuration, visually styled rather than native-looking,
- selected theme persisted using the existing settings persistence pattern,
- default from persisted setting first, then OS preference when useful, then `caduxo-light`,
- theme applied with `data-theme` at the app root or document root,
- every custom primitive must use DaisyUI tokens/classes rather than hard-coded colors where possible.

Caduxo-branded theme intent:

- primary: close to current Caduxo blue `#2563eb`
- base background: current app background family `#f6f8fb`
- neutral text: slate family
- semantic colors:
  - error: red for expired/destructive flows
  - warning: amber/orange for today/soon attention
  - info: blue for informational flows
  - success: green for completed actions

Built-in dark-compatible themes should work in v1, but the redesign does not promise hand-tuned dark-mode artwork for every screen. The guarantee is functional theme support, readable contrast, and no broken layouts across the curated theme set.

### Tailwind/DaisyUI configuration plan

Expected files:

- `vite.config.ts`
  - add Tailwind Vite plugin
- `src/app.css`
  - import Tailwind
  - register DaisyUI plugin/theme
  - keep any global app-level CSS and motion/accessibility utilities
- `src/main.ts`
  - import `src/app.css` instead of the old global CSS
- `src/style.css`
  - remove only after confirming all selectors are unused or migrated

Expected dependencies:

- `tailwindcss`
- `@tailwindcss/vite`
- `daisyui`

## Native/system control replacement policy

The redesign should deliberately remove the browser-default/system-default look from common controls. Keep native semantics where they help accessibility, but wrap or replace the visual surface so users experience one coherent Caduxo interface.

Controls to replace or restyle:

- language and theme selects in `ConfigurationPage`, using DaisyUI dropdown/select styling or an accessible custom listbox,
- report filter selects and date inputs, using DaisyUI `select`, `input`, and `join` patterns,
- product/category/unit controls, including datalist-looking inputs, with visually consistent combobox/listbox treatment,
- `<details>/<summary>` style expandable sections, replacing browser-default markers with designed disclosure controls,
- modal dialogs, replacing per-component homemade shells with the shared modal primitive,
- table controls, pagination/filter/action buttons, and inline row actions,
- tooltips/help text that currently relies on browser `title=` behavior, replacing it with designed helper text or tooltip components when the information is important.

Replacement rules:

- use DaisyUI primitives first when they satisfy accessibility and behavior,
- preserve keyboard behavior and form semantics,
- do not replace complex widgets such as `DatePicker`, `CalendarMonth`, or `CategoryPicker` logic in the first visual pass; restyle them to match the system,
- every new visible string must go through `typesafe-i18n`.

## Effects and motion inventory

The redesign should be visually appealing and more memorable, but motion must support comprehension. Every effect must respect `prefers-reduced-motion`.

Approved effects for the implementation plan:

- app shell: subtle gradient/accent band behind the top navigation or page header, theme-aware,
- page transitions: short fade/slide between main tabs, disabled for reduced motion,
- cards: hover lift, soft shadow increase, and active selection border,
- buttons: press feedback, loading spinner, and consistent disabled state,
- modals: backdrop blur plus fade/scale entry,
- dropdowns/popovers: short slide/fade entry with clear focus ring,
- tables: row hover highlight and selected/actionable row emphasis,
- skeleton loaders: soft shimmer only when reduced motion allows it, static skeleton otherwise,
- urgency states: restrained pulse/glow only for expired or critical inventory states, never for routine states,
- toasts/alerts: slide/fade entry and auto-dismiss only for transient success; errors remain stable until dismissed or corrected,
- calendar: selected/today day transitions and a small count badge animation for days with activity.

Effects that are explicitly out of scope:

- long decorative animations,
- bouncing/pulsing on normal controls,
- animation that delays inventory operations,
- particle/confetti effects,
- effects that communicate status by color alone.

## Shared UI primitives to create

Create these before migrating every page, so the app does not become a mix of DaisyUI classes and one-off local styles.

### `src/components/ui/Button.svelte`

Wrap DaisyUI button variants used by the app.

Supported variants:

- primary
- secondary
- ghost
- danger
- warning
- success
- icon
- link

Requirements:

- disabled state
- loading state
- optional icon slot
- accessible label for icon-only usage

### `src/components/ui/Card.svelte`

Base card with consistent padding, border, shadow, and optional header/footer slots.

Used by:

- dashboard urgency cards
- report type cards
- settings sections
- backup/import result summaries
- product/store detail panels

### `src/components/ui/Modal.svelte`

Shared modal shell.

Requirements:

- backdrop
- close button
- Escape closes when allowed
- click outside closes when allowed
- focus trap or at minimum focus restoration
- size variants: small, default, wide
- reduced-motion support

Do not migrate all modal logic at once. Use the primitive shell first and keep each dialog's business state local.

### `src/components/ui/Table.svelte`

Shared table wrapper and classes.

Requirements:

- dense/default sizing
- sticky header option
- zebra option
- empty-state slot
- loading-state slot
- numeric alignment helper

### `src/components/ui/Badge.svelte`

Shared status/urgency badge.

Requirements:

- urgency variants: expired, today, alert, soon, normal
- semantic variants: success, warning, error, info, neutral
- compact size

### `src/components/ui/Alert.svelte`

Shared feedback block.

Variants:

- success
- error
- warning
- info

### `src/components/ui/EmptyState.svelte`

Used when a page has no data or no matching filters.

Requirements:

- title
- body
- optional action slot
- optional icon

### `src/components/ui/LoadingState.svelte`

Used for initial loading and table loading rows.

Requirements:

- skeleton option
- text fallback
- reduced-motion safe shimmer or no shimmer

### `src/components/ui/Tabs.svelte`

Shared tabs for detail panels.

Requirements:

- keyboard navigable
- active state
- no hard-coded text

### `src/components/ui/Select.svelte` and `src/components/ui/Listbox.svelte`

Shared replacements for native-looking dropdowns and browser-default select surfaces.

Requirements:

- DaisyUI visual styling,
- keyboard navigation,
- visible focus state,
- screen-reader label support,
- compact and default density,
- safe fallback to native `<select>` only if a custom listbox would reduce accessibility.

Used by:

- language selector,
- theme selector,
- report filters,
- unit/category style selectors where appropriate.

### `src/components/ui/Tooltip.svelte`

Designed replacement for important browser `title=` hints.

Requirements:

- focus and hover activation,
- no essential information hidden only behind hover,
- i18n text,
- reduced-motion-safe entry.

### `src/components/ui/ToastHost.svelte`

Replace repeated inline transient success/error messages after the core visual primitives are stable. Persistent validation errors remain inline near the field/action.

## Migration phases

Each phase should be a reviewable work unit with its own commit. If the diff grows too large, split by page, not by random CSS changes.

### Phase 0 — Baseline and branch setup

Goal: capture the current UI state before visual changes.

Tasks:

- create screenshots or written baseline notes for the main screens:
  - Dashboard
  - Stores
  - Products
  - Product detail
  - Calendar
  - Reports
  - CSV Import
  - Backup/Restore
  - Configuration
  - Unit Review flows if reachable
- note viewport sizes:
  - desktop: 1440px
  - tablet-ish: 1024px
  - narrow: 720px

Acceptance:

- baseline artifact exists
- no UI code changed

### Phase 1 — Install and configure Tailwind + DaisyUI

Goal: add the styling foundation without changing the whole UI.

Tasks:

- install Tailwind/DaisyUI dependencies
- wire Tailwind into Vite
- create `src/app.css`
- define the initial DaisyUI light theme
- import the new CSS entry from `src/main.ts`
- verify existing UI still renders

Acceptance:

- `npm run check` passes
- `npm run build` passes
- no obvious global visual breakage from reset/preflight
- if reset causes widespread breakage, stop and fix foundation before page migration

### Phase 2 — Shared primitive components

Goal: create the UI primitives before page migration.

Tasks:

- add `src/components/ui/Button.svelte`
- add `src/components/ui/Card.svelte`
- add `src/components/ui/Badge.svelte`
- add `src/components/ui/Alert.svelte`
- add `src/components/ui/EmptyState.svelte`
- add `src/components/ui/LoadingState.svelte`
- add `src/components/ui/Table.svelte`
- add `src/components/ui/Modal.svelte`
- add `src/components/ui/Tabs.svelte` if needed by first migrated pages

Acceptance:

- primitives compile
- primitives do not duplicate visible strings outside i18n
- primitives expose slots/props rather than owning business text
- no page migrated yet unless needed for smoke usage

### Phase 3 — App shell and navigation

Goal: make the app immediately feel more polished while preserving tab behavior.

Files:

- `src/App.svelte`

Tasks:

- migrate top nav to DaisyUI/button primitive styling
- improve active state
- add responsive behavior for small widths
- keep current tab state behavior
- optionally add subtle view transition with reduced-motion guard

Acceptance:

- all existing tabs remain reachable
- active tab is obvious
- Spanish labels fit without awkward wrapping at common desktop widths
- keyboard navigation remains usable

### Phase 4 — Dashboard polish

Goal: improve the most visible and highest-value screen.

Files likely involved:

- `src/components/DashboardPage.svelte`
- shared UI primitives

Tasks:

- convert urgency cards to `Card`/DaisyUI stats pattern
- centralize urgency badge styling
- improve scan row spacing and hierarchy
- convert lot table to shared `Table`
- improve loading/empty/error states
- keep all existing filters and actions

Acceptance:

- scan/search flow still works
- urgency filters still work
- row actions still open the correct dialogs
- empty/loading/error states are visually consistent

### Phase 5 — Modal and action dialog migration

Goal: unify modal appearance and accessibility.

Files likely involved:

- `src/components/MoveStockModal.svelte`
- `src/components/RegisterExitModal.svelte`
- `src/components/AdjustCountModal.svelte`
- `src/components/ArchiveLotDialog.svelte`
- `src/components/ResolveQuantityDialog.svelte`
- modal usage inside dashboard/calendar/product detail screens

Tasks:

- migrate each modal shell to shared `Modal`
- standardize header/footer/action button layout
- preserve form validation behavior
- preserve Escape/cancel semantics
- improve focus handling

Acceptance:

- every modal opens/closes correctly
- destructive actions remain visually distinct
- keyboard-only usage is not worse than before
- no business flow changes

### Phase 6 — Forms and configuration pages

Goal: make forms consistent and easier to scan.

Files likely involved:

- `src/components/ProductForm.svelte`
- `src/components/LotForm.svelte`
- `src/components/ConfigurationPage.svelte`
- `src/components/ReportsPage.svelte` filters
- `src/components/BackupRestorePage.svelte`
- `src/components/CsvImportPage.svelte`

Tasks:

- migrate inputs/selects/toggles to DaisyUI classes through local wrappers where useful
- standardize required marker rendering
- standardize field help/error text
- keep all labels in i18n catalogs

Acceptance:

- form submit behavior unchanged
- validation messages still localized
- locale selector behavior unchanged
- required fields are visually and semantically clear

### Phase 7 — Tables and data-dense screens

Goal: normalize data presentation.

Files likely involved:

- `src/components/ReportsPage.svelte`
- `src/components/CsvImportPage.svelte`
- `src/components/LotMovementsPanel.svelte`
- `src/components/CalendarPage.svelte`
- `src/components/ProductCatalogPage.svelte`
- `src/components/StoresPage.svelte`

Tasks:

- migrate table wrappers to shared `Table`
- standardize hover/zebra/sticky header behavior
- align numeric columns
- improve table empty/loading states
- keep CSV preview readable

Acceptance:

- no data column lost
- sorting/filtering behavior unchanged where present
- large tables remain readable

### Phase 8 — Calendar and custom widget polish

Goal: restyle complex custom widgets without rewriting behavior.

Files likely involved:

- `src/components/CalendarMonth.svelte`
- `src/components/CalendarPage.svelte`
- `src/components/DatePicker.svelte`
- `src/components/inputs/CategoryPicker.svelte`

Tasks:

- restyle calendar cells, selected state, today state, and day badges
- improve date picker visual states
- improve category picker dropdown styling
- keep existing keyboard interactions
- increase touch target sizes where needed

Acceptance:

- DatePicker keyboard behavior still works
- CalendarMonth navigation still works
- CategoryPicker selection and outside-click behavior still works
- Spanish text still fits

### Phase 9 — Toasts and global feedback

Goal: replace scattered inline success/error messages with consistent feedback.

Files likely involved:

- new toast primitive
- pages with repeated `successMsg`/`errorMsg` patterns

Tasks:

- add toast host
- migrate success/error message flows page by page
- keep persistent validation errors inline where they affect form correction

Acceptance:

- transient successes use toasts
- actionable validation errors remain close to the field/action
- messages remain localized

### Phase 10 — Responsive pass

Goal: ensure the redesign works outside a wide desktop viewport.

Tasks:

- tune breakpoints around 1024px, 720px, 480px
- make nav collapse or wrap gracefully
- convert wide tables to scrollable wrappers or cards where appropriate
- make modals usable on narrow screens
- check calendar and forms on narrow widths

Acceptance:

- no horizontal page overflow except intentional table wrappers
- primary actions remain reachable
- modals fit within viewport
- nav remains usable

### Phase 11 — Motion and micro-interactions

Goal: add polish without distracting from work.

Tasks:

- subtle page/view fade
- button hover/press transitions
- modal fade/scale transition
- skeleton loading where useful
- respect `prefers-reduced-motion`

Acceptance:

- reduced-motion users get no unnecessary animation
- animations do not slow common workflows
- no flashing/pulsing except a deliberate, limited expired/urgent indicator if approved

### Phase 12 — Final cleanup

Goal: remove old visual debt.

Tasks:

- delete unused component-scoped CSS
- delete or repurpose old `src/style.css`
- centralize urgency/status mapping if still duplicated
- verify no old one-off button/card/table styles remain without reason
- update docs if the design system needs contributor guidance

Acceptance:

- no obvious duplicate CSS patterns remain
- build/check pass
- design-system usage is documented enough for future work

## Suggested implementation branch strategy

Use one long-lived feature branch for the full redesign effort:

- `feat/daisyui-redesign`

Within it, commit every phase as a separate work unit:

- `build: add tailwind and daisyui foundation`
- `feat(ui): add shared design primitives`
- `feat(ui): redesign app shell navigation`
- `feat(ui): polish dashboard layout`
- `feat(ui): unify modal styling`
- `feat(ui): standardize forms with daisyui`
- `feat(ui): standardize data tables`
- `feat(ui): polish calendar widgets`
- `feat(ui): add toast feedback system`
- `feat(ui): improve responsive layouts`
- `feat(ui): add reduced-motion-safe transitions`
- `refactor(ui): remove legacy scoped style duplication`

If review size becomes too large, split the final delivery into chained PRs from this plan, but keep the plan as the controlling artifact.

## Verification gates

Run these at minimum after each implementation phase:

```bash
npm run check
npm run build
```

Run these whenever i18n catalog files change:

```bash
npm run i18n:generate
npm run check
```

Manual smoke after visual phases:

- app launches in Tauri/dev browser
- language selector still works
- Dashboard loads and actions open/close dialogs
- product creation/edit flow still works
- lot creation/edit flow still works
- scan/search still works
- reports preview/export UI still works
- CSV import preview remains readable
- backup/restore actions remain clear
- date picker keyboard behavior preserved
- category picker keyboard/outside-click behavior preserved
- modals are usable with keyboard only
- narrow viewport does not create broken layout

## Accessibility checklist

Every phase should preserve or improve:

- visible focus states
- keyboard reachability
- Escape behavior for dialogs/popovers
- correct button vs link semantics
- sufficient color contrast
- non-color-only status communication
- reduced-motion support
- form labels and error association
- i18n-safe layouts for longer Spanish strings

## Known risks

### Tailwind preflight/global reset

Tailwind can change default element styling. The first implementation phase must detect whether existing buttons, headings, inputs, and tables shift unexpectedly.

Mitigation:

- add foundation first
- inspect main screens before migrating components
- stop if global reset causes widespread breakage

### Half-migrated UI

A partial migration can leave mixed visual systems.

Mitigation:

- create shared primitives before broad migration
- migrate by full screens
- delete old CSS as each screen converts
- keep this document as the controlling checklist

### Complex widget regressions

Calendar, DatePicker, and CategoryPicker carry custom behavior.

Mitigation:

- restyle before rewriting
- manually verify keyboard behavior
- avoid replacing logic in the visual redesign PR

### i18n regressions

Visual labels and new empty/loading/toast copy require catalog entries.

Mitigation:

- every new visible string goes through `src/i18n/en/index.ts` and `src/i18n/es/index.ts`
- run i18n generation/checks whenever copy changes

### Review size

The full redesign could touch many files.

Mitigation:

- commit by phase
- if PR size becomes too large, open chained PRs while preserving the same plan
- never mix backend/business changes into visual phases

## Recommended next action

After this planning branch is reviewed, create the implementation branch:

```bash
git checkout main
git pull --ff-only
git checkout -b feat/daisyui-redesign
```

Then start at Phase 0 and proceed phase by phase.

Do not begin with isolated button color tweaks. Begin with the DaisyUI foundation and shared primitives so the redesign remains coherent.
