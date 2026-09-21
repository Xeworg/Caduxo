# Proposal — caduxo-daisyui-redesign

## Change metadata

- **Change ID**: `caduxo-daisyui-redesign`
- **Domain**: `caduxo-expiry-tracker` (single-domain project)
- **Artifact store**: `openspec` (per session preflight); Engram mirror is
  not used for this change because `artifactStore: both` is not active
- **Review budget**: 400 changed lines per slice (session override;
  canonical 800) — chained PRs are mandatory when a single phase
  forecasts above this budget
- **Delivery strategy**: `auto-chain` (deferred until chaining is
  selected); the proposal-forecast below already triggers chained
  delivery, but the first phase may still pause for `ask-on-risk` if a
  specific slice crosses the budget
- **Chain strategy**: `deferred` — chaining will be selected in the
  tasks phase based on per-phase forecast, not decided here
- **Strict TDD**: `false` (project default); this is a visual redesign
  and relies on manual smoke plus `npm run check` / `npm run build`
- **Execution mode**: `auto` — this proposal is written and persisted
  without a proposal question round; remaining tactical choices are
  recorded as proposal assumptions the user may override during review
- **Authoring inputs**: `openspec/changes/caduxo-daisyui-redesign/explore.md`
  (architecture baseline, pain points, risk register) and
  `docs/daisyui-redesign-plan.md` (the 13-phase migration program and
  per-phase acceptance gates already accepted on the planning branch
  `feat/daisyui-redesign-plan`)

## Problem statement

Caduxo today looks like a hand-stitched prototype rather than a coherent
desktop application. Every screen reinvents its own buttons, cards,
modals, badges, tables, alerts, popovers, and toggle controls. There is
no design system, no theme support, and no shared motion vocabulary.
Concretely:

- **Twenty-four component files carry their own subset of the same
  visual primitives.** `btn-primary` / `btn-secondary` / `btn-ghost` /
  `btn-outline` / `btn-danger` / `btn-link` / `btn-sm` / `btn-small` /
  `action-btn` / `chip-clear` / `banner-btn` / `inline-unit-close` /
  `link-btn` co-exist across the codebase with no canonical definition
  of which to use when. Border radii drift between 4, 5, 6, 8, and 10
  pixels; font sizes drift between 0.85 and 1.5 rem; the primary blue
  exists as `#2563eb`, `#1d4ed8`, and `#93c5fd` in different files.
- **Native OS UI leaks into the chrome.** The locale `<select>` on the
  Configuration page renders with operating-system chrome on Windows,
  macOS, and Linux/GTK, breaking visual cohesion with everything else
  in the app. All form inputs are bare browser controls. The hand-rolled
  toggle, popover, and modal patterns repeat that inconsistency.
- **Eight plus modal dialogs rebuild the same overlay / header / body /
  footer / close-button structure with local styles.** No shared focus
  trap, no shared Escape handling, no shared `prefers-reduced-motion`
  treatment.
- **No theme support of any kind.** Colour tokens are inlined as
  hex/rgb literals across every component. Adding a dark theme or any
  accent theme today is not feasible without a wholesale rewrite.
- **Motion is ad-hoc.** Twenty-plus `transition: ... 0.1s–0.2s`
  declarations, one `@keyframes spin`, and a `0.2s left` transition on a
  custom toggle thumb. No shared easing/duration tokens, no reduced-
  motion gate anywhere in `src/`.
- **The result for users is a competent but visually flat inventory
  tool that does not feel like a polished desktop application.** It is
  also harder to maintain, harder to keep consistent as the app grows,
  and inaccessible to a dark-mode preference or any future brand
  refresh.

The user has explicitly decided that piecemeal tweaks are not the
answer: this change is a complete redesign program built on Tailwind CSS
+ DaisyUI as the design-system foundation, not a partial cleanup.

## Desired outcome

After this change ships, Caduxo feels like a cohesive desktop
application and the underlying visual system is durable:

