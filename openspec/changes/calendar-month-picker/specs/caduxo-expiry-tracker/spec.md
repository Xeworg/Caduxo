# Delta for `caduxo-expiry-tracker`

This delta modifies the `Calendar tab` requirement (both occurrences in
the canonical spec) and adds a new `Calendar month picker` requirement
that codifies the month-picker overlay behavior introduced in
`CalendarMonth.svelte`.

## MODIFIED Requirements

### Requirement: Calendar tab

The system MUST provide a top-level main-navigation Calendar tab
(added between Products and Reports) that opens the current month
with today highlighted and selected, displays per-day lot
expirations as dot badges, and lists the lots and products expiring
on a selected day in a day-detail panel.

The Calendar tab MUST reuse the same `CalendarMonth.svelte`
primitive used by the date picker popover so the day grid, year
picker, month picker, visual styling, and keyboard surface are
implemented once. The Calendar tab MUST source its expiration data
from the existing `list_dashboard_lots` Tauri command (no new
backend command in this slice); the day-bucket map MUST be computed
client-side from the returned active lots. Month navigation MUST
NOT trigger a refetch in this slice.

Clicking a day MUST set the selected date and reveal a day-detail
panel that lists every active lot whose `expiry_date` equals the
selected date, with columns for product, quantity, unit, store,
location, days remaining, and status. Clicking a lot row MUST open
the existing lot edit flow used elsewhere in the app. Clicking a
day with no expirations MUST still select it and show a `No
expirations on YYYY-MM-DD` placeholder.

The Calendar tab MUST inherit the full keyboard surface of the
calendar primitive (Tab, Esc, Enter, arrow keys, PageUp/Down,
Shift+PageUp/Shift+PageDown) with a tab order of prev-month
chevron → month label → year chip → day grid → next-month chevron
→ day-detail rows.

The month label inside the header MUST act as a **trigger for the
month picker** (click or `Enter`/`Space` opens the picker), not as a
button that advances the view by one month. Sequential month
navigation remains available through the prev/next-month chevrons
and the `PageUp` / `PageDown` keyboard shortcuts. The month
label's accessible name MUST use the `ariaOpenMonthPicker` i18n
key ("Open month picker" / "Abrir selector de mes").

(Previously, first occurrence: the requirement governed the
calendar's data source, the keyboard surface, the day-detail panel
columns, the lot row edit flow, and the inherited keyboard surface
from `CalendarMonth.svelte`. It did not yet reference a month
picker and used `ariaCycleMonth` for the month label's accessible
name.)

