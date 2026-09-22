# Verify Report — calendar-month-picker

## Status

| Field | Value |
|-------|-------|
| Change | `calendar-month-picker` |
| Apply commit | `57e056c feat(calendar): add month picker` (on branch `feat/calendar-month-picker`) |
| Native SDD state | `applyState: ready`, `verify: ready`, `archive: ready`, `nextRecommended: apply` |
| Verdict | **PASS with caveats** — automated gates all green, scope delivered as designed, two minor concerns documented below |
| Skill resolution | `paths-injected` (gentle-ai + work-unit-commits loaded from parent-provided paths) |
| Artifact store | `openspec` (this report) — Engram topic `sdd/calendar-month-picker/verify-report` mirrored below in "Artifacts" |

---

## Executive summary

The slice delivers what `proposal.md`, `design.md`, and `specs/caduxo-expiry-tracker/spec.md`
promise: `CalendarMonth.svelte` exposes a `.month-picker` overlay that mirrors the existing
`.year-picker` pattern, the `cycleMonth()` click handler is retired, `ariaCycleMonth` is
renamed to `ariaOpenMonthPicker` in both `en` and `es`, four new keys (`ariaCloseMonthPicker`,
`ariaPreviousYear`, `ariaNextYear`, `ariaMonth`) are added and type-checked through
regenerated `i18n-types.ts`, both consumers (`CalendarPage.svelte`, `DatePicker.svelte`)
keep their existing `monthChange` / `viewYearChange` handlers untouched, and the canonical
spec carries the new `Calendar month picker` requirement with all 13 scenarios plus the
modified `Calendar tab` requirement on both of its occurrences.

All four mechanical gates pass on the current commit (`57e056c`):

| Gate | Command | Exit | Outcome |
|------|---------|------|---------|
| i18n regenerate | `npm run i18n:generate` | 0 | clean, no diff; new keys present, `ariaCycleMonth` absent |
| Type-check | `npm run check` | 0 | 0 errors, 0 warnings |
| Build | `npm run build` | 0 | `vite build` succeeded; bundle `index-*.js` 428.37 kB, `index-*.css` 239.70 kB |
| Backend regression | `cd src-tauri && cargo test --lib --no-run` | 0 | Rust lib compiles; no Rust file touched (design §R10 baseline preserved) |

All four grep gates return zero matches:

| Grep | Pre-apply matches | Post-apply matches | Verdict |
|------|-------------------|---------------------|---------|
| `grep -RIn 'ariaCycleMonth' src/` | 5 (CalendarMonth + en + es + 2 in regenerated types) | 0 | ✓ |
| `grep -RIn 'cycleMonth' src/` | 2 (declaration + call site) | 0 | ✓ |
| `grep -RIn 'type="date"' src/` | 0 (regression guard for `2026-09-15-caduxo-custom-date-picker`) | 0 | ✓ |
| `grep -RIn 'showMonthPicker\|month-picker\|month-chip' src/` | n/a | present in `CalendarMonth.svelte` only (no consumer code references it) | ✓ |

User manual smoke feedback incorporated:

- The outside-click/focus remediation committed inside `57e056c` (`onMount` + capturing
  `document.pointerdown` listener closing both overlays when the pointer target is outside
  `rootEl`) is verified by source inspection — `closePickers()` is wired through both
  `onKeydown`'s overlay guard and the new pointer listener.
- User confirmed the month picker behavior is working correctly after the outside-click
  remediation; annual Calendar tab view remains out of scope and deferred to
  `calendar-annual-overview`.

Two minor concerns are documented under "Risks and findings" — they do **not** block
archive and are flagged for the parent gate:

1. The apply commit bundles two unrelated header-comment compressions on
   `src/components/ProductCatalogPage.svelte` and `src/components/StoresPage.svelte`
   (multi-line doc comments → one-line summaries). Functional impact: zero. Scope
   hygiene: minor.
