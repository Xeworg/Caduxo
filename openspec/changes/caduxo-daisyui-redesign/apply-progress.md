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
## PR 2 — Backend IPC delta for theme persistence

**Status:** Complete on `feat/daisyui-redesign`. Stacked onto PR 1 so
the frontend contract wired in PR 1 resolves against a real backend
that actually serves `theme` and `theme_configured`. Not pushed per
session preflight.

**Branch:** `feat/daisyui-redesign` (continuation of PR 1, no new
branch cut). Per the parent's chain strategy, PR 2 stacks onto PR 1
on the same branch — no merge commit between the two.

### Files changed

| File | Change |
|------|--------|
| `src-tauri/src/dto/stores.rs` | Add `theme: String` (default `"caduxo-light"`) and `theme_configured: bool` (default `false`) to `SettingsResponse`; add `theme: Option<String>` to `SettingsUpdate`. |
| `src-tauri/src/db/repositories/settings.rs` | Add `get_theme_setting` / `set_theme_setting` helpers (mirror the language pattern); extend `get_settings` to populate `theme` with the `"caduxo-light"` fallback when the row is absent / empty / unsupported and `theme_configured = false` in those cases. Add six new `#[tokio::test]` cases covering missing row, empty string, unsupported value (`"synthwave"`), persisted valid values, and language/theme independent configuration. |
| `src-tauri/src/services/settings.rs` | Branch on `input.theme` in `update_settings` so a partial theme-only update does not touch the other keys. Add `update_settings_partial_theme_update_preserves_other_keys` test. Update two existing test invocations to include the new `theme: None` field. |
| `src-tauri/src/services/user_messages.rs` | Add `UserMessage::ThemeNotAllowed { value }` mirroring `LanguageNotAllowed`. Add EN/ES message arms, parser helper `parse_theme_not_allowed`, and dispatch integration in `parse_user_message_kind`. |
| `src-tauri/src/commands/stores.rs` | Extract `validate_theme_value(value, loc)` helper and call it from `update_settings` to reject values outside `{"caduxo-light", "dark"}` with `CommandError::Validation` carrying the locale-aware rejection. Add eight `#[test]` cases (caduxo-light × {EN, ES}, dark × {EN, ES}, synthwave × {EN, ES}, empty string, uppercase `Dark`). Update two existing test invocations to include the new `theme: None` field. |
| `src-tauri/src/db/migrations.rs` | Add V18 (`app_settings_theme_key_milestone`) idempotent no-op marker migration that documents the theme-support milestone. Update three pre-existing assertions from `17` to `18` (V1–V18 total). Add `v18_theme_milestone_is_idempotent` and `v18_app_settings_accepts_theme_key_value` tests; add V18 confirmation step to `v15_previously_applied_database_accepts_v16`. |
| `src-tauri/src/services/backup_restore.rs` | Update two `expected 17 migrations` assertions to `18` (V1–V18 total) so the pre-V4 / pre-V5 restore tests track the new migration count. |
| `src-tauri/src/services/stores.rs` | Update `first_run_setup_end_to_end` test invocation to include the new `theme: None` field. |
| `src-tauri/src/services/expiry_lots.rs` | Update two test invocations to include the new `theme: None` field. |
| `src/lib/stores.ts` | Extend `SettingsResponse` with `theme: ThemeName` and `theme_configured: boolean`; extend `SettingsUpdate` with `theme?: ThemeName`; export a new `ThemeName = "caduxo-light" \| "dark"` literal union mirroring the backend whitelist. **Plumbing only** — no UI consumption; PR 5 (Configuration theme switcher) is the consumer. |

### Tasks completed (PR 2)

