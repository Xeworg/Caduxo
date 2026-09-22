# Explore — calendar-annual-overview

## Scope of this exploration

`calendar-month-picker` (now landed and archived via PR #20) intentionally
kept the Calendar tab **monthly** and explicitly deferred a multi-month /
annual Calendar tab view to this follow-on change. The two pieces of
follow-on language that pin this slice are recorded as a hard contract:

- `openspec/changes/calendar-month-picker/proposal.md` §"Non-goals":
  *"Redesign the Calendar tab as a full annual overview or 12-month
  grid — that is the follow-on change `calendar-annual-overview`."*
- `openspec/changes/calendar-month-picker/design.md`
  §"Out-of-scope confirmations": *"Annual overview / 12-month grid on the
  Calendar tab → follow-on `calendar-annual-overview`."*

This document is **substrate for the proposal phase**. It does not
propose the slice, the i18n surface, or the spec delta — those belong to
the proposal. It records (1) the current Calendar tab architecture,
(2) the existing primitive contract that constrains every annual-view
option, (3) the surface the change will touch, (4) the bounded first
slice we recommend, (5) the risks per option, and (6) the next SDD
phase.

Per the session preflight and the user's direction, **no source files
are written, no commits are made, and nothing is pushed** — this is
exploration only. The artifact produced by this phase is this
`explore.md` and its Engram mirror.

---

## 1. Change metadata

| Field                       | Value                                                    |
|-----------------------------|----------------------------------------------------------|
| Change ID                   | `calendar-annual-overview`                               |
| Domain                      | `caduxo-expiry-tracker` (single-domain project)          |
| Predecessor                 | `calendar-month-picker` (landed via PR #20; verify PASS) |
| Artifact store              | `both` (OpenSpec + Engram, per session preflight)        |
| Engram topic key            | `sdd/calendar-annual-overview/explore`                   |
| OpenSpec artifact          | `openspec/changes/calendar-annual-overview/explore.md`   |
| Project review budget       | 800 changed lines (`openspec/config.yaml`)              |
| Session review budget       | 3000 changed lines (session preflight override)          |
| Delivery strategy           | `auto-chain` (deferred until chaining is selected)       |
| Strict TDD                  | `false` (project default; see Risk R6)                  |
| Execution mode              | `interactive` — this phase completes only `explore`     |
| Skill resolution            | `paths-injected` (gentle-ai skill loaded from parent path) |
| Native SDD state (parent)   | `next: sdd-new`, `active change: not found`              |

### Session preflight overrides honoured

- `executionMode: interactive` — interactive phase gate is on; the
  next phase (`proposal`) MUST wait for explicit user approval.
- `artifactStore: both` — this artifact is persisted to OpenSpec and
  mirrored to Engram.
- `reviewBudgetChangedLines: 3000` — any forecast above 3000 LOC MUST
  trigger `deliveryStrategy: ask-on-risk` before `tasks.md`.
- `chainStrategy: deferred` — no chained split is decided at
  exploration time. The recommended first slice is bounded; multi-PR
  chaining is only needed if the proposal forecast exceeds the 3000-LOC
  budget.

---

## 2. Current Calendar tab architecture

### 2.1 Files that shape the Calendar tab

| Path                                              | Role today                                                                                  | Touch surface for this change                    |
|---------------------------------------------------|---------------------------------------------------------------------------------------------|--------------------------------------------------|
| `src/App.svelte`                                 | Mounts `<CalendarPage />` inside the `calendar` tab (`App.svelte` line 422)                 | Unchanged                                        |
| `src/components/CalendarPage.svelte`             | Page container: loading + error + `cal-layout` flex (grid + day panel)                      | **Page-level**: toggle, header, year nav, grid layout |
| `src/components/CalendarMonth.svelte`            | The 280-px single-month primitive (header / month-grid / footer slot)                       | **Not necessarily touched**: the primitive is monthly; the annual view is a sibling |
| `src/components/DatePicker.svelte`               | Popover consumer of `CalendarMonth.svelte` (must stay monthly)                              | **Out of scope**: popover stays single-month     |
| `src/lib/dashboard.ts`                           | `listDashboardLots(filters)` invoke → `DashboardResponse { lots }`                         | Unchanged: this is the existing data source      |
| `src-tauri/src/commands/dashboard.rs`            | `#[tauri::command] list_dashboard_lots` (no new command in this slice)                       | Unchanged                                        |
| `src/i18n/en/index.ts` / `es/index.ts`           | Calendar namespace `LL.calendar.*`                                                          | **Additive**: new keys for annual view labels    |
| `src/i18n/i18n-types.ts` (generated)             | Type-checks `$LL.calendar.*` across `en` / `es`                                              | Regenerated via `npm run i18n:generate`         |
| `openspec/specs/caduxo-expiry-tracker/spec.md`   | Two occurrences of `### Requirement: Calendar tab` (lines 1227 and 2739) + `Calendar month picker` (line 2886) | **Both** occurrences of "Calendar tab" modified; new capability likely added |
| `openspec/config.yaml`                           | Project SDD knobs                                                                             | Unchanged                                        |

### 2.2 Calendar tab layout today (post-`calendar-month-picker`)

The Calendar tab renders a centred 1100-px page with a header row
(title + Refresh button) and a flex `.cal-layout` row:

```
cal-layout (flex, gap 24px)
├── cal-grid-wrapper   ← single <CalendarMonth> (280 px wide)
└── day-panel          ← lots for the selected day, badge-counted
```

`CalendarMonth.svelte` itself:

- `display: inline-flex`, `width: 280px`, `flex-direction: column`
- Header: prev chevron ← (month label / year chip) → next chevron
- Weekday row (aria-hidden, 7 cells)
- Day grid (42 cells, every cell carries dot badge / today ring /
  selection ring / disabled state / weekend muted)
- Footer slot used by `DatePicker.svelte` for the Today shortcut

The calendar primitive is consumed in two contexts (Calendar tab and
`DatePicker` popover). The `calendar-month-picker` change deliberately
kept the contract shared so the annual-view change can also share it
without regression risk.

### 2.3 Data path

- Page mount calls `listDashboardLots({ store_id: null, location_id: null,
  preset: null, urgency: null })` **once** via `loadLots()` with a
  10-second watchdog, generation guard, and elapsed-ticker diagnostic.
- The page computes `dayBadges: Record<string, number>` (date → count of
  active lots) client-side from the returned `lots` payload.
- `dayRows` is a derived `lots.filter(l => l.expiry_date === selectedDate)`.
- No refetch on month or year change in the current slice.
- `listDashboardLots` reads from
  `src/db/repositories/dashboard.rs::list_dashboard_lots` against
  SQLite via `sqlx`; urgency is computed server-side per row.

**Implication for this change**: the annual view can read from the
same in-memory `lots` array; no new backend command, no refetch per
year, no refetch per month. The data is already on the page.

### 2.4 Spec surface that already exists

`openspec/specs/caduxo-expiry-tracker/spec.md` after the
`calendar-month-picker` apply carries (relevant excerpts):

- `## Capability: Calendar` (line 1225, repeated at line 2737 because
  the canonical spec has two `### Requirement: Calendar tab` copies —
  this is a daisyui-redesign artefact; both copies must be updated in
  lockstep).
- The "Calendar tab" requirement (1227) already says the day grid,
  month picker, year picker, and keyboard surface are inherited from
  `CalendarMonth.svelte`. It is phrased around **one month**.
- A new `### Requirement: Calendar month picker` (2886) codifies the
  month-picker overlay; the annual-view change can re-use it without
  edits.

### 2.5 i18n surface that already exists

The existing keys we can re-use for the annual view (no new copy in
`en` / `es` except the labels listed in §6):

- `calendar.pageTitle`, `calendar.refresh`, `calendar.refreshAria`,
  `calendar.today`, `calendar.noLotsOnDate`, `calendar.loadingLots`,
  `calendar.loadFailed`, `calendar.dayPanelTitle`,
  `calendar.urgencyLabels.*`, `calendar.table.*`,
  `calendar.monthNames`, `calendar.weekdayShort`, and every
  `calendar.aria*` key from `calendar-month-picker`.

---

## 3. Constraints carried by the predecessor

The `calendar-month-picker` change locks the following decisions into
the canonical spec and design.md. Every annual-view option MUST honour
them or surface an explicit deviation in the proposal question round.

| Locked from `calendar-month-picker/design.md`              | What it constrains for this change                          |
|------------------------------------------------------------|------------------------------------------------------------|
| D1: month picker mirrors year-picker pattern               | Annual view can re-use the same role=dialog / aria contract |
| D3: chevrons and `PageUp` / `PageDown` stay untouched      | Existing per-month keyboard surface stays monthly          |
| D5: no new backend command                                 | `list_dashboard_lots` stays the only data source           |
| D6: annual / compact panel / popover rewrite are out       | This change is the successor for "annual" specifically     |
| D12: `calendar-annual-overview` is a separate proposal     | The boundary is fixed at "annual" only, not "compact panel" |

---

## 4. Bounded first-slice options

Three concrete options were considered for this follow-on. Each is
described, then evaluated against the constraints above.

### Option A — Replace the Calendar tab with a single 12-mini-month "year grid"

- Default landing view for the Calendar tab.
- A new `CalendarYearGrid.svelte` (or `CalendarYearView.svelte`) renders
  12 mini-month tiles in a 4 columns × 3 rows CSS grid (or 3 × 4) inside
  the existing `.cal-grid-wrapper`.
- Each tile is a stripped-down mini-calendar grid: a small `{Month} {year}`
  header, 7-column weekday row, 6-week day grid with the existing badge
  dot, today ring, and selection highlight. **No month / year picker
  overlays inside the tile** — those become a single page-level header
  above the grid (prev-year chevron, "year {viewYear}" trigger, next-year
  chevron).
- The `.day-panel` slides down below the year grid (or replaces the right
  column). The user picks a day by clicking any tile; the existing
  `selectedDate` and `dayRows` flow unchanged.
- Year navigation (`prev-year` / `next-year` chevrons, `PageUp` /
  `PageDown`) is owned by a new `calendar-year-change` event (or it
  re-uses the existing `monthChange` shape with `(year, 1)`).
- i18n additions: `calendar.annualView`, `calendar.monthView`,
  `calendar.annualHeaderLabel`, `calendar.prevYear` / `calendar.nextYear`
  (only if not already covered by `ariaPreviousYear` / `ariaNextYear`),
  `calendar.todayIs` etc.
- Spec delta: modify both occurrences of the "Calendar tab" requirement
  + add a new "Calendar annual view" capability with 8–12 scenarios.

**Pros**

- Honest match for the user's stated intent ("annual / large calendar
  view, not just one monthly calendar").
- Zero changes to `CalendarMonth.svelte`, `DatePicker.svelte`, or any
  Tauri command.
- Re-uses the day-detail panel as-is, so the "click a day → see lots"
  flow is identical to today.
- Spec delta is tractable: one new capability + two occurrences of an
  existing requirement.

**Cons**

- The new `CalendarYearGrid.svelte` introduces a sibling primitive;
  the design system (`docs/design-system.md` from the daisyui-redesign
  archive) needs new tokens for tile sizing / spacing.
- 12 × 42 = 504 day cells is heavy DOM — needs a roving-tabindex or
  limited focus traversal to keep Tab order manageable.
- Cross-tile keyboard navigation (`←` from column 0 of March moves
  focus to column 6 of February) is a new code path.
- Selected day may live in any month; the year grid must surface
  today's location even when the user is viewing a different year.

### Option B — Toggle the Calendar tab between Month and Year views

- New page-level Tabs.svelte ("Month" / "Year") sits above the existing
  `.cal-layout`.
- Default selection is **Year** (per user intent).
- The Month view is the existing single-`CalendarMonth` layout.
- The Year view is the Option A grid described above.
- Day-detail panel layout adapts: beside the grid in Month view,
  below the grid in Year view.

**Pros**

- Conservative: existing Month view is preserved for users who want it.
- Re-uses the existing primitive in the Month view; the Year view is
  additive.
- Easy to revert / A-B test.

**Cons**

- Two views to maintain, both with keyboard surface and i18n labels.
- Slightly larger initial slice (~100–200 LOC more than Option A).
- Tab default per user is still Year, so the Month view becomes an
  opt-in escape hatch; if users stick with Year, Option A is the
  equivalent with less surface.

### Option C — Keep the month grid but enrich the day-detail panel + add a "year selector"

- Stay on the existing single-`CalendarMonth` view.
- Add prev-year / next-year chevrons to `CalendarPage.svelte` (a page-
  level "year" header above the existing `cal-layout`).
- The day-detail panel becomes taller / wider to keep parity.

**Pros**

- Smallest possible slice; no new primitive.

**Cons**

- **Does not deliver the user's stated intent**. The Calendar tab
  remains one monthly calendar; the year-picker year-chip already
  exists inside `CalendarMonth.svelte`. Option C is the same UX the
  user said is not enough.
- Should be rejected outright.

### Recommendation

**Option A is the recommended first slice.** It is the smallest
delivery that satisfies the stated intent, keeps the regression
surface narrow (`CalendarMonth.svelte`, `DatePicker.svelte`, the
backend, and the day-detail panel are all untouched), and reuses the
data path that already exists in `CalendarPage.svelte`.

If the user prefers that existing users keep the current month-first
experience as the default landing (and only opt into Year explicitly),
**Option B** is the deferred fallback. The proposal question round in
the next phase should disambiguate that.

---

## 5. Risk register for Option A

| ID  | Risk                                                                                          | Severity | Likelihood | Mitigation                                                                 |
|-----|-----------------------------------------------------------------------------------------------|----------|------------|----------------------------------------------------------------------------|
| R1  | 12 × 42 = 504 day buttons bloat the DOM and slow paint / a11y tree scan                       | Medium   | Medium     | Use roving tabindex; only the focused tile's grid owns `tabindex=0`; per-tile grid is `role="grid"` with per-day `role="gridcell"` |
| R2  | Cross-tile arrow-key navigation is a new code path (column 0 → previous tile column 6)        | Medium   | High       | Document the navigation rule in the spec; add unit smoke only if project later flips `strictTdd` |
| R3  | 4-col layout at 280 px = 1120 px, overshoots the 1100 px page container                      | Medium   | High       | Use 3 cols × 4 rows (3 × 280 = 840 px) by default; collapse to 2 cols × 6 rows below the existing breakpoint |
| R4  | The "today" cell is invisible if the user navigates to a previous / future year              | Medium   | Medium     | The page-level year header shows "today is {today}" or a "Jump to today" button inside the year header |
| R5  | `CalendarMonth.svelte`'s `width: 280px` is `<style>`-scoped; the new primitive may inherit globals accidentally | Low | Low | New primitive ships its own scoped style; verify through svelte-check and the CSS-literal grep gate |
| R6  | `strictTdd: false` so no unit tests; verification relies on the manual smoke matrix in `verify-report.md` | Low      | Certain    | Re-use the calendar-month-picker M-row format; pin to desktop runtime per parent policy |
| R7  | The "Calendar tab" requirement appears **twice** in the canonical spec (line 1227 and 2739); both must be edited in lockstep | Low | Certain | Spec delta runs against the textual `## MODIFIED Requirements` block from the change-scoped spec, matching the calendar-month-picker apply pattern |
| R8  | i18n drift between `en` and `es` for new keys                                                | Medium   | Low        | Run `npm run i18n:generate` and re-run `npm run check`; both keys must type-check via `i18n-types.ts` |
| R9  | Visual regression risk for the existing Single-Month UX before this change lands             | Low      | Low        | Spec delta preserves the existing single-month path (Option A: it stays the default in `CalendarMonth.svelte`'s two consumers until Option A's page mode is selected) |
| R10 | Follow-on `calendar-compact-day-panel` change may also want to reflow `.cal-layout`           | Low      | Medium     | Per-page layout stays inside `CalendarPage.svelte`; the compact-panel change can opt into the same flex / grid without conflict |
| R11 | Review-budget: project budget is 800 LOC; session override is 3000. A single-PR Option A is forecast at ~400 LOC, well under both | Low | Low | If the forecast grows past 800 LOC, trigger `deliveryStrategy: ask-on-risk` in the proposal phase before `tasks.md` |

---

## 6. i18n surface preview (only for context, not for applying)

This is a forecast of the keys Option A would likely add. It is NOT a
locked decision — the proposal phase owns the key shape.

- `calendar.annualViewLabel` → "Year" / "Año"
- `calendar.monthViewLabel` → "Month" / "Mes" (only needed if the
  Option B toggle variant is chosen)
- `calendar.annualHeaderLabel` → "Calendar year {year}" / "Año {year}"
  (or similar)
- `calendar.prevYearAria` → "Previous year" / "Año anterior" (may
  re-use the existing `calendar.ariaPreviousYear` from
  `calendar-month-picker`)
- `calendar.nextYearAria` → "Next year" / "Año siguiente"
- `calendar.jumpToToday` → "Today" / "Hoy" (only if not present
  already in `calendar.today`)
- `calendar.tileMonthLabel` → "{month} {year}" (mirrors
  `calendar.ariaMonth` from `calendar-month-picker`)

The actual key shape is resolved by the proposal question round.

---

## 7. Out of scope (carried over from the predecessor)

These remain explicitly OUT of this change and are reserved for
separate changes / future slices:

- **Compact right-side day-detail panel** → follow-on
  `calendar-compact-day-panel`.
- **Popover placement rewrite** in `DatePicker.svelte`
  (`placePopover()`-style helper).
- **Native `<input type="date">` reintroduction** (forbidden by the
  canonical DatePicker requirement).
- **Unit tests for the new annual primitive** — `strictTdd: false`
  applies; manual smoke matrix is the verify gate.
- **New locales beyond `en` / `es`**.
- **A new backend Tauri command** for annual aggregation — the
  in-memory `lots` array is sufficient.
- **Refactor of `CalendarMonth.svelte`** into a generic multi-mode
  component beyond what the month picker already added.
- **Cross-tab badge counts** (e.g. "expiring this year" pill on the
  Calendar nav tab) — separate concern.

---

## 8. Recommended next phase

**Phase: proposal** (interactive; requires explicit user approval
before any further phase runs).

Inputs the proposal phase will need from the user (preview, not the
final question set):

1. Default landing mode when the Calendar tab opens: **Year** (per the
   stated intent) or **Month** with a Year toggle (Option B fallback).
2. Tile layout: 3 cols × 4 rows (room for day-detail panel), 4 cols ×
   3 rows (full-width grid, day panel below), or **auto** based on
   container width.
3. Cross-tile keyboard rule: `←` at column 0 of a tile wraps to
   column 6 of the previous tile (recommended), or stays inside the
   tile (simpler but inconsistent).
4. Year-navigation anchor: a page-level header above the grid
   (prev-year / next-year chevrons + "year {viewYear}" trigger)
   **replaces** the per-month chevrons / year-picker trigger inside
   the day grid header; or it **coexists** with both.
5. Today visibility rule: always highlight today's cell inside its
   tile, with a "Jump to today" button in the page-level header
   (recommended), or only show today's tile (annoying on a future
   year).
6. Day-detail panel placement: **below** the year grid (recommended;
   the day panel is the second row of `.cal-layout`), or stacked
   side-by-side via a separate column (only if 3-col tile layout).

After the proposal question round, the proposal writes the change-scoped
spec delta (`openspec/changes/calendar-annual-overview/specs/caduxo-expiry-tracker/spec.md`),
modifies both occurrences of the "Calendar tab" requirement, and adds
the new "Calendar annual view" capability. Then `design.md` (locked
decisions D1–D{≥6}), `tasks.md` (forecast under 800 LOC for Option A;
above would trigger `ask-on-risk`), and the apply / verify / archive
phases run per the project's OpenSpec lifecycle.

---

## 9. Artifacts produced by this exploration

- This file:
  `openspec/changes/calendar-annual-overview/explore.md` (OpenSpec
  side, persisted).
- Engram topic key:
  `sdd/calendar-annual-overview/explore`, type `architecture`,
  project `Caduxo`, scope `project`, `capture_prompt: false`.

No source files were written, no commits were made, and nothing was
pushed. The interactive phase gate stays open: this phase completes
only `explore`; the next phase (`proposal`) MUST wait for explicit
user approval per `executionMode: interactive`.