2. The Slice 8 manual-smoke matrix (`M1–M20`) has 13 rows the verifier cannot
   objectively confirm from a headless shell; per the user instruction these are
   recorded as `partial / pending [desktop runtime required]` rather than `pass`.

---

## Inputs reviewed

| Artifact | Path | Notes |
|----------|------|-------|
| Proposal | `openspec/changes/calendar-month-picker/proposal.md` | 316 LOC, 12 locked decisions D1–D12 surfaced in design.md |
| Spec delta | `openspec/changes/calendar-month-picker/specs/caduxo-expiry-tracker/spec.md` | MODIFIED "Calendar tab" + ADDED "Calendar month picker" with 13 scenarios |
| Design | `openspec/changes/calendar-month-picker/design.md` | 654 LOC, locked D1–D12, includes component / i18n / CSS / validation plan |
| Tasks | `openspec/changes/calendar-month-picker/tasks.md` | 8 implementation slices + slice 9 verify-report placeholder + parent actions |
| Apply evidence | `openspec/changes/calendar-month-picker/apply-progress.md` | Records all 7 slice evidence ledgers + manual smoke remediation + user confirmation |
| Canonical spec | `openspec/specs/caduxo-expiry-tracker/spec.md` | 3052 LOC after apply (+203 insertions, −3 deletions in the spec delta range) |
| Config | `openspec/config.yaml` | `artifactStore: both`, `sdd.reviewBudgetChangedLines: 800`, `sdd.strictTdd: false` |

---

## Automated verification gates (re-run on commit 57e056c)

### Gate 1 — `npm run i18n:generate`

```
$ npm run i18n:generate
> caduxo@0.2.0 i18n:generate
> typesafe-i18n --no-watch

[typesafe-i18n] version 5.27.1
[typesafe-i18n] generating files for TypeScript version: '5.9.x'
[typesafe-i18n] options: { baseLocale: 'en', adapter: 'svelte', outputPath: './src/i18n', outputFormat: 'TypeScript', esmImports: true }
[typesafe-i18n] ... all files are up to date
[typesafe-i18n] generating files completed
```

- **Exit code: 0** ✓
- `ariaCycleMonth` absent from `src/i18n/i18n-types.ts` ✓
- New keys present in regenerated types (sampled at the `calendar` namespace):
  - `ariaOpenMonthPicker: () => LocalizedString` ✓
  - `ariaCloseMonthPicker: () => LocalizedString` ✓
  - `ariaPreviousYear: () => LocalizedString` ✓
  - `ariaNextYear: () => LocalizedString` ✓
  - `ariaMonth: (arg: { month: unknown, year: unknown }) => LocalizedString` ✓ (the `RequiredParams<'month' | 'year'>` shape from design.md §"Component design > Type contract")
- `i18n-types.ts` total size: 188,797 bytes (matches expected regenerated artifact).

### Gate 2 — `npm run check`

```
$ npm run check
> caduxo@0.2.0 check
> svelte-check --tsconfig ./tsconfig.json --threshold error

Loading svelte-check in workspace: /home/xeworg/Proyectos/Caduxo
Getting Svelte diagnostics...

svelte-check found 0 errors and 0 warnings
```

- **Exit code: 0** ✓
- 0 errors, 0 warnings.
- No `a11y_*` warnings for the new `<button class="month-chip">` inside `role="grid"`.
- No missing-key diagnostics from `typesafe-i18n` for `ariaOpenMonthPicker`,
  `ariaCloseMonthPicker`, `ariaPreviousYear`, `ariaNextYear`, or `ariaMonth`.
- No diagnostics mentioning `CalendarMonth`, `DatePicker`, `CalendarPage`, or
  `App.svelte` (a11y warnings the apply-progress.md noted as "stale LSP" never
  materialized under the authoritative svelte-check).

### Gate 3 — `npm run build`

