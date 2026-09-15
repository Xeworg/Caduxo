# Explore — caduxo-custom-date-picker

## Scope of this exploration

A user-observed defect on Linux (Tauri WebKit + GTK native date picker widget) breaks
the existing native date inputs. A previous SDD change, `caduxo-date-picker-dismissal`,
attempted a thin wrapper around `<input type="date">` (`src/components/inputs/DateInput.svelte`,
78 lines) and was settled clean against build + svelte-check + cargo test, but the
defect persisted on Linux because the wrapper still rendered the host browser's
native picker. The user has accepted that path is insufficient and asked for a
fully in-house custom date picker (more code is fine).

This document captures substrate for the proposal phase. It does **not** propose
the design — proposals for year-picker shape, popover placement, keyboard nav,
manual-input validation behavior, and the clearable/required prop split are
deferred to `proposal.md`.

---

## 1. Repository state

Working tree is clean (the previous dismissal change was removed before this
planning path was opened). `git status` clean per parent.

### Project config (`openspec/config.yaml`)

| Knob                       | Value     |
|----------------------------|-----------|
| `project`                  | Caduxo    |
| `artifactStore` (project)  | `both`    |
| `sdd.executionMode`        | `interactive` |
| `sdd.reviewBudgetChangedLines` | `800` |
| `sdd.chainedPrStrategy`    | `auto-forecast` |
| `sdd.strictTdd`            | `false`   |
| `product.priorityPlatforms` | `windows`, `linux` |
| `product.nonGoals`         | POS, billing, payments, accounting, full inventory |

This session's preflight overrides `artifactStore` to `openspec` for persistence
and keeps `executionMode: interactive`; the project-level values still apply
where the session did not explicitly say otherwise (review budget stays 800).

### Frontend / backend split

- Frontend: Svelte 5 + Vite + TypeScript, mounted via Tauri 2 (`src/main.ts`,
  `src/App.svelte`). Scripts in `package.json`: `dev`, `build`, `preview`, `tauri`.
  Type-check: `npx svelte-check --workspace . --threshold error`.
- Backend: Rust + Tauri in `src-tauri/` (SQLite, command/handler layer,
  services + repositories). Test harness: `cargo test --manifest-path
  src-tauri/Cargo.toml --lib`.
- **No frontend test harness.** `grep` confirms no `vitest`, no `*.test.ts`,
  `*.spec.ts`, `playwright`, or `cypress` under `src/`. The canonical spec's
  `Engineering safety > tests accompany implementation` requirement already
  accepts manual verification for frontend changes when the scope is recorded.

---

## 2. Native date inputs found in the codebase

Exhaustive grep for `type="date"` and `placeholder="YYYY-MM-DD"` across
`src/` returns exactly **three** instances. There is no other date input
on the frontend (no CSV-import date pickers, no settings page, no historical
"created after" filter).

| # | File (path under `src/`) | Lines | Variable | Required? | Cleared by user? |
|---|---|---|---|---|---|
| 1 | `components/LotForm.svelte` | 244–249 | `expiryDate` | Yes (`required` attr + JS guard `if (!expiryDate) errorMsg = "Expiry date is required";`) | No — required by spec |
| 2 | `components/ReportsPage.svelte` | 368–373 | `dateFrom` | No (passed as `null` when blank) | Yes — optional filter |
| 3 | `components/ReportsPage.svelte` | 376–382 | `dateTo` | No (passed as `null` when blank) | Yes — optional filter |

`LotForm` pre-fills `expiryDate` on create with `today + defaultAlertDays`:
`d.toISOString().slice(0, 10)`. That is `YYYY-MM-DD` already — the picker
contract must continue to bind exactly this shape.

### CSS scope

Both files already style `label input[type="date"]` (LotForm line 343) and
`label input` (ReportsPage `.filter-field input` — no `type="date"` selector).
After the picker lands, those scoped selectors can be deleted cleanly. The
picker should slot into the existing `.filter-field` / `label` + input
vocabulary, not introduce a new layout idiom.

---

## 3. What the previous attempt did and why it didn't work

From the sdd-runtime records at
`.git/gentle-ai/sdd-runtime/v1/caduxo-date-picker-dismissal/records/`:

- `attempt/finish` (ordinal 1) recorded:
  - added `src/components/inputs/DateInput.svelte` (78 lines)
  - adopted in both `LotForm.svelte` and `ReportsPage.svelte`
  - `npm run build` passed, `npx tsc --noEmit` passed
  - `cargo test` had 276 pass + 2 pre-existing failures unrelated to the slice
  - diagnosis explicitly noted: "manual Linux/Windows smoke remains for verify"
- The wrapper only adjusted the input's chrome (focus/blur, label spacing).
  The DOM element under it was still `<input type="date">`, which **still
  hands off rendering to the host WebView's native picker** — the very thing
  exhibiting the defect.

