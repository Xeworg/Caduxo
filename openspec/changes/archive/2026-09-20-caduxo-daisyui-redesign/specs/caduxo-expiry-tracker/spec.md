# Delta for Caduxo Expiry Tracker

This delta adds the visual redesign layer to `caduxo-expiry-tracker`:
the Tailwind CSS + DaisyUI design-system foundation, multi-theme support,
the shared UI primitive library, the native-control replacement policy,
the motion/effects inventory, and the per-page migration map. Existing
business behaviour, accessibility contracts for `DatePicker` /
`CalendarMonth` / `CategoryPicker`, and the `typesafe-i18n` discipline
are preserved. The change is delivered as chained PRs, one phase per PR
by default, with a 400-line review budget per PR.

Per the proposal (`auto` execution mode), the following tactical choices
are locked at proposal level and are not re-litigated in this spec:

- v1 GA theme set is `caduxo-light` (custom brand) + DaisyUI built-in
  `dark`. Accent themes (`corporate`, `dim`, `winter`, etc.) are deferred
  to a post-v1 follow-up.
- A theme switcher is shipped in v1 on the Configuration page.
- A shared `Popover.svelte` primitive extraction is **deferred** to a
  follow-up. v1 restyles `DatePicker` and `CategoryPicker` in place.
- An `urgency pulse` animation is approved for the *expired* variant
  only (badge + leading card dot), opacity-only, gated on
  `prefers-reduced-motion: no-preference`.
- A `ToastHost.svelte` primitive is **deferred** to a follow-up. v1
  keeps inline success / error patterns.

## ADDED Requirements

### Capability: Design system foundation

#### Requirement: Tailwind v4 + DaisyUI dependency stack wired into Vite

The project MUST depend on `tailwindcss` (v4), `@tailwindcss/vite`, and
`daisyui`. `@tailwindcss/vite` MUST be registered alongside
`@sveltejs/vite-plugin-svelte` in `vite.config.ts`. Vite dev and
`npm run build` MUST both succeed with the plugin chain present, Svelte
HMR MUST continue to work, and no console errors MAY be introduced by
the plugin order.

#### Scenario: vite dev and build both pass with the new plugin chain

- GIVEN the dependencies are installed and `vite.config.ts` registers
  `@tailwindcss/vite` before `svelte()`
- WHEN `npm run build` and `npm run dev` are executed
- THEN both commands exit successfully
- AND the Svelte HMR overlay reports no plugin-order errors

#### Requirement: project stylesheet imports Tailwind and the DaisyUI plugin

A new `src/app.css` MUST exist and MUST contain `@import "tailwindcss";`
followed by `@plugin "daisyui";` plus the project's custom theme blocks
and any global motion / accessibility utilities. `src/main.ts` MUST
import `src/app.css` and MUST NOT import the legacy `./style.css`.

#### Scenario: src/app.css is the active stylesheet entry

- GIVEN the project after this slice
- WHEN `src/main.ts` is read
- THEN it imports `./app.css`
- AND `grep -R 'style.css' src/main.ts` returns zero matches

#### Scenario: Tailwind preflight does not regress the unmigrated UI

- GIVEN the Tailwind + DaisyUI plugin is registered and no component
  has been migrated yet
- WHEN the app is launched in a Tauri / dev browser shell and the main
  screens (Dashboard, Stores, Products, Product detail, Calendar,
  Reports, CSV Import, Backup/Restore, Configuration, Unit Review) are
  visually inspected
- THEN preflight MAY shift default heading, list, button, input, or
  table styling
- AND no widespread breakage (illegible text, broken layouts,
  unusable controls) is present
- AND if widespread breakage appears, the foundation phase pauses
  before any page migration

#### Requirement: components reference DaisyUI theme tokens, not inline colour literals

Every visual primitive and migrated component MUST source its colour
values from DaisyUI theme tokens (`primary`, `secondary`, `accent`,
`neutral`, `base-100`, `base-200`, `base-300`, `info`, `success`,
`warning`, `error`, plus the corresponding `-content` tokens). No
migrated component MAY inline a hex / rgb colour that is not theme-
derived. This requirement applies to migrated surfaces; vestigial code
in archived legacy folders is out of scope.

#### Scenario: no hex / rgb colour literals remain in migrated surfaces

- GIVEN the migration has finished for a given component
- WHEN the component source is grepped for hex literals matching
  `#[0-9a-fA-F]{3,8}\b` and rgb / rgba literals
- THEN the grep returns zero matches inside that component file
- AND every colour reference resolves through a DaisyUI token or a
  Tailwind utility backed by a token

#### Requirement: legacy src/style.css retired after migration

The legacy `src/style.css` MUST be retired once the migration
confirms no production component references its classes. Until that
confirmation is in place, the file MAY remain; once the migration is
complete, the file MUST be deleted and `src/main.ts` MUST NOT import
it.

#### Scenario: legacy stylesheet is removed after migration

- GIVEN every migrated component no longer references any class
  declared in `src/style.css`
- WHEN `git grep -E '\.(shell|hero|eyebrow|cards)\b' src/` is executed
- THEN the command returns zero matches
- AND `src/style.css` is removed from the repository

### Capability: Theme support

#### Requirement: custom `caduxo-light` brand theme

A custom DaisyUI theme named `caduxo-light` MUST exist. The theme's
`primary` MUST be derived from the existing Caduxo brand blue
`#2563eb` (or its close semantic equivalent), the `base-100` family
MUST be derived from the existing base background `#f6f8fb`, and the
semantic `info` / `success` / `warning` / `error` tokens MUST be
derived from the existing red / amber / green / blue palette Caduxo
already uses for urgency and feedback. `caduxo-light` MUST be the
default theme when no other choice is in effect.

#### Scenario: caduxo-light resolves to brand-aligned tokens

- GIVEN the DaisyUI plugin is registered with the `caduxo-light` block
- WHEN a migrated component renders a `btn-primary`
- THEN the rendered primary colour family derives from `#2563eb`
- AND when the component renders a surface (`bg-base-100`), the
  background derives from `#f6f8fb`

#### Requirement: shipped theme set at v1 GA

The v1 GA build MUST bundle exactly two themes: `caduxo-light` (custom
brand) and DaisyUI built-in `dark`. No other DaisyUI built-in theme
MUST be enabled by default in v1. Accent themes are deferred to a
post-v1 follow-up change.

#### Scenario: only the curated theme set is bundled

- GIVEN the project builds for production
- WHEN the DaisyUI plugin configuration is read
- THEN the `themes` list contains exactly `["caduxo-light", "dark"]`
- AND no `cupcake`, `corporate`, `business`, `emerald`, `night`,
  `dim`, `nord`, or other DaisyUI built-in is enabled

#### Requirement: theme switcher on Configuration page

The Configuration page MUST expose a theme switcher adjacent to the
existing language selector. The switcher MUST display the available
themes by name in the active locale, MUST indicate which theme is
currently applied, and MUST allow the user to pick a different theme.
Switching a theme MUST update the rendered surface optimistically and
MUST persist the choice to `app_settings.theme` via the existing
`updateSettings` IPC command.

#### Scenario: switcher lists the curated themes and shows the active one

- GIVEN the active theme is `caduxo-light`
- WHEN the user opens the Configuration page
- THEN the theme switcher lists `caduxo-light` and `dark`
- AND `caduxo-light` is marked as the currently active selection
- AND the labels are rendered in the active locale

#### Scenario: switching a theme applies it optimistically and persists it

- GIVEN the user opens the Configuration page
- WHEN the user selects `dark` from the theme switcher
- THEN the rendered UI re-themes to `dark` without a page reload
- AND a subsequent `app_settings.theme` read returns `dark`
- AND a relaunch of the application restores `dark`

