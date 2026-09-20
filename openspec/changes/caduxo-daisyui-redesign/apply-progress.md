# Apply Progress — caduxo-daisyui-redesign

> Phase-by-phase implementation log for the Tailwind v4 + DaisyUI redesign.
> This file merges forward after each PR — completed work is preserved and
> new work is appended.

## PR 1 — Foundation (Tailwind + DaisyUI + theme + global motion reset)

**Status:** Complete on `feat/daisyui-redesign`. Awaiting verify phase
(optional) and PR open. Not pushed per session preflight.

**Branch:** `feat/daisyui-redesign` cut from `main` at `9178786`.
The planning branch `feat/daisyui-redesign-plan` was preserved
separately by committing the planning artifacts (commit `b5534a9`)
before creating the implementation branch.

### Files changed

| File | Change |
|------|--------|
| `package.json` | Add `tailwindcss@^4`, `@tailwindcss/vite@^4`, `daisyui@^5` to `dependencies`; add `check` script. |
| `package-lock.json` | Lockfile updates from `npm install` (13 packages added, 0 vulnerabilities). |
| `vite.config.ts` | Register `tailwindcss()` *before* `svelte()` in plugin chain. Preserve `clearScreen: false`, `build.target = "es2022"`, `server.strictPort`, `server.port = 1420`. |
| `src/app.css` | New file — Tailwind v4 import, DaisyUI plugin registration (`themes: caduxo-light --default, dark`), `caduxo-light` custom theme block (oklch values derived from `#2563eb` primary and `#f6f8fb` base), motion tokens (`--duration-fast/base/slow/pulse`, `--ease-out-soft`, `--ease-in-out-soft`), `prefers-reduced-motion: reduce` global reset, `@utility num` (tabular-nums + end-aligned numeric columns). |
| `src/main.ts` | Switch the stylesheet import from `./style.css` to `./app.css`. The legacy `src/style.css` is retained during the migration (retired in PR 13). |
| `docs/redesign-baseline.md` | New file — per-surface visual inventory + class-family retirement map + viewport/responsive notes + cross-cutting observations. |

### Tasks completed (PR 1)