1. **One design system across all surfaces.** Every button, card, table,
   modal, badge, alert, popover, input, select, toggle, tab, and empty
   state in Caduxo is rendered through a shared primitive that wraps
   DaisyUI. New screens compose from the same primitives. There are no
   one-off local styles left in the migrated components.
2. **Striking, polished visuals that respect user preferences.** A
   custom `caduxo-light` brand theme grounded in today's `#2563eb`
   primary and `#f6f8fb` base, paired with a hand-tuned dark theme.
   Tasteful motion (modal scale-in, dropdown slide-in, card hover lift,
   skeleton shimmer, a strictly limited urgency pulse on expired lots)
   that is automatically disabled when the host requests reduced motion.
3. **Native OS controls removed from the chrome.** The locale picker,
   every form input, every checkbox/radio/toggle, the date and category
   pickers, and every modal now render with Caduxo's own visual
   vocabulary. The user never sees a Windows/macOS/GTK-styled control
   inside the app.
4. **Built-in theme support, not a single fixed theme.** The app ships
   with a curated set of DaisyUI built-in themes (light + dark at
   minimum) and lets the user switch between them from the
   Configuration page, with the choice persisted across launches and
   defaulting to the host's `prefers-color-scheme`.
5. **Accessibility preserved or improved.** Visible focus rings,
   keyboard-reachable controls, Escape closes dialogs/popovers, correct
   button-vs-link semantics, non-color-only status communication, and
   reduced-motion compliance all hold or improve versus today.
6. **Existing i18n discipline preserved.** Every new visible string
   (empty states, toast copy, theme labels, motion copy) flows through
   `src/i18n/en/index.ts` and `src/i18n/es/index.ts` and is regenerated
   by `npm run i18n:generate`. The i18n-supported change closed the
   last hardcoded residues; the redesign does not re-open them.
7. **The app is reviewable in slices.** Despite being a complete
   redesign, the change is delivered as chained PRs (one phase per PR
   by default) so no single review ever crosses the 400-line budget.

## In-scope visual redesign work

The full program covers every visible surface of Caduxo today, plus the
cross-cutting foundations that make that possible.

### Foundations

- Add `tailwindcss` (v4), `@tailwindcss/vite`, and `daisyui` to the
  dependencies and wire the Tailwind Vite plugin in `vite.config.ts`.
- Create `src/app.css` with `@import "tailwindcss"; @plugin "daisyui";`
  plus the project's custom theme block; switch `src/main.ts` to import
  it instead of the legacy `src/style.css`.
- Define a custom `caduxo-light` theme that uses the existing brand blue
  `#2563eb`, the existing base background family `#f6f8fb`, and
  semantic overrides for primary / secondary / accent / neutral /
  base-100..300 / info / success / warning / error.
- Enable DaisyUI built-in `light` and `dark` themes alongside the
  custom theme, persist the chosen theme in `app_settings.theme`
  (mirroring the `language` setting pattern), and default to the host's
  `prefers-color-scheme` when no choice is on file.
- Add a theme switcher to the Configuration page (next to the language
  selector) showing the active theme and the available themes.
- Centralise motion tokens (durations, easings) and a
  `@media (prefers-reduced-motion: no-preference)` guard around all
  animated surfaces. Default to no animation under reduced motion.
- Verify Tailwind preflight does not regress existing global styles
  before any page migration begins (Phase 1 gate, already documented in
  `docs/daisyui-redesign-plan.md`).

### Shared UI primitives to extract into `src/components/ui/`

These primitives are created *before* page migration so the redesign
never presents a half-migrated UI:

- `Button.svelte` — wraps DaisyUI `btn` variants (primary, secondary,
  ghost, outline, error, warning, success, link, icon, sizes).
- `Card.svelte` — base card with consistent padding, border, shadow,
  optional header / footer slots; consumed by urgency cards, report
  type cards, settings sections, backup/import summaries, and
  product/store detail panels.
- `Modal.svelte` — shared shell backed by the native `<dialog>` element
  with `showModal()` for built-in focus trap and Escape handling,
  backdrop, size variants (small / default / wide), close button,
  click-outside-to-close, focus restoration, reduced-motion
  compliance.