#### Scenario: failed theme persistence rolls back the optimistic switch

- GIVEN the user opens the Configuration page with theme `caduxo-light`
- WHEN the user selects `dark` and `updateSettings` fails on the
  backend
- THEN the rendered UI returns to `caduxo-light`
- AND `app_settings.theme` is unchanged on the next read
- AND the user sees an inline error in the switcher describing the
  failure

#### Requirement: theme persistence via app_settings.theme

The active theme MUST be persisted in the existing `app_settings`
table using the same read / write pattern already used for the
`language` row. No new Tauri command surface outside the existing
`settings` command group is introduced. The persisted value MUST be
exactly one of the bundled theme names (`caduxo-light` or `dark`); any
other value MUST be rejected.

#### Scenario: theme is read from app_settings on launch

- GIVEN `app_settings.theme` exists with value `dark`
- WHEN the application launches
- THEN the active theme is `dark` without consulting the OS preference

#### Scenario: an unsupported stored theme value falls back to caduxo-light

- GIVEN `app_settings.theme` exists with value `synthwave` (not in
  the v1 curated set)
- WHEN the application launches
- THEN the active theme is `caduxo-light`
- AND no error is surfaced to the user

#### Requirement: OS-aware theme defaulting via prefers-color-scheme

When no `app_settings.theme` row exists, the bootstrap MUST read the
host's `prefers-color-scheme` and select `dark` when the value is
`dark` and `caduxo-light` otherwise. The OS preference MUST be re-read on
every app start; no `localStorage` cache may shadow the persisted
setting.

#### Scenario: no stored theme, OS prefers dark

- GIVEN `app_settings.theme` does not exist
- AND the host reports `prefers-color-scheme: dark`
- WHEN the application launches
- THEN the active theme is `dark`

#### Scenario: no stored theme, OS prefers light or no preference

- GIVEN `app_settings.theme` does not exist
- AND the host reports `prefers-color-scheme: light` or the media
  query is unavailable
- WHEN the application launches
- THEN the active theme is `caduxo-light`

#### Scenario: stored theme wins over OS preference

- GIVEN `app_settings.theme` exists with value `caduxo-light`
- AND the host reports `prefers-color-scheme: dark`
- WHEN the application launches
- THEN the active theme is `caduxo-light` and the OS preference is
  not consulted

#### Requirement: theme precedence is manual > persisted > OS > caduxo-light

When multiple sources of truth for the active theme are present, the
precedence MUST be: an in-session manual override (if any) beats a
persisted `app_settings.theme` row; the persisted row beats the host
`prefers-color-scheme`; the OS preference beats the literal
`caduxo-light` fallback. The Configuration page's theme switcher MUST
show which source currently determines the active theme so the user
can tell why the theme they see is the theme they see.

#### Scenario: switcher shows the source of the active theme

- GIVEN `app_settings.theme` does not exist
- AND the host reports `prefers-color-scheme: dark`
- WHEN the user opens the Configuration page
- THEN the switcher marks `dark` as active
- AND the switcher labels the source as the OS preference

#### Requirement: readable contrast and unbroken layouts in every shipped theme

Every main surface (Dashboard, Stores, Products, Product detail,
Calendar, Reports, CSV Import, Backup/Restore, Configuration, Unit
Review, plus every modal dialog) MUST remain visually legible and
unbroken in both `caduxo-light` and `dark`. Legibility means
WCAG-AA-readable text / background contrast on body text, headings,
inputs, badges, alerts, and primary button labels. Layout integrity
means no overflow, no clipped content, no overlap, no broken modal
backdrop, and no hidden focus ring in either theme.

#### Scenario: dark theme passes a manual contrast and layout pass on the main surfaces

- GIVEN the active theme is `dark`
- WHEN the main surfaces are manually inspected
- THEN no body / heading / input / badge text falls below WCAG-AA
  contrast against its background
- AND no layout is broken (no overflow, no clipping, no overlap)
- AND every focus ring remains visible

#### Scenario: caduxo-light theme passes the same pass

- GIVEN the active theme is `caduxo-light`
- WHEN the main surfaces are manually inspected
- THEN no body / heading / input / badge text falls below WCAG-AA
  contrast against its background
- AND no layout is broken
- AND every focus ring remains visible

### Capability: Shared UI primitives

The primitives below live under `src/components/ui/`. They are the only
canonical Svelte components for their respective surfaces in v1; ad-hoc
local classes that duplicate this responsibility MUST be retired as
their owning surfaces migrate.

#### Requirement: shared Button.svelte wraps DaisyUI button variants

A `src/components/ui/Button.svelte` primitive MUST exist and MUST
expose variants for `primary`, `secondary`, `ghost`, `outline`,
`danger`, `warning`, `success`, `link`, and `icon`. The primitive
MUST support a `disabled` state, a `loading` state, optional leading
or trailing icon slots, an accessible label for icon-only usage, and
consistent focus / hover / active / disabled treatment across all
variants.

#### Scenario: every variant renders a DaisyUI-themed button

- GIVEN a screen uses `<Button variant="primary">`, `<Button
  variant="danger">`, and `<Button variant="icon" aria-label="Close">`
- WHEN the screen is rendered
- THEN each instance renders the corresponding DaisyUI `btn-*` class
- AND the icon-only button carries the `aria-label`

#### Scenario: loading state shows a DaisyUI spinner and disables interaction

- GIVEN a submit button is in `loading` state
- WHEN the screen renders
- THEN the button contains the DaisyUI `loading loading-spinner`
- AND the button cannot be re-activated until `loading` clears

#### Requirement: shared Card.svelte with optional header/footer slots

A `src/components/ui/Card.svelte` primitive MUST exist with consistent
padding, border, shadow, and optional header / footer slots. The
primitive MUST be the only canonical card surface for urgency cards,
report type cards, settings sections, backup / import result
summaries, and product / store detail panels in v1.

#### Scenario: every migrated card surface uses the Card primitive

- GIVEN the migration has finished for a given screen that renders a
  card-like surface
- WHEN the screen source is grepped for the legacy `.urgency-card`,
  `.settings-section`, or one-off `.card` rules
- THEN the grep returns zero matches in that screen
- AND the screen composes the Card primitive instead

#### Requirement: shared Modal.svelte built on native dialog

A `src/components/ui/Modal.svelte` primitive MUST exist and MUST be
backed by the native `<dialog>` element driven by `dialog.showModal()`
to obtain native focus trap, native Escape handling, and a themed
backdrop. The primitive MUST support size variants (`small`, `default`,
`wide`), an optional close button, click-outside-to-close when allowed,
and focus restoration to the element that opened it.

#### Scenario: modal opens with native focus trap and Escape closes it

- GIVEN a user activates the trigger for any migrated modal
- WHEN the modal opens
- THEN focus is trapped inside the modal
- AND pressing `Escape` closes the modal
- AND focus returns to the trigger element after the modal closes

#### Scenario: click-outside closes the modal when allowed

- GIVEN a modal is open and click-outside-to-close is allowed
- WHEN the user clicks on the backdrop
- THEN the modal closes
- AND focus returns to the trigger element

#### Requirement: shared Table.svelte with sticky / zebra / alignment helpers

A `src/components/ui/Table.svelte` primitive MUST exist and MUST
expose `zebra`, `sticky header`, `dense`, `default` sizing, a numeric
alignment helper, an `empty-state` slot, and a `loading-state` slot.
The primitive MUST be the canonical table wrapper for every table in
the migrated app.

#### Scenario: shared Table covers all migrated table surfaces

- GIVEN the migration has finished for the dashboard lot table,
  LotMovementsPanel, ReportsPage, CsvImportPage preview, StoresPage,
  and BackupRestorePage info lists