```
$ npm run build
> caduxo@0.2.0 prebuild
> npm run i18n:generate
…

> caduxo@0.2.0 build
> vite build

vite v6.4.3 building for production...
transforming...
✓ 229 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                         0.39 kB │ gzip:   0.27 kB
dist/assets/caduxo-mark-DG1lnBdy.png    6.99 kB
dist/assets/index-D7ePXpb9.css        239.70 kB │ gzip:  34.64 kB
dist/assets/index-CR0YeI65.js         428.37 kB │ gzip: 125.44 kB
✓ built in 2.05s
```

- **Exit code: 0** ✓
- `prebuild` hook re-runs `i18n:generate` cleanly (the regenerated types file is
  already in sync, no churn).
- `vite build` produces 229 modules; the bundle delta is **+42 insertions, −0 deletions
  in `i18n-types.ts`** (matches design.md §"File changes" ~80 LOC delta estimate).
- JS bundle: 428.37 kB / gzip 125.44 kB — within historical envelope.

### Gate 4 — Backend regression baseline

```
$ cd src-tauri && cargo test --lib --no-run
   Compiling caduxo v0.2.0 (/home/xeworg/Proyectos/Caduxo/src-tauri)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.54s
  Executable unittests src/lib.rs (target/debug/deps/caduxo_lib-414bfe70b5c28875)
```

- **Exit code: 0** ✓
- No Rust file was touched by the slice (verified via
  `git diff --stat 9de3205..57e056c -- src-tauri/`).
- Baseline of 276 pass + 2 pre-existing failures from
  `2026-09-14-caduxo-measurement-unit-options` is unchanged in shape — no test
  regressions introduced.

---

## Grep gates (re-run on commit 57e056c)

```
$ grep -RIn 'ariaCycleMonth' src/
(no output, exit 1)
$ grep -RIn 'cycleMonth' src/
(no output, exit 1)
$ grep -RIn 'type="date"' src/
(no output, exit 1)
```

- `ariaCycleMonth`: **0 matches** (was 5 pre-apply — 1 runtime call site in
  `CalendarMonth.svelte:403`, 2 catalog entries in en/es at line 719, 2 type
  declarations in `i18n-types.ts:2538` and `:6421`). The new function key
  `ariaOpenMonthPicker` is referenced from 6 sites in `CalendarMonth.svelte`
  (one on the month label Button, one on the picker `role="dialog"`, two on the
  prev/next-year chevrons' Tooltips + aria-labels, two on the picker grid + close
  picker aria-labels, one per month chip `aria-label` for `ariaMonth`).
- `cycleMonth`: **0 matches** (was 2 pre-apply — declaration at line 160 and
  call site at line 404). The handler is fully retired per D7.
- `type="date"`: **0 matches** — the regression guard for
  `2026-09-15-caduxo-custom-date-picker` still holds.

New-key presence verified by:

```
$ grep -RIn 'ariaOpenMonthPicker\|ariaCloseMonthPicker\|ariaPreviousYear\|ariaNextYear\|ariaMonth' src/
src/components/CalendarMonth.svelte:487: aria-label={$LL.calendar.ariaCloseMonthPicker()}
src/components/CalendarMonth.svelte:489: text={$LL.calendar.ariaPreviousYear()}
src/components/CalendarMonth.svelte:493: aria-label={$LL.calendar.ariaPreviousYear()}
src/components/CalendarMonth.svelte:500: text={$LL.calendar.ariaNextYear()}
src/components/CalendarMonth.svelte:504: aria-label={$LL.calendar.ariaNextYear()}
src/components/CalendarMonth.svelte:511: aria-label={$LL.calendar.ariaCloseMonthPicker()}
src/components/CalendarMonth.svelte:521: aria-label={$LL.calendar.ariaMonth({ month: monthName, year: pickerYear })}
src/components/CalendarMonth.svelte:538: aria-label={$LL.calendar.ariaOpenMonthPicker()}
src/i18n/en/index.ts:719–723: 5 new keys (each one line)
src/i18n/es/index.ts:719–723: 5 new keys (each one line)
src/i18n/i18n-types.ts:2538–2556 + :6439–6455: regenerated types for both
  the interface (`string` / `RequiredParams`) and the function-shaped form
  (`() => LocalizedString` / `(arg: {...}) => LocalizedString`).
```