- `Table.svelte` — shared wrapper with `table-zebra`, `table-pin-rows`,
  numeric alignment helper, sticky header option, empty-state slot,
  loading-state slot, dense / default sizing.
- `Badge.svelte` — urgency variants (expired / today / alert / soon /
  normal) and semantic variants (success / warning / error / info /
  neutral) with optional leading status dot.
- `Alert.svelte` — success / error / warning / info blocks with
  consistent icon + text + action slot.
- `EmptyState.svelte` — title + body + optional icon + optional action
  slot; replaces the inconsistent empty paragraphs scattered across
  pages.
- `LoadingState.svelte` — skeleton row option, text fallback, and
  reduced-motion-safe shimmer.
- `Tabs.svelte` — shared `tabs-bordered` / `tabs-lifted` strip with
  keyboard navigation and an active state; consumed by product detail
  and dashboard/calendar detail panels.
- `Select.svelte`, `Input.svelte`, `Toggle.svelte` — DaisyUI-themed
  wrappers around the corresponding form primitives so forms share one
  visual vocabulary.
- `Popover.svelte` (deferred — see "Proposal assumptions" below).

### Native / system controls to replace or restyle

Every native-feeling control in the app today must move to Caduxo's
visual vocabulary. Specifically:

- The locale `<select>` on the Configuration page → DaisyUI
  `select select-bordered` with consistent caret, focus ring, and
  theme-aware hover / active states.
- The `<datalist>` autocomplete inputs in `ProductForm.svelte` →
  keep `<datalist>` for accessibility, wrap the input in DaisyUI
  `input` styling.
- Bare `<input type="text">` across all forms → DaisyUI
  `input input-bordered` with `input-error` for invalid state, paired
  with `label` + `label-text` + `label-text-alt` for label, required
  marker, and helper text.
- Plain `<input type="checkbox">` → DaisyUI `checkbox checkbox-primary`.
- Plain `<input type="radio">` → DaisyUI `radio radio-primary`.
- The custom toggle (`toggle-wrap` / `toggle-track` / `toggle-thumb`)
  → DaisyUI `toggle toggle-primary`.
- The `<details>` / `<summary>` dropdowns in `UnitReviewPage.svelte` →
  DaisyUI `dropdown` + `dropdown-content` with proper ARIA roles.
- The hand-rolled popovers in `DatePicker.svelte` and
  `inputs/CategoryPicker.svelte` → DaisyUI `dropdown` / `dropdown-content`
  styling with all existing keyboard contracts preserved (arrow keys,
  Enter, Escape, Backspace on CategoryPicker).
- The eight-plus modal dialogs (`MoveStockModal`, `RegisterExitModal`,
  `AdjustCountModal`, `ArchiveLotDialog`, `ResolveQuantityDialog`, plus
  the inline overlays inside DashboardPage) → native `<dialog
  class="modal">` with `showModal()` for built-in focus trap, Escape,
  and backdrop behaviour.
- Every `class="btn-primary"`, `class="btn-secondary"`, `class="action-btn"`,
  `class="chip-clear"`, `class="banner-btn"`, `class="link-btn"` → the
  unified `Button.svelte` primitive.
- Every table (DashboardPage lot table, LotMovementsPanel, ReportsPage,
  CsvImportPage preview, StoresPage store list, BackupRestorePage info
  lists) → shared `Table.svelte` with consistent zebra / sticky /
  alignment treatment.
- Every urgency badge (`urgency-badge`, `status-*`, `lot-picker-status`)
  → `Badge.svelte` with semantic modifier and optional status dot.
- Banner / alert strips (`UnitReviewBanner`, dashboard error banner) →
  `Alert.svelte`.
- The spinner in `ScanSearchBox.svelte` → DaisyUI `loading
  loading-spinner`.
- `title="..."` tooltips on icon-only buttons → DaisyUI `tooltip` (now
  keyboard-reachable, not hover-only).
- The app-shell navigation (`App.svelte` `.nav / .nav-btn / .nav-brand`)
  → DaisyUI `navbar` with `navbar-start` / `navbar-center` /
  `navbar-end` slots, responsive collapse via `dropdown` on narrow
  viewports.