- WHEN those surfaces are inspected
- THEN each table composes the Table primitive
- AND no legacy `lot-table`, `reports-table`, `reports-empty`,
  `lot-picker`, `info-list`, `checks-list`, or `confirm-box` local
  class remains as the table wrapper

#### Requirement: shared Badge.svelte with urgency and semantic variants

A `src/components/ui/Badge.svelte` primitive MUST exist and MUST
expose urgency variants (`expired`, `today`, `alert`, `soon`,
`normal`) and semantic variants (`success`, `warning`, `error`,
`info`, `neutral`). The primitive MAY optionally render a leading
status dot.

#### Scenario: urgency badge uses semantic DaisyUI classes

- GIVEN a lot has urgency `expired`
- WHEN the dashboard renders the lot's badge
- THEN the badge renders with the semantic `badge-error` (or the
  equivalent urgency token) and the visual treatment matches every
  other urgency badge

#### Requirement: shared Alert.svelte with semantic feedback variants

A `src/components/ui/Alert.svelte` primitive MUST exist and MUST
expose `success`, `error`, `warning`, and `info` variants. Each
variant MUST render a consistent icon, text slot, and optional
action slot.

#### Scenario: every migrated feedback surface uses Alert

- GIVEN the migration has finished for UnitReviewBanner and the
  dashboard error banner
- WHEN those surfaces are inspected
- THEN each surface composes the Alert primitive
- AND no legacy `.unit-banner`, `.banner-icon`, `.banner-text`,
  `.warning-banner`, or `.error-banner` local rule remains as the
  banner surface

#### Requirement: shared EmptyState.svelte

A `src/components/ui/EmptyState.svelte` primitive MUST exist and MUST
expose a title, body text, an optional icon, and an optional action
slot. The primitive MUST be the canonical empty surface for every
migrated page.

#### Scenario: every migrated empty surface uses EmptyState

- GIVEN the migration has finished for LotMovementsPanel, Dashboard
  lot table, StoresPage store list, ReportsPage, CalendarPage
  day-detail, ProductsPage, and CSV import preview
- WHEN those surfaces render their empty state
- THEN each composes the EmptyState primitive
- AND no legacy `.empty-state` / `.empty-hint` local rule remains as
  the empty surface

#### Requirement: shared LoadingState.svelte

A `src/components/ui/LoadingState.svelte` primitive MUST exist and
MUST expose a `skeleton` row option, a `text` fallback, and a
reduced-motion-safe shimmer (or a static skeleton when reduced motion
is requested).

#### Scenario: LoadingState respects prefers-reduced-motion

- GIVEN the host reports `prefers-reduced-motion: reduce`
- WHEN any LoadingState is rendered
- THEN the rendered skeleton is static (no shimmer animation)
- AND the same loading flow reaches the same end state as a
  non-reduced-motion user

#### Requirement: shared Tabs.svelte

A `src/components/ui/Tabs.svelte` primitive MUST exist and MUST
support `tabs-bordered` or `tabs-lifted` style, an active state, and
keyboard navigation. The primitive MUST be the canonical tab strip
for the product detail page, the dashboard / calendar detail panels,
and any other tab surface in v1.

#### Scenario: tabs keyboard surface is preserved across migrated tabs

- GIVEN the user has focus inside the migrated Tabs primitive
- WHEN the user presses `ArrowLeft` / `ArrowRight`, `Home`, `End`,
  `Enter`, or `Space`
- THEN the active tab updates to the focused or activated tab
- AND the corresponding panel becomes visible

#### Requirement: shared Select.svelte and Input.svelte themed form primitives

A `src/components/ui/Select.svelte` primitive and a
`src/components/ui/Input.svelte` primitive MUST exist. The Select
primitive MUST render a DaisyUI-themed select surface (no OS-styled
chrome) with a consistent caret, focus ring, and theme-aware hover /
active state. The Input primitive MUST render a DaisyUI `input
input-bordered` with an `input-error` invalid state, paired with
`label`, `label-text`, and `label-text-alt` for label, required
marker, and helper text.

#### Scenario: locale selector on Configuration page renders the DaisyUI select

- GIVEN the Configuration page renders the locale selector
- WHEN the page is inspected
- THEN the selector composes the Select primitive
- AND no `<select>` element renders with the host operating system's
  native dropdown chrome inside the app's visual chrome

#### Requirement: shared Toggle.svelte replaces the custom toggle

A `src/components/ui/Toggle.svelte` primitive MUST exist and MUST
render the DaisyUI `toggle toggle-primary` surface. The primitive
MUST replace the bespoke `toggle-wrap` / `toggle-track` /
`toggle-thumb` toggle pattern in `ConfigurationPage.svelte` and any
other migrated location.

#### Scenario: Configuration page toggle renders the DaisyUI toggle

- GIVEN the Configuration page renders the settings toggles
- WHEN the page source is grepped for `.toggle-wrap`, `.toggle-track`,
  and `.toggle-thumb`
- THEN the grep returns zero matches
- AND the rendered toggle carries the DaisyUI `toggle` class

#### Requirement: shared Tooltip.svelte keyboard-reachable

A `src/components/ui/Tooltip.svelte` primitive MUST exist and MUST
be activated by both hover and keyboard focus. The primitive MUST
be used wherever the existing app relies on the browser's native
`title=` attribute for icon-only buttons or otherwise essential
information, and it MUST render via DaisyUI `tooltip` classes.

#### Scenario: icon-only button exposes a keyboard-reachable tooltip

- GIVEN an icon-only button (for example the UnitReviewBanner
  dismiss button or a calendar navigation arrow) is rendered through
  the migrated surface
- WHEN the button receives keyboard focus
- THEN the tooltip text becomes visible
- AND the tooltip text is rendered in the active locale

### Capability: Native control replacement

#### Requirement: Configuration page locale select uses the DaisyUI Select primitive

The Configuration page's locale `<select>` MUST be replaced by the
`Select.svelte` primitive. The replacement MUST preserve all
existing locale choices, persist the selection through the existing
`app_settings.language` flow, default to the host's detected locale
when no row is on file, and behave identically on the existing
`hint reappears if persisted value is cleared` and `selector update
rolls back on IPC failure` scenarios carried over from the
internationalisation capability.

#### Scenario: locale selector no longer renders OS-styled chrome

- GIVEN the Configuration page renders the locale selector after
  migration
- WHEN the page is visually inspected
- THEN the selector shows a DaisyUI-themed caret and surface
- AND no operating-system dropdown chrome (Windows / macOS / GTK)
  appears inside the app's visual chrome

#### Requirement: ProductForm datalist inputs are wrapped with the Input primitive

The `<datalist>` autocomplete inputs in `ProductForm.svelte` (barcode
type and unit definition lists) MUST be wrapped by the `Input.svelte`
primitive for visual styling while preserving the underlying
`<datalist>` element for native keyboard and screen-reader
announcements. The wrapper MUST NOT replace the `<datalist>` markup.

#### Scenario: datalist inputs keep autocomplete semantics

- GIVEN the user opens `ProductForm.svelte` in create mode
- WHEN the barcode type and unit definition fields render
- THEN each field is wrapped by the Input primitive
- AND each underlying `<datalist>` element remains present with the
  same `id` and option list
- AND typing in the field continues to surface autocomplete
  suggestions

#### Requirement: text inputs themed via DaisyUI input across all forms

Every bare `<input type="text">` in the migrated components
(`ProductForm`, `LotForm`, `StoresPage`, `BackupRestorePage`,
`ConfigurationPage`, `ResolveQuantityDialog`, etc.) MUST render via
the `Input.svelte` primitive. The primitive MUST render `input
input-bordered`, MUST apply `input-error` when the host marks the
field invalid, MUST pair with `label` / `label-text` / `label-text-alt`
for label, required marker, and helper text, and MUST keep all
existing validation behaviour intact.

