# Delta for caduxo-expiry-tracker

This delta introduces an in-house custom date picker (Svelte component) that replaces every native `<input type="date">` in the app, plus a new top-level Calendar tab that reuses the same calendar grid primitive. The new picker contract lives under a new `Capability: Date input`; the new page-level capability is `Capability: Calendar`. Two existing requirements (`Expiry lots > lot registration` and `Reports > report filters`) gain a one-line pointer that references the new picker contract.

## ADDED Requirements

### Requirement: Custom date picker

The system MUST provide an in-house Svelte custom date picker (no native `<input type="date">`) for every expiry date and report date-range field, and the picker MUST be the single source of truth for date input across the app.

The picker MUST bind ISO `YYYY-MM-DD` strings in and out, MUST reject any year outside the inclusive range `[1900-01-01, 2100-12-31]`, and MUST support both a typed-text manual entry path and a calendar popover path that share the same bound value. The picker MUST render no `<input type="date">` element in the DOM.

The picker MUST expose a `clearable` prop (default `true`); when `clearable={false}` the picker MUST NOT render any clear affordance and MUST NOT silently commit an empty value. The host (`LotForm` for the required expiry date) is responsible for the `required` JS guard; the picker preserves the bound value on empty input when `clearable={false}` and surfaces a visual invalid state instead.

The picker MUST support basic accessible keyboard ergonomics: Tab enters and leaves the trigger without trapping focus inside the popover; Escape closes the popover without committing a calendar selection and without rewriting typed text; Enter on a focused day cell emits that day's ISO date and closes the popover. Arrow keys MUST move the focused day by ±1 (Left/Right) or ±7 (Up/Down) days; PageUp/PageDown MUST move by ±1 month; Shift+PageUp/Shift+PageDown MUST move by ±1 year; every move MUST be clamped to the configured year range.

Manual `YYYY-MM-DD` text input MUST visually validate on blur (red border + helper text for invalid shape, invalid calendar date, or out-of-range year); the bound value MUST NOT update while the text is invalid and the typed text MUST remain visible in the input element (no silent rewrite).

#### Scenario: picker is in-house Svelte, not native

- GIVEN any date field in the app (LotForm expiry date, Reports `dateFrom`, Reports `dateTo`)
- WHEN `grep -R 'type="date"' src/` is executed after this slice lands
- THEN the command returns zero matches
- AND each field is rendered by the in-house `DatePicker.svelte` Svelte component composed around `CalendarMonth.svelte`

#### Scenario: ISO YYYY-MM-DD in/out with year range enforcement

- GIVEN the user opens the picker on any date field
- WHEN the user selects a day in any month of any year between 1900 and 2100 inclusive (via the calendar popover or manual entry)
- THEN the picker emits the selected day as an ISO `YYYY-MM-DD` string
- AND the bound value is updated to that string
- AND the text input displays the selected ISO date

#### Scenario: selection closes the popover and commits

- GIVEN the picker popover is open and the user is focused on a day cell
- WHEN the user clicks or presses Enter on a day cell
- THEN the popover closes immediately
- AND the selected `YYYY-MM-DD` is emitted and committed to the host
- AND the text input displays the selected ISO date

#### Scenario: manual YYYY-MM-DD text input validates without silent rewrite

- GIVEN the picker text input is focused
- WHEN the user types any of `2026-13-40`, `2026-02-30`, `1899-12-31`, or `2101-01-01`
- AND blurs the input
- THEN the input displays a visual invalid state (red border + helper text)
- AND the bound value is NOT updated
- AND the typed text remains visible in the input element (no silent rewrite)

#### Scenario: clearable=false hides the clear icon and prevents silent empty commit

- GIVEN the picker is mounted with `clearable={false}` (LotForm expiry date)
- WHEN the picker is rendered
- THEN no `×` clear icon is present in the DOM
- AND the user cannot clear the bound value through the picker
- WHEN the user deletes the typed text down to empty and blurs
- THEN the picker shows a visual invalid state
- AND the bound value is NOT silently committed to `""`

#### Scenario: clearable=true exposes the clear icon

- GIVEN the picker is mounted with the default `clearable={true}` (Reports `dateFrom`, `dateTo`)
- AND the bound value is non-empty
- WHEN the picker is rendered
- THEN a `×` clear icon is visible in the trigger
- AND clicking `×` clears the bound value to `""` and closes the popover

#### Scenario: basic accessible keyboard ergonomics

- GIVEN the picker trigger is focused
- WHEN the user presses Tab
- THEN focus moves through the trigger and the calendar-icon button in source order
- WHEN the user presses Tab again
- THEN focus leaves the picker (no focus trap inside the popover)
- GIVEN the popover is open
- WHEN the user presses Escape
- THEN the popover closes without committing a calendar selection
- AND any typed text remains visible (still subject to blur-validation rules)
- GIVEN a day cell is focused inside the popover
- WHEN the user presses Enter
- THEN that day's ISO date is emitted and the popover closes
- AND ArrowLeft/ArrowRight move the focused day by ±1 day
- AND ArrowUp/ArrowDown move the focused day by ±7 days
- AND PageUp/PageDown move the focused day by ±1 month
- AND Shift+PageUp/Shift+PageDown move the focused day by ±1 year
- AND every move is clamped to `[1900-01-01, 2100-12-31]`

#### Scenario: Today shortcut inside popover

- GIVEN the picker popover is open
- AND today is within `[minDate, maxDate]`
- WHEN the user clicks the `Today` button in the popover footer
- THEN the bound value is set to today's ISO `YYYY-MM-DD`
- AND the popover closes

### Requirement: Calendar tab