**Key learning:** a wrapper around `<input type="date">` cannot remove the
native picker bug. Any new design MUST NOT render `<input type="date">` for
the LotForm expiry or Reports `dateFrom`/`dateTo` fields. The trigger can be
a plain `<input type="text">` (with ISO `YYYY-MM-DD` shape) or a button that
opens the popover; the actual date picking widget must be in-house Svelte.

---

## 4. Canonical spec surface

`openspec/specs/caduxo-expiry-tracker/spec.md` (only domain in the project,
no other domains today). The picker change touches three capabilities:

| Capability | Requirement | Why the picker matters |
|---|---|---|
| `Expiry lots` | `lot registration` | `expiry date` is a **Required lot field**. The form semantics (`required`, JS-guard required) must remain: an empty value blocks submit. |
| `Reports` | `report filters` | `date range` is a listed filter. Empty `date_from` / `date_to` currently becomes `null` in `buildFilters()` (ReportsPage line 139–140). The picker must preserve "empty → null" semantics. |
| `Engineering safety` | `tests accompany implementation` (and `safe log content`) | No new test harness for frontend; manual verification is allowed and recorded. Backend is unaffected (no Rust changes expected in this slice). |

No current requirement enumerates *how* date fields are picked. The change
will likely add **one** requirement under a new `## Capability: Date input`
(or extend the two relevant capabilities) covering:

- The picker is a Svelte component, not a native `<input type="date">`
- Selection semantics (click → `YYYY-MM-DD` → close)
- Manual `YYYY-MM-DD` text input is supported and visually validated
- `clearable` prop contract (required vs optional fields)
- Year range 1900–2100
- Keyboard navigation (to be specified — see open questions)
- Accessibility (`role=dialog`, focus trap, Esc to close — to be specified)

---

## 5. Substrate the proposal phase needs

### Hard constraints (already user-confirmed or canonical)