#### Scenario: every migrated text input goes through the Input primitive

- GIVEN the migration has finished for `ProductForm`, `LotForm`,
  `StoresPage`, `BackupRestorePage`, `ConfigurationPage`, and
  `ResolveQuantityDialog`
- WHEN those source files are grepped for `<input type="text"` and
  the legacy `.form-group` / `.input` / `.field-label` /
  `.small-label` local classes
- THEN the grep returns zero matches for `<input type="text"` outside
  the Input primitive
- AND the legacy local classes no longer act as the input surface on
  those files

#### Requirement: checkboxes and radios themed via DaisyUI

Plain `<input type="checkbox">` controls in `ProductForm`, `LotForm`,
`ReportsPage`, and any other migrated form MUST render through
DaisyUI `checkbox checkbox-primary checkbox-sm`. Plain `<input
type="radio">` controls in `UnitReviewPage` (kind integer / decimal)
and any other migrated location MUST render through DaisyUI `radio
radio-primary radio-sm`. The native `<input>` element MUST remain in
the DOM for form semantics; only the visual surface changes.

#### Scenario: checkbox and radio controls keep form semantics

- GIVEN the user opens `ProductForm.svelte` after migration
- WHEN the "set as primary barcode" checkbox is rendered
- THEN the visible surface is the DaisyUI-themed checkbox
- AND the underlying `<input type="checkbox">` element remains in the
  DOM
- AND toggling the visible surface toggles the underlying input

#### Requirement: details / summary panels in UnitReviewPage use DaisyUI dropdown

The `<details>` / `<summary>` "Map to preset" and "Keep as custom"
panels in `UnitReviewPage.svelte` MUST be replaced by DaisyUI
`dropdown` / `dropdown-content` surfaces. The replacement MUST
preserve the underlying toggle behaviour and MUST use proper ARIA
roles instead of the `details` / `summary` semantics.

#### Scenario: UnitReviewPage dropdown panels keep their toggle behaviour

- GIVEN the user opens `UnitReviewPage.svelte`
- WHEN the user activates the "Map to preset" or "Keep as custom"
  panel header
- THEN the corresponding content panel expands or collapses
- AND no `<details>` or `<summary>` element remains in the DOM

#### Requirement: DatePicker popover restyled with DaisyUI dropdown / popover classes

The popover surface of `DatePicker.svelte` MUST be restyled using
DaisyUI `dropdown` / `dropdown-content` and the `popover` class. The
restyle MUST NOT change any of the keyboard ergonomics, ISO bind
contract, year-range enforcement, validation rules, or
`clearable={true | false}` semantics carried over from the canonical
`Custom date picker` requirement.

#### Scenario: DatePicker visual surface uses DaisyUI classes while behaviour is preserved

- GIVEN `DatePicker.svelte` after migration
- WHEN the popover is rendered
- THEN the popover root carries DaisyUI `dropdown` / `dropdown-content`
  classes (and any project theme overrides)
- AND every scenario in the canonical `Custom date picker`
  requirement continues to pass without modification

#### Requirement: CategoryPicker popover restyled with DaisyUI dropdown / popover classes

The popover surface of `inputs/CategoryPicker.svelte` MUST be
restyled using DaisyUI `dropdown` / `dropdown-content` and the
`popover` class. The restyle MUST preserve every keyboard contract
(arrow keys, Enter, Escape, Backspace) and every chip / pseudo-row
semantic carried over from the canonical `category picker keyboard
ergonomics` and `category picker Uncategorized pseudo-row is
selectable and rendered distinctly` requirements.

#### Scenario: CategoryPicker keyboard contract is preserved

- GIVEN `CategoryPicker.svelte` after migration
- WHEN the user navigates the popover with `ArrowDown`, `Enter`,
  `Escape`, and `Backspace`
- THEN `aria-activedescendant` moves through the result list and
  clamps at the ends, `Enter` toggles the active result, `Escape`
  closes the popover without committing typed text, and `Backspace`
  on an empty input removes the last chip
- AND the `Uncategorized` pseudo-row continues to be selectable with
  a distinct style

#### Requirement: modal dialogs migrate to native dialog class modal

Every modal surface — `MoveStockModal`, `RegisterExitModal`,
`AdjustCountModal`, `ArchiveLotDialog`, `ResolveQuantityDialog`, plus
the inline product / lot / quick-create overlays inside
`DashboardPage` — MUST be migrated to the native `<dialog
class="modal">` surface driven by `dialog.showModal()`. The
`Modal.svelte` primitive is the only canonical shell for these
surfaces in v1.

#### Scenario: every migrated modal opens, closes, traps focus, and restores focus

- GIVEN the user activates the trigger for any migrated modal
- WHEN the modal opens
- THEN focus is trapped inside the modal
- AND pressing `Escape` closes the modal
- AND focus returns to the trigger element after close
- AND no legacy `.modal-overlay` / `.modal-box` / `.modal-box-wide` /
  `.modal-header` / `.modal-body` / `.modal-footer` / `.modal-close`
  / `.modal-loading` local class remains as the shell on the migrated
  files

#### Scenario: existing Escape / cancel semantics are preserved

- GIVEN any migrated modal that previously cancelled in-progress edits
  on `Escape`
- WHEN the user presses `Escape` while the modal is open
- THEN the in-progress edits are discarded exactly as they were
  before the redesign

#### Requirement: app shell navigation uses DaisyUI navbar

The top navigation in `App.svelte` MUST render through DaisyUI
`navbar` with `navbar-start`, `navbar-center`, and `navbar-end`
slots. Narrow-viewport collapse MUST go through DaisyUI `dropdown`
so the navigation remains reachable on small widths.

#### Scenario: app shell renders the DaisyUI navbar

- GIVEN the user launches the application on a wide viewport
- WHEN the app shell renders
- THEN the brand sits in `navbar-start`, the tab buttons sit in
  `navbar-center`, and any overflow / utility actions (including the
  Configuration tab) sit in `navbar-end`
- AND no `.nav` / `.nav-btn` / `.nav-brand` local class remains as
  the navigation surface

#### Scenario: navbar collapses to a dropdown on narrow viewports

- GIVEN the user launches the application at a viewport width of
  720px or less
- WHEN the app shell renders
- THEN the tab buttons collapse behind a dropdown trigger
- AND every tab remains reachable via keyboard
- AND the active tab is still obvious

#### Requirement: no native OS-styled control surfaces remain inside the app chrome

After v1 lands, every main screen MUST show zero OS-styled controls
inside the app's visual chrome. The check is verifiable by a manual
screenshot pass on every main surface in both `caduxo-light` and
`dark`: the locale selector, every form input, every checkbox / radio
/ toggle, the date and category pickers, every modal, every list /
picker surface, and every tooltip must render through Caduxo's
visual vocabulary.

#### Scenario: screenshot pass finds zero OS-styled controls

- GIVEN v1 is shipped
- WHEN every main surface is screenshotted in `caduxo-light` and
  `dark` and the screenshots are reviewed for operating-system
  chrome
- THEN no Windows / macOS / GTK dropdown chrome, native checkbox or
  radio surface, native input border, or native tooltip appears
  inside the app's visual chrome

### Capability: Visual surface redesign

#### Requirement: dashboard redesign through shared primitives

