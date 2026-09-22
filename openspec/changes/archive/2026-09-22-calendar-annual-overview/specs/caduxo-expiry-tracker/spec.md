# Delta for `caduxo-expiry-tracker`

This delta modifies the `Calendar tab` requirement (both occurrences in
the canonical spec) so that the Calendar tab opens in an annual
year-grid view by default, with a page-level year header and Today
button, and so that a day click keeps the annual view open while
updating the existing day-detail panel. It also adds a new
`Calendar annual view` requirement that codifies the new
`CalendarYearGrid.svelte` sibling primitive.

## MODIFIED Requirements

### Requirement: Calendar tab

The system MUST provide a top-level main-navigation Calendar tab
(added between Products and Reports) that opens the **current
year as a 3-column × 4-row grid of mini-month tiles** (an annual
overview landing) with today highlighted inside its tile and
selected as `selectedDate`, displays per-day lot expirations as
dot badges on every tile in the year grid, and lists the lots and
products expiring on a selected day in a day-detail panel.

The Calendar tab MUST render the new `CalendarYearGrid.svelte`
sibling primitive as its default landing. The existing
`CalendarMonth.svelte` primitive is the source of truth for each
tile body and is consumed by the year-grid primitive unchanged so
the day grid, badge-dot contract, today ring, selection ring, and
weekend-muted styling are implemented once. The Calendar tab MUST
continue to source its expiration data from the existing
`list_dashboard_lots` Tauri command (no new backend command in
this slice); the day-bucket map MUST be computed client-side from
the returned active lots. Year navigation MUST NOT trigger a
refetch in this slice.

The Calendar tab MUST render a page-level header row above the
year grid containing: a previous-year chevron, a clickable
`viewYear` year-picker trigger, a next-year chevron, and a `Today` button. The
chevrons MUST be rendered as `‹ {viewYear} ›` (chevrons flanking
the year-picker trigger). The page-level header owns all year
navigation; the year-grid primitive knows nothing about it.
Clicking the `Today` button MUST update `viewYear` to the current
calendar year and `selectedDate` to today's ISO `YYYY-MM-DD` in
the same tick, and MUST NOT introduce a new event for the
primitive to handle (the existing `selectDate` flow on
`CalendarYearGrid.svelte` is reused for the `selectedDate`
update).

Clicking a day inside any tile MUST set `selectedDate` to that
day's ISO `YYYY-MM-DD` and update the day-detail panel that
lists every active lot whose `expiry_date` equals the selected
date, with columns for product, quantity, unit, store, location,
days remaining, and status. The year view MUST NOT collapse to a
single month on day click — the year grid stays open and the
day-detail panel updates in place. Clicking a lot row MUST open
the existing lot edit flow used elsewhere in the app. Clicking a
day with no expirations MUST still select it and show a `No
expirations on YYYY-MM-DD` placeholder.

When `viewYear` differs from the current calendar year, today's
ring MUST still render inside today's tile in the year grid so
the user can find the anchor without paging back.

