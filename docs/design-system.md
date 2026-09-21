# Caduxo design system

The contributor guide for the Caduxo frontend redesign that
landed through PR 1 → PR 13 of the
`caduxo-daisyui-redesign` OpenSpec change. The redesign
foundation is **Tailwind CSS v4 + DaisyUI v5 + two DaisyUI
themes** (`caduxo-light` as the default and `dark`). Every
visible surface composes from the shared primitives listed
below; every visible string enters the i18n catalogue in the
same PR that introduces it; every animated surface respects
`prefers-reduced-motion`.

Read this document before touching the UI. The historical
rationale and the per-PR provenance trail live in
[`docs/daisyui-redesign-plan.md`](./daisyui-redesign-plan.md)
and `openspec/changes/caduxo-daisyui-redesign/apply-progress.md`
— come back to those when you need to justify a specific
design decision, not when you need to write new code.

---

## 1. Primitive inventory

The shared UI primitives live under `src/components/ui/`. They
are leaf nodes — primitives do not import from each other and
do not consume the i18n catalogue. Consumers compose them via
Svelte snippets (`children`, `iconStart`, `iconEnd`,
`actions`, `footer`, etc.) and pass visible copy through
props. This keeps the primitives theme-aware, dependency-free,
and testable in isolation.

| Primitive          | Path                                   | Purpose                                                                                          |
| ------------------ | -------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `Button`           | `src/components/ui/Button.svelte`      | DaisyUI `btn` variants (`primary`, `secondary`, `ghost`, `outline`, `danger`, `warning`, `success`, `link`, `icon`). Sizes `xs` / `sm` / `md` / `lg`. Optional `loading` renders a DaisyUI spinner. The `icon` variant is square-shaped and requires `aria-label`. |
| `Card`             | `src/components/ui/Card.svelte`        | DaisyUI `card card-body` with `tone: default \| muted \| warning \| error \| success \| info`. Optional `header` / `footer` snippets. Surfaces `role="region"` + `aria-labelledby` when `labelled`. |
| `Badge`            | `src/components/ui/Badge.svelte`       | Two parallel APIs: `urgency` (expiry-tracker domain — `expired \| today \| alert \| soon \| normal`) and `semantic` (`success \| warning \| error \| info \| neutral`). Optional `dot` for a leading status dot. The `expired` urgency variant composes `motion-safe:animate-urgency-pulse` (the keyframes live in `src/app.css`). |
| `Alert`            | `src/components/ui/Alert.svelte`       | DaisyUI `alert alert-soft` with leading inline-SVG heroicon. Variants `success \| warning \| error \| info`. Optional `dismissible` + consumer-supplied `dismissLabel`. `role` defaults to `alert` for `error` / `warning`, `status` for `success` / `info`. |
| `EmptyState`       | `src/components/ui/EmptyState.svelte`  | Centered column with heroicon (`inbox \| calendar \| document \| tag \| search \| warning \| info \| none`), `title`, `body`, optional `actions` slot. |
| `LoadingState`     | `src/components/ui/LoadingState.svelte`| `variant: skeleton \| spinner \| text`. `rows` (default 3) for skeleton. `label` for spinner / text. `aria-live="polite"` toggled by `announce`. Skeleton uses DaisyUI `skeleton`; reduced-motion users see a static grey block via the global reset in `src/app.css`. |
| `Toggle`           | `src/components/ui/Toggle.svelte`      | DaisyUI `toggle toggle-primary toggle-{size}`. Native `<input type="checkbox">` stays in the DOM. Visible `label` OR plain `aria-label`. Sizes `sm \| md`. |
| `Tooltip`          | `src/components/ui/Tooltip.svelte`     | DaisyUI `tooltip tooltip-{position} tooltip-open`. Child receives `aria-describedby={id}`. Positions `top \| bottom \| left \| right`. |
| `Modal`            | `src/components/ui/Modal.svelte`       | Native `<dialog class="modal">` shell. `open` is `$bindable()`. `size: sm \| md \| wide`. `closeOnBackdrop`, `closeOnEscape`, `showClose`, `returnFocusTo`, `titleId`, `descriptionId`. Owns the focus-trap lifecycle + focus restoration with the `document.body.contains` guard. Scoped CSS on `<dialog>::backdrop` applies the backdrop blur only when `prefers-reduced-motion: no-preference`. |
| `Table`            | `src/components/ui/Table.svelte`       | DaisyUI `table table-zebra` / `table-pin-rows` / `table-sm`. Optional `caption`, `describedBy`, mutually exclusive `body / empty / loading` slots (priority `body > empty > loading`). `scrollable` wraps in `overflow-x-auto scrollbar-thin scrollbar-thumb-base-300`. Numeric columns use `<td class="num">` (the `num` utility lives in `src/app.css`). |
| `Tabs`             | `src/components/ui/Tabs.svelte`        | DaisyUI `tabs tabs-{style}`. `items: TabItem[]`, `activeId` bindable, `style: bordered \| lifted \| boxed`. Roving tabindex + ArrowLeft / ArrowRight / Home / End / Enter / Space keyboard handling. Each tab carries `role="tab"` + `aria-selected` + `aria-controls`; each panel carries `role="tabpanel"` + `aria-labelledby`. |
| `Select`           | `src/components/ui/Select.svelte`      | DaisyUI `select select-{size}` + `select-error` when `invalid`. Native `<select>` stays in the DOM. `value` is `$bindable()`. `options: Option[]` carries `value \| label \| disabled`. Optional `leading` snippet for placeholder. |
| `Input`            | `src/components/ui/Input.svelte`       | DaisyUI `input input-{size}` + `input-error` when `invalid`, wrapped in DaisyUI v5 `fieldset` + `fieldset-legend` + `<p class="label">` helper text. `value` is `$bindable()`. Types `text \| search \| number \| email \| url \| password`. `required` renders an accessible `*` + sr-only `(required)` annotation. When `list` is supplied, the primitive renders a `<datalist id={list}>` slot the consumer fills. |