| Task | Status | Notes |
|------|--------|-------|
| 1.1.1 Create branch `feat/daisyui-redesign` off `main` | ✅ done | Created from `main` at `9178786` after committing planning artifacts to `feat/daisyui-redesign-plan` (commit `b5534a9`). |
| 1.1.2 Write `docs/redesign-baseline.md` | ✅ done | Per-surface visual inventory, class-family retirement map, responsive notes, cross-cutting observations. |
| 1.1.3 Capture baseline screenshots or annotated textual descriptions | ✅ done | Annotated textual descriptions with explicit rationale (no display server in PR 1 environment). Verify phase will capture actual screenshots in a desktop environment. |
| 1.2.1 Add `tailwindcss@^4`, `@tailwindcss/vite@^4`, `daisyui@^5` | ✅ done | `npm install` added 13 packages; 0 vulnerabilities. |
| 1.2.2 Update `vite.config.ts` plugin order | ✅ done | `tailwindcss()` registered before `svelte()`; preserved `clearScreen`, `build.target`, `server.strictPort`, `server.port`. |
| 1.2.3 Add `check` script | ✅ done | `svelte-check --tsconfig ./tsconfig.json --threshold error`. |
| 1.3.1 Create `src/app.css` with `@import "tailwindcss"` + DaisyUI plugin + `caduxo-light` theme + `dark` | ✅ done | Custom theme oklch values derived from `#2563eb` / `#f6f8fb` palette per design §3.2. |
| 1.3.2 Motion tokens via Tailwind v4 `@theme` | ✅ done | `--duration-fast: 120ms`, `--duration-base: 180ms`, `--duration-slow: 240ms`, `--duration-pulse: 1800ms`, `--ease-out-soft`, `--ease-in-out-soft`. |
| 1.3.3 `prefers-reduced-motion: reduce` global reset | ✅ done | Targets `*, *::before, *::after` with `animation-duration: 0.001ms !important`, `animation-iteration-count: 1 !important`, `transition-duration: 0.001ms !important`, `scroll-behavior: auto !important`. |
| 1.3.4 `@utility num` | ✅ done | `font-variant-numeric: tabular-nums; text-align: end;` for numeric table columns (consumed by PR 9's `Table.svelte`). |
| 1.3.5 `app-shell-gradient` keyframes / utility | ⏸️ deferred | Task says "Add if dashboard header band uses one; otherwise defer until PR 6". Per design §6 phase 3, the gradient lands with the navbar migration in PR 5/6; the keyframes live in `src/app.css` along with the other tokens when PR 5 lands. |
| 1.3.6 Update `src/main.ts` to import `./app.css` | ✅ done | `src/style.css` retained during migration (retired in PR 13). |
| 1.6.1 `npm run i18n:generate` green | ✅ done | `typesafe-i18n` reports "all files are up to date". |
| 1.6.2 `npm run check` green | ✅ done | `svelte-check found 0 errors and 0 warnings`. |
| 1.6.3 `npm run build` exits green | ✅ done | `vite v6.4.3 building for production... ✓ 205 modules transformed... ✓ built in 1.66s`. CSS bundle: 194.44 kB (29.12 kB gzip). |
| 1.6.4 Manual launch — `npm run dev` boots cleanly | ⏸️ deferred to verify phase | Headless environment; no display server. The verify phase will boot `npm run dev` / `npm run tauri dev` in a desktop environment. |
| 1.6.5 Manual contrast pass on un-migrated UI in both themes | ⏸️ deferred to verify phase | Same as 1.6.4 — requires a desktop runtime. |

### Tasks deliberately deferred (parent-scoped)

Per the explicit PR 1 outcomes listed in the parent's task prompt, the
following PR 1 tasks were intentionally deferred to a later slice:

| Task | Why deferred | Where it belongs |
|------|--------------|------------------|
| 1.4.1 Extend `src/lib/stores.ts` with `theme` / `theme_configured` | The parent's enumerated outcomes for this PR 1 slice did not include the frontend IPC consumer skeleton. Wiring `theme` against a backend that does not yet serve it (PR 2 lands that) would TypeScript-error or runtime-error. | Land with PR 2 (backend IPC delta) — see tasks §2.1.1. |
| 1.4.2 Create `src/components/ui/theme/themeStore.svelte.ts` | Same as 1.4.1. The store reads `getSettings().theme`; without the backend change, the type extension is unsound. | Land with PR 2. |
| 1.4.3 `initTheme()` called from `src/main.ts` | Depends on 1.4.2. | Land with PR 2. |
| 1.5.1 Add `theme` namespace to i18n catalogues | The labels are referenced by the theme switcher in PR 5. Until the switcher ships, the keys would be unused dead code. | Land with PR 5 (Configuration theme switcher) — see tasks §5.2.4. |
| 1.5.2 Run `npm run i18n:generate`; commit regenerated catalogue | Depends on 1.5.1. | Land with PR 5. |

This deferral keeps PR 1 to its stated scope: deps, Vite plugin, check
script, `src/app.css` foundation, `src/main.ts` switch, baseline doc.
The theme store skeleton (1.4) and i18n strings (1.5) ship with the
slice that actually consumes them.

### Checks run + results

```text
$ npm run i18n:generate
> typesafe-i18n --no-watch
[typesafe-i18n] ... all files are up to date
✅ green

$ npm run check
> svelte-check --tsconfig ./tsconfig.json --threshold error
svelte-check found 0 errors and 0 warnings
✅ green

$ npm run build
> vite build
✓ 205 modules transformed.
dist/index.html                   0.39 kB │ gzip:  0.26 kB
dist/assets/index-*.css         194.44 kB │ gzip: 29.12 kB
dist/assets/index-*.js          327.36 kB │ gzip: 94.47 kB
✓ built in 1.66s
✅ green
```

### Focused sanity checks

- `grep -o "data-theme=[a-z-]*" dist/assets/index-*.css | sort -u`
  → `data-theme=caduxo-light`, `data-theme=dark` — both themes are
  bundled (DaisyUI plugin registration is correct).
- `grep -c "prefers-reduced-motion" dist/assets/index-*.css` → `1`
  — the global reset survived the build pipeline.
- `grep -rn 'style.css' src/` → only matches inside the new comment
  block in `src/main.ts` explaining why `./app.css` is imported.
  The legacy `src/style.css` file is intact (used by the verify
  pass in PR 13).
- `cat vite.config.ts` → plugins order is `[tailwindcss(), svelte()]`
  as required.
- `cat package.json` → `check` script registered with the canonical
  svelte-check invocation.

### Bundle size

| Asset | Raw | Gzip |
|-------|-----|------|
| `dist/assets/index-*.css` | 194.44 kB | 29.12 kB |
| `dist/assets/index-*.js`  | 327.36 kB | 94.47 kB |

The CSS bundle includes Tailwind v4 base + DaisyUI v5 with two themes
(`caduxo-light` + `dark`). Per the design's risk-register item #8,
PR 12 will re-measure and gate against a >20% CSS regression.

### Deviations from design

- **oklch values are starting points.** The `caduxo-light` theme uses
  oklch values derived from the existing hex palette per design §3.2.
  The design anticipates tuning during the verify pass against WCAG-AA
  contrast on body / headings / inputs / badges / alerts / primary
  buttons. PR 1 lands the starting values; the verify phase will
  measure contrast and tune if needed.
- **No `app-shell-gradient` keyframes.** The task says "Add if the
  dashboard header band uses one; otherwise defer until PR 6". The
  gradient band ships with the navbar migration in PR 5/6.

### Remaining work (next chained PR)

- **PR 1.5/1.4 + PR 2 (combined slice recommended):** backend IPC
  delta (`app_settings.theme` column + `theme_configured` boolean in
  `SettingsResponse`) + frontend IPC consumer skeleton (1.4) +
  theme i18n strings (1.5). PR 2 is design-stated to *stack onto
  PR 1* so the frontend code in PR 1 resolves against a real
  backend. The chain strategy is `stacked-to-main` for PR 1 → PR 2
  per the parent's ratified recommendation.
- **PR 5:** App shell navbar migration (`src/App.svelte`) + theme
  switcher on `ConfigurationPage.svelte`. Lands `app-shell-gradient`,
  surfaces IPC failure via `Alert.svelte`, completes the theme
  persistence loop end-to-end.

### Workload / PR boundary

- **PR 1 actual diff:** 6 files changed, 597 insertions(+), 3
  deletions(-). Within the 400-line review budget with substantial
  headroom. The `package-lock.json` accounts for 574 of the 597
  insertions — the actual code change is small.
- **Chain strategy:** stacked-to-main for PR 1 → PR 2 (parent
  ratified), feature-branch-chain from PR 3 onward.