- OS scrollbars in wide tables → themed `scrollbar-thin` /
  `scrollbar-thumb-base-300` so scrolling feels intentional.
- The vestigial `src/style.css` (hero/landing classes no production
  component references) → retired once the migration confirms no
  remaining references.

### Effects and motion (documented as an explicit deliverable)

Motion is part of the design system, not an afterthought. The redesign
ships these effects, each gated on
`@media (prefers-reduced-motion: no-preference)`:

- Subtle default transitions (~150–200 ms ease-out) on hover, focus,
  dropdown open/close, and modal show/hide, centralised through a
  shared `transition-base` utility group rather than per-component
  hand-tuning.
- Modal fade + scale (~150 ms) on open and close.
- Dropdown / popover slide-in (~150 ms).
- Card hover lift (`hover:-translate-y-0.5 hover:shadow-lg`) on urgency
  cards and report type cards.
- Button press feedback (`active:translate-y-px` or `active:scale-95`).
- Strictly limited urgency pulse: a slow (1.5–2 s) pulse on the
  "expired" urgency badge and on a leading dot inside the
  `.urgency-card-expired` card, *only* on the expired variant, *only*
  when the user has not requested reduced motion. The purpose is to
  draw the eye to genuinely expired lots, not decorative motion.
- Skeleton shimmer on table rows during initial load on Dashboard,
  Reports, Stores, and Products.
- Subtle backdrop blur on modal backdrops on capable platforms.
- Subtle app-shell gradient band (`bg-gradient-to-br from-primary/5 to
  base-100`) behind the dashboard brand area for visual interest above
  the urgency cards.

Every animated surface must disable its animation when
`prefers-reduced-motion: reduce` is set. The Design phase documents the
per-effect easing/duration tokens; this proposal locks the requirement
that motion is declared, not accidental.

### Page migrations

Every existing page is migrated to the shared primitives as part of the
13 phases already laid out in `docs/daisyui-redesign-plan.md` (Phase 3
through Phase 12). Specifically:

- App shell navigation.
- Dashboard (urgency cards, lot table, scan row, modals).
- Modal dialogs (stock movement, exit, adjust, archive, resolve).
- Forms (Product, Lot, Configuration, Reports filters, Backup/Restore,
  CSV import).
- Data tables (Dashboard, Reports, CSV preview, Stores, Lot movements,
  Calendar).
- Calendar and custom widgets (CalendarMonth, CalendarPage, DatePicker,
  CategoryPicker) — visual restyle only, behaviour preserved.
- Responsive pass at 1024 / 720 / 480 px breakpoints.

The design phase produces the per-page migration order with the
acceptance gates already documented in the planning doc.

### i18n copy

- New visible strings (theme labels, empty states, motion copy, alert
  copy) enter `src/i18n/en/index.ts` and `src/i18n/es/index.ts` and are
  regenerated by `npm run i18n:generate`. The predev/prebuild scripts
  already enforce the regeneration gate.

## Non-goals

The redesign is complete in the *visual* sense, but explicitly bounded
elsewhere. None of the following is part of this change:

- **Backend or Tauri command changes** unless the migration exposes a
  required bug fix. No DB schema migration is required for the theme
  preference (it piggybacks on the existing `app_settings` table, same
  pattern as the `language` row).
- **Business logic rewrites.** Unit-review flow, scan/search, stock
  movements, CSV import, backup/restore, and reports keep their current
  behaviour.
- **Replacing DatePicker, CalendarMonth, or CategoryPicker with
  off-the-shelf widgets.** Visual restyle only; behaviour and
  accessibility contracts are preserved exactly.
- **Router-based refactor.** The tab-based shell stays.
- **New product features.** No bulk actions, advanced filters,
  multi-user, dashboards 2.0, automation rules, etc.
- **Time-of-day auto-switching.** Only `prefers-color-scheme` drives
  the default theme for v1.