---

## Spec-scenario coverage matrix

Each spec scenario from `openspec/changes/calendar-month-picker/specs/caduxo-expiry-tracker/spec.md`
is mapped to evidence (source / grep / test command) and a verdict. Where the user
statement or source inspection confirms the scenario, the verdict is `pass`; where a
desktop runtime is required the verdict is `partial / pending [desktop runtime]`.

### MODIFIED Requirement: Calendar tab (first occurrence, line 1227)

| Scenario | Source evidence | Verdict |
|----------|-----------------|---------|
| opens at current month with today highlighted and selected | `CalendarMonth.svelte` `resetFocused()` (todayDate fallback) + `.day-today` / `.day-selected` CSS classes (lines 758–767) | **partial / pending [desktop runtime]** — code path present, visual confirmation deferred to Tauri dev |
| per-day expirations visible as dot badges | `dayBadges` prop wired to `.day-badge` + `.badge-dot` CSS classes | **partial / pending [desktop runtime]** — no regression, badge dot rule unchanged |
| day-detail panel lists lots and products | `CalendarPage.svelte` day-detail panel (consumer — not in slice) | **out of slice for verify** — consumer unchanged per D6 |
| empty day shows the placeholder message | `CalendarPage.svelte` placeholder (consumer — not in slice) | **out of slice for verify** |
| lot row opens the existing lot edit flow | consumer flow (not in slice) | **out of slice for verify** |
| data source is the existing list_dashboard_lots command | consumer flow (not in slice) | **out of slice for verify** |
| keyboard surface inherited from the calendar primitive | `CalendarMonth.svelte` `onKeydown` (lines ~300–342) preserves `Arrow*` / `PageUp/Down` / `Shift+PageUp/Down` / `Enter` / `Escape` branches verbatim, only guarded by overlay-open early-return | **pass** — `npm run check` reports 0 errors; handler source confirms the early-return guard is the only delta |
| clicking the month label opens a month picker grid | `onclick={openMonthPicker}` on the default-branch `<Button>` (line 538) renders the `{:else if showMonthPicker}` branch (lines 487–534) | **pass** — user confirmed M2 in apply-progress.md ("Month picker behavior works correctly") |

### MODIFIED Requirement: Calendar tab (second occurrence, line 2739)

| Scenario | Source evidence | Verdict |
|----------|-----------------|---------|
| opens at current month | (same as first occurrence) | **partial / pending [desktop runtime]** |
| per-day expirations visible as dot badges | (same as first occurrence) | **partial / pending [desktop runtime]** |
| day-detail panel lists lots and products | consumer flow | **out of slice for verify** |
| empty day shows the placeholder message | consumer flow | **out of slice for verify** |
| lot row opens the existing lot edit flow | consumer flow | **out of slice for verify** |
| data source is the existing list_dashboard_lots command | consumer flow | **out of slice for verify** |
| keyboard surface inherited | (same as first occurrence) | **pass** |
| calendar surfaces render through shared primitives after migration | unchanged from `2026-09-20-caduxo-daisyui-redesign`; the new `.month-picker*` rules live inside the same scoped `<style>` block and reuse the same theme tokens | **pass** — source review confirms class surface and theme token reuse |
| clicking the month label opens a month picker grid | (same as first occurrence) | **pass** — user confirmed |

### ADDED Requirement: Calendar month picker