(Previously, second occurrence: the requirement additionally pinned
the day grid, day cells, today ring, selection ring, badge dots,
and day-detail panel to the shared DaisyUI themed primitives. It
did not yet reference a month picker and used `ariaCycleMonth` for
the month label's accessible name.)

#### Scenario: opens at current month with today highlighted and selected

- GIVEN the user clicks the Calendar tab in main navigation
- WHEN the page renders
- THEN the calendar grid displays the current month (`viewYear`
  and `viewMonth` derived from `new Date()`)
- AND today is visually highlighted (distinct from the selected
  highlight)
- AND today is the selected date (`selectedDate = today`)

#### Scenario: per-day expirations visible as dot badges

- GIVEN active lots exist in the local database with various
  `expiry_date` values
- WHEN the Calendar tab renders
- THEN each day cell with at least one active lot displays a small
  dot under the day number
- AND the dot corresponds to the count of active lots whose
  `expiry_date` equals that day
- AND the count is also shown in the day-detail panel for the
  selected date

#### Scenario: day-detail panel lists lots and products

- GIVEN a day is selected on the Calendar tab
- WHEN the day-detail panel renders
- THEN the panel lists every active lot whose `expiry_date` equals
  the selected date
- AND each row shows product, quantity, unit, store, location, days
  remaining, and status

#### Scenario: empty day shows the placeholder message

- GIVEN a day is selected on the Calendar tab
- AND no active lot has an `expiry_date` equal to that day
- WHEN the day-detail panel renders
- THEN the panel shows `No expirations on YYYY-MM-DD` (or an
  equivalent English placeholder that names the date)

#### Scenario: lot row opens the existing lot edit flow

- GIVEN the day-detail panel shows one or more lot rows
- WHEN the user clicks a lot row
- THEN the existing lot edit overlay opens for that lot (the same
  pattern used by `DashboardPage`)
- AND no new edit-flow primitive is introduced

#### Scenario: data source is the existing list_dashboard_lots command

- GIVEN the user opens the Calendar tab
- WHEN the page mounts
- THEN the page calls `list_dashboard_lots({ store_id: null,
  location_id: null, preset: null, urgency: null })` exactly once
- AND the day-bucket map is computed client-side from the returned
  active lots
- AND month navigation does NOT trigger a refetch in this slice

#### Scenario: keyboard surface inherited from the calendar primitive

- GIVEN the Calendar tab is rendered
- WHEN the user navigates with Tab, Esc, Enter, arrow keys, and
  PageUp/Down
- THEN focus traverses prev-month chevron → month label → year
  chip → day grid → next-month chevron → day-detail rows
- AND arrow keys / PageUp / PageDown / Shift+PageUp /
  Shift+PageDown move the focused day with the same clamping rules
  as the picker
- AND Enter on a focused day cell emits the ISO date and updates
  `selectedDate`
- AND `Enter` / `Space` on the month label opens the month picker
  overlay without changing `viewMonth`

#### Scenario: clicking the month label opens a month picker grid

- GIVEN the Calendar tab is rendered
- AND the month picker overlay is not currently open
- WHEN the user clicks the month label in the calendar header
- THEN a month picker overlay opens anchored under the header
- AND the overlay lists all 12 months in a 3-column × 4-row grid
- AND the chip for the currently viewed month is rendered with the
  selected visual treatment
- AND the year shown in the overlay's header matches `viewYear`
- AND no `monthChange` event has been dispatched yet

#### Scenario: calendar surfaces render through the shared primitives after migration

- GIVEN the visual migration of `CalendarPage.svelte` and
  `CalendarMonth.svelte` has landed
- WHEN the Calendar tab is rendered
- THEN the day grid, day cells, today ring, selection ring, badge
  dots, month picker overlay, year picker overlay, and day-detail
  panel render through the shared DaisyUI themed primitives
- AND every scenario above continues to pass without modification
- AND the year picker, decade navigation, badge dot count, and
  out-of-range guards carried by `CalendarMonth.svelte` continue to
  work exactly as before

## ADDED Requirements

### Requirement: Calendar month picker

`CalendarMonth.svelte` MUST expose a month picker overlay that
mirrors the existing year picker pattern, so users can jump to any
month in one click instead of clicking the next-month chevron
repeatedly.

The month picker MUST be triggered by the month label button in
the calendar header (click, `Enter`, or `Space`) and MUST render
anchored under the same header as the year picker. While the month
picker is open, the underlying day grid, weekday header, badge
dots, today highlight, and selection ring MUST remain rendered in
their previous state (they are not unmounted behind the overlay).

The month picker header MUST show the current `viewYear` as a
read-only label with a previous-year chevron and a next-year
chevron on either side. The two chevrons MUST adjust a local
`pickerYear` value used only inside the overlay and MUST NOT
dispatch `monthChange` or `viewYearChange`; clicking a month chip
is what commits a year change.

The month picker body MUST render a 3-column × 4-row grid of 12
month chips, one chip per calendar month (January through
December), labeled with the localized month name from the
existing `MONTH_NAMES` array.

A month chip for a month `m` in `pickerYear` MUST be considered
**in-range** when there exists at least one day in that
`(pickerYear, m)` pair whose ISO date falls within
`[minDate, maxDate]`, and **out-of-range** otherwise. Out-of-range
chips MUST render with the `month-disabled` visual treatment and
MUST be non-interactive (no click, no keyboard activation). The
month picker MUST NOT collapse, group, or hide a row that contains
disabled chips; the row layout is preserved and only individual
chips are dimmed.

The chip for the month equal to the currently viewed `viewMonth`
(in `viewYear`) MUST render with the `month-selected` visual
treatment. No other month in the grid is highlighted as selected,
including the current calendar month if it differs from
`viewMonth`.

Clicking an enabled month chip MUST dispatch a single `monthChange`
event with `{ year: pickerYear, month: m }` and MUST close the
month picker overlay. The picker MUST NOT stay open for a
follow-up selection; the click count is one.

The year chip button (the existing trigger that opens the year
picker) MUST remain clickable while the month picker is open and
MUST continue to open the year picker as before. Opening the year
picker while the month picker is open MUST close the month picker.
Opening the month picker while the year picker is open MUST close
the year picker. The two overlays MUST NEVER be open at the same
time.

The keyboard surface inside the month picker MUST be:
- `Enter` or `Space` on a focused, enabled month chip selects it
  and closes the overlay.
- `←` / `→` / `↑` / `↓` move focus through the 3-column × 4-row
  grid, wrapping according to the same row/column conventions as
  the day grid.
- `Esc` closes the overlay without changing `viewMonth` or
  `viewYear`.
- `Tab` returns focus to the day grid (the day grid remains in the
  tab order behind the overlay).

When no overlay is open, the existing keyboard surface MUST behave
exactly as before: `PageUp` / `PageDown` move by one month,
`Shift+PageUp` / `Shift+PageDown` move by one year, `←` / `→` /
`↑` / `↓` move by one day, `Enter` selects the focused day.

The month picker MUST NOT introduce a new backend Tauri command,
MUST NOT change the dispatch surface for `monthChange` or
`viewYearChange` (only the trigger that dispatches them changes), and
MUST NOT move the popover anchor logic in `DatePicker.svelte` (the
overlay sits inside the `CalendarMonth.svelte` header).

The month picker MUST apply identically in both consumers of
`CalendarMonth.svelte`: the Calendar tab (`CalendarPage.svelte`)
and the form-field popover (`DatePicker.svelte`). Behavior,
keyboard surface, visual classes, and dispatch semantics MUST NOT
differ between the two contexts.

The previous `cycleMonth()` behavior — clicking the month label
to advance the view by one month with year rollover at December —
MUST be retired completely. The label's only job is to open the
picker. Sequential month navigation remains available through the
prev/next-month chevrons and `PageUp` / `PageDown`.

The i18n surface MUST add the following keys under `calendar.*` in
both `src/i18n/en/index.ts` and `src/i18n/es/index.ts`, and
`src/i18n/i18n-types.ts` MUST be regenerated so the keys remain
type-checked:
- `ariaOpenMonthPicker` ("Open month picker" /
  "Abrir selector de mes") — replaces the previous
  `ariaCycleMonth` key, which MUST be removed.
- `ariaCloseMonthPicker` ("Close month picker" /
  "Cerrar selector de mes").
- `ariaPreviousYear` ("Previous year" / "Año anterior").
- `ariaNextYear` ("Next year" / "Año siguiente").
- `ariaMonth` ("{month} {year}") — accessible name for each month
  chip, reusing `MONTH_NAMES`.

`DatePicker.svelte` consumers MUST continue to receive `monthChange`
on selection and MUST NOT need any new prop to enable the month
picker.

#### Scenario: month label opens the month picker overlay

- GIVEN any instance of `CalendarMonth.svelte` (Calendar tab or
  `DatePicker` popover)
- AND the month picker overlay is not currently open
- WHEN the user clicks the month label
- THEN the month picker overlay becomes visible
- AND the overlay header shows `viewYear`
- AND the overlay body renders 12 month chips arranged in a
  3-column × 4-row grid
- AND the chip for the current `viewMonth` carries the
  `month-selected` visual treatment
- AND `viewMonth`, `viewYear`, and `selectedDate` are unchanged

#### Scenario: clicking a month chip selects and closes

- GIVEN the month picker overlay is open
- WHEN the user clicks the chip for month `m` in the currently
  displayed `pickerYear`
- THEN a single `monthChange` event is dispatched with
  `{ year: pickerYear, month: m }`
- AND the month picker overlay closes
- AND the calendar header label and the day grid both reflect the
  new `(year, month)`
- AND the day-detail panel (Calendar tab) or popover preview
  (`DatePicker`) is updated by the consumer's existing
  `monthChange` handler

#### Scenario: Esc closes without changing the view

- GIVEN the month picker overlay is open
- WHEN the user presses `Esc`
- THEN the overlay closes
- AND no `monthChange` event is dispatched
- AND `viewMonth`, `viewYear`, and `selectedDate` are unchanged

#### Scenario: prev/next-year chevrons update only the overlay

- GIVEN the month picker overlay is open
- AND `pickerYear` is currently `Y`
- WHEN the user clicks the previous-year chevron
- THEN `pickerYear` becomes `Y - 1`
- AND no `monthChange` or `viewYearChange` event is dispatched
- AND the calendar header and day grid still show year `Y`
- AND the next click on a month chip dispatches
  `monthChange { year: Y - 1, month }`

#### Scenario: month picker and year picker are mutually exclusive

- GIVEN the month picker overlay is open
- WHEN the user clicks the year chip
- THEN the month picker overlay closes
- AND the year picker overlay opens anchored at the same header

- GIVEN the year picker overlay is open
- WHEN the user clicks the month label
- THEN the year picker overlay closes
- AND the month picker overlay opens anchored at the same header

- GIVEN neither overlay is open
- WHEN the user opens either overlay
- THEN only that overlay is rendered
- AND the other overlay is not rendered at the same time

#### Scenario: year chip stays clickable while month picker is open

- GIVEN the month picker overlay is open
- WHEN the user clicks the year chip button
- THEN the year picker overlay opens
- AND the month picker overlay closes
- AND the consumer's existing `viewYearChange` behavior continues
  to apply once a year chip is selected

#### Scenario: out-of-range months render as disabled chips

- GIVEN the calendar is configured with a `minDate` and `maxDate`
  tighter than the default range (for example, restricted to a
  single calendar year)
- WHEN the month picker overlay opens
- THEN the chips for months outside the effective `[minDate,
  maxDate]` range carry the `month-disabled` visual treatment
- AND clicking a disabled chip does NOT dispatch `monthChange` and
  does NOT close the overlay
- AND enabled chips in the same row remain clickable and laid out
  in the same 3-column grid (no row collapse or grouping)

#### Scenario: only the viewed month is highlighted

- GIVEN `viewMonth` is `M` and `viewYear` is `Y`
- WHEN the month picker overlay opens
- THEN exactly one chip carries the `month-selected` visual
  treatment — the chip for month `M` in year `Y`
- AND no other chip is rendered as `selected`, including the chip
  for the current calendar month if `M` is not the current
  calendar month

#### Scenario: keyboard opens the month picker and navigates the grid

- GIVEN focus is on the month label in the calendar header
- WHEN the user presses `Enter` or `Space`
- THEN the month picker overlay opens
- AND focus moves to the chip for the currently viewed month

- GIVEN the month picker overlay is open and focus is on a chip
- WHEN the user presses `←` / `→` / `↑` / `↓`
- THEN focus moves through the 3-column × 4-row grid accordingly
- AND when focus moves to a disabled chip, the disabled chip is
  not activated

- GIVEN the month picker overlay is open
- WHEN the user presses `Enter` or `Space` on a focused, enabled
  chip
- THEN the chip is selected, `monthChange` is dispatched, and the
  overlay closes

#### Scenario: sequential chevron and PageUp/Down navigation is unchanged

- GIVEN the month picker overlay is closed
- WHEN the user clicks the prev-month chevron, the next-month
  chevron, presses `PageUp`, or presses `PageDown`
- THEN `monthChange` is dispatched with the new `(year, month)`
  exactly as before this change
- AND the month label is NOT activated as a side effect

#### Scenario: cycleMonth behavior is retired

- GIVEN any instance of `CalendarMonth.svelte`
- WHEN the user clicks, `Enter`s, or `Space`s on the month label
- THEN the result is opening the month picker overlay
- AND `monthChange` is NOT dispatched as a side effect of opening
  the picker
- AND no hidden gesture (for example `Shift+click`) re-enables the
  retired cycle-by-one behavior

#### Scenario: i18n surface carries the renamed and new keys after migration

- GIVEN the i18n migration of `src/i18n/en/index.ts` and
  `src/i18n/es/index.ts` has landed
- AND `src/i18n/i18n-types.ts` has been regenerated
- WHEN the calendar primitive is rendered
- THEN the month label's accessible name resolves to
  `$LL.calendar.ariaOpenMonthPicker()` ("Open month picker" /
  "Abrir selector de mes")
- AND `$LL.calendar.ariaCloseMonthPicker()`,
  `$LL.calendar.ariaPreviousYear()`, and
  `$LL.calendar.ariaNextYear()` resolve to their expected English
  and Spanish strings
- AND each month chip's accessible name resolves to
  `$LL.calendar.ariaMonth({ month, year })` using the localized
  month name
- AND `$LL.calendar.ariaCycleMonth` is no longer referenced from
  `CalendarMonth.svelte`

#### Scenario: shared behavior between Calendar tab and DatePicker

- GIVEN both `CalendarPage.svelte` (Calendar tab) and
  `DatePicker.svelte` (form-field popover) render an instance of
  `CalendarMonth.svelte`
- WHEN the user triggers the month picker in either context
- THEN the overlay, keyboard surface, visual classes, disabled
  rules, mutual-exclusion rule, and `monthChange` dispatch are
  identical in both contexts
- AND `DatePicker.svelte` does NOT introduce a new prop to enable
  the picker
- AND the popover positioning logic in `DatePicker.svelte` is not
  modified