### Theme store

`src/components/ui/theme/themeStore.svelte.ts` exports
`ThemeName` (the `"caduxo-light" \| "dark"` literal union),
`ThemeSource = "manual" \| "persisted" \| "os" \| "fallback"`,
`AVAILABLE_THEMES`, the `theme` rune (`current`, `source`),
`applyTheme(name)`, `initTheme()`, and `setTheme(next)`
(optimistic apply with rollback on `updateSettings` failure).
`initTheme()` is called from `src/main.ts` next to
`initLocale()` before the first paint.

---

## 2. Theme block layout

`src/app.css` is the single source of truth for the project's
visual foundation. Three blocks in order:

### 2.1 Tailwind import + DaisyUI plugin registration

```css
@import "tailwindcss";
@plugin "daisyui" {
  themes: caduxo-light --default, dark;
  root: ":root";
  logs: false;
}
```

`caduxo-light` is the project-branded default; `dark` is the
DaisyUI v5 built-in dark theme. `root: ":root"` ensures the
active theme is applied to `<html data-theme="…">`. `logs:
false` silences DaisyUI's build-time warnings.

### 2.2 `caduxo-light` custom theme block

Derived from the pre-redesign palette (`#2563eb` primary,
`#f6f8fb` base-100, slate neutrals, semantic red / amber /
green / blue) and expressed in `oklch` so the theme stays
perceptually uniform across both themes. Tune the
`oklch` values when you need to adjust contrast; do not
introduce new flat hex literals in migrated surfaces.

### 2.3 Motion tokens + reduced-motion reset + utilities

```css
@theme {
  --duration-fast: 120ms;
  --duration-base: 180ms;
  --duration-slow: 240ms;
  --duration-pulse: 1800ms;
  --ease-out-soft: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-in-out-soft: cubic-bezier(0.4, 0, 0.2, 1);
  --animate-urgency-pulse: urgency-pulse var(--duration-pulse)
    var(--ease-in-out-soft) infinite;
}

@keyframes urgency-pulse {
  0%, 100% { opacity: 1; }
  50%      { opacity: 0.55; }
}

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.001ms !important;
    scroll-behavior: auto !important;
  }
}

@utility num {
  font-variant-numeric: tabular-nums;
  text-align: end;
}
```