The system MUST provide a top-level main-navigation Calendar tab (added between Products and Reports) that opens the current month with today highlighted and selected, displays per-day lot expirations as dot badges, and lists the lots and products expiring on a selected day in a day-detail panel.

The Calendar tab MUST reuse the same `CalendarMonth.svelte` primitive used by the date picker popover so the day grid, year picker, visual styling, and keyboard surface are implemented once. The Calendar tab MUST source its expiration data from the existing `list_dashboard_lots` Tauri command (no new backend command in this slice); the day-bucket map MUST be computed client-side from the returned active lots. Month navigation MUST NOT trigger a refetch in this slice.

Clicking a day MUST set the selected date and reveal a day-detail panel that lists every active lot whose `expiry_date` equals the selected date, with columns for product, quantity, unit, store, location, days remaining, and status. Clicking a lot row MUST open the existing lot edit flow used elsewhere in the app. Clicking a day with no expirations MUST still select it and show a `No expirations on YYYY-MM-DD` placeholder.

The Calendar tab MUST inherit the full keyboard surface of the calendar primitive (Tab, Esc, Enter, arrow keys, PageUp/Down, Shift+PageUp/Shift+PageDown) with a tab order of prev-month chevron → month label → year chip → day grid → next-month chevron → day-detail rows.

#### Scenario: opens at current month with today highlighted and selected

- GIVEN the user clicks the Calendar tab in main navigation
- WHEN the page renders
- THEN the calendar grid displays the current month (`viewYear` and `viewMonth` derived from `new Date()`)
- AND today is visually highlighted (distinct from the selected highlight)
- AND today is the selected date (`selectedDate = today`)

#### Scenario: per-day expirations visible as dot badges

- GIVEN active lots exist in the local database with various `expiry_date` values
- WHEN the Calendar tab renders
- THEN each day cell with at least one active lot displays a small dot under the day number
- AND the dot corresponds to the count of active lots whose `expiry_date` equals that day
- AND the count is also shown in the day-detail panel for the selected date

#### Scenario: day-detail panel lists lots and products

- GIVEN a day is selected on the Calendar tab
- WHEN the day-detail panel renders
- THEN the panel lists every active lot whose `expiry_date` equals the selected date
- AND each row shows product, quantity, unit, store, location, days remaining, and status

#### Scenario: empty day shows the placeholder message

- GIVEN a day is selected on the Calendar tab
- AND no active lot has an `expiry_date` equal to that day
- WHEN the day-detail panel renders
- THEN the panel shows `No expirations on YYYY-MM-DD` (or an equivalent English placeholder that names the date)

#### Scenario: lot row opens the existing lot edit flow

- GIVEN the day-detail panel shows one or more lot rows
- WHEN the user clicks a lot row
- THEN the existing lot edit overlay opens for that lot (the same pattern used by `DashboardPage`)
- AND no new edit-flow primitive is introduced

#### Scenario: data source is the existing list_dashboard_lots command

- GIVEN the user opens the Calendar tab
- WHEN the page mounts
- THEN the page calls `list_dashboard_lots({ store_id: null, location_id: null, preset: null, urgency: null })` exactly once
- AND the day-bucket map is computed client-side from the returned active lots
- AND month navigation does NOT trigger a refetch in this slice

#### Scenario: keyboard surface inherited from the calendar primitive

- GIVEN the Calendar tab is rendered
- WHEN the user navigates with Tab, Esc, Enter, arrow keys, and PageUp/Down
- THEN focus traverses prev-month chevron → month label → year chip → day grid → next-month chevron → day-detail rows
- AND arrow keys / PageUp / PageDown / Shift+PageUp / Shift+PageDown move the focused day with the same clamping rules as the picker
- AND Enter on a focused day cell emits the ISO date and updates `selectedDate`

## MODIFIED Requirements

### Requirement: lot registration

Caduxo shall allow users to register expiry lots linked to a product.

Required lot fields:

- product
- quantity
- unit — the resolved free-text label displayed on the lot. When the product references a catalog unit via `default_unit_id`, the value SHALL be the catalog unit's `display_name`. When no catalog unit is set, the value SHALL be the legacy `products.default_unit` text, or, if that is empty, the seeded `units` preset's `display_name` (`Unidades`). The lot form SHALL render the `unit` field as a read-only chip when the product has a catalog unit, and SHALL keep it editable for legacy lots whose product has `unit_type = null`.
- expiry date — uses the `Date input` picker with `clearable={false}`. The host `required` JS guard (`if (!expiryDate) errorMsg = "Expiry date is required";`) remains the source of truth for blocking empty submits. No `<input type="date">` is rendered for this field.
- alert days before expiry

Conditionally required:

- store/local when multiple stores exist

Optional:

- internal location
- batch/lot code
- notes

(Previously: the `unit` field was listed as a free-text label with no note about catalog-driven resolution or about how the value falls back when no catalog unit is selected. This delta additionally adds a pointer noting that the `expiry date` field uses the `Date input` picker with `clearable={false}` and that no native `<input type="date">` is rendered for this field.)

### Requirement: report filters

Reports shall support filters for:

- store/local
- internal location
- category
- status
- date range — the `dateFrom` and `dateTo` fields use the `Date input` picker (clearable, optional). Empty values are translated to `null` by the existing `dateFrom.trim() || null` / `dateTo.trim() || null` chain in `ReportsPage.buildFilters()`. No `<input type="date">` is rendered for these fields.

(Previously: this delta adds a pointer noting that the `dateFrom` and `dateTo` fields use the `Date input` picker (clearable, optional) and that no native `<input type="date">` is rendered for these fields.)