- **First-class dark-mode polish beyond what DaisyUI gives for free.**
  v1 ships a tuned `caduxo-light` plus a hand-checked `dark`; per-screen
  polish of every surface in dark mode is a follow-up.
- **Tauri-specific work** (icons, bundling, installer, plugins beyond
  what's already used).
- **Currency, POS, billing, accounting, full inventory management** —
  all remain project non-goals per `openspec/config.yaml`.
- **The shared `Popover` primitive extraction in v1.** DatePicker and
  CategoryPicker are restyled in place for v1; extracting a shared
  `Popover.svelte` is deferred to a follow-up.
- **The `ToastHost.svelte` primitive in v1.** Inline success / error
  patterns stay for v1 to limit scope; toast host is deferred to a
  follow-up.

## UX principles

The redesign commits to these principles; every phase that touches a
surface evaluates its work against them.

1. **Calm visual hierarchy.** Spacing, type, and colour carry meaning;
   size alone should never be the only signal of importance.
2. **One button vocabulary.** Primary, secondary, ghost, danger, icon,
   link. Each has a defined role; ad-hoc button classes do not survive
   the migration.
3. **Predictable controls.** Every input, select, toggle, popover, and
   modal behaves the same way across the app: same focus ring, same
   hover state, same Escape / click-outside behaviour, same reduced-
   motion compliance.
4. **Status is multi-channel.** Urgency and feedback use colour, icon,
   text, and (when helpful) motion. Colour is never the only signal.
5. **Motion is purposeful.** Every animation serves either feedback
   (something happened) or hierarchy (this matters now). Decorative
   motion is removed; the one exception is the strictly limited
   expired-urgency pulse, which exists to protect users from real
   losses.
6. **Respect user preferences.** Reduced motion, system colour scheme,
   and keyboard navigation are first-class, not afterthoughts.
7. **Spanish and English sit equally well.** Long Spanish strings have
   room; layouts do not depend on the locale to remain usable.
8. **Empty and loading are real states.** They get the same design
   attention as loaded states.

## DaisyUI theme support requirements

The app supports multiple themes from day one, not as a future
extension:

- **Default theme:** `caduxo-light`, a custom theme whose primary is
  grounded in the current brand blue `#2563eb`, whose base background
  is in the `#f6f8fb` family, and whose semantic colours map to the
  existing red / amber / green / blue palette Caduxo already uses for
  urgency and feedback.
- **Additional themes shipped with v1:** DaisyUI built-in `light` and
  `dark`. The `dark` theme is hand-checked for contrast on the main
  surfaces before v1 ships; per-screen dark-mode polish beyond what
  DaisyUI provides for free is a follow-up (see non-goals).
- **Theme switcher:** A new section on the Configuration page next to
  the language selector lists the available themes by name (in the
  active locale) and lets the user pick one. The choice is optimistic
  and persisted to `app_settings.theme` via the existing
  `updateSettings` IPC command, mirroring the language pattern.
- **Default behaviour:** When no `app_settings.theme` row exists, the
  app reads the host's `prefers-color-scheme` and chooses
  `caduxo-light` or `dark` accordingly. The user can override at any
  time; the override persists and wins on every subsequent launch.
- **Theme tokens:** All colour values in component CSS come from DaisyUI
  theme tokens (`primary`, `base-100..300`, `info`, `success`,
  `warning`, `error`, etc.). No component inlines a hex/rgb colour that
  is not theme-derived.
- **Tree-shaking:** Only the themes listed above are bundled. DaisyUI
  themes are tree-shakeable; the build verifies CSS size does not
  regress >20% per phase.

## Native / system control replacement requirement

Every native-feeling control that currently leaks OS chrome into Caduxo
must be removed from the user-visible surface:

- The Configuration page's `<select>` is the single most visible
  offender and is replaced with the DaisyUI-themed `Select` primitive.
- All bare form inputs (text, checkbox, radio, toggle, datalist) are
  themed through `Input.svelte`, `Select.svelte`, `Toggle.svelte`, and
  the DaisyUI `checkbox` / `radio` classes.
- The eight-plus modal dialogs are replaced with native `<dialog
  class="modal">` driven by `showModal()`. The user gets native focus
  trap, native Escape handling, and a themed backdrop — not an OS-styled
  system dialog.
- The hand-rolled DatePicker and CategoryPicker popovers become
  DaisyUI-themed popovers / dropdowns with all existing keyboard
  contracts preserved.
- The custom toggle is replaced with the DaisyUI `toggle` primitive,
  eliminating the bespoke `toggle-wrap` / `toggle-track` /
  `toggle-thumb` CSS.

The acceptance criterion is straightforward: a screenshot of every
main screen must show zero native-styled controls inside the app
chrome.

## Effects / motion requirement with reduced-motion guard

The redesign documents effects as an explicit deliverable and treats
reduced-motion compliance as a first-class requirement.

- The Design phase records the full effect inventory (modal fade+scale,
  dropdown slide-in, card hover lift, button press feedback, skeleton
  shimmer, urgency pulse on expired, app-shell gradient band,
  backdrop blur on modal backdrops, calendar day-cell colour
  transitions, category picker popover fade + y-translate) with their
  durations and easings centralised in motion tokens.
- Every animated surface is wrapped in
  `@media (prefers-reduced-motion: no-preference) { ... }` (or its
  Tailwind equivalent: `motion-safe:` variants). Under reduced motion,
  the surface still works, it just does not animate.
- The urgency pulse is approved for the *expired* variant only, is
  never hover-triggered, and is verified to be visually subtle (1.5–2 s
  period, gentle opacity change, no scaling or translation that could
  trigger vestibular sensitivity).
- No animation is allowed to slow a common workflow. The verify report
  for each phase includes a manual check that reduced-motion users
  reach the same end state as everyone else.

## Delivery plan expectation using chained PRs

The full program is large; the canonical delivery shape is chained PRs,
not a single pull request. Concretely:

- The implementation branch is `feat/daisyui-redesign`, mirroring the
  planning branch's intent and superseding it once work begins.
- Each of the 13 phases in `docs/daisyui-redesign-plan.md` becomes a
  work unit on that branch, ideally a single commit and a single PR.
- When a phase's forecast crosses the 400-line review budget (which
  most of them will), the tasks phase splits that phase further — by
  page, by primitive, or by primitive-batch — and delivers it as a
  chain of smaller PRs that all reference the same plan. The plan
  remains the controlling artifact; the chain is the delivery vehicle.