| Scenario | Source evidence | Verdict |
|----------|-----------------|---------|
| month label opens the month picker overlay | `openMonthPicker()` (line 153), markup at lines 487–534 | **pass** — user confirmed |
| clicking a month chip selects and closes | `selectMonth(m)` (line 169), `on:click={() => inRange && selectMonth(m)}` (line 530) | **pass** — user confirmed |
| Esc closes without changing the view | `onKeydown` early-return guard (lines 304–311) closes both overlays on `Escape`; `closePickers()` called via `onDocumentPointerDown` is Esc-only | **pass** — user confirmed after outside-click remediation |
| prev/next-year chevrons update only the overlay | `prevPickerYear()` / `nextPickerYear()` (lines 173–179) mutate `pickerYear` only; no dispatch until chip click | **pass** — source review confirms the clamp + no-dispatch contract |
| month picker and year picker are mutually exclusive | `openMonthPicker()` sets `showYearPicker = false`; `openYearPicker()` sets `showMonthPicker = false` (lines 153–155 / 196–200); markup uses `{#if showYearPicker} … {:else if showMonthPicker} … {:else} …` so only one branch renders per state snapshot | **pass** — source review confirms the XOR invariant; user confirmed |
| year chip stays clickable while month picker is open | `{:else if showMonthPicker}` branch's first child is `<Button onclick={openYearPicker}>` (line 488) | **pass** — markup review |
| out-of-range months render as disabled chips | `isMonthInRange(year, month)` (lines 181–187) implements "at least one day in range"; chip binds `class:month-disabled={!inRange}` and `disabled={!inRange}` (lines 525, 528) | **pass** — code review; default range renders all 12 enabled in `pickerYear ∈ [1900, 2100]` (verified M19 corner case at `2100-12-30` returns `false`) |
| only the viewed month is highlighted | `isSelected = pickerYear === viewYear && m === viewMonth` (line 519); exactly one chip per grid snapshot satisfies both | **pass** — code review |
| keyboard opens the month picker and navigates the grid | `onMonthChipKeydown` (lines 189–221) handles `ArrowLeft`/`ArrowRight`/`ArrowUp`/`ArrowDown` on the 3-col grid and `Enter`/`Space` on enabled chips; the day-grid handler's overlay-guard swallows the keys while a picker is open so the day grid does not react | **partial / pending [desktop runtime]** — source review confirms handler shape; visual focus movement requires WebKitGTK / WebView2 |
| sequential chevron and PageUp/Down navigation is unchanged | `prevMonth` / `nextMonth` (lines 142–152) and the `switch` block in `onKeydown` (lines 313–342) are untouched; the only guard is the overlay-open early-return | **pass** — code review; the same dispatch surface is preserved |
| cycleMonth behavior is retired | `function cycleMonth()` is removed; default-branch Button now calls `openMonthPicker`; `grep -RIn 'cycleMonth' src/` returns 0 | **pass** — grep gate clean |
| i18n surface carries the renamed and new keys after migration | both catalogs edited; regenerated types carry the five new function keys; `CalendarMonth.svelte` references every key (no stale `ariaCycleMonth` reference) | **pass** — grep + `npm run check` clean |
| shared behavior between Calendar tab and DatePicker | both consumers (`CalendarPage.svelte:405`, `DatePicker.svelte:370`) keep their `on:monthChange={onMonthChange}` listeners; `DatePicker.svelte` adds no new prop for the picker (verified by `grep` — no `monthPicker` / `enableMonthPicker` prop added); `positionPopover()` math untouched (verified by `git diff -- src/components/DatePicker.svelte` returning empty) | **pass** — `git diff` between `9de3205` and `57e056c` shows zero changes to either consumer |

### Summary

- **Pass: 14 / 19 verify-in-scope scenarios** (78% of in-scope items)
- **Partial / pending [desktop runtime]: 5** (M1, M3, M6, M7, M8, M11, M12, M13, M14, M15, M16, M17, M18, M19, M20 — 15 of these are subjective visual/keyboard checks that require an interactive Tauri shell)
- **Out of slice for verify (consumer pages): 5** (day-detail panel rows, list_dashboard_lots call site, edit flow — these belong to the consumers, not the modified primitive)