The Dashboard MUST be migrated to use the shared primitives
(`Card`, `Badge`, `Button`, `Table`, `EmptyState`, `LoadingState`,
`Alert`, `Modal`) for every visible surface. The migration MUST
preserve every existing canonical scenario under `Capability:
Dashboard` (sections align with the matching filter buttons,
category filter ANY-of semantics, urgency-card bucket counts,
quick-filter row predicates, no separate movement widget in v1,
etc.) and MUST preserve scan / search, urgency filters, row actions,
and the existing modal triggers.

#### Scenario: dashboard preserves every canonical Dashboard scenario

- GIVEN the Dashboard is migrated
- WHEN every scenario under the canonical `Capability: Dashboard`
  is exercised after migration
- THEN each scenario continues to pass without modification

#### Scenario: dashboard surfaces render through shared primitives

- GIVEN the migration has finished for `DashboardPage.svelte`
- WHEN the file is grepped for the legacy `.urgency-card`,
  `.urgency-badge`, `.status-*`, `.lot-table`, `.error-banner`,
  `.tab-btn`, `.detail-tabs`, `.tab-content`, `.loading`,
  `.loading-row`, `.scan-spinner` local classes
- THEN the grep returns zero matches as the active surface class
  on that file

#### Requirement: forms redesign through shared primitives

`ProductForm.svelte`, `LotForm.svelte`, `ConfigurationPage.svelte`,
`ReportsPage.svelte` filters, `BackupRestorePage.svelte`, and
`CsvImportPage.svelte` MUST migrate to the shared `Input`, `Select`,
`Toggle`, `Button`, `Alert`, and field-label primitives. The
migration MUST preserve every existing canonical validation
behaviour, every existing canonical i18n flow, every existing
canonical locale selector behaviour, and every existing canonical
required-marker semantics.

#### Scenario: every migrated form preserves its canonical contract

- GIVEN the form migrations are complete
- WHEN each migrated form is exercised against its canonical
  scenarios (e.g., `product picks a preset unit`, `product creates a
  custom unit inline`, `lot creation with auto-generated batch`,
  `locale selector update rolls back on IPC failure`)
- THEN each scenario continues to pass without modification

#### Requirement: data tables redesign through shared primitives

The dashboard lot table, `LotMovementsPanel`, `ReportsPage`,
`CsvImportPage` preview, `StoresPage`, `BackupRestorePage` info
lists, `ConfigurationPage` info lists, and `CalendarPage` day-detail
panel MUST migrate to the shared `Table.svelte` primitive. The
migration MUST preserve every existing canonical data contract: no
column is lost, no sort / filter behaviour is altered, the CSV
import preview remains readable, the calendar day-detail panel
continues to render one row per active lot with the same columns.

#### Scenario: every migrated table preserves its canonical contract

- GIVEN the table migrations are complete
- WHEN each migrated table is exercised against its canonical
  scenarios
- THEN each scenario continues to pass without modification

#### Requirement: empty and loading states use shared primitives

Every migrated empty and loading surface MUST render through
`EmptyState.svelte` and `LoadingState.svelte` instead of the
existing scattered `<p class="loading">`, `<p class="loading-row">`,
`<span class="loading-msg">`, `<p class="empty-hint">` patterns.

#### Scenario: every migrated empty and loading surface uses the shared primitives

- GIVEN the migration is complete
- WHEN every migrated surface is grepped for the legacy
  `.loading`, `.loading-row`, `.loading-msg`, `.empty-state`, and
  `.empty-hint` local rules as the active empty / loading surface
- THEN the grep returns zero matches on those files

#### Requirement: responsive pass at 1024 / 720 / 480 px

The migrated UI MUST remain usable and unbroken at viewport widths
of 1024, 720, and 480 pixels. The pass MUST cover the navbar
collapse (see the navbar requirement above), wide-table horizontal
scroll wrappers, modal fit on narrow screens, and the calendar and
forms on narrow widths.

#### Scenario: responsive pass has no horizontal page overflow

- GIVEN the user resizes the viewport to 1024, 720, and 480 pixels in
  turn
- WHEN each main surface is rendered
- THEN no horizontal page overflow appears (intentional table
  wrappers excepted)
- AND every primary action remains reachable
- AND every modal fits within the viewport
- AND the navbar remains usable

### Capability: Motion and effects

#### Requirement: motion token inventory with named durations and easings

A shared motion-token inventory MUST exist and MUST be the single
source of truth for every animated surface in v1. The inventory MUST
define at minimum the durations and easings used by: default hover /
focus transitions, modal fade + scale, dropdown / popover slide-in,
card hover lift, button press feedback, skeleton shimmer, urgency
pulse, and backdrop blur. Every component animation MUST source its
duration and easing from this inventory rather than from a hand-
written literal.

#### Scenario: every animated surface resolves through the motion-token inventory

- GIVEN the motion-token inventory is defined
- WHEN the codebase is grepped for `transition: ... 0\.[0-9]+s` or
  `animation: ... 0\.[0-9]+s` literals on migrated files
- THEN the grep returns zero matches as the source of an animation
  on migrated files
- AND every animated surface resolves its duration and easing
  through a token

#### Requirement: prefers-reduced-motion guard on every animated surface

Every animated surface in v1 MUST be wrapped in
`@media (prefers-reduced-motion: no-preference) { ... }` (or its
Tailwind equivalent: the `motion-safe:` variant). When the host
reports `prefers-reduced-motion: reduce`, the surface MUST still
work — it MUST just not animate. The guard applies to the default
hover / focus transitions, modal show / hide, dropdown / popover
slide-in, card hover lift, button press feedback, skeleton shimmer,
urgency pulse, and backdrop blur.

#### Scenario: reduced-motion users reach the same end state without animation

- GIVEN the host reports `prefers-reduced-motion: reduce`
- WHEN the user opens every modal, every dropdown / popover, every
  skeleton-loading surface, and every hover / focus surface
- THEN no animation runs on that surface
- AND the same end state is reached as for a non-reduced-motion user

#### Requirement: modal fade and scale motion

Modal show / hide MUST render a fade + scale transition (~150 ms)
sourced from the motion-token inventory. The transition MUST be
disabled by the `prefers-reduced-motion` guard.

#### Scenario: modal show / hide is animated when motion is allowed

- GIVEN the host reports `prefers-reduced-motion: no-preference`
- WHEN the user opens or closes any migrated modal
- THEN a fade + scale transition runs once
- AND its duration and easing come from the motion-token inventory

#### Requirement: dropdown and popover slide-in motion

DaisyUI `dropdown` / `dropdown-content` open / close MUST render a
subtle slide-in (~150 ms) sourced from the motion-token inventory.
The transition MUST be disabled by the `prefers-reduced-motion`
guard.

#### Scenario: dropdown / popover slide-in is animated when motion is allowed

- GIVEN the host reports `prefers-reduced-motion: no-preference`
- WHEN the user opens any migrated dropdown / popover
- THEN a slide-in transition runs once
- AND its duration and easing come from the motion-token inventory

#### Requirement: card hover lift motion

Urgency cards and report type cards MUST render a card hover lift
(`hover:-translate-y-0.5 hover:shadow-lg`) sourced from the
motion-token inventory. The translation MUST be disabled by the
`prefers-reduced-motion` guard; the shadow MAY remain.

#### Scenario: card hover lift runs only when motion is allowed

- GIVEN the user hovers an urgency or report-type card
- WHEN the host reports `prefers-reduced-motion: no-preference`
- THEN the card lifts and the shadow increases
- WHEN the host reports `prefers-reduced-motion: reduce`
- THEN the card does not translate but the hover affordance remains
  visually distinct

#### Requirement: button press feedback motion

Buttons MUST render an `active:translate-y-px` or `active:scale-95`
press feedback sourced from the motion-token inventory. The
translation MUST be disabled by the `prefers-reduced-motion` guard.