- The first chained PR is the foundation: dependencies + theme config
  + design tokens + no page migration yet. This is the lowest-risk
  slice and the one that proves Tailwind preflight does not regress
  the existing UI.
- Phases that touch business logic or backend (theme persistence is
  the only one, and it is small) are isolated in their own PR.
- No PR mixes backend changes with visual changes. No PR mixes two
  unrelated surfaces.

The proposal does not lock the exact chain sequence — that is the
tasks phase's job — but it does lock the rule: one phase per PR by
default, split further if the budget is exceeded, never mixed with
unrelated work.

## Risks and mitigations

| # | Risk | Mitigation |
|---|------|-----------|
| 1 | **Review-budget overflow.** A complete redesign will exceed the 400-line budget many times over. | Phases are chained PRs by default; the tasks phase forecasts per-phase size and splits any phase that still overshoots; the proposal-forecast already triggers chained delivery. |
| 2 | **Tailwind preflight resets existing CSS** (headings, buttons, inputs, lists, tables). | Phase 1 foundation gate: install Tailwind/DaisyUI without migrating any component, run `npm run check` + `npm run build`, manually screenshot the main screens, stop and fix foundation before page migration. |
| 3 | **Half-migrated UI.** Mixed local styles and DaisyUI primitives look worse than a fully consistent system. | Shared primitives are created *before* page migration; migration proceeds by full screen; local styles are deleted as each surface converts; no PR is merged that leaves a surface in a visibly mixed state. |
| 4 | **Accessibility regressions in custom popovers.** DatePicker and CategoryPicker carry hand-rolled keyboard contracts. | Restyle only; preserve all keyboard interactions; manual keyboard pass on every primitive; non-regression manual smoke after each phase that touches a popover. |
| 5 | **Custom calendar widget regressions.** CalendarMonth and DatePicker carry domain logic (year picker, decade navigation, badge dots, out-of-range guards). | Restyle only; manual smoke test for keyboard nav, year picker, day selection, and out-of-range guards after every Phase 8 change. |
| 6 | **i18n catalog drift.** New empty / loading / toast / theme strings must land in EN and ES. | Every visible string flows through `src/i18n/en/index.ts` and `src/i18n/es/index.ts`; `predev` / `prebuild` already run `npm run i18n:generate`; CI includes `npm run i18n:generate && npm run check`. |
| 7 | **Theme persistence requires a new settings row.** | Extend `app_settings` with `theme` using the same read/write pattern as `language` (the i18n change already established this). No new Tauri command surface is required. |
| 8 | **CSS bundle size growth.** Tailwind v4 + DaisyUI adds weight. | DaisyUI is tree-shakeable per theme; ship only the curated theme set; verify `npm run build` final CSS size per phase; gate any PR that regresses CSS size >20%. |
| 9 | **WebKitGTK quirk in Tauri.** Older WebKitGTK builds can fail on top-level await; Tailwind/DaisyUI should not reintroduce this, but Phase 1 verifies a Tauri dev launch. | Phase 1 acceptance includes a Tauri / manual smoke; if Tauri is unavailable, document a browser fallback. |
| 10 | **Plugin ordering.** `@tailwindcss/vite` must coexist correctly with `@sveltejs/vite-plugin-svelte`. | Phase 1 verification: vite dev + build both pass; Svelte HMR works; no console errors. |
| 11 | **Font inconsistency.** Inter is currently declared in `src/style.css`; DaisyUI uses its own font stack. | The Design phase locks the canonical font (Inter vs DaisyUI default) and applies it via `font-sans` or a custom theme token. |
| 12 | **Urgency pulse risk.** A pulsing badge could trigger vestibular sensitivity. | Pulse is gated on `prefers-reduced-motion: no-preference`; the pulse is subtle (opacity only, no scale or translation); the rationale is documented in the Design phase. |
| 13 | **Review attention across many PRs.** Thirteen chained PRs is still heavy. | Chained delivery follows the `auto-forecast` chain strategy from `openspec/config.yaml`; no `size:exception` is requested at proposal time; any oversized phase pauses via `ask-on-risk` before `tasks.md`. |
| 14 | **Theme switcher + theme defaulting surprises users** who expected the OS preference to win after a manual override. | The Design phase documents the precedence: manual override > persisted theme > OS preference > `caduxo-light` fallback. The Configuration page's theme switcher shows the active source so users can tell why they got the theme they got. |
| 15 | **Native `<dialog>` and the existing Escape / focus-restoration behaviour diverge.** | Phase 5 modal migration manually verifies every modal opens/closes correctly via keyboard, focus is restored to the trigger element, and Escape still cancels in-progress edits the way it does today. |