| Task | Status | Notes |
|------|--------|-------|
| 2.1.1 Add `theme` / `theme_configured` to `SettingsResponse` + `theme: Option<String>` to `SettingsUpdate` | ✅ done | Default `"caduxo-light"` / `false` enforced in the repository layer. |
| 2.1.2 `get_settings` carries theme with `"caduxo-light"` fallback + `theme_configured` mirrors language pattern | ✅ done | Mirrors the language pattern verbatim: absent row / empty string / unsupported value all surface as `theme_configured: false`. |
| 2.1.3 Branch on `input.theme` in `update_settings` | ✅ done | `update_settings_partial_theme_update_preserves_other_keys` test seeds every other key, sends theme-only update, verifies language + store id + require_initial_location_on_lot_create are all preserved. |
| 2.1.4 Reject theme values outside `{"caduxo-light", "dark"}` at the IPC boundary | ✅ done | `validate_theme_value` returns `CommandError::Validation` with the locale-aware message; eight test cases cover the accept / reject matrix in EN + ES. |
| 2.1.5 Extend migration runner with idempotent V18 | ✅ done | V18 is a pure `SELECT 1;` marker so re-runs leave every row count unchanged; `_sqlx_migrations` records the V18 row from the first run, after which sqlx refuses to re-apply it. |
| 2.2.1 Add `#[tokio::test]` cases for theme validation / default / persistence | ✅ done | Six new tests in `db/repositories/settings.rs` plus the partial-update service test plus eight command-validation tests. |
| 2.2.2 Idempotency test for the new V-N migration | ✅ done | `v18_theme_milestone_is_idempotent` asserts the row count in `app_settings` is unchanged after re-running V1–V18. `v18_app_settings_accepts_theme_key_value` pins the contract that the generic key-value schema already accepts a `theme` row. |
| 2.3.1 `cargo test --lib` green | ✅ done | `test result: ok. 675 passed; 0 failed`. |
| 2.3.2 `cargo build` green | ✅ done | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 18.69s`. |
| 2.3.3 Manual smoke — `app_settings.theme` round-trip | ⏸️ deferred to verify phase | The headless environment has no display server; the round-trip is exercised by the test suite (`get_settings_theme_configured_when_persisted`, `update_settings_partial_theme_update_preserves_other_keys`). The verify phase will boot Tauri in a desktop environment and exercise the IPC end-to-end. |

### Checks run + results

```text
$ cargo test --manifest-path src-tauri/Cargo.toml --lib
test result: ok. 675 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.11s
✅ green

$ cargo build --manifest-path src-tauri/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.69s
✅ green

$ npm run check
> svelte-check --tsconfig ./tsconfig.json --threshold error
svelte-check found 0 errors and 0 warnings
✅ green

