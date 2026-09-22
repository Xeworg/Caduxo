# Proposal — calendar-annual-overview

## Change metadata

- **Change ID**: `calendar-annual-overview`
- **Domain**: `caduxo-expiry-tracker` (single-domain project)
- **Artifact store**: `both` (per session preflight; persisted to OpenSpec and
  Engram topic key `sdd/calendar-annual-overview/proposal`)
- **Review budget**: 800 changed lines (project configuration) / 3000 changed
  lines (session preflight override). A single-PR forecast for this slice is
  well under 800 LOC; `deliveryStrategy: ask-on-risk` is not expected to
  trigger before `tasks.md`.
- **Delivery strategy**: `auto-chain` (deferred until chaining is selected)
- **Chain strategy**: `deferred` — no chained split planned at proposal time
- **Strict TDD**: `false` (project default; the existing `CalendarMonth.svelte`
  and `DatePicker.svelte` have no unit tests today, and the project does not
  require TDD for UI primitives)
- **Execution mode**: `interactive` — this phase completes only `proposal`;
  `spec`, `design`, `tasks`, and the apply/verify/archive phases must wait
  for explicit user approval
- **Skill resolution**: `paths-injected` (gentle-ai skill + SDD proposal
  contract from the parent addendum; no SDD-proposal-specific skill was
  indexed in the registry)
- **Predecessor**: `calendar-month-picker` (landed via PR #20, archived; the
  non-goals and design.md both reserve this slice as a follow-on change)
- **Native SDD state (parent)**: pending `proposal` approval; `next:
  sdd-new` until the spec phase starts

## Problem statement

The Calendar tab today (post-`calendar-month-picker`) renders a single
`CalendarMonth.svelte` primitive — one month, 280 px wide — on the left, plus
a day-detail panel on the right that lists lots expiring on the selected
day. The predecessor change explicitly deferred the follow-on question that
this change now answers: should the Calendar tab open in an annual overview
instead of a single month?

Two pieces of predecessor language pin this slice as a hard contract:

- `openspec/changes/calendar-month-picker/proposal.md` §"Non-goals":
  *"Redesign the Calendar tab as a full annual overview or 12-month grid —
  that is the follow-on change `calendar-annual-overview`."*
- `openspec/changes/calendar-month-picker/design.md`
  §"Out-of-scope confirmations": *"Annual overview / 12-month grid on the
  Calendar tab → follow-on `calendar-annual-overview`."*

Why this slice is worth doing now:

1. Users browsing "expirations this year" have no fast path; they have to
   click the year-picker and page through twelve monthly grids to scan the
   year.
2. The existing primitive's 280-px width leaves substantial empty vertical
   space inside the 1100-px page container.
3. The calendar-month-picker work standardised the keyboard surface and
   the picker overlay pattern, so the annual-view primitive can re-use the
   dot badges, today ring, selection ring, weekend muted, and weekday row
   without redefining them.

The fix is a sibling primitive that the Calendar tab swaps in as its
default landing: a 3-column × 4-row year grid of mini-month tiles, with
page-level year chevrons and a Today button, leaving `CalendarMonth.svelte`,
`DatePicker.svelte`, the backend, and the day-detail panel untouched.

## Outcomes (success criteria)

After this change ships, a Caduxo user can:

1. Open the Calendar tab and see the **current year as a 3-column × 4-row
   grid of mini-month tiles**; today's cell is highlighted inside its tile
   (the current year is the default view).
2. Click any day in any tile; the year view **stays open**, the clicked
   day becomes `selectedDate`, and the existing day-detail panel updates
   to list the lots expiring on that date.
3. Navigate years with a **page-level chevron row** rendered as
   `‹ {viewYear} ›` directly above the year grid (chevrons flanking the
   year-picker trigger).
4. Click a **Today button** in the page-level header to jump the view
   back to the current year and re-select today (updates both `viewYear`
   and `selectedDate`).
5. See today's ring inside any tile of any year — the year grid always
   shows today's tile, even when `viewYear` differs from the current
   year, so users can find the anchor.