Rules:

- **Every animated surface sources duration / easing from
  the motion tokens above.** Do not introduce hand-written
  `transition: ... 0.Xs` or `animation: ... 0.Xs` literals —
  the PR 12 grep gate rejects them. Use Tailwind utilities
  like `motion-reduce:transition-none` and
  `motion-safe:animate-urgency-pulse`.
- **The reduced-motion reset is the back-stop.** Every
  DaisyUI v5 built-in animation (`skeleton`,
  `loading-spinner`, `dropdown`, `modal`) is caught by the
  global `*` selector. The pulse utility is double-gated:
  Tailwind v4 wraps the emitted rule in
  `@media (prefers-reduced-motion: no-preference)` and the
  global reset clamps the duration regardless.
- **`num` is the canonical numeric-cell utility.** Use
  `<td class="num">{value}</td>` (or `class="cell-x num"`)
  for right-aligned numeric columns. Tailwind v4 emits
  `font-variant-numeric: tabular-nums; text-align: end;`
  once across the bundle.

---

## 3. i18n discipline

Every visible string that ships with a new surface enters
`src/i18n/en/index.ts` and `src/i18n/es/index.ts` in the
**same PR** that introduces the surface. The Spanish
translations mirror the English key tree verbatim — do not
re-key the tree when adding a new namespace.

Rules:

- **No hardcoded English defaults inside primitives.** When a
  primitive surfaces visible copy (Alert `dismissLabel`,
  LoadingState `label`, Modal `closeLabel`, Toggle visible
  `label`, etc.), the consumer must pass the copy as a prop
  bound to a `$LL.*` key. PR 3 removed the old `"Dismiss"` /
  `"Loading…"` fallbacks explicitly to enforce this.
- **`predev` / `prebuild` already run
  `npm run i18n:generate`** on every build. The CI gate is
  `npm run i18n:generate && npm run check`. If the catalogue
  is out of sync with the source, the build fails before
  `svelte-check` runs.
- **The PR that adds new copy commits the regenerated
  catalogue** (`src/i18n/i18n-types.ts`). `typesafe-i18n`
  reports "all files are up to date" when no new keys are
  added; the regenerated catalogue matches the source
  unchanged.
- **Reuse existing keys before adding new ones.** When a
  surface reuses an existing label, prefer the existing
  key. Cross-namespace reuse (e.g. `dashboard.status()`
  driving a `stores.table.status` column header) is allowed
  for generic English / Spanish words; flag the cross-
  namespace reuse in the PR description so reviewers can
  ratify the choice.

---

## 4. New-surface rules

When you add a new surface (a new page, a new modal, a new
form, or a new widget):

1. **Compose from the existing primitives.** Do not introduce
   a new bespoke `.modal-*` / `.btn-*` / `.form-group-*` /
   `.table-*` family. If a primitive does not cover the
   need, grow the primitive's prop surface or add a new
   primitive in `src/components/ui/` — never inline a
   one-off DaisyUI composition in a consumer.
2. **Add EN + ES strings in the same PR.** See §3 above.
3. **Respect `prefers-reduced-motion`.** Every animated
   surface must use motion tokens (`duration-fast/base/
   slow/pulse`, `ease-out-soft`, `ease-in-out-soft`) and a
   `motion-reduce:` variant where transitions are visible.
   The PR 12 grep gate (`git grep -nE
   'transition:.*0\.[0-9]+s|animation:.*0\.[0-9]+s'
   src/components/`) must return zero matches on the new
   surface.
4. **Gate CSS bundle size.** `npm run build` reports the CSS
   bundle size on stdout. The design's regression gate is
   ±20 % against the post-PR 1 baseline (194.44 kB raw /
   29.12 kB gzip). If the new surface pushes the bundle
   over the gate, split the migration into multiple PRs or
   revisit the primitive composition (DaisyUI's emitted
   classes are usually smaller than the bespoke shell you
   are replacing).