- Platforms: Linux + Windows (Tauri's WebKitGTK on Linux, WebView2 on Windows).
- ISO string in/out: `YYYY-MM-DD`, no timezone shift. Bound value is a plain
  string (today's wiring).
- Empty value semantics: `""` for blank → caller decides (LotForm treats as
  missing/required; ReportsPage treats as `null` filter). The picker should
  write `""` on clear and let the host decide.
- Year range: 1900–2100 inclusive.
- Required vs clearable: split by `clearable` prop, default `true` (Reports);
  `LotForm` must pass `clearable={false}`.
- Manual `YYYY-MM-DD` input must visually validate — invalid text is kept as
  typed (no silent rewrite), with inline error / red border on blur.

### Open product questions (defer to proposal/parent round)

1. **Year-picker shape.** "Explicit year, month, day selection" can mean:
   (a) a separate year picker screen with a 12-year decade grid and prev/next
   decade arrows; (b) a year dropdown next to the month grid; (c) a numeric
   stepper. For 1900–2100 (201 cells) the decade-grid is the most common
   pattern — confirm direction.
2. **Trigger element.** A plain `<input type="text">` keeps the "type an ISO
   date manually" path natural; a button-only trigger means a separate text
   input for manual entry. The manual-input requirement implies a text input
   must remain visible. Recommend: keep one text input that doubles as the
   trigger (focus or icon click opens the popover).
3. **"Today" shortcut.** A `Today` button inside the popover is convenient
   for both hosts but only useful when the user is picking near today's
   date. Required?
4. **Locale.** No i18n infrastructure exists in `src/`. Spec language is `en`
   per `openspec/config.yaml` `language: en`. Recommend: ship English-only
   month/weekday labels in this slice, no i18n dict yet.
5. **Keyboard navigation scope.** Arrow keys + Enter + Esc are typically
   expected. Should we also bind PageUp/PageDown (month) and Shift+PageUp
   (year)? Parent should confirm minimum acceptable keyboard ergonomics.

### Codebase constraints the proposal should mirror

- The codebase has **no popover / portal / floating UI primitive**. Existing
  modals are page-scoped overlays in `DashboardPage.svelte`
  (`role="dialog"`, `aria-modal="true"`, `.modal-overlay` / `.modal-box`).
  The picker is a non-modal floating popover — it is genuinely a new primitive.
  It can be implemented without a portal by absolute-positioning relative to
  the trigger; Svelte 5 + a `bind:this` ref + `getBoundingClientRect` is enough.
- No CSS framework. Existing components use hand-written CSS in a `<style>`
  block per component. The picker follows the same convention (no Tailwind,
  no shared theme file beyond `--root` colors).
- Already-styled form patterns to reuse: `.filter-field` (Reports) and
  bare `label > input` (LotForm). The picker trigger should fit visually
  inside both.
- TS `strict` is not configured in `tsconfig.json` (no `tsconfig.json` at the
  repo root was found by grep — frontend relies on `svelte-check`). The
  proposal should not introduce a vitest harness in this slice.

### Build / verify command set for this slice

Frontend-only slice; no Rust changes expected. The verify gate that matters:

- `npx svelte-check --workspace . --threshold error` (canonical)
- `npm run build` (Vite build)
- Manual smoke on Linux WebKitGTK and Windows WebView2 covering the three host sites

Backend commands (record-and-monitor, no new tests):

- `cargo test --manifest-path src-tauri/Cargo.toml --lib` should remain GREEN
  with the same pre-existing 2-failure baseline as the last verify record
  (`services::reports::tests::preview_report_in_alert_window_returns_alert_lots`,
  `services::reports::tests::preview_report_next_30_days_returns_30d_lots`),
  per `openspec/changes/archive/2026-09-14-caduxo-measurement-unit-options/apply-progress.md`.

---

## 6. Risks the parent should know before proposal

1. **Scope size.** Replacing three native inputs with a custom calendar is
   materially larger than the 78-line wrapper that just settled. Realistic
   line-count envelope (calendar component + 3 host edits + CSS): ~250–500
   LOC, comfortably under the 800-line canonical review budget but a step
   up from the wrapper slice.
2. **Calendar UI is frequently underspecified.** Year picker / decade grid vs.
   year dropdown, popup placement (below vs. above vs. auto), focus trap
   semantics on open, and Esc-to-cancel behavior are common mid-implementation
   pivots. The proposal phase should explicitly enumerate these before design
   locks.
3. **Required vs clearable contract must propagate.** `LotForm` currently
   carries the required semantics in two places (`required` HTML attr +
   `if (!expiryDate)` JS guard). The picker must respect the host contract
   and not auto-clear when `clearable={false}`. A missed prop wiring here
   would silently regress expiry submission.
4. **Manual input must not be silently rewritten.** The user typed text must
   remain visible until validation flips it. Any "normalize" logic that
   silently edits the bound text would violate the user's explicit
   instruction.
5. **Year boundary strictness.** 1900 and 2100 are the inclusive bounds.
   The picker must reject (visually + on commit) any year outside this range.
6. **Backend untouched.** This slice is intentionally frontend-only; if the
   proposal accidentally implies backend changes (e.g., to `ReportFilters`
   date semantics) it is a scope expansion.
7. **No `<input type="date">` left behind.** The verification step must
   confirm `grep -R "type=\"date\"" src/` returns nothing. This is the
   defect fix in one line.

---

## 7. Recommended next phase

Proceed to **`proposal.md`** with the `delivery_strategy: ask-on-risk` round
on the 3–5 product questions above (year-picker shape, trigger element,
Today shortcut, locale, keyboard nav). After locking those, write the
proposal + spec delta against `## Capability: Date input` (new) or extend
`Expiry lots > lot registration` and `Reports > report filters` to refer to
the new component; whichever is smaller and easier to verify. Then `design.md`
- `tasks.md`.

## Key Learnings

- The previous `caduxo-date-picker-dismissal` settled clean against every
  automated gate yet the defect persisted: a thin wrapper around
  `<input type="date">` does NOT remove the OS-native picker widget. Any
  new design MUST genuinely not render the native input.
- Project has no frontend test harness (`strictTdd: false`, canonical spec
  accepts manual frontend verification). Do not introduce vitest in this
  slice.
- There are exactly **three** date input sites; spreading the wrapper /
  picker across them is mechanical once the component is built.
- Linux/WebKitGTK and Windows/WebView2 are the two target webviews; both
  ship native picker widgets controlled by `<input type="date">`. The
  user's diagnosis (Linux native picker bug) is consistent with the
  platform config.
- `priorityPlatforms` from project config matches user-confirmed platforms.
- Empty string → `null` semantics in ReportsPage filters (`dateFrom.trim()
  || null`) must be preserved by the new picker.
- Required semantics in LotForm (`required` attr + JS guard) must propagate
  through the picker via a `clearable` prop.
- The codebase has no popover primitive — the picker introduces a new Svelte
  floating-UI primitive; this is acceptable scope but should not be
  retro-fitted into the existing page-scoped modal pattern.
- Manual `YYYY-MM-DD` text input must visually validate but must NOT
  silently rewrite user text — explicit user constraint.

## Artifact store

`openspec` (per session preflight; project config says `both` but session
preflight overrides). Persisted to `openspec/changes/caduxo-custom-date-picker/explore.md`.

## skill_resolution

`none` (the parent did not inject any `## Skills to load before work` paths
in the user message; this phase was executed without project/user skill
discovery per the contract).