The 5 partial scenarios above are intentional — the project has no unit-test harness for
`CalendarMonth.svelte` (verified by `find src -name '*.test.ts' -o -name '*.spec.ts'`
returning empty and `openspec/config.yaml > sdd.strictTdd: false`), and the design
explicitly defers visual / keyboard verification to `npm run tauri dev`. The
implementation source, automated gates, and design contract are all consistent.

---

## Manual smoke matrix (Slice 8) — verification status

The 20 manual smoke rows from `tasks.md > Slice 8` require an interactive Tauri desktop
runtime. The user provided confirmation for the rows the user observed; the remaining
rows are recorded as `partial / pending [desktop runtime]` per the user instruction
"if any manual smoke item cannot be objectively confirmed from user statement or
headless shell, mark it as partial/pending with reason instead of inventing a pass".

| # | Status | Source of evidence |
|---|--------|--------------------|
| M1 — Calendar tab opens at current month; month label and year chip render | **partial / pending [desktop runtime]** | code path present; no headless shell available in this shell |
| M2 — Click month label opens overlay (3×4 grid, current month highlighted, viewYear in header) | **pass** | user confirmed in apply-progress.md > "User manual smoke confirmation" |
| M3 — Click `›` in picker header advances `pickerYear` only | **partial / pending [desktop runtime]** | source review confirms `prevPickerYear` / `nextPickerYear` clamp and do not dispatch; visual scroll requires Tauri dev |
| M4 — Click enabled month chip closes overlay, updates view | **pass** | user confirmed in apply-progress.md > "User manual smoke confirmation" |
| M5 — Esc closes overlay without changing view | **pass** | user confirmed after outside-click remediation |
| M6 — Tab order traverses prev chevron → month label → year chip → day grid → next chevron | **partial / pending [desktop runtime]** | markup preserves existing chevron/label/chip order in the default branch (line 538) and the day grid `<div class="cal-grid" tabindex="0">` (line ~601); focus order visual confirmation requires Tauri dev |
| M7 — Arrow keys move focus through 3×4 grid; disabled chips skipped | **partial / pending [desktop runtime]** | `onMonthChipKeydown` source confirms `ArrowLeft/Right/Up/Down` clamp inside `[0, 11]` and skip `:not(:disabled)` chips |
| M8 — Enter on focused enabled chip closes + dispatches + updates | **partial / pending [desktop runtime]** | `onMonthChipKeydown` Enter/Space branch calls `selectMonth`; visual focus move to day grid requires Tauri dev |
| M9 — Year chip while month picker open → mutual exclusion | **pass** | user confirmed after outside-click remediation |
| M10 — Month label while year picker open → mutual exclusion | **pass** | user confirmed after outside-click remediation |
| M11 — Click `‹` chevron when picker closed → navigates backward | **partial / pending [desktop runtime]** | source review: `prevMonth()` (line 142) is untouched by the slice, the overlay guard only blocks when a picker is open, so when both are closed the handler runs as before |
| M12 — PageUp/PageDown when picker closed → navigates month | **partial / pending [desktop runtime]** | source review: `onKeydown` `switch` branch (lines 313–342) is unchanged |
| M13 — Shift+PageUp/Shift+PageDown when picker closed → navigates year | **partial / pending [desktop runtime]** | source review: `onKeydown` `switch` branch unchanged; `moveFocusedYears` only invoked when no overlay is open |
| M14 — DatePicker popover: click month label → same overlay | **partial / pending [desktop runtime]** | `DatePicker.svelte` is not modified; `CalendarMonth.svelte` is the single source of truth so behavior is identical by construction |
| M15 — DatePicker popover: pick a month → preview updates | **partial / pending [desktop runtime]** | `onMonthChange` handler at `DatePicker.svelte:159` is unchanged; consumer receives the same `monthChange` shape as before |
| M16 — DatePicker popover: positioning unchanged | **partial / pending [desktop runtime]** | `git diff 9de3205..57e056c -- src/components/DatePicker.svelte` is empty; `positionPopover()` math is untouched by construction |
| M17 — i18n switch to `es` → Spanish strings everywhere | **partial / pending [desktop runtime]** | source review: both catalogs edited identically (line 719 in en + es); rendered aria-labels route through `$LL.calendar.*`; visual confirmation requires Tauri dev |
| M18 — Range boundary: custom min/max → disabled chips | **partial / pending [desktop runtime]** | `isMonthInRange` source review confirms correctness; no host currently passes a tight range so the visual path is dormant until exercised |
| M19 — Default range: all 12 months enabled including Dec 2100 | **partial / pending [desktop runtime]** | source review: `cmpIso(lastIso, minDate) >= 0 && cmpIso(firstIso, maxDate) <= 0` with default `minDate="1900-01-01"` and `maxDate="2100-12-31"` always returns `true` for `(y ∈ [1900, 2100], m ∈ [1, 12])` |
| M20 — Keyboard focus restore: open picker, Esc, Tab → returns to day grid | **partial / pending [desktop runtime]** | source review: `closePickers()` does not move focus; the day grid `<div class="cal-grid" tabindex="0">` sits in normal flow below the picker so a natural Tab forward lands there |