The Calendar tab MUST expose the calendar primitive's keyboard
surface through the new year-grid primitive: roving tabindex per
tile (only the focused tile's grid owns `tabindex=0`), per-day
`role="gridcell"` inside a per-tile `role="grid"`, `Enter` /
`Space` on a focused day emits the ISO date and updates
`selectedDate`, and arrow keys traverse within a tile and wrap
to the previous / next month tile at column boundaries. Tab order
at the page level is: prev-year chevron → `viewYear` year-picker trigger →
next-year chevron → Today button → year grid (first focused
tile) → day-detail rows.

The year grid MUST fit the default page container (`1100 px`
content area) without horizontal scroll at the default
breakpoint. The `.cal-layout` flex wraps the year grid and the
day-detail panel; the flex direction is unchanged, and the
day-detail panel stays side-by-side with the grid inside the same
`.cal-layout` at the default breakpoint (the panel moves below
the grid only when the existing layout breakpoint collapses the
flex).

The calendar grid, day tiles, today ring, selection ring, badge
dots, and day-detail panel MUST render through the shared
DaisyUI themed primitives as defined in the visual surface
redesign and native control replacement capabilities. The
visual restyle MUST NOT change any domain logic carried by
`CalendarMonth.svelte` (year picker, decade navigation, badge
dots, out-of-range guards) and MUST NOT remove any keyboard
shortcut.

The i18n surface MUST add the following keys under `calendar.*`
in both `src/i18n/en/index.ts` and `src/i18n/es/index.ts`, and
`src/i18n/i18n-types.ts` MUST be regenerated so the keys remain
type-checked:
- `annualHeaderLabel` ("Calendar year {year}" / "Año {year}").
- `jumpToToday` ("Today" / "Hoy") (or re-use `calendar.today` if
  the wording matches; re-use is preferred to avoid drift).
- `tileMonthLabel` ("{month} {year}") — accessible name for each
  tile header, mirroring the existing `ariaMonth` pattern from
  `CalendarMonth.svelte`.

(Previously, first occurrence: the requirement governed the
calendar's data source, the keyboard surface, the day-detail
panel columns, the lot row edit flow, and the inherited keyboard
surface from `CalendarMonth.svelte`. It then pinned the month
label to the `ariaOpenMonthPicker` trigger and retired
`ariaCycleMonth`. It did not yet open in annual view; the
default landing primitive was `<CalendarMonth />` rendering a
single month. It did not yet describe a page-level year header,
year chevrons, a `Today` button, today's-anchor-across-years, or
the year-grid keyboard surface.)

(Previously, second occurrence: the requirement additionally
pinned the day grid, day cells, today ring, selection ring, badge
dots, month picker overlay, year picker overlay, and day-detail
panel to the shared DaisyUI themed primitives. It did not yet
open in annual view; the default landing primitive was
`<CalendarMonth />` rendering a single month. It did not yet
describe a page-level year header, year chevrons, a `Today`
button, today's-anchor-across-years, or the year-grid keyboard
surface.)

#### Scenario: opens at the current year in annual view with today highlighted and selected

- GIVEN the user clicks the Calendar tab in main navigation
- WHEN the page renders
- THEN the calendar displays a 3-column × 4-row grid of 12
  mini-month tiles for `viewYear` derived from `new Date()`
- AND today is visually highlighted inside its tile with the
  today ring (distinct from the selected highlight)
- AND today is the selected date (`selectedDate = today`)
- AND the year-grid primitive is the rendered primitive (the
  page does NOT render a single-month `<CalendarMonth />` as its
  default landing)

#### Scenario: page-level year header renders as ‹ {viewYear} › with Today button

- GIVEN the Calendar tab is rendered
- WHEN the page-level header row above the year grid is shown
- THEN the row contains a previous-year chevron, a clickable
  `viewYear` year-picker trigger, a next-year chevron, and a `Today` button
- AND the chevrons flank the trigger as `‹ {viewYear} ›`
- AND clicking the `viewYear` trigger opens a decade/year picker overlay
  modeled on the existing `CalendarMonth.svelte` year picker
- AND selecting a year from that overlay updates `viewYear`, closes the
  overlay, and does not change `selectedDate`
- AND the accessible name of the header resolves to
  `$LL.calendar.annualHeaderLabel({ year: viewYear })`
- AND the `Today` button's accessible name resolves to
  `$LL.calendar.jumpToToday()` (or the re-used `calendar.today`
  key, whichever matches)

#### Scenario: prev-year chevron decrements viewYear without refetch

- GIVEN the Calendar tab is rendered
- AND `viewYear` is currently `Y`
- WHEN the user clicks the previous-year chevron
- THEN `viewYear` becomes `Y - 1`
- AND the year grid re-renders the 12 tiles for year `Y - 1`
- AND no `list_dashboard_lots` refetch occurs
- AND `selectedDate` is unchanged unless the new year still
  contains a valid day for the same ISO `YYYY-MM-DD`

#### Scenario: next-year chevron increments viewYear without refetch

- GIVEN the Calendar tab is rendered
- AND `viewYear` is currently `Y`
- WHEN the user clicks the next-year chevron
- THEN `viewYear` becomes `Y + 1`
- AND the year grid re-renders the 12 tiles for year `Y + 1`
- AND no `list_dashboard_lots` refetch occurs
- AND `selectedDate` is unchanged unless the new year still
  contains a valid day for the same ISO `YYYY-MM-DD`

#### Scenario: Today button resets viewYear to current year and selectedDate to today

- GIVEN the user has navigated the Calendar tab to a year
  `viewYear = Y'` that is NOT the current calendar year
- AND the user has selected a day `D'` that is NOT today
- WHEN the user clicks the `Today` button in the page-level
  header
- THEN `viewYear` becomes the current calendar year
- AND `selectedDate` becomes today's ISO `YYYY-MM-DD`
- AND the year grid re-renders the 12 tiles for the current
  year
- AND the day-detail panel updates in place to list the lots
  expiring on today
- AND no new backend command is dispatched

#### Scenario: per-day expirations visible as dot badges on every tile

- GIVEN active lots exist in the local database with various
  `expiry_date` values
- WHEN the Calendar tab renders the year grid
- THEN each day cell inside every tile that has at least one
  active lot displays a small dot under the day number
- AND the dot corresponds to the count of active lots whose
  `expiry_date` equals that day
- AND the count is also shown in the day-detail panel for the
  selected date
- AND dot badges come from the same `dayBadges` map consumed by
  `CalendarMonth.svelte` in the date picker popover

#### Scenario: clicking a day stays in annual view and updates the day-detail panel

- GIVEN the Calendar tab is rendered in annual view
- WHEN the user clicks a day inside any tile
- THEN `selectedDate` becomes that day's ISO `YYYY-MM-DD`
- AND the year grid stays open (the page does NOT collapse to a
  single-month view)
- AND the existing day-detail panel updates in place to list
  the active lots whose `expiry_date` equals the new
  `selectedDate`
- AND if no active lot has an `expiry_date` equal to the new
  `selectedDate`, the panel shows `No expirations on YYYY-MM-DD`
- AND the year header's `viewYear` is unchanged

#### Scenario: today's ring is visible in any year's tile

- GIVEN the user has navigated the Calendar tab to a year
  `viewYear = Y'` that is NOT the current calendar year
- WHEN the year grid renders for `Y'`
- THEN today's tile inside `Y'` still highlights today's cell
  with the today ring
- AND the today ring renders identically to the today ring in
  the current year's tile (no visual de-emphasis)
- AND the `Today` button in the page-level header remains
  available as the explicit reset path

#### Scenario: year grid fits the 1100-px container without horizontal scroll

- GIVEN the page container is rendered at the default
  breakpoint with a `1100 px` content area
- WHEN the Calendar tab is rendered in annual view
- THEN the 3-column × 4-row year grid plus gutters and the
  page-level header fit inside the container without horizontal
  scroll
- AND no horizontal scrollbar is shown on the page
- AND the day-detail panel remains visible in the existing
  `.cal-layout` flex (side-by-side at the default breakpoint)

#### Scenario: keyboard surface inherited through the year-grid primitive

- GIVEN the Calendar tab is rendered in annual view
- WHEN the user navigates with Tab and arrow keys
- THEN focus traverses prev-year chevron → `viewYear` year-picker trigger →
  next-year chevron → Today button → year grid (the first
  focused tile's grid owns `tabindex=0`) → day-detail rows
- AND only one tile's grid owns `tabindex=0` at any time
  (roving tabindex per tile); the other 11 tiles' grids have
  `tabindex="-1"`
- AND `Enter` / `Space` on a focused day cell emits the ISO
  date and updates `selectedDate`
- AND arrow keys move focus within a tile and wrap to the
  previous / next month tile at column boundaries (see the
  cross-month arrow rule on the new `Calendar annual view`
  requirement)

#### Scenario: calendar surfaces render through the shared primitives after migration

- GIVEN the visual migration of `CalendarPage.svelte`,
  `CalendarYearGrid.svelte`, and `CalendarMonth.svelte` has
  landed
- WHEN the Calendar tab is rendered in annual view
- THEN the year grid, day tiles, today ring, selection ring,
  badge dots, page-level year header, and day-detail panel
  render through the shared DaisyUI themed primitives
- AND every scenario above continues to pass without
  modification
- AND the year picker, decade navigation, badge dot count,
  out-of-range guards carried by `CalendarMonth.svelte` (used
  inside each tile) continue to work exactly as before
- AND `DatePicker.svelte` consumers continue to render a
  single-month `<CalendarMonth />` without regression

## ADDED Requirements

### Requirement: Calendar annual view

`CalendarYearGrid.svelte` MUST render a 3-column × 4-row CSS
grid of 12 mini-month tiles for a `viewYear` prop, so the
Calendar tab can offer a yearly overview as its default landing
without losing the day-grid, badge-dot, today-ring, and
selection-ring contracts already implemented in
`CalendarMonth.svelte`.

`CalendarYearGrid.svelte` MUST consume the same `dayBadges` map
that `CalendarMonth.svelte` consumes today — the dot badges,
today ring, selection ring, weekend muted, and 6-week day grid
MUST render through the same per-tile mini-month layout. The
year-grid primitive MUST NOT introduce its own day-bucket
computation; the `dayBadges` map is passed in by
`CalendarPage.svelte` and rendered identically across tiles.

Each tile MUST be a self-contained month rendering. The tile
header MUST show the localized month name and the year for that
tile (via the existing `MONTH_NAMES` array and the new
`$LL.calendar.tileMonthLabel({ month, year })` accessible name).
The tile header MUST be read-only in this slice — clicking the
tile header MUST NOT drill down to a single-month view, and MUST
NOT dispatch any event. Clicking the tile body MUST still emit
the per-day `selectDate: string` event as today.

The year-grid primitive MUST expose a single event:
`selectDate: string` (ISO `YYYY-MM-DD`). Clicking a day inside
any tile MUST dispatch exactly one `selectDate` event. The
primitive MUST NOT collapse to a single month on day click and
MUST NOT introduce any new event for the page-level year header
or Today button — those are owned by `CalendarPage.svelte`.

Today's ring MUST render inside today's tile in any year the
year grid renders, regardless of whether `viewYear` matches the
current calendar year. The today ring MUST be visually identical
to the today ring inside `CalendarMonth.svelte` (same class
contract; no de-emphasis when `viewYear` differs from the
current year).

The keyboard surface on the year grid MUST be:

- The year grid contains 12 tile-level `role="grid"` regions,
  one per tile, each containing 42 day-cell `role="gridcell"`
  buttons.
- Roving tabindex per tile: at any moment, exactly one tile's
  grid owns `tabindex="0"` and the other 11 tiles' grids own
  `tabindex="-1"`. `Tab` moves focus between the year grid and
  the surrounding page-level controls; within the year grid,
  arrow keys (not `Tab`) move focus between tiles.
- `Enter` / `Space` on a focused day cell dispatches
  `selectDate` with that day's ISO `YYYY-MM-DD`.
- `←` from column 0 of a tile moves focus to column 6 of the
  previous month tile.
- `→` from column 6 of a tile moves focus to column 0 of the
  next month tile.
- Arrow keys MUST NOT cross year boundaries inside the year
  grid; cross-year navigation is the page-level chevrons' job
  only.
- `↑` / `↓` move focus within the same column of the same week
  row, wrapping to the previous / next month tile when the
  week crosses a month boundary (a focused day in the first
  week of February at column 3 wraps to the last week of
  January at column 3 when `↑` is pressed, and to the second
  week of February at column 3 when `↓` is pressed).
- `Esc` blurs the focused day cell and returns focus to the
  page-level header (no event is dispatched).
- `PageUp` / `PageDown` and `Shift+PageUp` / `Shift+PageDown`
  are NOT exposed by the year-grid primitive (the page-level
  chevrons own year navigation; the primitive is year-scoped).

The year-grid primitive MUST NOT expose:

- a per-tile month picker overlay,
- a per-tile year picker overlay,
- a Month / Year view toggle,
- any new backend Tauri command,
- any change to the dispatch surface of `monthChange`,
  `viewYearChange`, `ariaOpenMonthPicker`, or `ariaOpenYearPicker`.

The year-grid primitive MUST fit the default page container
(`1100 px` content area) inside the existing `.cal-layout`
without horizontal scroll at the default breakpoint. The grid
MUST be implemented with CSS Grid (`grid-template-columns:
repeat(3, 1fr)` and four rows of three tiles); the tile size
MUST be sized so the grid plus gutters stays inside the
container at the default page width.

The year-grid primitive MUST apply identically regardless of
which `viewYear` is passed in — there is no "viewYear equals
current year" branch in the primitive. The
"today's-anchor-across-years" behavior is a primitive-level
invariant: today's ring renders whenever the current calendar
date falls inside a tile of the rendered year.

`DatePicker.svelte` consumers MUST continue to render a
single-month `<CalendarMonth />`; the year-grid primitive is
consumed ONLY by the Calendar tab's `CalendarPage.svelte` in
this slice. `CalendarMonth.svelte` is NOT consumed by
`CalendarYearGrid.svelte` as a Svelte component import; the
year-grid primitive renders its own 12 mini-month bodies
inline, reusing the same visual contracts (badge dots, today
ring, selection ring, weekend muted, 6-week day grid) so the
two primitives look like siblings, not like nested
components.

#### Scenario: year grid renders 12 mini-month tiles in a 3×4 grid

- GIVEN `CalendarYearGrid.svelte` is rendered with `viewYear =
  Y` and `dayBadges = M` (a `Record<string, number>` mapping
  ISO `YYYY-MM-DD` to lot count)
- WHEN the year grid mounts
- THEN 12 tiles are rendered in a 3-column × 4-row CSS grid
- AND each tile shows the localized month name from
  `MONTH_NAMES` and the year `Y` in its header
- AND each tile body is a 7-column weekday row + 6-week day
  grid
- AND each tile's day cells draw dot badges from `M` using the
  same contract as `CalendarMonth.svelte`

#### Scenario: clicking a day inside any tile dispatches a single selectDate event

- GIVEN the year grid is rendered with `viewYear = Y`
- WHEN the user clicks a day `D` inside any tile
- THEN a single `selectDate: string` event is dispatched with
  payload `YYYY-MM-DD` (the ISO for `D`)
- AND the year grid stays open (it does NOT collapse to a
  single-month view)
- AND `selectedDate` in `CalendarPage.svelte` becomes `D`'s
  ISO date via the existing `selectDate` handler
- AND the day-detail panel updates in place to list the lots
  expiring on `D`

#### Scenario: today's ring renders in any year's tile

- GIVEN the year grid is rendered with `viewYear = Y'`
- WHEN `Y' !== currentYear` (a non-current calendar year)
- THEN today's tile inside `Y'` highlights today's cell with
  the today ring
- AND today's tile inside `Y'` is visually identical to
  today's tile inside the current year (no de-emphasis)
- AND the today ring class contract matches
  `CalendarMonth.svelte`'s today ring exactly

#### Scenario: tile header is read-only and does not drill down

- GIVEN the year grid is rendered
- WHEN the user clicks the tile header for month `M` in
  `viewYear`
- THEN no event is dispatched
- AND the year grid does NOT collapse to a single-month view
- AND the user remains on the year grid
- AND the tile header's accessible name resolves to
  `$LL.calendar.tileMonthLabel({ month: "Mayo", year: 2027 })`

#### Scenario: roving tabindex per tile (only one tile owns tabindex=0)

- GIVEN the year grid is rendered
- WHEN the user first tabs into the year grid
- THEN exactly one tile's grid owns `tabindex="0"`
- AND the other 11 tiles' grids own `tabindex="-1"`
- AND `Tab` leaves the year grid (it does NOT cycle through
  every tile)
- AND arrow keys move focus between tiles while preserving
  the roving tabindex invariant (the tile that just received
  focus now owns `tabindex="0"` and the previous one is reset
  to `tabindex="-1"`)

#### Scenario: cross-month arrow wrapping at column boundaries

- GIVEN focus is on day `D` at column 0 (Sunday) of the
  February tile in `viewYear`
- WHEN the user presses `←`
- THEN focus moves to the day at column 6 (Saturday) of the
  January tile in `viewYear`
- AND no event is dispatched
- AND `viewYear` is unchanged
- GIVEN focus is on day `D` at column 6 (Saturday) of the
  November tile in `viewYear`
- WHEN the user presses `→`
- THEN focus moves to the day at column 0 (Sunday) of the
  December tile in `viewYear`
- AND no event is dispatched
- AND `viewYear` is unchanged
- GIVEN focus is on a day in the first week row of February
  (column 3) and the previous month tile (January) does NOT
  have a day at the same row/column
- WHEN the user presses `↑`
- THEN focus wraps to the previous month tile's last week row
  at column 3 (a January day)
- GIVEN focus is on a day in the last week row of January
  (column 3) and the next month tile (February) does NOT have
  a day at the same row/column
- WHEN the user presses `↓`
- THEN focus wraps to the next month tile's second week row at
  column 3 (a February day)

#### Scenario: Esc returns focus to the page-level header

- GIVEN focus is on a day cell inside the year grid
- WHEN the user presses `Esc`
- THEN focus moves to the page-level header (specifically, the
  `viewYear` label or the prev-year chevron, whichever was
  last focused before the year grid took focus)
- AND no `selectDate` event is dispatched
- AND `viewYear` and `selectedDate` are unchanged

#### Scenario: arrow keys never cross year boundaries inside the year grid

- GIVEN focus is on a day inside the January tile of
  `viewYear = Y`
- WHEN the user presses `←` from column 0
- THEN focus moves to the December tile of the same
  `viewYear = Y` (NOT to December of `viewYear = Y - 1`)
- AND no event is dispatched
- AND `viewYear` is unchanged
- GIVEN focus is on a day inside the December tile of
  `viewYear = Y`
- WHEN the user presses `→` from column 6
- THEN focus moves to the January tile of the same
  `viewYear = Y` (NOT to January of `viewYear = Y + 1`)
- AND no event is dispatched
- AND `viewYear` is unchanged
- AND the page-level prev/next-year chevrons remain the only
  year-boundary navigation path

#### Scenario: year grid fits the 1100-px container without horizontal scroll

- GIVEN the page container is rendered at the default
  breakpoint with a `1100 px` content area
- WHEN `CalendarYearGrid.svelte` mounts inside
  `.cal-grid-wrapper`
- THEN the 3-column × 4-row tile grid plus gutters fits
  inside the container without horizontal scroll
- AND the implementation uses CSS Grid
  (`grid-template-columns: repeat(3, 1fr)`) with sizing that
  fits the default page width
- AND no `overflow-x` is set on the year-grid root

#### Scenario: dot badges come from the same dayBadges map as CalendarMonth.svelte

- GIVEN `CalendarYearGrid.svelte` is rendered with `dayBadges =
  M` and `viewYear = Y`
- WHEN the year grid renders each tile
- THEN dot badges are drawn from `M` exactly the way
  `CalendarMonth.svelte` draws them from the same map
- AND a day `D` inside any tile shows a dot iff
  `M[YYYY-MM-DD] > 0` for `D`'s ISO date
- AND the dot count visual treatment matches
  `CalendarMonth.svelte`'s dot count visual treatment
- AND the year-grid primitive does NOT compute its own
  day-bucket map (it consumes the parent's `dayBadges` prop
  verbatim)

#### Scenario: DatePicker is not affected by CalendarYearGrid

- GIVEN `DatePicker.svelte` renders a single-month
  `<CalendarMonth />` as today
- WHEN the Calendar annual view change lands
- THEN `DatePicker.svelte` continues to render
  `<CalendarMonth />` (no `<CalendarYearGrid />` is imported
  or rendered)
- AND the popover positioning logic in `DatePicker.svelte` is
  NOT modified
- AND `DatePicker.svelte` does NOT need a new prop to opt out
  of the year grid
- AND the existing month picker / year picker / keyboard
  surface inside `CalendarMonth.svelte` (as consumed by
  `DatePicker.svelte`) continues to work exactly as before