$ npm run build
> vite build
✓ 205 modules transformed.
dist/index.html                   0.39 kB │ gzip:  0.27 kB
dist/assets/index-DSZ3GZ2x.css  225.65 kB │ gzip: 33.13 kB
dist/assets/index-C4_4UfT0.js   327.36 kB │ gzip: 94.47 kB
✓ built in 1.84s
✅ green
```

### New tests added (PR 2)

| Test | Layer | Verifies |
|------|-------|----------|
| `db::repositories::settings::tests::get_theme_setting_missing_row_returns_none` | repository | `get_theme_setting` returns `None` on a fresh pool. |
| `db::repositories::settings::tests::get_settings_theme_defaults_on_fresh_install` | repository | `theme = "caduxo-light"`, `theme_configured = false` on a fresh pool. |
| `db::repositories::settings::tests::get_settings_theme_unconfigured_for_empty_string` | repository | Empty stored value falls back to `"caduxo-light"` with `theme_configured = false`. |
| `db::repositories::settings::tests::get_settings_theme_unconfigured_for_unsupported_value` | repository | Stored `"synthwave"` falls back to `"caduxo-light"` with `theme_configured = false`. |
| `db::repositories::settings::tests::get_settings_theme_configured_when_persisted` | repository | Persisted `"dark"` / `"caduxo-light"` both surface as `theme_configured = true`. |
| `db::repositories::settings::tests::get_settings_language_and_theme_configure_independently` | repository | Persisting only the theme does not flip `language_configured` and vice-versa. |
| `services::settings::tests::update_settings_partial_theme_update_preserves_other_keys` | service | Theme-only update preserves `last_selected_store_id`, `require_initial_location_on_lot_create`, and `language`. |
| `commands::stores::tests::validate_theme_accepts_caduxo_light_in_{english,spanish}` | command | Caduxo-light accepted in both locales. |
| `commands::stores::tests::validate_theme_accepts_dark_in_{english,spanish}` | command | Dark accepted in both locales. |
| `commands::stores::tests::validate_theme_rejects_synthwave_in_{english,spanish}` | command | Synthwave rejected in both locales with locale-aware message. |
| `commands::stores::tests::validate_theme_rejects_empty_string` | command | Empty string rejected. |
| `commands::stores::tests::validate_theme_rejects_uppercase_dark` | command | Case-sensitive curation rejects `"Dark"` so a typo never reaches persistence. |
| `db::migrations::tests::v18_theme_milestone_is_idempotent` | migration | Re-running V1–V18 on a migrated pool leaves `app_settings` row count and `applied_count` unchanged. |
| `db::migrations::tests::v18_app_settings_accepts_theme_key_value` | migration | The generic key-value schema accepts a `theme` row directly. |
| `db::migrations::tests::v15_previously_applied_database_accepts_v16` (extended) | migration | V18 row appears in `_sqlx_migrations` after a V15 → V18 chain (drift recovery). |

### Deviations from design

- **V18 is a `SELECT 1;` marker, not a schema change.** The design notes
  that `app_settings` is already a generic key-value store (the V2
  schema declares `value TEXT NOT NULL` keyed on an arbitrary `key`).
  No column add is required; V18 exists to (a) make the theme-support
  milestone visible in the migration history, (b) keep the on-disk
  migration checksum stable so `_sqlx_migrations` does not flag drift
  on databases that already shipped V1–V17, and (c) provide an
  idempotent anchor for future migrations that may need to add an
  index on the `theme` key. The data layer does not rewrite any row;
  re-running on an already-migrated pool leaves every row count
  unchanged.
- **`update_settings` partial-update semantic for `last_selected_store_id`
  is unchanged.** The existing "None = clear" semantic is preserved
  (this is the pre-PR2 behaviour the i18n-support change shipped). The
  Option-typed new keys (`theme`, mirroring `language` and
  `require_initial_location_on_lot_create`) branch on `Some` so a
  partial update does not touch sibling settings. The
  `update_settings_partial_theme_update_preserves_other_keys` test
  re-supplies `last_selected_store_id: Some(store.id.clone())` to
  verify the partial update does not silently clear it.
- **`theme` uses a `String` wire type, not a typed enum.** The
  Rust `SettingsResponse.theme: String` mirrors the existing
  `language: String` shape so the type stays a non-nullable string
  for consumers. The frontend narrows to `ThemeName` (`"caduxo-light"
  | "dark"`) at the call site. This avoids serde renaming ceremony
  for a two-value whitelist while preserving a single source of truth
  on the frontend.
- **`UserMessage::ThemeNotAllowed` is added.** Mirrors the
  `LanguageNotAllowed` variant end-to-end (variant declaration,
  EN/ES message arms, parser helper, dispatch integration). The
  inline `if value != "en" && value != "es"` language check is
  refactored alongside its sibling theme gate into a small
  `validate_theme_value` helper so the unit tests can exercise the
  boundary without the Tauri command harness.

### Remaining work (next chained PR)

- **PR 5:** App shell navbar migration (`src/App.svelte`) + theme
  switcher on `ConfigurationPage.svelte`. Lands the Configuration
  page `theme` namespace in `src/i18n/en/index.ts` and
  `src/i18n/es/index.ts`, the `app-shell-gradient` keyframes,
  the `Alert.svelte` IPC-failure surface for the switcher, and the
  frontend consumer of the new `theme` / `theme_configured` /
  `ThemeName` surface shipped in this PR. The frontend now resolves
  against the real backend contract.

### Workload / PR boundary

- **PR 2 actual diff:** 10 files changed (9 backend + 1 frontend),
  well under the 400-line review budget. The new backend code
  (~200 lines including tests) lands inside the forecast (~120
  lines of new logic, ~80 lines of new tests). The frontend change
  is plumbing-only (~10 lines in `src/lib/stores.ts`).
- **Chain strategy:** stacked-to-main for PR 1 → PR 2 (parent
  ratified), feature-branch-chain from PR 3 onward. PR 2 lands on
  the same branch as PR 1; no merge commit splits the two.
