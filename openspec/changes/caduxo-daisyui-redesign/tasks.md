# Tasks — caduxo-daisyui-redesign

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~2 400 – 3 200 net (frontend ~2 100 – 2 800, backend contract + IPC delta ~60 – 120, design doc touch-ups ~80, baseline notes ~80). Wide range because per-PR actual size depends on how many phases the chain-strategy choice forces to split further. |
| 400-line budget risk | High if delivered as a single PR; Low if delivered as chained PRs (the design's default plan). Several phases (6, 7, 8, 9) already sit at ~350 – 400 lines and must split if any single file bloats during apply. |
| Chained PRs recommended | Yes — design §10.1 commits to one-phase-per-PR by default with further splits when a phase overshoots the 400-line review budget. |
| Suggested split | PR 1 foundation → PR 2 backend IPC delta → PR 3 primitives A → PR 4 primitives B → PR 5 app shell + switcher → PR 6 dashboard → PR 7 modals (split 7a + 7b if apply-time diff > 400) → PR 8 forms (split 8a + 8b if apply-time diff > 400) → PR 9 tables → PR 10 calendar + custom widgets → PR 11 responsive → PR 12 motion + effects → PR 13 cleanup → PR 14 verify + archive |
| Delivery strategy | auto-chain (session preflight; chain strategy is `deferred` until PR 1 apply) |
| Chain strategy | pending — design recommends `stacked-to-main` for PR 1 → PR 2 (so the frontend can resolve the new IPC contract) and `feature-branch-chain` for the remaining chained PRs that only touch frontend code. Parent ratifies before PR 1 apply. |

```text
Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: stacked-to-main|feature-branch-chain|size-exception|pending
400-line budget risk: Low|Medium|High
```

The chain strategy is `deferred` at preflight; the design's recommended shape
is `stacked-to-main` for PR 1 → PR 2 and `feature-branch-chain` from PR 3
onward. The parent must ratify the chain strategy (or accept a `size:exception`
if they prefer to ship PR 6 / 7 / 8 / 9 as single PRs despite the budget risk)
before PR 1 apply. No `size:exception` is requested at this phase.

Phase-level forecasts below assume the per-phase shape described in
`docs/daisyui-redesign-plan.md` and `openspec/changes/caduxo-daisyui-redesign/design.md`
§6. Forecasts are intentionally conservative; the apply phase re-measures per
PR and splits further when a PR's actual diff exceeds 400 changed lines.

> **Project check command note.** `package.json` does not yet define a
> `check` script. The canonical check command used by prior changes is
> `npx svelte-check --tsconfig ./tsconfig.json --threshold error`. We
> treat that as `npm run check` for this document and for every verify
> gate. Adding a `"check": "svelte-check --tsconfig ./tsconfig.json --threshold error"`
> script to `package.json` is part of PR 1 (foundation) so subsequent
> PRs and the verify phase can call `npm run check` directly.

---

## Cross-cutting constraints (apply to every phase)

- **i18n discipline preserved.** Every new visible string enters
  `src/i18n/en/index.ts` and `src/i18n/es/index.ts` in the same PR that
  surfaces it; `predev` / `prebuild` already run
  `npm run i18n:generate`; CI gate is
  `npm run i18n:generate && npm run check`.
- **No business-logic changes.** Visual phases never touch a Tauri
  command, a store function, or a domain helper, except where the spec
  explicitly requires an IPC delta (PR 2 for the theme field).
- **No half-migrated surfaces.** Every PR that opens a surface lands the
  full migration of that surface. A PR that leaves a surface in a
  visibly mixed state is rejected at review.
- **No mixed surfaces.** A PR that touches two unrelated surfaces is
  split at review. The split rule is page-level, not feature-level.
- **i18n strings land with the surface.** A new visible string enters
  the i18n catalogue in the same PR that introduces the surface, not in
  a follow-up.
- **Reduced-motion compliance is global.** The `prefers-reduced-motion:
  reduce` reset in `src/app.css` (introduced in PR 1) is the safety net
  for every animated surface; primitives opt into custom animations
  knowing the reset clamps them.
- **Theme tokens only — no hex literals.** From PR 3 onward, no migrated
  file inlines a hex / rgb colour that is not theme-derived.
- **DaisyUI built-in theme set is `["caduxo-light", "dark"]`** until the
  parent approves an accent-theme follow-up change.

---

## PR 1 — Foundation (Tailwind + DaisyUI + theme + global motion reset)

`docs/daisyui-redesign-plan.md` Phase 0 + Phase 1. Foundation-first per
the spec's "foundation-first delivery order" requirement. **Lands first;
gates every subsequent chained PR.**

### 1.1 Baseline + branch

- [x] Create branch `feat/daisyui-redesign` off `main` (do NOT start from
      `feat/daisyui-redesign-plan`; that branch carries only the planning
      doc and will be retired once work starts). <!-- sdd-owner: implementation -->
- [x] Write `docs/redesign-baseline.md` listing the current visual state
      of every main surface (Dashboard, Stores, Products, Product
      detail, Calendar, Reports, CSV Import, Backup/Restore,
      Configuration, Unit Review) with explicit notes on viewport
      widths (1440, 1024, 720 px), existing modal shells, and the OS
      chrome that today leaks into the Configuration locale selector.
      No UI code changes. <!-- sdd-owner: implementation -->
- [x] Capture baseline screenshots (or annotated textual descriptions
      when a screenshot is impractical) and link them from the baseline
      doc. <!-- sdd-owner: implementation -->

### 1.2 Dependencies + Vite plugin + scripts

- [x] Add `tailwindcss@^4`, `@tailwindcss/vite@^4`, and `daisyui@^5` to
      `dependencies` in `package.json`; run `npm install` and confirm
      the lockfile updates without breaking existing entries. <!-- sdd-owner: implementation -->
- [x] Update `vite.config.ts` so the plugin chain is
      `plugins: [tailwindcss(), svelte()]`, with `tailwindcss()` first
      so Tailwind's class-collection pass sees every Svelte component's
      class usage during the same build pass. Preserve the existing
      `clearScreen`, `build.target = "es2022"`, `server.strictPort`,
      and `server.port = 1420`. <!-- sdd-owner: implementation -->
- [x] Add `"check": "svelte-check --tsconfig ./tsconfig.json --threshold error"`
      to `package.json` scripts so every subsequent phase can call
      `npm run check` as the canonical check command. <!-- sdd-owner: implementation -->

### 1.3 Stylesheet entry + main.ts switch

- [x] Create `src/app.css` containing `@import "tailwindcss";` followed
      by `@plugin "daisyui" { themes: caduxo-light --default, dark;
      root: ":root"; logs: false; }` plus the `caduxo-light` custom
      theme block derived from the existing palette
      (`#2563eb` primary, `#f6f8fb` base-100, slate neutrals, semantic
      red / amber / green / blue). Use the oklch starting values from
      design §3.2 and tune during the verify pass. <!-- sdd-owner: implementation -->
- [x] Add project motion tokens to `src/app.css` via Tailwind v4
      `@theme`: `--duration-fast: 120ms`, `--duration-base: 180ms`,
      `--duration-slow: 240ms`, `--duration-pulse: 1800ms`,
      `--ease-out-soft: cubic-bezier(0.16, 1, 0.3, 1)`,
      `--ease-in-out-soft: cubic-bezier(0.4, 0, 0.2, 1)`. <!-- sdd-owner: implementation -->
- [x] Add a `@media (prefers-reduced-motion: reduce)` global reset to
      `src/app.css` that clamps every animation / transition to
      `0.001ms` and forces `scroll-behavior: auto`. The reset targets
      `*, *::before, *::after` so no animated surface escapes the gate. <!-- sdd-owner: implementation -->
- [x] Add a `@utility num { font-variant-numeric: tabular-nums;
      text-align: end; }` Tailwind v4 utility to `src/app.css` for
      numeric column alignment (used by PR 9 tables). <!-- sdd-owner: implementation -->
- [ ] Add `app-shell-gradient` keyframes / utility if the dashboard
      header band uses one; otherwise defer until PR 6. <!-- sdd-owner: implementation -->
- [x] Update `src/main.ts` to import `./app.css` instead of `./style.css`.
      Leave `src/style.css` in place during the migration; it is
      retired in PR 13 once the grep gate passes. <!-- sdd-owner: implementation -->

### 1.4 Theme store skeleton (frontend IPC consumer)

- [ ] Extend `src/lib/stores.ts` `SettingsResponse` and `SettingsUpdate`
      types with `theme: ThemeName` and `theme_configured: boolean`
      (response) plus optional `theme?: ThemeName` (update). Re-export
      `ThemeName` from the theme store so consumers can type against it. <!-- sdd-owner: implementation -->
- [ ] Create `src/components/ui/theme/themeStore.svelte.ts` exporting
      `ThemeName = "caduxo-light" | "dark"`, `ThemeSource = "manual" |
      "persisted" | "os" | "fallback"`, an `AVAILABLE_THEMES` array,
      the `theme` rune, `applyTheme(name)` (writes
      `document.documentElement.setAttribute("data-theme", name)`),
      `initTheme()` (persisted > OS > fallback precedence, re-reads
      `prefers-color-scheme` on every call), and `setTheme(next)`
      (optimistic apply with rollback on `updateSettings` failure).
      `initTheme()` is called from `src/main.ts` next to
      `initLocale()`. <!-- sdd-owner: implementation -->

### 1.5 i18n strings for the foundation

- [ ] Add a `theme` namespace to `src/i18n/en/index.ts` and
      `src/i18n/es/index.ts` with the display names
      `theme.caduxoLight`, `theme.dark`, and the source labels
      `theme.source.manual`, `theme.source.persisted`,
      `theme.source.os`, `theme.source.fallback`. Spanish translations
      follow the same `theme.*` key tree verbatim. <!-- sdd-owner: implementation -->
- [ ] Run `npm run i18n:generate`; commit the regenerated catalogue.
      <!-- sdd-owner: implementation -->

### 1.6 PR 1 verify gate

- [x] `npm run i18n:generate` green. <!-- sdd-owner: implementation -->
- [x] `npm run check` (svelte-check --threshold error) green; no new
      errors or warnings introduced by the Vite plugin chain. <!-- sdd-owner: implementation -->
- [x] `npm run build` exits green. <!-- sdd-owner: implementation -->
- [ ] Manual launch — `npm run dev` boots in the dev browser; no
      console errors, Svelte HMR works, the existing UI renders without
      widespread preflight breakage. If Tauri is available locally,
      `npm run tauri dev` also boots cleanly (document if unavailable). <!-- sdd-owner: implementation -->
- [ ] Manual contrast pass on the un-migrated UI in both `caduxo-light`
      and `dark` themes (toggle via DevTools by setting
      `data-theme="dark"` on `<html>`) — no widespread legibility
      regressions; preflight may shift default heading / list / button
      styling but every main surface remains usable. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `package.json`, `package-lock.json` (deps + scripts).
- `vite.config.ts` (plugin chain).
- `src/app.css` (new — Tailwind + DaisyUI + theme + motion tokens +
  reduced-motion reset + `num` utility).
- `src/main.ts` (import switch).
- `src/lib/stores.ts` (extended `SettingsResponse` / `SettingsUpdate`).
- `src/components/ui/theme/themeStore.svelte.ts` (new).
- `src/i18n/en/index.ts`, `src/i18n/es/index.ts` (theme namespace).
- `docs/redesign-baseline.md` (new).

**Forecast:** ~200 net additions (design §6 estimates ~150; the
extended `SettingsResponse` shape and the theme rune add ~50). **Rollback:**
revert PR 1; the un-migrated UI is unchanged. **`src/style.css` is
preserved** — deleting it would break anything not yet migrated.

---

## PR 2 — Backend IPC delta for theme persistence

The frontend contract is wired in PR 1; this PR makes the backend
actually serve `theme` and `theme_configured`. **Stacks onto PR 1** so
the frontend code in PR 1 resolves against a real backend.

### 2.1 Backend settings DTO

- [x] Add `pub theme: String` (default `"caduxo-light"`) and
      `pub theme_configured: bool` (default `false`) to
      `SettingsResponse` in `src-tauri/src/dto/stores.rs`; add
      `pub theme: Option<String>` to `SettingsUpdate` in the same file. <!-- sdd-owner: implementation -->
- [x] Update `src-tauri/src/db/repositories/settings.rs::get_settings`
      so the response carries `theme` with the `caduxo-light` fallback
      when the row is absent or empty, and `theme_configured` mirrors
      the existing `language_configured` pattern (`theme IS NOT NULL`). <!-- sdd-owner: implementation -->
- [x] Branch on `input.theme` in
      `src-tauri/src/services/settings.rs::update_settings` so a partial
      update of only `theme` does not touch the other settings keys. <!-- sdd-owner: implementation -->
- [x] Reject `theme` values outside `{"caduxo-light", "dark"}` at the
      IPC boundary in `src-tauri/src/commands/stores.rs::update_settings`;
      return a `CommandError::Validation` and leave the persisted row
      untouched. <!-- sdd-owner: implementation -->
- [x] Extend the migration runner (or add a new V-N migration) so the
      `app_settings` table accepts a `theme` text column defaulting to
      `NULL`. Use the existing migration pattern (`MIGRATIONS` array in
      `src-tauri/src/db/migrations.rs`); the migration MUST be
      idempotent and MUST NOT rewrite existing rows. <!-- sdd-owner: implementation -->

### 2.2 Backend tests

- [x] Add `#[tokio::test]` cases in
      `src-tauri/src/db/repositories/settings.rs`: missing row returns
      `theme = "caduxo-light"`; empty string returns
      `"caduxo-light"`; `"synthwave"` (unsupported) returns
      `"caduxo-light"`; partial update of only `theme` preserves the
      other keys; `set_theme("synthwave")` is rejected by the command
      handler. <!-- sdd-owner: implementation -->
- [x] Add an idempotency test asserting re-running the new V-N
      migration on an already-migrated pool leaves row counts
      unchanged. <!-- sdd-owner: implementation -->

### 2.3 PR 2 verify gate

- [x] `cargo test --manifest-path src-tauri/Cargo.toml --lib` green. <!-- sdd-owner: implementation -->
- [x] `cargo build --manifest-path src-tauri/Cargo.toml` green. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — Tauri (or test harness) reads `app_settings.theme`
      round-trip: setting `dark` is persisted across restart; clearing
      the row falls back to OS preference. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src-tauri/src/dto/stores.rs` (DTO wire shape).
- `src-tauri/src/db/repositories/settings.rs` (read / write helpers).
- `src-tauri/src/services/settings.rs` (partial update branch).
- `src-tauri/src/commands/stores.rs` (boundary validation).
- `src-tauri/src/db/migrations.rs` (V-N migration).

**Forecast:** ~120 net additions. **Rollback:** revert PR 2; the
frontend code in PR 1 falls back to the `caduxo-light` default
through the existing fallback path.

---

## PR 3 — Shared UI primitives batch A

`docs/daisyui-redesign-plan.md` Phase 2 batch A. Pure presentational
primitives; no surface migration in this PR.

- [x] Create `src/components/ui/Button.svelte` per design §2.3: variants
      `primary | secondary | ghost | outline | danger | warning |
      success | link | icon`; sizes `xs | sm | md | lg`; `disabled`,
      `loading`, optional `iconStart` / `iconEnd` snippets;
      `aria-label` required when `variant === "icon"`. Composes DaisyUI
      `btn btn-{variant}` and `loading loading-spinner loading-sm`. <!-- sdd-owner: implementation -->
- [x] Create `src/components/ui/Card.svelte`: optional `header`,
      `footer`, `tone: default | muted | warning | error | success`
      props; composes `card card-body`, with `bg-base-200` for the muted
      tone and the matching `border-{tone}` modifier for status tones.
      Surfaces `role="region"` + `aria-labelledby` when `labelled`. <!-- sdd-owner: implementation -->
- [x] Create `src/components/ui/Badge.svelte`: `urgency: expired | today
      | alert | soon | normal` and `semantic: success | warning | error
      | info | neutral` props (urgency takes precedence); optional
      `dot` for a leading status dot; sizes `sm | md`. The expired
      variant optionally renders `motion-safe:animate-urgency-pulse`
      (keyframes defined in PR 12). <!-- sdd-owner: implementation -->
- [x] Create `src/components/ui/Alert.svelte`: `variant: success | error
      | warning | info`; optional `dismissible`, `title`; `actions`
      slot. Composes `alert alert-{variant}` with a leading inline SVG
      heroicon (no icon dependency; inline SVG only). <!-- sdd-owner: implementation -->
- [x] Create `src/components/ui/EmptyState.svelte`: `title`, `body`,
      optional `icon` (name resolved to an inline-SVG heroicon map in
      the component file), optional `actions` slot. Composes centered
      column with `text-base-content/70`. <!-- sdd-owner: implementation -->
- [x] Create `src/components/ui/LoadingState.svelte`: `variant:
      skeleton | spinner | text`; `rows` (default 3) for the skeleton
      variant; `label` for the text variant; `announce` flag toggles
      `aria-live="polite"`. Skeleton composes DaisyUI `skeleton` —
      reduced-motion users see a static grey block via the global
      reset. <!-- sdd-owner: implementation -->
- [x] Create `src/components/ui/Toggle.svelte`: `checked`, `label`,
      `size: sm | md`, `disabled`. Composes DaisyUI `toggle
      toggle-primary toggle-{size}`; the underlying
      `<input type="checkbox">` stays in the DOM for form semantics. <!-- sdd-owner: implementation -->
- [x] Create `src/components/ui/Tooltip.svelte`: `text`, `position:
      top | bottom | left | right`, optional `id`; composes DaisyUI
      `tooltip tooltip-{position}` and `tooltip-open` on hover /
      `:focus-visible`; child element receives `aria-describedby={id}`.
      <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src/components/ui/Button.svelte`, `Card.svelte`, `Badge.svelte`,
  `Alert.svelte`, `EmptyState.svelte`, `LoadingState.svelte`,
  `Toggle.svelte`, `Tooltip.svelte` (all new).

**Forecast:** ~280 net additions (design §6 estimates ~250; 8 files
× ~30 – 40 lines each). **Rollback:** each primitive is reversible
independently; no surface consumes them yet so no consumer breaks.

---

## PR 4 — Shared UI primitives batch B

`docs/daisyui-redesign-plan.md` Phase 2 batch B. Heavier primitives
that need positioning logic and ARIA wiring.

- [ ] Create `src/components/ui/Modal.svelte` per design §2.3: backed
      by `<dialog class="modal">`; `open` rune, `size: sm | md | wide`,
      `closeOnBackdrop`, `closeOnEscape`, `showClose`, `returnFocusTo`,
      `titleId`, `descriptionId`; default-slot body, `footer` slot.
      Owns the focus-trap lifecycle (listen for `focusin`, bounce focus
      back inside; listen for `cancel` event, surface cancellation
      callback), focus restoration to `returnFocusTo` on close (no-op
      when `document.body.contains(returnFocusTo) === false`). <!-- sdd-owner: implementation -->
- [ ] Add scoped CSS in `Modal.svelte` for `<dialog>::backdrop` so a
      `backdrop-blur-sm` is applied when
      `@media (prefers-reduced-motion: no-preference)` is set (the
      blur render itself triggers a paint transition on some
      platforms). <!-- sdd-owner: implementation -->
- [ ] Create `src/components/ui/Table.svelte`: `zebra`, `stickyHeader`,
      `size: dense | default`, optional `caption`, `describedBy`,
      mutually exclusive `empty` / `loading` slots. Composes
      `table table-zebra`, `table-pin-rows`, `table-sm`. Numeric
      columns use `<td class="num">` from the PR 1 utility. <!-- sdd-owner: implementation -->
- [ ] Create `src/components/ui/Tabs.svelte`: `items: TabItem[]`,
      `activeId`, `style: bordered | lifted | boxed`, `onchange`,
      `aria-label`. Handles ArrowLeft / ArrowRight / Home / End /
      Enter / Space. Renders `role="tablist"` on the wrapper, each tab
      `role="tab"`, each panel `role="tabpanel"` with `aria-labelledby`. <!-- sdd-owner: implementation -->
- [ ] Create `src/components/ui/Select.svelte`: `value`, `options:
      Option[]`, `size: sm | md`, `disabled`, `invalid`,
      `aria-label` / `aria-labelledby`. Composes DaisyUI
      `select select-bordered select-{size}` and `select-error` when
      `invalid`. Native `<select>` stays in the DOM for form
      semantics; the visible surface is the DaisyUI wrapper. <!-- sdd-owner: implementation -->
- [ ] Create `src/components/ui/Input.svelte`: `value`, `type: text |
      search | number | email | url | password`, `label`, `required`,
      `helper`, `invalid`, `list`, `size: sm | md | lg`, `disabled`,
      `aria-label` / `aria-describedby`. Composes DaisyUI
      `input input-bordered input-{size}` with `input-error` when
      `invalid`, paired with `label`, `label-text`,
      `label-text-alt`. When `list` is supplied the primitive renders a
      `<datalist id={list}>` slot the consumer fills. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src/components/ui/Modal.svelte`, `Table.svelte`, `Tabs.svelte`,
  `Select.svelte`, `Input.svelte` (all new).

**Forecast:** ~260 net additions (design §6 estimates ~200; the modal
needs positioning + ARIA wiring that pushes the per-file average
higher). **Rollback:** each primitive is reversible independently.

---

## PR 5 — App shell navbar + Configuration theme switcher

`docs/daisyui-redesign-plan.md` Phase 3. Replaces the bespoke
`.nav / .nav-btn / .nav-brand` markup with DaisyUI `navbar` and
lands the theme switcher next to the language selector.

### 5.1 App shell

- [ ] Migrate `src/App.svelte` top nav to DaisyUI `navbar bg-base-200`
      with `navbar-start` (brand), `navbar-center` (tab buttons via the
      `Button.svelte` `ghost` variant), `navbar-end` (overflow actions
      on narrow viewports). The active-tab state stays on the existing
      `activeTab` rune; tab buttons become DaisyUI-themed buttons. <!-- sdd-owner: implementation -->
- [ ] On viewports ≤ 720 px, collapse the tab buttons behind a
      DaisyUI `dropdown` trigger; every tab remains reachable via
      keyboard, active tab stays obvious. <!-- sdd-owner: implementation -->
- [ ] Render the subtle dashboard gradient band
      (`bg-gradient-to-br from-primary/5 to-base-100`) behind the
      brand area on the Dashboard landing tab only. No animation; no
      reduced-motion gate needed. <!-- sdd-owner: implementation -->

### 5.2 Theme switcher on Configuration page

- [ ] Migrate `src/components/ConfigurationPage.svelte` to use the
      `Select.svelte` primitive for the language selector (preserves
      every existing locale behaviour from the i18n-support change,
      including the detected-hint copy and rollback on IPC failure). <!-- sdd-owner: implementation -->
- [ ] Migrate the bespoke `toggle-wrap / toggle-track / toggle-thumb`
      controls on `ConfigurationPage.svelte` to the `Toggle.svelte`
      primitive (preserves the existing "Ubicación inicial obligatoria"
      toggle behaviour). <!-- sdd-owner: implementation -->
- [ ] Add a new "Tema / Theme" section to `ConfigurationPage.svelte`
      that renders the theme switcher: lists `AVAILABLE_THEMES` (from
      `themeStore.svelte.ts`) in the active locale via
      `$LL.theme.caduxoLight()` / `$LL.theme.dark()`, marks the
      currently active theme, and surfaces the active source via
      `$LL.theme.source.{manual,persisted,os,fallback}()` next to the
      active entry. <!-- sdd-owner: implementation -->
- [ ] Wire the switcher to `setTheme(next)` from the theme store;
      ensure the
      "failed theme persistence rolls back the optimistic switch"
      scenario from the spec renders an inline error in the switcher
      section (use the `Alert.svelte` primitive, `variant="error"`). <!-- sdd-owner: implementation -->
- [ ] Add new i18n keys under `settings.theme.title`,
      `settings.theme.description`, and `settings.theme.error` to
      `src/i18n/en/index.ts` and `src/i18n/es/index.ts`. <!-- sdd-owner: implementation -->
- [ ] Run `npm run i18n:generate`; commit the regenerated catalogue. <!-- sdd-owner: implementation -->

### 5.3 PR 5 verify gate

- [ ] `npm run i18n:generate` green. <!-- sdd-owner: implementation -->
- [ ] `npm run check` green. <!-- sdd-owner: implementation -->
- [ ] `npm run build` green. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — `caduxo-light` and `dark` themes render correctly
      app-wide; theme switcher shows the active theme + source;
      switching persists across restart; failed IPC rolls back the
      optimistic switch and surfaces the error; navbar collapses at
      720 px; locale selector no longer renders OS-styled chrome. <!-- sdd-owner: implementation -->
- [ ] Manual a11y pass — Tab order is preserved across the navbar;
      collapsed nav remains keyboard-reachable; switcher is
      keyboard-reachable; focus rings remain visible against both
      themes. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src/App.svelte` (navbar + responsive collapse + gradient band).
- `src/components/ConfigurationPage.svelte` (Select + Toggle + theme
  switcher section + error surface).
- `src/i18n/en/index.ts`, `src/i18n/es/index.ts` (settings.theme
  namespace).

**Forecast:** ~300 net additions (design §6 estimates ~250; the
switcher + error surface + new i18n keys add ~50). **Rollback:**
revert the two files; the existing nav and the i18n-supported
Configuration page return.

---

## PR 6 — Dashboard polish

`docs/daisyui-redesign-plan.md` Phase 4. Migrates the most visible
screen first to give stakeholder reviewers something to look at.

- [ ] Migrate `src/components/DashboardPage.svelte` urgency cards to
      `Card.svelte` (`tone` for the urgency-driven tint) + `Badge.svelte`
      for the leading status dot + DaisyUI `stats` / `stat` pattern
      for the bucket counts. Replace local `.urgency-card`,
      `.urgency-card-expired`, `.urgency-card-today`,
      `.urgency-card-alert`, `.urgency-card-soon`,
      `.urgency-card-normal` classes. <!-- sdd-owner: implementation -->
- [ ] Migrate urgency badges (`.urgency-badge`,
      `urgencyClass(lot.urgency)`, `.status-*`, `.lot-picker-status`)
      to `Badge.svelte` with the appropriate `urgency` /
      `semantic` prop and optional `dot`. <!-- sdd-owner: implementation -->
- [ ] Migrate the dashboard lot table to `Table.svelte` (`zebra`,
      `stickyHeader`, `loading` slot pointing at `LoadingState.svelte`,
      `empty` slot pointing at `EmptyState.svelte`). Numeric columns
      use the `num` utility. <!-- sdd-owner: implementation -->
- [ ] Replace local `.tab-btn`, `.detail-tabs`, `.tab-content`
      with `Tabs.svelte` (`bordered` style). <!-- sdd-owner: implementation -->
- [ ] Replace the dashboard error banner (`.error-banner`) with
      `Alert.svelte` (`variant="error"`). <!-- sdd-owner: implementation -->
- [ ] Replace the scan-row spinner (`.scan-spinner`) with the DaisyUI
      `loading loading-spinner loading-sm` primitive (used inline; the
      shared `LoadingState.svelte` also exists for full-page loading
      surfaces). <!-- sdd-owner: implementation -->
- [ ] Migrate every action button on the dashboard to `Button.svelte`
      with the appropriate variant (primary for the "Nuevo producto"
      quick-create, ghost for filter row chips, danger / warning /
      success per urgency action). Title-attribute tooltips on
      icon-only buttons become `Tooltip.svelte`. <!-- sdd-owner: implementation -->
- [ ] Add new i18n keys for any new dashboard copy (empty-state
      titles / bodies, loading-state text, alert copy, action
      labels). EN + ES in the same PR. <!-- sdd-owner: implementation -->
- [ ] Run `npm run i18n:generate`; commit the regenerated catalogue. <!-- sdd-owner: implementation -->

### 6.x PR 6 verify gate

- [ ] `npm run i18n:generate` green. <!-- sdd-owner: implementation -->
- [ ] `npm run check` green. <!-- sdd-owner: implementation -->
- [ ] `npm run build` green. <!-- sdd-owner: implementation -->
- [ ] `git grep -nE '\.(urgency-card|urgency-badge|status-|tab-btn|detail-tabs|tab-content|error-banner|scan-spinner|loading-row)\b' src/components/DashboardPage.svelte`
      returns zero matches as the active surface class. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — every canonical Dashboard scenario from the
      unchanged Dashboard capability continues to pass: sections
      align with filter buttons, category filter ANY-of semantics,
      urgency-card bucket counts, quick-filter row predicates, scan
      / search, urgency filters, row actions, modal triggers. <!-- sdd-owner: implementation -->
- [ ] Manual reduced-motion pass — toggling
      `prefers-reduced-motion: reduce` removes the urgency-pulse
      animation on the expired variant; the badge remains visually
      distinct through colour + icon + text. <!-- sdd-owner: implementation -->
- [ ] Manual screenshot pass in `caduxo-light` and `dark`; both
      recorded in the verify report. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src/components/DashboardPage.svelte` (single-file migration; the
  inline modal overlays inside DashboardPage stay in place — they
  migrate in PR 7).

**Forecast:** ~350 net additions (design §6 estimate). **Rollback:**
revert `DashboardPage.svelte`; the page returns to the un-migrated
state.

---

## PR 7 — Modal migration (split 7a + 7b if apply-time diff > 400)

`docs/daisyui-redesign-plan.md` Phase 5. Five dedicated modal
components plus the inline product / lot / quick-create overlays
inside `DashboardPage`. The single-PR forecast sits at ~400 lines; if
apply-time exceeds the budget, split into:

- **PR 7a** — `MoveStockModal`, `AdjustCountModal`, `ArchiveLotDialog`
  (three migration primitives, ~250 lines forecast).
- **PR 7b** — `RegisterExitModal`, `ResolveQuantityDialog`, plus the
  three inline overlays inside `DashboardPage` (~280 lines forecast).

Apply-time gate: after the first modal lands, run `git diff --stat`;
if `+` + `-` lines exceed 400, abort PR 7a mid-stream and continue the
remaining three modals as PR 7b.

### 7.1 Modal-by-modal migration (per modal)

- [ ] Replace the legacy `.modal-overlay / .modal-box / .modal-box-wide
      / .modal-header / .modal-body / .modal-footer / .modal-close /
      .modal-loading` markup with `<Modal bind:open={visible} …>`. <!-- sdd-owner: implementation -->
- [ ] Move the existing close handler to `onClose`; preserve the
      "discard in-progress edits on Escape" semantics by hooking the
      dialog's `cancel` event (the native `<dialog>` cancellation
      hook). <!-- sdd-owner: implementation -->
- [ ] Replace the `✕` close button with `showClose` (a
      `btn btn-circle btn-ghost btn-sm absolute top-2 end-2`). <!-- sdd-owner: implementation -->
- [ ] Move header / body / footer markup into the corresponding slots. <!-- sdd-owner: implementation -->
- [ ] Preserve all existing business state, validation, and submit
      handlers — only the shell changes. <!-- sdd-owner: implementation -->

### 7.2 Per-modal targets

- [ ] `src/components/MoveStockModal.svelte` — migrate shell; preserve
      source / destination select + quantity input. <!-- sdd-owner: implementation -->
- [ ] `src/components/AdjustCountModal.svelte` — migrate shell; the
      `real physical quantity` input becomes `Input.svelte`. <!-- sdd-owner: implementation -->
- [ ] `src/components/ArchiveLotDialog.svelte` — migrate shell;
      confirmation button stays as `Button.svelte variant="danger"`. <!-- sdd-owner: implementation -->
- [ ] `src/components/RegisterExitModal.svelte` — migrate shell;
      motivo select becomes `Select.svelte`; notes textarea remains
      inline (textareas are out of scope for `Input.svelte`). <!-- sdd-owner: implementation -->
- [ ] `src/components/ResolveQuantityDialog.svelte` — migrate shell;
      quantity input becomes `Input.svelte`. <!-- sdd-owner: implementation -->
- [ ] `DashboardPage.svelte` inline product / lot / quick-create
      overlays — migrate all three to `Modal.svelte`. <!-- sdd-owner: implementation -->

### 7.3 PR 7 verify gate

- [ ] `npm run check` green. <!-- sdd-owner: implementation -->
- [ ] `npm run build` green. <!-- sdd-owner: implementation -->
- [ ] `git grep -nE '\.(modal-overlay|modal-box|modal-box-wide|modal-header|modal-body|modal-footer|modal-close|modal-loading)\b' src/components/`
      returns zero matches on the migrated files. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — every migrated modal opens via `showModal()`,
      traps focus inside the dialog, closes via Escape and (when
      allowed) click-outside, restores focus to the trigger. Existing
      "discard in-progress edits on Escape" semantics continue to
      work — manual verify against the lot-movement-ledger change's
      modal acceptance scenarios. <!-- sdd-owner: implementation -->
- [ ] Manual a11y pass — Tab order inside the modal; focus ring
      visible against the backdrop; screen-reader announces the
      modal title; backdrop-blur is applied on
      `prefers-reduced-motion: no-preference` hosts and absent on
      `reduce`. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src/components/MoveStockModal.svelte`,
  `RegisterExitModal.svelte`, `AdjustCountModal.svelte`,
  `ArchiveLotDialog.svelte`, `ResolveQuantityDialog.svelte`.
- `src/components/DashboardPage.svelte` (inline overlays only).

**Forecast:** ~400 net additions combined (split if exceeded).
**Rollback:** revert the modal files; the existing bespoke modals
return.

---

## PR 8 — Forms (split 8a + 8b if apply-time diff > 400)

`docs/daisyui-redesign-plan.md` Phase 6. Six forms / form-bearing
pages. Same apply-time budget discipline as PR 7.

- **PR 8a** — `ProductForm.svelte`, `LotForm.svelte`,
  `ConfigurationPage.svelte` (already partially migrated in PR 5;
  this PR finishes the form surfaces). Forecast ~350 lines.
- **PR 8b** — `ReportsPage.svelte` filters,
  `BackupRestorePage.svelte`, `CsvImportPage.svelte` (one of the
  three-stage screens). Forecast ~320 lines.

Apply-time gate: if `git diff --stat` after PR 8a exceeds 400 lines,
continue the remaining forms as PR 8b.

### 8.1 Per-form migration

- [ ] Replace local `.form-group / .field-label / .small-label /
      .input / .error-msg / .saving-msg / .inline-error /
      .field-error` with `Input.svelte` / `Select.svelte` /
      `Toggle.svelte` / `Alert.svelte`. Pair every input with the
      `label` / `label-text` / `label-text-alt` trio. <!-- sdd-owner: implementation -->
- [ ] Migrate plain `<input type="checkbox">` to the DaisyUI
      `checkbox checkbox-primary checkbox-sm` wrapper; keep the
      native `<input>` in the DOM for form semantics. <!-- sdd-owner: implementation -->
- [ ] Migrate plain `<input type="radio">` to the DaisyUI
      `radio radio-primary radio-sm` wrapper; keep the native
      `<input>` in the DOM. <!-- sdd-owner: implementation -->
- [ ] Migrate `<datalist>` autocomplete inputs (`ProductForm` barcode
      type + unit definitions) to `Input.svelte` with the `list` prop
      pointing at the existing `<datalist id>` — autocomplete semantics
      (keyboard, screen-reader) preserved exactly. <!-- sdd-owner: implementation -->
- [ ] Replace every `class="btn-primary / btn-secondary / btn-ghost /
      btn-outline / btn-danger / btn-link / btn-sm / btn-small /
      action-btn / chip-clear / banner-btn / inline-unit-close /
      link-btn / caret` with `Button.svelte` and the appropriate
      `variant` / `size`. <!-- sdd-owner: implementation -->
- [ ] Standardise the required-marker rendering through
      `Input.svelte`'s `required` prop; helper text through the
      `helper` prop; invalid state through the `invalid` prop. <!-- sdd-owner: implementation -->
- [ ] Add new i18n keys for any new copy introduced by the form
      migration (helper text variants, validation messages, action
      labels). EN + ES in the same PR. <!-- sdd-owner: implementation -->
- [ ] Run `npm run i18n:generate`; commit the regenerated catalogue. <!-- sdd-owner: implementation -->

### 8.2 Per-form targets

- [ ] `src/components/ProductForm.svelte` — full migration; preserve
      `<datalist>` for barcode type + unit definitions. <!-- sdd-owner: implementation -->
- [ ] `src/components/LotForm.svelte` — full migration; preserve
      expiry-date picker contract from the canonical DatePicker
      capability. <!-- sdd-owner: implementation -->
- [ ] `src/components/ConfigurationPage.svelte` — finish form
      migration (PR 5 handled the switcher + locale selector). <!-- sdd-owner: implementation -->
- [ ] `src/components/ReportsPage.svelte` filters — migrate selects,
      inputs, urgency radio group, action buttons. <!-- sdd-owner: implementation -->
- [ ] `src/components/BackupRestorePage.svelte` — migrate section
      actions + restore-confirmation flow. <!-- sdd-owner: implementation -->
- [ ] `src/components/CsvImportPage.svelte` — migrate stage buttons,
      strategy radios, detail editor fields. <!-- sdd-owner: implementation -->

### 8.3 PR 8 verify gate

- [ ] `npm run i18n:generate` green. <!-- sdd-owner: implementation -->
- [ ] `npm run check` green. <!-- sdd-owner: implementation -->
- [ ] `npm run build` green. <!-- sdd-owner: implementation -->
- [ ] `git grep -nE '\.(form-group|field-label|small-label|inline-error|field-error|saving-msg|action-btn|chip-clear|banner-btn|link-btn|caret)\b' src/components/ProductForm.svelte src/components/LotForm.svelte src/components/ReportsPage.svelte src/components/BackupRestorePage.svelte src/components/CsvImportPage.svelte`
      returns zero matches as the active surface class. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — every canonical form scenario from the
      unchanged forms capability continues to pass: `product picks a
      preset unit`, `product creates a custom unit inline`, `lot
      creation with auto-generated batch`, `locale selector update
      rolls back on IPC failure`, `CSV import preview`, `backup
      export + validate + restore`, `reports preview + export`. <!-- sdd-owner: implementation -->
- [ ] Manual reduced-motion pass — form animations (none expected
      for validations / state changes) do not fire. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src/components/ProductForm.svelte`, `LotForm.svelte`,
  `ConfigurationPage.svelte`, `ReportsPage.svelte`,
  `BackupRestorePage.svelte`, `CsvImportPage.svelte`.
- `src/i18n/en/index.ts`, `src/i18n/es/index.ts` (new copy).

**Forecast:** ~700 net additions across 8a + 8b (~350 + ~350; design
§6 estimate). **Rollback:** revert the form files; the i18n-supported
forms return.

---

## PR 9 — Tables and data-dense screens

`docs/daisyui-redesign-plan.md` Phase 7. Six table-bearing surfaces.

- [ ] Migrate the `LotMovementsPanel.svelte` ledger table to
      `Table.svelte` (`zebra`, `stickyHeader`); numeric columns use
      `num`. Empty state via `EmptyState.svelte`; loading state via
      `LoadingState.svelte`. <!-- sdd-owner: implementation -->
- [ ] Migrate the `ReportsPage.svelte` data table (the post-filter
      results table, separate from the form work in PR 8) to
      `Table.svelte`. Preserve sort / filter behaviour. <!-- sdd-owner: implementation -->
- [ ] Migrate the `CsvImportPage.svelte` preview table to `Table.svelte`;
      preserve readability of long rows. <!-- sdd-owner: implementation -->
- [ ] Migrate the `StoresPage.svelte` store list (table-like surface)
      to `Table.svelte`. <!-- sdd-owner: implementation -->
- [ ] Migrate the `BackupRestorePage.svelte` info lists (`.info-list`,
      `.checks-list`, `.confirm-box`) to `Table.svelte` (or
      `Card.svelte` + DaisyUI `list` patterns where appropriate). <!-- sdd-owner: implementation -->
- [ ] Migrate the `CalendarPage.svelte` day-detail panel to a
      `Table.svelte` (it is the list of active lots for the selected
      day; the calendar grid itself stays in PR 10). <!-- sdd-owner: implementation -->
- [ ] Migrate the `ProductCatalogPage.svelte` product list to
      `Table.svelte` (or to `Card.svelte` cards if a future iteration
      prefers cards — pick whichever preserves the existing visual
      hierarchy). <!-- sdd-owner: implementation -->
- [ ] Migrate the `ConfigurationPage.svelte` info-list surface (the
      "Acerca de" / "Configuración" summary block) to
      `Table.svelte` or DaisyUI `list`. <!-- sdd-owner: implementation -->
- [ ] Add new i18n keys for any new empty / loading / table-header
      copy. EN + ES in the same PR. <!-- sdd-owner: implementation -->
- [ ] Run `npm run i18n:generate`; commit the regenerated catalogue. <!-- sdd-owner: implementation -->

### 9.x PR 9 verify gate

- [ ] `npm run i18n:generate` green. <!-- sdd-owner: implementation -->
- [ ] `npm run check` green. <!-- sdd-owner: implementation -->
- [ ] `npm run build` green. <!-- sdd-owner: implementation -->
- [ ] `git grep -nE '\.(lot-table|reports-table|reports-empty|lot-picker|lot-picker-item|lot-picker-status|info-list|checks-list|confirm-box)\b' src/components/`
      returns zero matches as the active table wrapper class on the
      migrated files. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — every migrated table preserves its canonical
      contract: no column lost, sort / filter behaviour unchanged,
      CSV preview readable, calendar day-detail panel still renders
      one row per active lot with the same columns, store list still
      CRUD-able, backup info-lists still readable. <!-- sdd-owner: implementation -->
- [ ] Manual reduced-motion pass — skeleton shimmer falls back to a
      static grey block on `reduce`. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src/components/LotMovementsPanel.svelte`,
  `ReportsPage.svelte` (data table portion),
  `CsvImportPage.svelte` (preview portion),
  `ProductCatalogPage.svelte`,
  `StoresPage.svelte`, `BackupRestorePage.svelte`,
  `CalendarPage.svelte` (day-detail panel),
  `ConfigurationPage.svelte` (info-list portion).

**Forecast:** ~360 net additions (design §6 estimate). **Rollback:**
revert the table files; the i18n-supported tables return.

---

## PR 10 — Calendar + custom widget polish

`docs/daisyui-redesign-plan.md` Phase 8. Restyle-only; behaviour
preserved exactly per the spec's
"DatePicker and CategoryPicker keyboard contracts are preserved"
requirement.

- [ ] Restyle `src/components/CalendarMonth.svelte`: day cells,
      selected state, today ring, badge dots. Use the `num` utility
      for the right-aligned badge dots (current `CalendarMonth` is
      already end-aligned — confirm). Preserve year picker, decade
      navigation, badge dot count, and out-of-range guards
      unchanged. <!-- sdd-owner: implementation -->
- [ ] Restyle `src/components/CalendarPage.svelte`: page header,
      prev/next chevron buttons (now `Button.svelte variant="ghost"
      size="icon"` with `Tooltip.svelte` labels), month / year labels,
      and the day-detail panel column headers. Preserve the
      keyboard surface verbatim. <!-- sdd-owner: implementation -->
- [ ] Restyle `src/components/DatePicker.svelte`: the popover root
      acquires `class="dropdown dropdown-content ..."` (along with the
      existing positioning classes). Hand-rolled outside-click /
      scroll / resize listeners, fixed positioning, ISO bind
      contract, year-range enforcement, validation rules, and
      `clearable` semantics stay unchanged. <!-- sdd-owner: implementation -->
- [ ] Restyle `src/components/inputs/CategoryPicker.svelte`: the
      popover root acquires `class="dropdown dropdown-content ..."`
      alongside the existing positioning. Preserves the chip /
      pseudo-row semantics, arrow-key / Enter / Escape / Backspace
      keyboard contract, and `aria-activedescendant` semantics. <!-- sdd-owner: implementation -->
- [ ] Migrate the `<details>` / `<summary>` panels in
      `UnitReviewPage.svelte` ("Map to preset", "Keep as custom") to
      DaisyUI `collapse collapse-arrow` with proper ARIA roles. <!-- sdd-owner: implementation -->
- [ ] Add new i18n keys for any new tooltip or helper copy on these
      surfaces. EN + ES in the same PR. <!-- sdd-owner: implementation -->
- [ ] Run `npm run i18n:generate`; commit the regenerated catalogue. <!-- sdd-owner: implementation -->

### 10.x PR 10 verify gate

- [ ] `npm run i18n:generate` green. <!-- sdd-owner: implementation -->
- [ ] `npm run check` green. <!-- sdd-owner: implementation -->
- [ ] `npm run build` green. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — every canonical DatePicker scenario from the
      unchanged DatePicker capability continues to pass without
      modification (year range, ISO bind, manual text input
      validation, `clearable`, Tab / Escape / Enter / arrow keys /
      PageUp / PageDown / Shift+PageUp / Shift+PageDown / Today
      shortcut). <!-- sdd-owner: implementation -->
- [ ] Manual smoke — every canonical CategoryPicker keyboard scenario
      from the canonical Categories capability continues to pass
      (`aria-activedescendant` moves through the result list and
      clamps at the ends, `Enter` toggles, `Escape` closes without
      committing typed text, `Backspace` removes the last chip, the
      `Uncategorized` pseudo-row remains selectable and visually
      distinct). <!-- sdd-owner: implementation -->
- [ ] Manual smoke — every canonical Calendar tab scenario from the
      unchanged Calendar capability continues to pass (current month
      with today highlighted + selected, per-day expirations as dot
      badges, day-detail panel columns, lot row opens the existing
      edit flow, no-expirations placeholder, keyboard surface
      inherited from the calendar primitive, no refetch on month
      navigation). <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src/components/CalendarMonth.svelte`, `CalendarPage.svelte`,
  `DatePicker.svelte`, `inputs/CategoryPicker.svelte`,
  `UnitReviewPage.svelte`.

**Forecast:** ~350 net additions (design §6 estimate). **Rollback:**
revert the five files; behaviour preservation guarantees a clean
revert.

---

## PR 11 — Responsive pass

`docs/daisyui-redesign-plan.md` Phase 10. Touches every migrated
file at the breakpoint layer.

- [ ] Verify navbar collapse at ≤ 720 px (already wired in PR 5;
      confirm after the visual migrations in PRs 6 – 10). <!-- sdd-owner: implementation -->
- [ ] Add `overflow-x-auto scrollbar-thin scrollbar-thumb-base-300`
      wrappers around wide tables (`DashboardPage` lot table,
      `ReportsPage` data table, `LotMovementsPanel` ledger,
      `CsvImportPage` preview). <!-- sdd-owner: implementation -->
- [ ] Verify modals fit within the viewport at 720 px and 480 px;
      use `modal-bottom` (DaisyUI variant) for full-width modal
      surfaces on narrow viewports where it improves ergonomics. <!-- sdd-owner: implementation -->
- [ ] Verify the calendar and forms on 720 px and 480 px; ensure
      primary actions remain reachable and no horizontal page
      overflow appears (intentional table wrappers excepted). <!-- sdd-owner: implementation -->
- [ ] Stack the dashboard urgency cards vertically at < 720 px. <!-- sdd-owner: implementation -->

### 11.x PR 11 verify gate

- [ ] `npm run check` green. <!-- sdd-owner: implementation -->
- [ ] `npm run build` green. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — resize the viewport to 1024, 720, and 480 px in
      turn across every migrated main surface; no horizontal page
      overflow appears; every primary action remains reachable;
      every modal fits within the viewport; the navbar remains
      usable. <!-- sdd-owner: implementation -->
- [ ] Manual screenshot pass at the three breakpoints in
      `caduxo-light` and `dark`; recorded in the verify report. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- Touches every migrated file at the breakpoint layer; primarily
  `DashboardPage.svelte`, `LotMovementsPanel.svelte`,
  `ReportsPage.svelte`, `CsvImportPage.svelte`,
  `App.svelte` (responsive navbar confirmation).

**Forecast:** ~250 net additions (design §6 estimate). **Rollback:**
revert the responsive wrapper additions; the migrated surfaces keep
their default desktop layout.

---

## PR 12 — Motion + effects inventory wiring

`docs/daisyui-redesign-plan.md` Phase 11. Centralises motion tokens
in the migrated components; ensures every animated surface sources
its duration / easing from `src/app.css` rather than from a hand-
written literal.

- [ ] Add the `urgency-pulse` keyframes to `src/app.css` (per design
      §5.5): `0%, 100% { opacity: 1; } 50% { opacity: 0.55; }`, with
      `--duration-pulse` and `--ease-in-out-soft`. Wire the
      `motion-safe:animate-urgency-pulse` Tailwind v4 utility via
      `@theme` if not already exposed by PR 1. <!-- sdd-owner: implementation -->
- [ ] Audit `Badge.svelte`, `Card.svelte`, `Button.svelte`,
      `Modal.svelte`, `Table.svelte`, `Tabs.svelte` to confirm every
      transition / animation uses the motion tokens
      (`duration-fast`, `duration-base`, `duration-slow`,
      `ease-out-soft`, `ease-in-out-soft`) and a
      `motion-reduce:` variant. <!-- sdd-owner: implementation -->
- [ ] Confirm the global reduced-motion reset in `src/app.css` is
      sufficient for primitives that opt into DaisyUI's built-in
      animations (modal scale, dropdown slide-in, skeleton shimmer).
      Add scoped overrides only if DaisyUI's defaults escape the
      reset. <!-- sdd-owner: implementation -->
- [ ] Apply the dashboard gradient band on the Dashboard landing tab
      only (already wired in PR 5; confirm the band does not animate
      and does not need a reduced-motion gate). <!-- sdd-owner: implementation -->
- [ ] Add `git grep -nE 'transition:.*0\.[0-9]+s|animation:.*0\.[0-9]+s'
      src/` check to the verify gate script — the PR that introduces
      a hand-written animation literal is rejected at review. <!-- sdd-owner: implementation -->

### 12.x PR 12 verify gate

- [ ] `npm run check` green. <!-- sdd-owner: implementation -->
- [ ] `npm run build` green. <!-- sdd-owner: implementation -->
- [ ] `git grep -nE 'transition:.*0\.[0-9]+s|animation:.*0\.[0-9]+s' src/components/`
      returns zero matches as the source of an animation on migrated
      files (motion tokens are imported from `app.css`). <!-- sdd-owner: implementation -->
- [ ] Manual reduced-motion pass — toggle
      `prefers-reduced-motion: reduce` via DevTools; confirm no
      animated surface fires; reduced-motion users reach the same
      end state as everyone else. <!-- sdd-owner: implementation -->
- [ ] Manual pulse-isolation pass — the urgency-pulse runs only on
      the expired variant; it is opacity-only (no scale or
      translation); it is not hover-triggered; it is absent on
      `reduce`. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src/app.css` (urgency-pulse keyframes + utility).
- `src/components/ui/Badge.svelte`, `Card.svelte`, `Button.svelte`,
  `Modal.svelte`, `Table.svelte`, `Tabs.svelte` (audit + scoped
  overrides if needed).

**Forecast:** ~150 net additions (design §6 estimate). **Rollback:**
revert the keyframes + audit; the global reduced-motion reset in
PR 1 keeps every animated surface safe.

---

## PR 13 — Final cleanup + docs

`docs/daisyui-redesign-plan.md` Phase 12. Removes visual debt and
documents the design system for future work.

- [ ] Delete `src/style.css` once
      `git grep -nE '\.(shell|hero|eyebrow|cards)\b' src/` returns
      zero matches (per the spec's
      "legacy src/style.css retired after migration" requirement).
      <!-- sdd-owner: implementation -->
- [ ] Sweep one-off component-scoped CSS across every migrated
      component: remove dead selectors introduced by the migration;
      consolidate duplicate `.urgency-card-*`, `.btn-*`, `.table-*`
      selectors that survived the migration. <!-- sdd-owner: implementation -->
- [ ] Update `docs/daisyui-redesign-plan.md` to a "post-v1 status"
      note pointing at the archived OpenSpec change folder (after
      archive in PR 14). <!-- sdd-owner: implementation -->
- [ ] Add a `docs/design-system.md` contributor guide documenting
      the primitive inventory, the theme block layout, the motion
      token inventory, the i18n discipline, and the rules for adding
      a new surface (must compose from existing primitives; must
      add EN + ES strings in the same PR; must respect
      `prefers-reduced-motion`). <!-- sdd-owner: implementation -->
- [ ] Verify CSS bundle size: record `npm run build` final CSS size
      before PR 13 and after; gate merge if the size regresses
      >20%. <!-- sdd-owner: implementation -->

### 13.x PR 13 verify gate

- [ ] `npm run check` green. <!-- sdd-owner: implementation -->
- [ ] `npm run build` green. <!-- sdd-owner: implementation -->
- [ ] `git grep -nE '\.(shell|hero|eyebrow|cards)\b' src/` returns
      zero matches. <!-- sdd-owner: implementation -->
- [ ] `git grep -nE '#[0-9a-fA-F]{3,8}\b|rgba?\(' src/components/` returns zero matches on migrated files (every colour is theme-derived). <!-- sdd-owner: implementation -->
- [ ] CSS bundle size delta within ±20% of PR 1 baseline. <!-- sdd-owner: implementation -->
- [ ] Manual screenshot pass — every main surface in both
      `caduxo-light` and `dark`; no OS-styled control surfaces
      inside the app chrome; review the screenshots against the
      `docs/redesign-baseline.md` notes from PR 1. <!-- sdd-owner: implementation -->

**Files / discovery targets**

- `src/style.css` (delete).
- Every migrated component file (one-off CSS sweep).
- `docs/daisyui-redesign-plan.md` (status note).
- `docs/design-system.md` (new contributor guide).

**Forecast:** ~250 net additions (design §6 estimate). **Rollback:**
restore `src/style.css` from VCS; revert the sweep; revert the docs.

---

## PR 14 — Verify + archive (parent-owned)

Parent-only lifecycle gate per the spec's
"Chained PR delivery" capability. Bound to the verify report and
the archive procedure established by prior changes.

- [ ] Compile the verify report aggregating per-PR evidence
      (CI gate output, manual smoke notes, manual a11y notes,
      manual reduced-motion notes, manual screenshot pass, manual
      contrast pass) for every chained PR 1 → PR 13. <!-- sdd-owner: parent -->
- [ ] Confirm zero unchecked implementation tasks remain in
      `openspec/changes/caduxo-daisyui-redesign/tasks.md`
      (`grep -nE '^\s*- \[ \]' …/tasks.md` returns only the
      parent-owned boxes below). <!-- sdd-owner: parent -->
- [ ] Re-read `tasks.md` immediately before archive (per the
      project's "Final Task Completion Gate" pattern) and reconcile
      any drift between source-of-truth files and the task list. <!-- sdd-owner: parent -->
- [ ] Author and land the
      `openspec/changes/caduxo-daisyui-redesign/verify-report.md`
      with verdict `pass` (or `fail` with explicit blockers) and the
      per-PR evidence rollup. <!-- sdd-owner: parent -->
- [ ] Archive the change via the project's OpenSpec archive
      procedure: move the change folder under
      `openspec/changes/archive/<date>-caduxo-daisyui-redesign/`,
      preserving every artifact (`proposal.md`, `explore.md`,
      `specs/…/spec.md`, `design.md`, `tasks.md`,
      `apply-progress.md`, `verify-report.md`). <!-- sdd-owner: parent -->
- [ ] File the bounded review receipt covering: (a) spec parity
      between the `en` and `es` i18n catalogues for new visual
      copy, (b) theme persistence round-trip in both themes, (c)
      theme switcher source label accuracy across the four source
      values (`manual`, `persisted`, `os`, `fallback`), (d) zero
      OS-styled controls in the post-PR-13 screenshot pass, (e)
      zero animation in the `prefers-reduced-motion: reduce`
      pass, (f) DatePicker / CategoryPicker keyboard contract
      preservation, (g) WCAG-AA contrast in both themes on every
      main surface. <!-- sdd-owner: parent -->
- [ ] Record follow-up OpenSpec changes for the deferred items
      captured in the proposal assumptions: shared `Popover.svelte`
      primitive extraction, `ToastHost.svelte` primitive, dark-mode
      per-screen polish, accent-theme follow-up. <!-- sdd-owner: parent -->

---

## Verify gate shared across PRs 1 → 13

Every chained PR 1 → PR 13 runs, at minimum:

```bash
npm run i18n:generate   # if i18n catalogue changes
npm run check           # svelte-check --threshold error
npm run build           # vite build
```

PRs that touch the backend also run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo build --manifest-path src-tauri/Cargo.toml
```

Visual PRs (PR 3 onward) also include:

- A manual Tauri / dev-browser launch in both `caduxo-light` and
  `dark` themes.
- A keyboard pass on every migrated primitive (Tab order, Escape,
  Enter / Space activation, focus restoration).
- A reduced-motion pass (`prefers-reduced-motion: reduce` via
  DevTools or OS settings) confirming every animated surface
  reaches the same end state.
- A screenshot pass on every touched surface, recorded in the
  verify report.
- (Theme PRs) A WCAG-AA contrast pass on body / heading / input /
  badge / alert / primary button label.

---

## Forecast summary

| PR  | Forecast (lines) | 400-line risk | Split if exceeded? |
|-----|------------------|----------------|---------------------|
| 1   | ~200             | Low            | No                  |
| 2   | ~120             | Low            | No                  |
| 3   | ~280             | Low            | No                  |
| 4   | ~260             | Low            | No                  |
| 5   | ~300             | Low            | No                  |
| 6   | ~350             | Low            | No                  |
| 7   | ~400             | Medium         | **Yes** → 7a + 7b  |
| 8   | ~700 (8a + 8b)   | Medium         | **Yes** → 8a + 8b  |
| 9   | ~360             | Low            | No                  |
| 10  | ~350             | Low            | No                  |
| 11  | ~250             | Low            | No                  |
| 12  | ~150             | Low            | No                  |
| 13  | ~250             | Low            | No                  |
| **Total** | **~3 970** | — | PRs 7 + 8 already split |

Per-PR discipline keeps every reviewable slice within the budget.
The `size:exception` lever is reserved for the parent only; the
default path is "split further when the apply-time diff exceeds 400
lines", never "ship a single oversized PR".

---

## Parent actions (lifecycle gates and review)

- [ ] Before PR 1 apply: ratify the chain strategy
      (`stacked-to-main` for PR 1 → PR 2 then `feature-branch-chain`
      for the remaining chained PRs is the design's recommended
      shape). Update this `tasks.md` if the parent prefers a
      different chain strategy. <!-- sdd-owner: parent -->
- [ ] Before PR 1 apply: confirm the project check script
      (`"check": "svelte-check --tsconfig ./tsconfig.json --threshold error"`)
      lands in `package.json` as part of PR 1, and that every verify
      gate in PRs 1 → 13 uses it. <!-- sdd-owner: parent -->
- [ ] Open PR 1 against `main`. Merge PR 1 once
      `npm run i18n:generate`, `npm run check`, `npm run build`,
      and the manual launch + preflight-regression check pass. <!-- sdd-owner: parent -->
- [ ] After PR 1 merges: open PR 2 against `main` (or stack onto PR 1
      depending on the ratified chain strategy). PR 2 is backend-only
      and should be small enough to review in one pass. <!-- sdd-owner: parent -->
- [ ] For every chained PR 3 → PR 13: enforce the apply-time diff
      budget (`git diff --stat` must stay below 400 lines). If a
      PR exceeds, abort mid-stream and split per the design's
      per-phase split rule (page-level, not feature-level). <!-- sdd-owner: parent -->
- [ ] Start or reuse a bounded review for the change covering the
      list in PR 14 above. <!-- sdd-owner: parent -->
- [ ] After all chained PRs land and review passes: execute PR 14
      (verify + archive) per the project's OpenSpec lifecycle. <!-- sdd-owner: parent -->