#### Scenario: button press feedback runs only when motion is allowed

- GIVEN the user activates a primary button
- WHEN the host reports `prefers-reduced-motion: no-preference`
- THEN a tactile press feedback runs once
- WHEN the host reports `prefers-reduced-motion: reduce`
- THEN the press feedback does not run

#### Requirement: skeleton shimmer during initial load with reduced-motion fallback

Tables on the Dashboard, Reports, Stores, and Products screens MUST
render a DaisyUI `skeleton` shimmer during the initial load. The
shimmer MUST be disabled by the `prefers-reduced-motion` guard; the
skeleton MUST remain as a static grey block under reduced motion.

#### Scenario: skeleton shimmer falls back to static on reduced motion

- GIVEN the host reports `prefers-reduced-motion: reduce`
- WHEN the Dashboard, Reports, Stores, or Products page is loading
- THEN the table rows render as a static skeleton (no shimmer
  animation)
- AND the same data lands once loading completes

#### Requirement: backdrop blur on modal backdrops

Modal backdrops MUST support a subtle `backdrop-blur-sm` on capable
platforms. The effect MUST be disabled by the `prefers-reduced-motion`
guard.

#### Scenario: backdrop blur is applied when motion is allowed

- GIVEN the user opens any migrated modal
- WHEN the host reports `prefers-reduced-motion: no-preference`
- AND the host platform supports `backdrop-filter`
- THEN the modal backdrop renders with a subtle blur
- WHEN the host reports `prefers-reduced-motion: reduce`
- THEN the backdrop blur is not applied

#### Requirement: urgency pulse is allowed only on the expired variant

A slow pulse (1.5–2 s period, opacity-only, no scale or translation)
MUST be approved for the `expired` urgency badge and the leading
status dot inside the `.urgency-card-expired` surface. The pulse
MUST NOT be applied to `today`, `alert`, `soon`, or `normal`. The
pulse MUST be disabled by the `prefers-reduced-motion` guard, MUST
NOT be hover-triggered, and MUST be visually subtle.

#### Scenario: only the expired variant pulses, and only when motion is allowed

- GIVEN the dashboard renders urgency badges for an expired lot, a
  today lot, and a soon lot
- WHEN the host reports `prefers-reduced-motion: no-preference`
- THEN the expired badge (and only the expired badge) renders a
  pulse animation
- AND no other urgency badge pulses
- WHEN the host reports `prefers-reduced-motion: reduce`
- THEN no urgency badge pulses
- AND every badge remains visually distinct through colour, icon,
  and text

### Capability: i18n preservation in visual layer

#### Requirement: every new visible string flows through typesafe-i18n

Every new visible string introduced by this change (theme switcher
labels, empty-state titles and bodies, loading-state text, alert
copy, motion copy, tooltip text, configuration section titles
introduced by the redesign) MUST be added to both
`src/i18n/en/index.ts` and `src/i18n/es/index.ts` and MUST be
regenerated by `npm run i18n:generate`. The predev and prebuild
hooks MUST continue to enforce the regeneration gate. No new
visible string MAY be hard-coded in a Svelte template or component
file.

#### Scenario: a new visible string lands in EN and ES before it ships

- GIVEN a new visible string is introduced (for example the empty
  state of the migrated CSV import preview)
- WHEN the implementation is prepared for review
- THEN the string exists under a key in `src/i18n/en/index.ts`
- AND the same key exists under the equivalent Spanish text in
  `src/i18n/es/index.ts`
- AND `npm run i18n:generate` regenerates the catalogue without
  diagnostics
- AND `npm run check` passes
- AND no hard-coded copy of the new string exists in the migrated
  Svelte file

#### Scenario: the canonical internationalisation scenarios still pass

- GIVEN the redesign is in flight
- WHEN the canonical scenarios under `Capability: Internationalisation`
  are exercised (missing key fails the build, locale switch
  re-renders bound text, no hard-coded user-visible strings, EN
  fallback, persisted value wins, closest-supported mapping, etc.)
- THEN each scenario continues to pass without modification

#### Requirement: theme labels and motion copy are localised

The theme switcher MUST show theme names in the active locale.
Motion copy (for example any motion-related alert copy or any
loading-state wording tied to a motion affordance) MUST flow through
the translation tree and MUST be regenerated by `npm run
i18n:generate`.

#### Scenario: theme names render in the active locale

- GIVEN the active locale is `es`
- WHEN the user opens the theme switcher
- THEN each theme label is rendered in Spanish
- WHEN the active locale is `en`
- THEN each theme label is rendered in English

#### Requirement: empty / loading / error copy is localised in EN and ES

Every empty, loading, and error copy introduced by the redesign
MUST be present in both `src/i18n/en/index.ts` and
`src/i18n/es/index.ts`. Spanish strings MUST have enough room in the
layout that they do not force awkward wrapping at common desktop
widths.

#### Scenario: Spanish empty / loading / error copy fits without awkward wrapping

- GIVEN the active locale is `es`
- WHEN the user renders the migrated empty, loading, and error
  surfaces on a 1280 px viewport
- THEN no surface forces the Spanish copy to wrap mid-word or to
  overflow its container

### Capability: Accessibility preservation in visual layer

#### Requirement: visible focus rings preserved or improved

Every migrated interactive surface MUST render a visible focus ring
sourced from DaisyUI's default outline ring (or an equivalent
project token). The focus ring MUST remain visible in both
`caduxo-light` and `dark`.

#### Scenario: every migrated interactive surface shows a visible focus ring

- GIVEN the user tabs through every migrated interactive surface in
  `caduxo-light` and `dark`
- WHEN each surface receives keyboard focus
- THEN a visible focus ring renders against the surface background

#### Requirement: keyboard reachability for all primitives

Every migrated primitive MUST be reachable by keyboard in source
order. No migrated primitive MAY rely on hover alone for activation.

#### Scenario: every migrated primitive is reachable by keyboard

- GIVEN the user navigates with `Tab` and `Shift+Tab` from the app
  shell into every migrated page
- WHEN the user reaches the last interactive element on the page
- THEN every interactive surface has been focusable along the way
- AND no surface has required pointer-only activation

#### Requirement: Escape and click-outside behaviour for modals and popovers

Every migrated modal and popover MUST close on `Escape`. Every
migrated modal and popover that previously closed on click-outside
MUST continue to close on click-outside. The migrated
`CategoryPicker` popover MUST close on `Escape` without committing
the typed query (the input keeps the typed text).

#### Scenario: modal and popover Escape closes without committing

- GIVEN any migrated modal or popover is open
- WHEN the user presses `Escape`
- THEN the surface closes
- AND any in-progress edit that should be cancelled is cancelled
- AND any typed text in the popover input remains visible

#### Requirement: dialog focus restoration to trigger element

Every migrated modal MUST restore focus to the element that opened
it after the modal closes. The focus restoration MUST be a no-op if
the trigger element is no longer in the DOM at the time of close.

#### Scenario: focus returns to the trigger after a modal closes

- GIVEN the user opens a migrated modal from a specific trigger
  button
- AND the modal closes via `Escape`, the close button, or
  click-outside
- WHEN focus is observed
- THEN it is on the same trigger button

#### Requirement: WCAG-readable contrast in every shipped theme

Body text, headings, inputs, badges, alerts, and primary button
labels MUST meet WCAG-AA contrast in both `caduxo-light` and `dark`
on every main surface. See the `readable contrast and unbroken
layouts in every shipped theme` requirement for the manual check
gating.

#### Scenario: contrast pass covers body, heading, input, badge, alert, and button surfaces

- GIVEN the active theme is `caduxo-light` or `dark`
- WHEN the manual contrast pass on the main surfaces is performed
- THEN no body / heading / input / badge / alert / button label
  falls below WCAG-AA contrast