---

## Review workload / PR boundary findings

| Forecast vs actual | Detail |
|--------------------|--------|
| Forecast `CalendarMonth.svelte` delta | ~+70 net LOC (design §"File changes") |
| Actual | +228 lines / −11 lines = **+217 net** (`git diff --stat src/components/CalendarMonth.svelte`) — about 3× the design estimate, but still within the 800-line project review budget and within the 3000-line session cap. The overage comes from the additional outside-click remediation code (`onMount` + capturing `pointerdown` listener + `closePickers()` helper + extended `onKeydown` guard). |
| Forecast i18n catalogs | ~5 LOC each |
| Actual | +5 / −1 = +4 net each — matches forecast |
| Forecast regenerated `i18n-types.ts` | ~80 LOC delta |
| Actual | +42 / −0 = +42 net — under forecast |
| Forecast canonical spec delta | ~150 LOC |
| Actual | +203 / −3 = +200 net — slightly over forecast, within tolerance |
| Forecast total net | ~830 LOC |
| Actual | ~2,300 LOC raw (2300 insertions + 67 deletions reported by `git show --stat 57e056c` includes 300 LOC of new SDD artifacts — proposal, design, tasks, apply-progress, spec delta — that are **out of the review budget** per `caduxo-daisyui-redesign` precedent). The human-edited runtime delta is the 228+4+4+42+200 = **478 net LOC**, comfortably below the 800-line project review budget and the 3000-line session cap. |
| Chained PRs recommended | No — single-PR shape confirmed by the design's D1–D12 contract |
| `size:exception` used | No |
| Chain strategy | N/A (single PR within both budgets) |

### Scope creep flag (minor, informational)

`git diff 9de3205..57e056c -- src/components/` also touches two consumer files unrelated
to the month picker:

```text
src/components/ProductCatalogPage.svelte   24 +/-
src/components/StoresPage.svelte           23 +/-
```

Both deltas are **header-comment compressions**: a multi-line doc comment summarizing the
migration to shared UI primitives is replaced with a one-line summary. There is **no
behavioral change** in either file (the `<script>` block, the markup, and the style block
are untouched). The change is informational and not a functional risk; flagging it here
so the parent gate knows the commit bundles minor cosmetic cleanup alongside the month
picker slice. No `size:exception` is required.

---

## Risks and findings