## Acceptance criteria at proposal level

This proposal is approved when the following are true. They are
proposal-level, not phase-level — the design and tasks phases will
flesh them out into the canonical verification gates.

1. The problem statement and desired outcome above reflect the user's
   stated goal: a complete redesign program (not piecemeal tweaks),
   visual polish as the priority, native OS controls removed from the
   chrome, effects documented as an explicit deliverable, and DaisyUI
   built-in theme support (not a single fixed theme).
2. The scope covers every visible component in `src/` today, mapped to
   a DaisyUI primitive or shared wrapper, plus the foundation,
   themes, and motion work required to make that possible.
3. The non-goals lock the boundaries so the redesign does not become a
   feature rewrite: no backend changes, no business logic rewrites, no
   router refactor, no new product features, no time-of-day theme
   switching, no first-class dark-mode polish beyond what DaisyUI
   provides for free.
4. The UX principles are explicit and testable: one button vocabulary,
   predictable controls, multi-channel status, purposeful motion,
   user-preference respect, locale parity.
5. The theme requirements ship multi-theme support on day one: a custom
   `caduxo-light` brand theme, DaisyUI built-in `dark`, a theme
   switcher on the Configuration page, persistence via
   `app_settings.theme`, and OS-aware defaulting via
   `prefers-color-scheme`.