#### Requirement: status communicated through multiple channels

Status, urgency, and feedback MUST be communicated through colour,
icon, and text together. Colour MUST NOT be the only signal. This
applies to urgency badges, alerts, loading vs loaded vs empty vs
error states, and motion affordances.

#### Scenario: no status uses colour alone

- GIVEN the user views an urgency badge, alert, loading / loaded /
  empty / error state, or motion affordance
- WHEN the surface is rendered with a desaturated palette (for
  example the browser's high-contrast or a colour-blind simulator)
- THEN the user can still tell the status apart from other statuses
  through icon, text, or both

#### Requirement: DatePicker and CategoryPicker keyboard contracts are preserved

The keyboard contracts for `DatePicker.svelte` (Tab, Escape, Enter,
arrow keys, PageUp / PageDown, Shift+PageUp / Shift+PageDown, Today
shortcut) and `inputs/CategoryPicker.svelte` (arrow keys, Enter,
Escape, Backspace, `aria-activedescendant`, `Uncategorized`
pseudo-row) MUST be preserved exactly across the visual migration.
Restyling the visual surface MUST NOT introduce a focus trap,
remove a keyboard shortcut, or change any of the ARIA semantics.

#### Scenario: picker keyboard contracts pass after migration

- GIVEN the visual migration of `DatePicker.svelte` and
  `inputs/CategoryPicker.svelte` is complete
- WHEN every canonical scenario under `Capability: Date input > Custom
  date picker` and every CategoryPicker keyboard scenario under
  `Capability: Categories > editable category list` is exercised
- THEN each scenario continues to pass without modification

### Capability: Chained PR delivery

#### Requirement: one phase per chained PR by default

The implementation is delivered as chained PRs. Each of the 13
phases in `docs/daisyui-redesign-plan.md` becomes a work unit on
the `feat/daisyui-redesign` branch and is delivered as a single
chained PR by default. When a phase's forecast crosses the 400-line
review budget, the tasks phase splits that phase further (by page,
by primitive, or by primitive-batch) and delivers it as a chain of
smaller PRs that all reference the same plan doc. No chained PR
mixes backend changes with visual changes, and no chained PR mixes
two unrelated surfaces.

#### Scenario: each chained PR has its own review

- GIVEN the implementation branch is `feat/daisyui-redesign`
- WHEN each chained PR is opened
- THEN the PR description names the phase and references the plan
  doc
- AND the PR diff is reviewable in isolation
- AND no PR mixes two unrelated surfaces

#### Requirement: 400-line review budget enforced per PR

Each chained PR MUST stay within the 400-line review budget unless
an explicit `size: exception` is approved by the user via
`ask-on-risk`. The forecast is performed by the tasks phase; the
budget is enforced at apply time.

#### Scenario: an oversized phase is split before apply

- GIVEN a phase's tasks forecast exceeds 400 changed lines
- WHEN the tasks phase declares the phase
- THEN the phase is split by page or by primitive into smaller
  chained PRs
- OR the user is asked explicitly via `ask-on-risk` before any
  oversized PR is opened

#### Requirement: foundation-first delivery order

The first chained PR MUST be the foundation: dependencies + theme
config + design tokens + shared primitives, with no page migration
yet. The foundation phase MUST verify `npm run check`, `npm run
build`, and a manual launch in the Tauri / dev browser shell before
any subsequent chained PR may begin.

#### Scenario: foundation phase verifies build and manual launch

- GIVEN the foundation PR is opened
- WHEN the PR is reviewed
- THEN `npm run check` passes
- AND `npm run build` passes
- AND a manual Tauri / dev browser launch shows no widespread
  preflight breakage
- AND the diff is limited to dependencies, theme config, design
  tokens, and shared primitives

## MODIFIED Requirements

### Capability: Date input

#### Requirement: Custom date picker

The system MUST provide an in-house Svelte custom date picker (no
native `<input type="date">`) for every expiry date and report
date-range field, and the picker MUST be the single source of truth
for date input across the app.

The picker MUST bind ISO `YYYY-MM-DD` strings in and out, MUST
reject any year outside the inclusive range `[1900-01-01,
2100-12-31]`, and MUST support both a typed-text manual entry path
and a calendar popover path that share the same bound value. The
picker MUST render no `<input type="date">` element in the DOM.

The picker MUST expose a `clearable` prop (default `true`); when
`clearable={false}` the picker MUST NOT render any clear affordance
and MUST NOT silently commit an empty value. The host (`LotForm`
for the required expiry date) is responsible for the `required` JS
guard; the picker preserves the bound value on empty input when
`clearable={false}` and surfaces a visual invalid state instead.

The picker MUST support basic accessible keyboard ergonomics: Tab
enters and leaves the trigger without trapping focus inside the
popover; Escape closes the popover without committing a calendar
selection and without rewriting typed text; Enter on a focused day
cell emits that day's ISO date and closes the popover. Arrow keys
MUST move the focused day by ±1 (Left/Right) or ±7 (Up/Down) days;
PageUp/PageDown MUST move by ±1 month; Shift+PageUp/Shift+PageDown
MUST move by ±1 year; every move MUST be clamped to the configured
year range.

Manual `YYYY-MM-DD` text input MUST visually validate on blur (red
border + helper text for invalid shape, invalid calendar date, or
out-of-range year); the bound value MUST NOT update while the text
is invalid and the typed text MUST remain visible in the input
element (no silent rewrite).

The picker popover surface MUST render through DaisyUI `dropdown`
/ `dropdown-content` and the `popover` class as defined in the
`DatePicker popover restyled with DaisyUI dropdown / popover
classes` requirement. The visual restyle MUST NOT introduce a focus
trap, MUST NOT remove a keyboard shortcut, and MUST NOT change any
ARIA semantics.

(Previously: the requirement governed only the in-house Svelte
behaviour, the ISO bind contract, the year-range enforcement, the
`clearable` semantics, and the keyboard ergonomics. It did not
constrain the visual surface; the redesign pins the popover surface
to DaisyUI `dropdown` / `dropdown-content` and `popover` while
preserving every existing keyboard, validation, and ISO contract.)

#### Scenario: picker is in-house Svelte, not native

- GIVEN any date field in the app (LotForm expiry date, Reports
  `dateFrom`, Reports `dateTo`)
- WHEN `grep -R 'type="date"' src/` is executed after this slice
  lands
- THEN the command returns zero matches
- AND each field is rendered by the in-house `DatePicker.svelte`
  Svelte component composed around `CalendarMonth.svelte`

#### Scenario: ISO YYYY-MM-DD in/out with year range enforcement

- GIVEN the user opens the picker on any date field
- WHEN the user selects a day in any month of any year between
  1900 and 2100 inclusive (via the calendar popover or manual
  entry)
- THEN the picker emits the selected day as an ISO `YYYY-MM-DD`
  string
- AND the bound value is updated to that string
- AND the text input displays the selected ISO date

#### Scenario: selection closes the popover and commits

- GIVEN the picker popover is open and the user is focused on a day
  cell
- WHEN the user clicks or presses Enter on a day cell
- THEN the popover closes immediately
- AND the selected `YYYY-MM-DD` is emitted and committed to the host
- AND the text input displays the selected ISO date

#### Scenario: manual YYYY-MM-DD text input validates without silent rewrite

- GIVEN the picker text input is focused
- WHEN the user types any of `2026-13-40`, `2026-02-30`,
  `1899-12-31`, or `2101-01-01`
- AND blurs the input
- THEN the input displays a visual invalid state (red border +
  helper text)
- AND the bound value is NOT updated
- AND the typed text remains visible in the input element (no
  silent rewrite)

#### Scenario: clearable=false hides the clear icon and prevents silent empty commit

- GIVEN the picker is mounted with `clearable={false}` (LotForm
  expiry date)
- WHEN the picker is rendered
- THEN no `×` clear icon is present in the DOM
- AND the user cannot clear the bound value through the picker
- WHEN the user deletes the typed text down to empty and blurs
- THEN the picker shows a visual invalid state
- AND the bound value is NOT silently committed to `""`

#### Scenario: clearable=true exposes the clear icon

- GIVEN the picker is mounted with the default `clearable={true}`
  (Reports `dateFrom`, `dateTo`)
- AND the bound value is non-empty
- WHEN the picker is rendered
- THEN a `×` clear icon is visible in the trigger
- AND clicking `×` clears the bound value to `""` and closes the
  popover

#### Scenario: basic accessible keyboard ergonomics

- GIVEN the picker trigger is focused
- WHEN the user presses Tab
- THEN focus moves through the trigger and the calendar-icon button
  in source order
- WHEN the user presses Tab again
- THEN focus leaves the picker (no focus trap inside the popover)
- GIVEN the popover is open
- WHEN the user presses Escape
- THEN the popover closes without committing a calendar selection
- AND any typed text remains visible (still subject to
  blur-validation rules)
- GIVEN a day cell is focused inside the popover
- WHEN the user presses Enter
- THEN that day's ISO date is emitted and the popover closes
- AND ArrowLeft/ArrowRight move the focused day by ±1 day
- AND ArrowUp/ArrowDown move the focused day by ±7 days
- AND PageUp/PageDown move the focused day by ±1 month
- AND Shift+PageUp/Shift+PageDown move the focused day by ±1 year
- AND every move is clamped to `[1900-01-01, 2100-12-31]`

#### Scenario: Today shortcut inside popover

- GIVEN the picker popover is open
- AND today is within `[minDate, maxDate]`
- WHEN the user clicks the `Today` button in the popover footer
- THEN the bound value is set to today's ISO `YYYY-MM-DD`
- AND the popover closes

#### Scenario: popover surface carries DaisyUI dropdown classes after migration

- GIVEN the visual migration of `DatePicker.svelte` has landed
- WHEN the picker popover is rendered
- THEN the popover root carries DaisyUI `dropdown` /
  `dropdown-content` classes (and any project theme overrides)
- AND every scenario above continues to pass without modification

### Capability: Calendar

#### Requirement: Calendar tab

The system MUST provide a top-level main-navigation Calendar tab
(added between Products and Reports) that opens the current month
with today highlighted and selected, displays per-day lot
expirations as dot badges, and lists the lots and products expiring
on a selected day in a day-detail panel.

The Calendar tab MUST reuse the same `CalendarMonth.svelte`
primitive used by the date picker popover so the day grid, year
picker, visual styling, and keyboard surface are implemented once.
The Calendar tab MUST source its expiration data from the existing
`list_dashboard_lots` Tauri command (no new backend command in this
slice); the day-bucket map MUST be computed client-side from the
returned active lots. Month navigation MUST NOT trigger a refetch
in this slice.

Clicking a day MUST set the selected date and reveal a day-detail
panel that lists every active lot whose `expiry_date` equals the
selected date, with columns for product, quantity, unit, store,
location, days remaining, and status. Clicking a lot row MUST open
the existing lot edit flow used elsewhere in the app. Clicking a
day with no expirations MUST still select it and show a `No
expirations on YYYY-MM-DD` placeholder.

The Calendar tab MUST inherit the full keyboard surface of the
calendar primitive (Tab, Esc, Enter, arrow keys, PageUp/Down,
Shift+PageUp/Shift+PageDown) with a tab order of prev-month
chevron → month label → year chip → day grid → next-month chevron
→ day-detail rows.

The calendar grid, day cells, today ring, selection ring, badge
dots, and day-detail panel MUST render through the shared DaisyUI
themed primitives as defined in the visual surface redesign and
native control replacement capabilities. The visual restyle MUST
NOT change any domain logic carried by `CalendarMonth.svelte`
(year picker, decade navigation, badge dots, out-of-range guards)
and MUST NOT remove any keyboard shortcut.

(Previously: the requirement governed the calendar's data source,
the keyboard surface, the day-detail panel columns, the lot row
edit flow, and the inherited keyboard surface from
`CalendarMonth.svelte`. It did not constrain the visual surface;
the redesign pins the day cells, today ring, selection ring, badge
dots, and day-detail panel to the shared DaisyUI themed primitives
while preserving every existing keyboard, data, and domain logic
contract.)

#### Scenario: opens at current month with today highlighted and selected

- GIVEN the user clicks the Calendar tab in main navigation
- WHEN the page renders
- THEN the calendar grid displays the current month (`viewYear`
  and `viewMonth` derived from `new Date()`)
- AND today is visually highlighted (distinct from the selected
  highlight)
- AND today is the selected date (`selectedDate = today`)

#### Scenario: per-day expirations visible as dot badges

- GIVEN active lots exist in the local database with various
  `expiry_date` values
- WHEN the Calendar tab renders
- THEN each day cell with at least one active lot displays a small
  dot under the day number
- AND the dot corresponds to the count of active lots whose
  `expiry_date` equals that day
- AND the count is also shown in the day-detail panel for the
  selected date

#### Scenario: day-detail panel lists lots and products

- GIVEN a day is selected on the Calendar tab
- WHEN the day-detail panel renders
- THEN the panel lists every active lot whose `expiry_date` equals
  the selected date
- AND each row shows product, quantity, unit, store, location, days
  remaining, and status

#### Scenario: empty day shows the placeholder message

- GIVEN a day is selected on the Calendar tab
- AND no active lot has an `expiry_date` equal to that day
- WHEN the day-detail panel renders
- THEN the panel shows `No expirations on YYYY-MM-DD` (or an
  equivalent English placeholder that names the date)

#### Scenario: lot row opens the existing lot edit flow

- GIVEN the day-detail panel shows one or more lot rows
- WHEN the user clicks a lot row
- THEN the existing lot edit overlay opens for that lot (the same
  pattern used by `DashboardPage`)
- AND no new edit-flow primitive is introduced

#### Scenario: data source is the existing list_dashboard_lots command

- GIVEN the user opens the Calendar tab
- WHEN the page mounts
- THEN the page calls `list_dashboard_lots({ store_id: null,
  location_id: null, preset: null, urgency: null })` exactly once
- AND the day-bucket map is computed client-side from the returned
  active lots
- AND month navigation does NOT trigger a refetch in this slice

#### Scenario: keyboard surface inherited from the calendar primitive

- GIVEN the Calendar tab is rendered
- WHEN the user navigates with Tab, Esc, Enter, arrow keys, and
  PageUp/Down
- THEN focus traverses prev-month chevron → month label → year
  chip → day grid → next-month chevron → day-detail rows
- AND arrow keys / PageUp / PageDown / Shift+PageUp /
  Shift+PageDown move the focused day with the same clamping rules
  as the picker
- AND Enter on a focused day cell emits the ISO date and updates
  `selectedDate`

#### Scenario: calendar surfaces render through the shared primitives after migration

- GIVEN the visual migration of `CalendarPage.svelte` and
  `CalendarMonth.svelte` has landed
- WHEN the Calendar tab is rendered
- THEN the day grid, day cells, today ring, selection ring, badge
  dots, and day-detail panel render through the shared DaisyUI
  themed primitives
- AND every scenario above continues to pass without modification
- AND the year picker, decade navigation, badge dot count, and
  out-of-range guards carried by `CalendarMonth.svelte` continue to
  work exactly as before