6. Read every new affordance (year chevrons, Today button, tile header)
   in English and Spanish via the existing `LL.calendar.*` i18n surface;
   regenerate `i18n-types.ts` so the type system stays in lockstep with
   `en` and `es`.
7. Continue to use `DatePicker.svelte` and any future `CalendarMonth`
   consumer without regression — the monthly primitive and the popover
   are untouched.

## Scope (in scope)

- The Calendar tab's default landing primitive changes from
  `<CalendarMonth />` to a new `<CalendarYearGrid />` rendered by
  `src/components/CalendarPage.svelte` inside the existing `.cal-grid-wrapper`.
- The new `src/components/CalendarYearGrid.svelte` primitive renders a
  **3-column × 4-row CSS grid** of 12 mini-month tiles for a `viewYear`
  prop. Each tile carries the same visual contract as `CalendarMonth.svelte`
  (7-column weekday row, 6-week day grid, dot badges, today ring,
  selection ring, weekend muted) at a smaller scale and **without** per-tile
  month / year picker overlays — year navigation lives at the page level
  only.
- A page-level year header above the grid inside `CalendarPage.svelte`:
  previous-year chevron (`‹`), clickable `viewYear` year-picker trigger, next-year
  chevron (`›`), and a `Today` button. The header owns all year navigation;
  the year-grid primitive is unaware of it.
- Day click inside any tile dispatches `selectDate: string` (ISO
  `YYYY-MM-DD`) and the parent updates `selectedDate` via the existing
  reactive flow. The year view does **not** collapse to a single month on
  day click.
- Today's anchor rule: when `viewYear` differs from the current year,
  today's tile still highlights today's cell with the today ring so the
  user can find the anchor.
- The existing day-detail panel stays visible and updates in place when a
  day is clicked. Its placement inside `.cal-layout` adapts inside
  `CalendarPage.svelte` only; no compact-panel redesign is included.