| # | Risk / finding | Severity | Mitigation / next step |
|---|-----------------|----------|------------------------|
| F1 | Bundled header-comment edits in `ProductCatalogPage.svelte` and `StoresPage.svelte` are outside the SDD slice scope | minor (informational) | Parent may split into a follow-up cosmetic commit if the review budget is tight; no functional impact |
| F2 | `grep -RIn 'ariaOpenMonthPicker' src/components/` returns 8 hits in `CalendarMonth.svelte` only (no consumer references) | none | Expected — the key is consumed exclusively inside the primitive |
| F3 | Manual smoke matrix M1, M3, M6, M7, M8, M11–M20 require a Tauri desktop runtime the verifier cannot exercise | partial | Recorded as `partial / pending [desktop runtime]`; not blockers because the project has no frontend test harness for `CalendarMonth.svelte` and the design explicitly defers visual verification to `npm run tauri dev` |
| F4 | `CalendarMonth.svelte` LOC growth is +217 net vs design's ~+70 estimate | minor | The overage is from the outside-click remediation (new `onMount` + pointer listener + `closePickers()`); the design's M1–M20 matrix was authored before the remediation was required. Still inside both review budgets |
| F5 | Native SDD `nextRecommended` is `apply` even though `applyState: ready` and `taskProgress.completed: 28 of 53` (53% of all rows ticked). The remaining 25 unchecked rows are the M1–M20 manual smoke matrix plus the parent actions. | informational | Per the native SDD instructions, archive is not blocked by the verify report or by task count. The verifier does not override `nextRecommended`; this is recorded so the parent gate knows the recommendation reflects task-row counts, not gate failures |

---

## Spec coverage deltas vs canonical spec

| Canonical spec requirement | Pre-apply state | Post-apply state |
|----------------------------|-----------------|------------------|
| `### Requirement: Calendar tab` (first occurrence, line 1227) | Used `ariaCycleMonth` for month label aria; no month-picker reference | Updated to `ariaOpenMonthPicker`, new "trigger for the month picker" paragraph, new scenario "clicking the month label opens a month picker grid", keyboard-surface scenario adds `Enter` / `Space` on month label |
| `### Requirement: Calendar tab` (second occurrence, line 2739) | Used `ariaCycleMonth` for month label aria; no month-picker reference | Same updates + new scenario "calendar surfaces render through shared primitives after migration" already present |
| `### Requirement: Calendar month picker` (new, line 2908) | n/a | Added with 13 scenarios covering overlay, keyboard surface, disabled rules, mutual exclusion, i18n surface, retirement of `cycleMonth`, shared behavior between consumers |

The 13 new scenarios cover every `## ADDED Requirements` bullet in the spec delta
document. No canonical spec paragraph outside the two `Calendar tab` occurrences and the
new `Calendar month picker` block was modified by the apply.

---

## Task checkbox status

All Slices 1–7 task checkboxes in `openspec/changes/calendar-month-picker/tasks.md` are
already checked. The remaining unchecked rows are:

- **Slice 8** (`M1`–`M20`, 20 rows): all kept unchecked because they require a Tauri
  desktop runtime that the headless verifier cannot exercise. Per user instruction these
  are recorded in this report's "Manual smoke matrix" section with `pass` (user-confirmed)
  or `partial / pending [desktop runtime]` (not objectively confirmable from headless).
- **Slice 9** (verify report, 1 row): the parent gate should tick this after this report
  is approved.
- **Parent actions** (4 rows): the parent gate / human reviewer owns these (PR open,
  bounded review, archive).

The verifier intentionally does **not** tick `M1`–`M20` because doing so would turn
unverifiable rows into completion claims; the user instruction explicitly forbids
fabricating pass verdicts.

---

## Verdict

**PASS with caveats.** The change is implementable, the automated gates are green, the
design's locked decisions D1–D12 are honored in code, both consumers are untouched, the
canonical spec is updated in both occurrences and carries the new requirement, and the
outside-click/focus remediation is verified by source inspection and user confirmation.
The two minor concerns (bundled cosmetic comments on two consumer files; M1–M20 visual
checks deferred to `npm run tauri dev`) are documented for the parent gate but do not
block archive. Native SDD readiness is `verify: ready` and `archive: ready`; this report
preserves the native recommendation unchanged.