5. **Run the verify gate.** Every PR runs
   `npm run i18n:generate && npm run check && npm run
   build`. Visual PRs (PR 3 onward) additionally run:
   - A manual `npm run dev` launch in both `caduxo-light`
     and `dark` themes (toggle via DevTools by setting
     `data-theme="dark"` on `<html>`).
   - A keyboard pass on every migrated primitive (Tab
     order, Escape, Enter / Space activation, focus
     restoration).
   - A reduced-motion pass (`prefers-reduced-motion: reduce`
     via DevTools or OS settings) confirming every animated
     surface reaches the same end state.
   - A screenshot pass on every touched surface, recorded
     in the verify report.
   - (Theme PRs) A WCAG-AA contrast pass on body / heading
     / input / badge / alert / primary button label.
6. **No half-migrated surfaces.** Every PR that opens a
   surface lands the full migration of that surface. A PR
   that leaves a surface in a visibly mixed state is
   rejected at review. No mixed surfaces — a PR that
   touches two unrelated surfaces is split at review
   (page-level, not feature-level).
7. **No business-logic changes.** Visual PRs never touch a
   Tauri command, a store function, or a domain helper,
   except where the spec explicitly requires an IPC delta
   (e.g. PR 2 for the theme field). Submit handlers,
   validation, focus-trap lifecycles, and IPC contracts
   stay verbatim.
8. **Preserve user-confirmed fixes.** Calendar and Import
   dialogs must remain visible; do not introduce DaisyUI
   `.modal` on non-dialog wrappers. The PR 9b / PR 10
   carve-out (`.modal-box-wide` + `LotDetail` inline
   overlay in CalendarPage) is intentional — those surfaces
   use a plain `<div class="modal-overlay">` parent instead
   of `<dialog>`, so the DaisyUI `modal-box` opacity-0 base
   rule needs a manual override to stay visible. Future PRs
   that re-target these surfaces should migrate to
   `<Modal bind:open={...}>` from `src/components/ui/` to
   drop the manual override.

---

## 5. Conventions and patterns

- **Plain `<button>` for nav tabs.** `src/App.svelte` renders
  the navbar tabs as plain `<button class="btn btn-ghost
  btn-sm">` so the `aria-current="page"` attribute flows
  through to the DOM. The `Button.svelte` primitive does
  not yet expose `aria-current` — when it does, the nav can
  move back to the primitive.
- **Plain `<button>` for chips that need `btn-active`.**
  `DashboardPage.svelte`'s quick-filter chips render as
  plain `<button class="btn btn-ghost btn-sm"
  class:btn-active={isActive} aria-pressed={isActive}>` for
  the same reason — `Button.svelte` does not expose the
  `btn-active` modifier. Same fix when the primitive grows.
- **DaisyUI `alert` always uses `alert-soft`.** The
  `Alert.svelte` primitive composes `alert alert-{variant}
  alert-soft` for every variant so the chrome reads as
  filled-but-muted rather than outline-only.
- **DaisyUI v5 emits different class names than v4.** The
  redesign uses DaisyUI v5.7.42. The v4-era names
  (`tabs-bordered`, `tabs-lifted`, `tabs-boxed`,
  `input-bordered`, `select-bordered`, `form-control`,
  `label-text`, `label-text-alt`) are not emitted. The
  v5 names are `tabs-border`, `tabs-lift`, `tabs-box`,
  `fieldset`, `fieldset-legend`, `label`, `input`,
  `select`. The `Tabs.svelte` public `style` prop keeps
  the v4 string union (`"bordered" | "lifted" | "boxed"`)
  for task-contract stability; only the internal class
  emission changed.
- **Color literals are theme-derived.** Every colour in a
  migrated surface is either a DaisyUI semantic token
  (`var(--color-primary)`, `var(--color-base-content)`,
  `var(--color-error)`, etc.) or a `color-mix(in oklch,
  var(--color-…) X%, transparent)` derivation. Flat hex
  literals (`#2563eb`, `#fee2e2`, etc.) appear only as
  fallbacks inside `var(--color-…, #fallback)` so a missing
  theme token still renders. New flat hex literals are
  rejected at review.