- Keyboard surface on the year grid: roving tabindex per tile (only the
  focused tile's grid owns `tabindex=0`); per-day `role="gridcell"`
  inside a per-tile `role="grid"`; arrow keys traverse within a tile and
  wrap to the previous / next month tile at the column boundaries.
- i18n surface additions under `LL.calendar.*`:
  - `annualHeaderLabel` ("Calendar year {year}" / "Año {year}"),
  - `ariaPreviousYear` ("Previous year" / "Año anterior") (re-uses the
    calendar-month-picker key if already present, otherwise added),
  - `ariaNextYear` ("Next year" / "Año siguiente") (same re-use rule),
  - `jumpToToday` ("Today" / "Hoy") (or re-use `calendar.today` if
    wording matches),
  - `tileMonthLabel` ("{month} {year}" — mirrors the existing `ariaMonth`
    pattern).
- Spec edits to `openspec/specs/caduxo-expiry-tracker/spec.md`:
  - Modify both occurrences of the `### Requirement: Calendar tab`
    requirement (lines 1227 and 2739) to describe the new default
    landing primitive, the page-level year header, the Today button,
    and the "click a day stays in annual view" rule.
  - Add a new `### Requirement: Calendar annual view` capability with
    scenarios for default landing, day click staying in annual view,
    year chevron navigation, Today button reset, today's anchor in any
    year, keyboard traversal, and dot badges driven by the same
    `dayBadges` map.

## Non-goals

This change does NOT:

- Redesign the right-side day-detail panel into a more compact form
  (follow-on `calendar-compact-day-panel`).
- Change anything in `src/components/DatePicker.svelte` — the popover
  stays single-month and continues to consume `CalendarMonth.svelte`.
- Introduce a new backend Tauri command — the existing
  `list_dashboard_lots` payload is the only data source; the in-memory
  `lots` array drives both the tile dot badges and the day-detail
  panel, and no refetch happens on year change.
- Replace any `<input type="date">` with a native picker (the canonical
  DatePicker requirement forbids it).
- Refactor `CalendarMonth.svelte` into a generic multi-mode component.
  The annual-view primitive is a sibling, not a refactor.
- Add a Month / Year view toggle (Option B from exploration). The user
  has chosen Year-only as the default landing; an opt-in Month toggle
  is deferred.
- Drill-down from a tile header to a single-month view — clicking the
  tile header in this slice is read-only; clicking the tile body selects
  a day. Drill-down is a future slice.
- Add unit tests for the new primitive (project default
  `strictTDD: false`; manual smoke matrix on the Calendar tab is the
  verify gate, mirroring the calendar-month-picker pattern).
- Add new locales beyond `en` / `es`.
- Add cross-tab badge counts (e.g. "expiring this year" pill on the
  Calendar nav tab) — separate concern.
- Change `minDate` / `maxDate` semantics — the existing default range
  (1900-01-01 → 2100-12-31) already covers the user's plausible
  navigation horizon.
- Rename or reorder existing events (`monthChange`, `viewYearChange`,
  `ariaOpenMonthPicker`, `ariaOpenYearPicker`); only the *consumer*
  (`CalendarPage.svelte`) switches its default child primitive.

## Confirmed product decisions

| #   | Decision                                                                                                                                  | Source                                                          |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| 1   | Calendar tab opens in annual view by default                                                                                              | User answer to exploration §8 Q1 (default landing mode)         |
| 2   | Year grid layout is **3 columns × 4 rows** of mini-month tiles                                                                            | User answer to exploration §8 Q2 (tile layout)                  |
| 3   | Clicking a day **stays in annual view** and updates the detail panel; the panel shows expirations for the selected date                   | User answer to exploration §8 Q1 + click-stays rule             |
| 4   | Year navigation uses page-level chevrons rendered as `‹ {viewYear} ›` above the year grid                                                 | User answer to exploration §8 Q4 (year-navigation anchor)       |
| 5   | A **Today button** in the page-level header returns the view to the current year and selects today                                        | User answer to exploration §8 Q5 (today-visibility rule)        |

## Proposed capabilities

### Modified capability: Calendar tab (`CalendarPage.svelte` + spec)

- The page's default `viewYear` / `selectedDate` initialization stays
  `new Date()`-derived. The **default landing primitive** switches from
  `<CalendarMonth />` to `<CalendarYearGrid viewYear={viewYear}
  dayBadges={dayBadges} selectedDate={selectedDate} on:selectDate={...} />`.
- The existing `loadLots()` call, the 10-second watchdog, the
  `dayBadges` map, and the `dayRows` derived from `selectedDate` stay
  untouched; the new primitive consumes `dayBadges` the same way
  `CalendarMonth.svelte` does today.
- A page-level header row appears above the year grid inside
  `CalendarPage.svelte`: previous-year chevron, clickable `viewYear`
  year-picker trigger, next-year chevron, and a Today button. The
  header row owns all year navigation; the year-grid primitive knows
  nothing about it.
- The existing `.cal-layout` flex wraps the year grid and the day-detail
  panel. The flex direction is unchanged; the year-grid wrapper grows
  vertically to accommodate the 3 × 4 tile grid, and the day-detail
  panel either stays side-by-side or moves below the grid based on the
  page-width breakpoint (this proposal does not add a new breakpoint;
  the existing one is reused).
- Spec delta: both occurrences of `### Requirement: Calendar tab`
  (lines 1227 and 2739 in
  `openspec/specs/caduxo-expiry-tracker/spec.md`) are updated to describe
  the new default landing primitive, the page-level year header, the
  Today button, and the "click a day stays in annual view" rule. The
  existing scenarios for day-grid badges, day-detail panel, lot row
  edit, and data source remain valid with updated wording.

### New capability: Calendar annual view (`CalendarYearGrid.svelte` + spec)

- A new `src/components/CalendarYearGrid.svelte` primitive renders a
  3 × 4 CSS grid of 12 mini-month tiles for the `viewYear` it receives
  as a prop.
- Each tile is a self-contained month — no per-tile month / year picker
  overlays. The tile header shows the month name and year (using
  `LL.calendar.tileMonthLabel`), and the tile body is a 7-column
  weekday row + 6-week day grid with dot badges (from `dayBadges`),
  today ring, selection ring, and weekend muted (visually identical
  contracts to `CalendarMonth.svelte`).
- Tile click on a day emits a single `selectDate: string` event (ISO
  `YYYY-MM-DD`). The parent (`CalendarPage.svelte`) keeps `selectedDate`
  and re-renders the day-detail panel from the existing `dayRows` flow.
- The primitive exposes the same keyboard surface as `CalendarMonth.svelte`
  (Tab, Esc, Enter, arrow keys, PageUp/PageDown) but with **roving
  tabindex per tile**: only the focused tile's grid owns `tabindex=0`.
  Cross-tile arrow navigation follows the documented rule (`←` from
  column 0 of a tile moves to column 6 of the previous month tile;
  `→` from column 6 to column 0 of the next month tile; `↑` / `↓` move
  within the same column of the same week, wrapping to the previous /
  next month when the week crosses a month boundary). The full
  keyboard rule is documented in the spec delta, not in this proposal.
- Today's ring renders inside any tile in any year so users browsing a
  non-current year can still see the anchor.
- Spec delta: a new `### Requirement: Calendar annual view` capability
  with scenarios for default landing, day click staying in annual view,
  year chevron navigation, Today button reset, today's anchor in
  non-current years, keyboard traversal, and dot badges from the same
  `dayBadges` map.

### Modified capability: i18n (`src/i18n/en/index.ts`, `src/i18n/es/index.ts`)

- Add new keys under `LL.calendar.*` for the year-header aria labels,
  the Today button, and the per-tile header. Re-use the existing
  `ariaPreviousYear` / `ariaNextYear` keys from calendar-month-picker
  where they already match the proposed copy.
- Regenerate `src/i18n/i18n-types.ts` via `npm run i18n:generate` so the
  keys type-check across `en` and `es`.

## Risks and considerations

- **DOM weight** (R1 from exploration): 12 tiles × 42 day cells = 504 day
  buttons. Mitigated with roving tabindex per tile (only the focused
  tile's grid owns `tabindex=0`); per-tile ARIA `role="grid"` with
  per-day `role="gridcell"`. The explore forecast is unchanged; this
  proposal commits to the recommended mitigation.
- **Cross-tile keyboard navigation** (R2 from exploration): `←` from
  column 0 of one tile to column 6 of the previous tile is a new code
  path. The spec delta documents the rule; manual smoke covers it
  (project default `strictTDD: false`). Behaviour with row 0 / row 5 of
  the day grid is also documented in the spec.
- **Layout overflow** (R3 from exploration): a 3-column tile grid at
  full tile width would overshoot the 1100-px page container. Tile
  dimensions are sized so the grid plus gutters stays inside the
  container; the existing breakpoint handles sub-1100-px widths by
  collapsing the flex. The spec delta includes a "year grid fits the
  1100-px container without horizontal scroll" scenario.
- **Today invisible across years** (R4 from exploration): today's ring
  renders in any year's tile even when `viewYear` differs from the
  current year, so the user can find the anchor without paging back.
  The Today button is the explicit reset path.
- **`CalendarMonth.svelte` regression surface** (R5, R9): the existing
  primitive is unchanged. `DatePicker.svelte` is untouched. Any future
  consumer that imports `CalendarMonth.svelte` sees no regression.
- **Spec duplicate** (R7 from exploration): the `### Requirement:
  Calendar tab` requirement appears twice in the canonical spec (lines
  1227 and 2739 — a daisyui-redesign artefact). Both occurrences are
  edited in lockstep, mirroring the calendar-month-picker apply
  pattern.
- **i18n drift** (R8 from exploration): every new key is added to both
  `en` and `es` before regeneration. `npm run i18n:generate` +
  `npm run check` catch drift in CI.
- **Follow-on coordination** (R10 from exploration): the
  `calendar-compact-day-panel` follow-on may also want to reflow
  `.cal-layout`. Per-page layout stays inside `CalendarPage.svelte`;
  the compact-panel change can opt into the same flex / grid without
  conflict because this change does not introduce a new layout
  primitive.
- **Review budget** (R11 from exploration): project budget 800 LOC;
  session override 3000 LOC. The proposal's touched files are
  `src/components/CalendarYearGrid.svelte` (new, ~250 LOC),
  `src/components/CalendarPage.svelte` (~80 LOC of additions for the
  year header + default primitive switch), the two i18n files (~10
  lines added), the regenerated `i18n-types.ts`, and two scoped edits
  to `spec.md` plus a new requirement. Forecast well under 800 LOC;
  `ask-on-risk` is not expected to trigger.

## Next phase

After user approval of this proposal, the next phase writes the delta
spec for `caduxo-expiry-tracker` covering:

1. The `### Requirement: Calendar tab` update (both occurrences) for the
   new default landing primitive, the page-level year header, the Today
   button, and the "click a day stays in annual view" rule.
2. A new `### Requirement: Calendar annual view` capability with the
   tile grid, the keyboard surface, the day-click stays-in-view rule,
   the year chevron navigation, the Today button reset, and the
   today's-anchor rule.

Then `design.md` (locked decisions on tile sizing, roving tabindex
rule, cross-tile arrow rule, day-detail panel placement, range
handling, i18n key shape), `tasks.md` (forecast under 800 LOC for this
slice), and the apply / verify / archive phases run per the project's
OpenSpec lifecycle.

## Proposal question round

The five product questions the orchestrator confirmed close all the
open questions from exploration §8. The edge-case questions below are
intended to surface what the spec phase will need so the spec is
unambiguous; if you would rather skip this round and approve the
proposal as written, the spec phase writes against the listed
assumptions.

1. **Day-detail panel placement when annual view is active.** Today
   the day-detail panel sits as the second column of `.cal-layout`
   (side-by-side via flex). In annual view the panel can stay
   side-by-side with the grid (the year grid occupies the left
   column, the panel the right) or move below the grid (panel as
   the second row of the page). The proposed default mirrors
   today's side-by-side flex inside the existing `.cal-layout`;
   if the 3 × 4 grid can't fit comfortably alongside the panel
   at the default page width, the spec phase will move the panel
   below the grid inside the same flex. _Assumed default:
   side-by-side flex, panel moves below only if the layout
   overflows at the default breakpoint._

2. **Drill-down from the tile header.** Should clicking the tile
   header (the `{month} {year}` label) drill down to a single-month
   view of that month (closing the year grid, like a "drill-down"),
   or is the year grid terminal in this slice (drill-down is a
   follow-on change)? The proposed default is **tile header is
   read-only in this slice**; clicking the body still selects a
   day; clicking the header does nothing except receive focus.
   _Assumed default: tile header is read-only._

3. **Today ring when viewing a non-current year.** Today's ring
   renders inside today's tile even when `viewYear` differs from
   the current year (today's month always anchors inside the year
   grid). The proposed default is **always show today's ring** in
   any year — this is the discoverability rule from exploration
   §8 Q5. _Assumed default: always show._

4. **Cross-tile arrow navigation rule.** `←` from column 0 of a
   tile moves to column 6 of the previous month tile; `→` from
   column 6 of a tile moves to column 0 of the next month tile;
   `↑` / `↓` move within the same column of the same week,
   wrapping to the previous / next month when the week crosses a
   month boundary. The proposed default is **wrap across month
   boundaries**; the spec delta documents the rule. _Assumed
   default: wrap across boundaries._

5. **Today-button event shape.** The Today button is owned by
   `CalendarPage.svelte` (not the new primitive). When the user
   clicks Today, both `viewYear` (back to current year) and
   `selectedDate` (back to today) update together. The proposed
   default is **two state updates in sequence inside
   `CalendarPage.svelte`**: `viewYear = currentYear; selectedDate
   = today;`. The new primitive receives the new `viewYear` prop
   and the parent's `selectDate` flow runs unchanged. _Assumed
   default: two state updates, no new event._

After the first answers (or a skip), the proposal assumptions update
and I ask whether you want corrections or a second question round
before the spec phase.