6. The native-control replacement requirement is verifiable by
   screenshot: zero OS-styled controls inside the app chrome after
   v1.
7. The motion requirement is verifiable by inspection: every animated
   surface has a `prefers-reduced-motion` guard, and the urgency
   pulse is the only deliberately animated indicator.
8. The delivery plan expects chained PRs, with one phase per PR by
   default and further splits for any phase that crosses the 400-line
   budget.
9. The risks above cover preflight regression, half-migration,
   accessibility regressions in popovers and the calendar, i18n
   drift, theme persistence, CSS bundle growth, WebKitGTK, plugin
   ordering, font consistency, urgency-pulse sensitivity, review
   attention, theme-switcher precedence, and native `<dialog>`
   behaviour.
10. The acceptance criterion for the design phase is locked: it must
    record the per-effect motion tokens, the per-surface migration
    order, and the per-page migration map, with each page gated on the
    existing verification commands (`npm run check`, `npm run build`,
    `npm run i18n:generate` when copy changes) plus a manual smoke and
    a manual a11y pass.

## Proposal assumptions (auto mode — open for user override)

The preflight is `auto`, so no proposal question round was held. The
following tactical choices are recorded as proposal-level assumptions
the user may override when reviewing this proposal. Each is a small,
isolated decision; overriding any one of them does not invalidate the
rest.

1. **Theme set shipped at v1 GA:** `caduxo-light` (custom brand) +
   DaisyUI built-in `dark`. Optional accent themes (`corporate`, `dim`,
   `winter`, etc.) deferred until after v1 polish lands. Override by
   asking for a curated accent set at GA.
2. **Theme switching in v1:** yes — the Configuration page switcher and
   the `app_settings.theme` row ship together. Override by deferring
   the switcher until dark-mode polish lands (in which case v1 ships
   `caduxo-light` only).
3. **Shared `Popover.svelte` primitive extraction:** deferred to a
   follow-up. v1 restyles DatePicker and CategoryPicker in place.
   Override by promoting the primitive extraction into v1.
4. **Urgency pulse animation:** approved, gated on
   `prefers-reduced-motion: no-preference`, applied only to the
   *expired* urgency variant (badge + leading card dot), opacity-only,
   no scale or translation. Override by removing the pulse entirely.
5. **Toast host (`ToastHost.svelte`):** deferred to a follow-up. v1
   keeps inline success / error patterns to limit scope. Override by
   promoting the toast host into v1.

## Next steps (after proposal approval)

- **Spec phase** — translate the proposal into OpenSpec delta
  requirements under
  `openspec/changes/caduxo-daisyui-redesign/specs/caduxo-expiry-tracker/spec.md`
  covering: design-system primitives, theme support, native-control
  removal, motion + reduced-motion, and per-page migration gates.
- **Design phase** — record per-effect motion tokens, per-surface
  DaisyUI mapping, the `caduxo-light` theme block, the per-page
  migration map, and the urgency-pulse rationale.
- **Tasks phase** — forecast each of the 13 phases against the 400-line
  budget; chain any Phase that still overshoots; declare `auto-chain` as
  the delivery strategy and the chain sequence.
- **Apply phase** — execute phase by phase; each phase ends with
  `npm run check`, `npm run build`, manual smoke, and a manual a11y
  pass.
- **Verify phase** — produce a verify report listing the per-phase
  evidence plus a final cross-app screenshot pass confirming zero
  OS-styled controls, working theme switcher, working reduced-motion
  compliance, and full EN + ES coverage.
- **Archive phase** — record the outcome and any follow-up slices
  (shared Popover primitive, toast host, dark-mode polish, accent
  themes) as their own OpenSpec changes so they remain individually
  reviewable.

No code edits, commits, or pushes happen in this proposal phase.