- **Per-row tinting lives in the consumer.** The
  `Table.svelte` primitive does not own per-row colour
  tinting. Row-level visual cues (`row-expired` /
  `row-today` / `row-alert` / `row-soon` / `row-normal` on
  ReportsPage; `badge-ok` / `badge-warn` / `badge-error`
  on CsvImportPage) are intentional and live in the
  consumer's `<style>` block. They are not in any migration
  scope.

---

## 6. File map

```
src/
├── app.css                              ← Tailwind v4 + DaisyUI v5 + caduxo-light theme + motion tokens + reduced-motion reset
├── main.ts                              ← bootstrap; imports `./app.css`; calls `Promise.all([initLocale(), initTheme()])`
├── App.svelte                           ← DaisyUI navbar + dashboard gradient band + responsive collapse at 720 px
├── components/
│   ├── ui/                              ← shared primitives (Button, Card, Badge, Alert, EmptyState, LoadingState, Toggle, Tooltip, Modal, Table, Tabs, Select, Input)
│   ├── ui/theme/themeStore.svelte.ts    ← ThemeName / ThemeSource / AVAILABLE_THEMES / theme rune / applyTheme / initTheme / setTheme
│   ├── inputs/CategoryPicker.svelte     ← chip combobox with WAI-ARIA combobox pattern + DaisyUI `dropdown dropdown-content` popover root
│   ├── CalendarMonth.svelte             ← DaisyUI Button / Tooltip on chevron nav + bespoke day cells / year chips
│   ├── CalendarPage.svelte              ← DaisyUI Button / Tooltip on refresh + Badge on count pill + Table on day-detail (PR 9b)
│   ├── DatePicker.svelte                ← DaisyUI Button on calendar / clear icons + bespoke trigger input + `dropdown dropdown-content` popover root
│   ├── UnitReviewPage.svelte            ← DaisyUI collapse-arrow panels + Input / Button / Alert / Badge primitives
│   ├── DashboardPage.svelte             ← first consumer of every primitive (PR 6)
│   ├── *Modal.svelte / *Dialog.svelte   ← all migrated to `<Modal bind:open={...}>` from `src/components/ui/`
│   ├── ProductForm.svelte, LotForm.svelte, ConfigurationPage.svelte
│   ├── ReportsPage.svelte, BackupRestorePage.svelte, CsvImportPage.svelte
│   ├── LotMovementsPanel.svelte, StoresPage.svelte, ProductCatalogPage.svelte
│   ├── ColumnMapper.svelte, ScanSearchBox.svelte, ProductDetailPage.svelte, UnitReviewBanner.svelte
│   └── …                                ← see `openspec/changes/caduxo-daisyui-redesign/tasks.md` for the full inventory
├── i18n/                                ← typesafe-i18n catalogues; every visible string lives here
└── lib/                                 ← business logic; never touched by visual PRs except where the spec requires an IPC delta
```

---

## 7. Verify gates (every PR)

Every PR runs, at minimum:

```bash
npm run i18n generate   # if the i18n catalogue changed
npm run check           # svelte-check --threshold error
npm run build           # vite build
```

PRs that touch the backend also run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo build --manifest-path src-tauri/Cargo.toml
```

Visual PRs (PR 3 onward) include the manual checks listed in
§4.5.

---

## 8. Pointers

- [`docs/daisyui-redesign-plan.md`](./daisyui-redesign-plan.md) —
  the historical plan + risk register; consult when you need
  provenance for a specific design decision.
- [`openspec/changes/caduxo-daisyui-redesign/proposal.md`](../openspec/changes/caduxo-daisyui-redesign/proposal.md) —
  the OpenSpec proposal that started this change.
- [`openspec/changes/caduxo-daisyui-redesign/design.md`](../openspec/changes/caduxo-daisyui-redesign/design.md) —
  the visual / interaction design that backs the primitives
  and the migration order.
- [`openspec/changes/caduxo-daisyui-redesign/tasks.md`](../openspec/changes/caduxo-daisyui-redesign/tasks.md) —
  the per-PR task list with the per-task verify gates.
- [`openspec/changes/caduxo-daisyui-redesign/apply-progress.md`](../openspec/changes/caduxo-daisyui-redesign/apply-progress.md) —
  the per-PR evidence rollup (diff sizes, bundle deltas,
  grep gate results, deviations from design).
