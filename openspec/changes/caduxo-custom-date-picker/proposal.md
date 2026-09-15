# Proposal: caduxo-custom-date-picker

> **Naming note.** The change directory is `caduxo-custom-date-picker` because the
> original defect fix was a date picker; the user-confirmed scope has since
> grown to include a main-navigation Calendar tab/page that reuses the same
> calendar grid logic. The change name is kept as the user instructed
> ("Same SDD change"), but the slice is now materially larger than a picker
> replacement. See `## Review workload decision` below for the explicit
> user-approved 3,000-line review budget for this local solo-review slice.

## Research note: Svelte Playground day-grid reference

The user pointed at a Svelte Playground calendar example (`version=5.57.0`) and clarified that this is the intended reference: a **day-only month grid**, not a complex event/hour scheduler and not third-party code to vendor.

The useful part for Caduxo is the day-grid idea: caller-owned date math builds the month days, and a presentational calendar component renders those days and emits day/header events. Caduxo should keep that separation, but implement its own small component tailored to expiry dates.

This proposal should use the Playground snippet as product/design inspiration only:

- useful pattern: one reusable grid component receives normalized days and emits day events;
- keep for Caduxo: day-level calendar only, because product expirations are date-based (`YYYY-MM-DD`) and have no time-of-day/hour semantics;
- avoid for Caduxo: overlapping task bars, event duration layout, hour/day scheduling concepts, and any third-party calendar code;
- picker-specific additions Caduxo still needs: typed `YYYY-MM-DD` input, validation, popover open/close, required/clearable split, keyboard behavior, and year range enforcement;
- calendar-tab additions Caduxo still needs: lot/product expiry counts and a selected-day detail panel.

Evidence collected: user-provided Svelte Playground URL (`https://svelte.dev/playground/5aa7d011c2104b0c8906248f66da22ea?version=5.57.0`) plus the code snippet supplied in chat. No third-party calendar implementation is an accepted source or implementation input for this SDD change.

## Intent

Two user-visible outcomes in one slice, sharing the same calendar primitives:

1. **Defect fix — native date picker.** Replace every `<input type="date">` in
   the app with an in-house custom calendar popup. The previous SDD change
   `caduxo-date-picker-dismissal` settled clean against every automated gate
   yet the underlying Linux/WebKitGTK + GTK native picker defect persisted
   because the wrapper still rendered `<input type="date">`, which hands the
   widget off to the host WebView. Any new design MUST NOT render
   `<input type="date">` for `LotForm.svelte` expiry or `ReportsPage.svelte`
   `dateFrom` / `dateTo`. The DOM element under each field becomes either
   `<input type="text">` (for typed `YYYY-MM-DD` entry) or a plain button,
   with an in-house Svelte calendar inside the popover.
2. **New feature — Calendar tab.** Add a fourth top-level navigation entry
   ("Calendar", alongside Dashboard / Products / Reports) that opens at the
   current month, highlights/selects today by default, shows product and lot
   expirations on each day, and lets the user click a day to see a detail
   list of lots and products expiring on that date. This view reuses the
   calendar/date-grid primitives from outcome 1 so neither outcome carries a
   duplicate month-grid implementation.

The two outcomes share one primitive (`CalendarMonth`) so that keyboard
semantics, year/month navigation, year range, and visual styling are
implemented once. The picker's popover is a thin wrapper around the same
month grid plus a typed-text input; the Calendar page is a full-page wrapper
around the same month grid plus a day-detail panel.

## Scope

### In scope (this slice)

#### A. Reusable calendar primitive — `CalendarMonth.svelte` (new)

A presentational, popover-agnostic Svelte component that renders a single
month grid plus month/year navigation header. Used by both the picker popover
and the new Calendar tab. No popover, no input, no I/O — pure view + events.

- **Props:** `viewYear: number`, `viewMonth: number` (1–12), `selectedDate: string | null` (ISO `YYYY-MM-DD`), `todayDate: string` (ISO `YYYY-MM-DD`), `dayBadges?: Record<string, number>` (optional `YYYY-MM-DD → count` map, used by Calendar tab to show expirations per day), `minDate?: string`, `maxDate?: string` (inclusive, defaults to `1900-01-01` and `2100-12-31`).
- **Events:** `daySelect` (emits `YYYY-MM-DD`), `monthChange` (emits `{ year, month }`), `viewYearChange`, `viewMonthChange`.
- **Year/month navigation:** explicit prev / next buttons for month; explicit year picker (decade-grid: 12 years per page with prev / next decade buttons, clamped to 1900–2100 inclusive). Year picker opens from the year header chip; clicking the month header cycles month; prev / next chevrons step month.
- **Year range:** 1900–2100 inclusive. Years outside the range are disabled in the year picker and clamped by the navigation buttons.
- **Day cells:** `today` highlight (distinct from `selected`); `selected` ring; `disabled` state for `minDate` / `maxDate` violations; `hasBadge` ring when `dayBadges[date] > 0`; Saturday/Sunday muted (visual only, not functional); weekday row in English (`Sun Mon Tue Wed Thu Fri Sat`); month/day headers in English (e.g. `January 2026`).
- **Locale:** English only. The codebase has no i18n infrastructure for frontend strings; the spec language is `en` per `openspec/config.yaml`. Re-localization is an explicit follow-up, not in scope.
- **No portal.** Floating positioning is the caller's responsibility (popover wrapper for the picker, page-level layout for the Calendar tab). The primitive emits its own bounding box via `bind:this` if needed.

#### B. Picker primitive — `DatePicker.svelte` (new)

A composite component used by `LotForm.svelte` and `ReportsPage.svelte`. Wraps a typed-text input, a calendar-icon button, and the `CalendarMonth` popover.

- **Trigger:** a single `<input type="text">` (not `type="date"`) with an adjacent calendar-icon button. Focusing the input or clicking the icon opens the popover; both paths behave identically. Tab enters the input / icon, Tab again leaves the picker.
- **Manual input:** the text input is the source of truth for typed entry. The user types `YYYY-MM-DD` directly. On every keystroke the picker validates the partial text and shows an inline red border + helper text when the text is not a parseable ISO date within 1900–2100. **The picker never silently rewrites the user's text** — invalid text remains visible until the user fixes it or selects a date from the popover. On blur with a valid value, the input commits and emits the new `YYYY-MM-DD`.
- **Selection semantics:** clicking a day in the popover emits `YYYY-MM-DD`, immediately closes the popover, and commits the value back to the host (and into the text input).
- **Keyboard:**
  - **Tab** enters the trigger and the calendar button in source order; Tab again leaves the picker (no focus trap inside the popover).
  - **Esc** closes the popover without committing changes; the text input keeps whatever the user typed (still subject to validation on blur).
  - **Enter** on a focused day cell emits that day and closes the popover.
  - **Arrow keys** (Left/Right/Up/Down) move the focused day by ±1 day / ±7 days; **PageUp / PageDown** step by month; **Shift + PageUp / Shift + PageDown** step by year. (User-confirmed "basic accessible keyboard" specifies Tab/Esc/Enter; arrow + PageUp/Down are the standard calendar expectation and are documented here so the design phase can either include or strip them.)
- **Today shortcut:** a `Today` button at the bottom-left of the popover that emits today's `YYYY-MM-DD` and closes. Disabled when `todayDate` is outside `minDate` / `maxDate`.
- **Clear button:** an `×` icon inside the text input's right edge. Emits `""` (empty string) and closes the popover. Visibility controlled by `clearable: boolean` prop (default `true`). When `clearable={false}`, the `×` is not rendered.
- **Required vs optional behavior:** the picker does **not** enforce `required` HTML semantics; the host owns that. When `clearable={false}`, the value cannot be cleared via the picker (the `×` is absent, and typing/deleting text down to empty still shows validation rather than silently committing empty). When `clearable={true}`, the user can clear via `×` or by deleting the text; on blur with empty text, the picker emits `""`.
- **Empty / null semantics:** the picker emits `""` for blank; the host decides what `""` means. `LotForm` treats it as missing (its own `required` JS guard fires). `ReportsPage` treats it as `null` filter input via its existing `dateFrom.trim() || null` / `dateTo.trim() || null` chain — unchanged.
- **Popover placement:** absolute-positioned relative to the trigger's bounding rect, prefer-below with overflow-flip-up if the popover would clip the viewport. No portal — `getBoundingClientRect` is sufficient.
- **Props:** `value: string` (bound `YYYY-MM-DD` or `""`), `clearable?: boolean` (default `true`), `minDate?: string` (default `1900-01-01`), `maxDate?: string` (default `2100-12-31`), `ariaLabel?: string` (e.g. "Expiry date"), `id?: string` (for label association), `placeholder?: string` (default `YYYY-MM-DD`), `name?: string` (for form submission parity, even though the element is `type="text"`).
- **Events:** standard Svelte `bind:value` two-way binding to keep host wiring identical to today's `<input type="date" bind:value={date}>`.

#### C. Host adoptions — three sites

| # | File | Field | Adopter change |
|---|---|---|---|
| 1 | `src/components/LotForm.svelte` | `expiryDate` | Replace `<input type="date" bind:value={expiryDate} required>` with `<DatePicker bind:value={expiryDate} clearable={false} ariaLabel="Expiry date" />`. Keep `required` semantics in JS guard (existing line 244–249). The `label input[type="date"]` CSS selector at line 343 is removed. |
| 2 | `src/components/ReportsPage.svelte` | `dateFrom` | Replace `<input type="date" bind:value={dateFrom} placeholder="YYYY-MM-DD">` with `<DatePicker bind:value={dateFrom} ariaLabel="Date from" />`. `clearable` defaults to `true`. |
| 3 | `src/components/ReportsPage.svelte` | `dateTo` | Same as `dateFrom`. |

No host data flow changes. No host CSS theme changes beyond removing the now-orphan `input[type="date"]` selector in `LotForm.svelte`.

#### D. Calendar tab — `CalendarPage.svelte` (new) + nav entry

- **Route:** add `"calendar"` to the `Tab` union in `src/App.svelte`, a `nav-btn` between `Products` and `Reports`, and an `{:else if activeTab === "calendar"}` arm that renders `<CalendarPage />`.
- **Initial state:** opens at the current month (`viewYear` / `viewMonth` from `new Date()`), `selectedDate = today` (ISO `YYYY-MM-DD`), and `todayDate = today`.
- **Data source:** reuses `list_dashboard_lots` (Tauri command already registered in `src-tauri/src/lib.rs`). `CalendarPage.svelte` calls the same `listDashboardLots({ store_id: null, location_id: null, preset: null, urgency: null })` and buckets rows by `expiry_date` in TypeScript. This deliberately avoids adding a new backend command in v1 — the same payload already powers the Dashboard table, so the Calendar view is essentially a different presentation of the same data.
- **Per-day badges:** `dayBadges: Record<string, number>` maps `YYYY-MM-DD → count of active lots with that expiry_date`. `CalendarMonth` receives it as the `dayBadges` prop and renders a small dot under the day number when `> 0`. The number itself is shown in a tooltip and in the day-detail panel; the dot alone is enough on the grid.
- **Clicking a day:** sets `selectedDate = clickedDate` and reveals the day-detail panel. The panel shows a table: columns `Product`, `Quantity`, `Unit`, `Store`, `Location`, `Days remaining`, `Status`. Rows are the lots whose `expiry_date === selectedDate`. Clicking a row opens that lot's edit flow (existing `LotForm` reused via the catalog page's pattern).
- **Empty day:** clicking a day with no expirations still selects it; the panel shows "No expirations on YYYY-MM-DD".
- **Keyboard / accessibility on the Calendar tab:** same `CalendarMonth` keyboard surface as the picker popover (Esc / Enter / arrows / PageUp/Down). Tab order: prev-month → month label → year label → prev-year → next-year → day grid → next-month → Today button → day-detail panel.

#### E. Spec delta on `openspec/specs/caduxo-expiry-tracker/spec.md`

- Add a new capability `## Capability: Date input` with one requirement covering:
  - picker is a Svelte component, not `<input type="date">`,
  - ISO `YYYY-MM-DD` in/out, year range 1900–2100 inclusive,
  - selection closes popover and commits,
  - manual `YYYY-MM-DD` text input with visual validation and no silent rewrite,
  - `clearable` prop contract (default `true`; `LotForm` passes `false`),
  - basic keyboard ergonomics (Tab / Esc / Enter at minimum).
- Update `## Capability: Expiry lots > ### Requirement: lot registration` to say the `expiry date` field uses the `Date input` picker with `clearable={false}`.
- Update `## Capability: Reports > ### Requirement: report filters` to say `date range` uses the `Date input` picker (clearable, optional).
- Add a new requirement under a new or existing capability for the Calendar tab, covering: opens at current month / today selected by default; per-day expirations visible; day-detail panel listing lots and products for the selected date.

#### F. Documentation

- `docs/prd.md` references to the picker and a possible new Calendar section are aligned during design (specific line numbers noted in the explore doc).

### Out of scope (this slice)

- Any backend schema migration or new Tauri command. The Calendar tab reuses `list_dashboard_lots` and buckets client-side; a future slice may add `list_expirations_by_day({ from, to })` if lot volumes grow past a comfortable client-side cap.
- Locale / i18n. English-only labels ship in v1; re-localization is a follow-up.
- A vitest / playwright / cypress harness. The project has no frontend test harness (`strictTdd: false`; canonical spec's `Engineering safety > tests accompany implementation` accepts manual verification). Manual smoke on Linux/WebKitGTK and Windows/WebView2 is the verify gate.
- Restyling of any host page beyond the picker swap and the new Calendar tab.
- Reusing or preserving any code from the removed `caduxo-date-picker-dismissal` workaround attempt. That attempt was deleted from the working tree; only runtime history records that the native-wrapper approach was insufficient.
- A Settings page or admin surface for picker preferences.
- Date range picker (a single calendar with `from` + `to` selection). Out of scope; Reports still uses two separate `DatePicker` instances.
- Time-of-day, timezone, or DST handling. `expiry_date` is `YYYY-MM-DD` local-date semantics and stays that way.
- Restyling of the Dashboard urgency cards or quick-filter buttons.
- Backend service or DTO changes (`expiry_lots`, `dashboard`, etc.). Untouched.

## Affected areas

| Area | Change | Approx. LOC delta |
|---|---|---|
| `src/components/CalendarMonth.svelte` (new) | Presentational month grid with year picker, today highlight, badges, day selection events. No popover, no input. | 200–300 |
| `src/components/DatePicker.svelte` (new) | Typed-text input + icon button + popover wrapper around `CalendarMonth`. Manual-input validation. `clearable` prop. Today shortcut. Keyboard handlers. | 250–400 |
| `src/components/CalendarPage.svelte` (new) | Full-page Calendar view. Fetches via `list_dashboard_lots`, buckets by `expiry_date`, day-detail panel. | 250–450 |
| `src/components/LotForm.svelte` | Replace native `<input type="date">` with `<DatePicker clearable={false} ...>`. Remove `label input[type="date"]` CSS selector. Keep `required` JS guard. | 20–40 |
| `src/components/ReportsPage.svelte` | Replace two native `<input type="date">` fields with `<DatePicker>` (clearable). Existing `.filter-field input` selector still applies (no `type="date"` qualifier). | 30–50 |
| `src/App.svelte` | Add `"calendar"` to `Tab` union, `nav-btn`, and `{:else if}` view arm. | 10–20 |
| `src/lib/expiry_lots.ts`, `src/lib/dashboard.ts` | No DTO or type changes (the picker binds `YYYY-MM-DD` like today). Possible tiny addition: an `ExpiryLotRow` type alias if the Calendar page benefits from a shared row shape; if not, this row is zero. | 0–30 |
| `src-tauri/**` | None. The slice does not introduce a new Tauri command, a new migration, a new service, or a new DTO. | 0 |
| `openspec/specs/caduxo-expiry-tracker/spec.md` | New `## Capability: Date input` requirement; small pointer edits under `Expiry lots > lot registration` and `Reports > report filters`; new requirement for the Calendar tab. | 30–60 |
| `docs/prd.md` | Aligned references to the picker and the new Calendar tab. | 10–30 |
| CSS (component-scoped) | Visual styling for the three new components plus minor removal in `LotForm.svelte`. | 150–250 |
| **Total estimate** | | **~950–1,600 LOC** |

The 950–1,600 LOC envelope deliberately spans a wide range because the
realistic size depends on design decisions locked in `design.md` (decade-grid
year picker, popover flip-up math, manual-input validation messaging copy,
calendar page layout density). The lower bound assumes a tight implementation
reusing existing CSS variables and a single-column Calendar page layout; the
upper bound assumes a polished popover with arrow + flip-up + outside-click
handler + full a11y attributes. See `## Review workload decision`.

## Reusable primitives — single source of truth for the date grid

To avoid a duplicate month-grid implementation between the picker popover and
the Calendar tab, the proposal pins the responsibility split as follows:

| Concern | Owned by | Notes |
|---|---|---|
| Month grid math (which days, weekday alignment, leap years) | `CalendarMonth.svelte` | Pure function of `viewYear`, `viewMonth`. |
| Year picker (decade grid, prev/next decade, range clamp) | `CalendarMonth.svelte` | 1900–2100 inclusive. |
| Day selection event | `CalendarMonth.svelte` | Emits `YYYY-MM-DD`. |
| Today highlight vs. selected highlight | `CalendarMonth.svelte` | Two distinct visual classes. |
| Day badges (per-day count overlay) | `CalendarMonth.svelte` | `dayBadges` prop. |
| Min/max date enforcement | `CalendarMonth.svelte` | Disabled state, not just visual. |
| Keyboard nav within the grid | `CalendarMonth.svelte` | Arrows, PageUp/Down, Enter, Esc-via-popover. |
| Trigger button + text input + icon | `DatePicker.svelte` | Composes `CalendarMonth` inside a popover. |
| Manual text input + validation messaging | `DatePicker.svelte` | Owns the typed-entry path. |
| Popover positioning (open / close / outside-click) | `DatePicker.svelte` | Absolute-positioned relative to trigger. |
| Page-level layout + day-detail panel | `CalendarPage.svelte` | Composes `CalendarMonth` in a full-page shell. |
| Per-day expiration lookup | `CalendarPage.svelte` | Calls `listDashboardLots`, buckets by `expiry_date`. |

The picker popover does **not** re-implement the grid. The Calendar tab does
**not** re-implement the grid. Both compose `CalendarMonth`. A future change
to keyboard nav or year-range styling lands in one place.

The only divergence between the two consumers is whether the `dayBadges` prop
is supplied: the picker passes `undefined` (no badges), the Calendar page
passes the bucketed expiration count.

## Review workload decision

The estimated line envelope is **~950–1,600 LOC**, including new components,
three host adoptions, a new top-level page, navigation wiring, a spec delta,
PRD alignment, and CSS for everything new.

This exceeds the original 400-line session budget and the persistent 800-line
project default. The user explicitly approved raising the review budget for
this SDD change to **3,000 changed lines** because they will be the sole code
reviewer and prefer picker + Calendar tab in one coherent slice.

Resolved delivery shape for this change:

- **Single SDD change:** `caduxo-custom-date-picker`.
- **Single implementation slice:** picker replacement and main Calendar tab
  ship together.
- **No chained PR split required by size:** the ~950–1,600 LOC estimate is
  below the explicit 3,000-line budget.
- **Review caution still applies:** the implementation must keep the shared
  primitive split (`CalendarMonth` reused by both `DatePicker` and
  `CalendarPage`) so the large slice remains reviewable by structure.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Total LOC exceeds the earlier 400/800 review budgets; reviewer fatigue / shallow review of a large cross-cutting slice | Defect slips past review; spec delta and primitives get inconsistent treatment | User explicitly raised this change's review budget to 3,000 lines and kept picker + Calendar tab in one SDD. Mitigate with a strict shared-primitive design and focused verify evidence. |
| Duplicate month-grid logic between picker and Calendar tab | Maintenance burden; two implementations drift | `## Reusable primitives` table pins the responsibility split; design phase enforces it. |
| Manual-input validation silently rewrites user text | Violates explicit user constraint; user loses trust | Validation only flips red border + helper text; the bound value only updates on valid input or selection. The implementation MUST keep the typed string in the input element until validity changes; this is a testable design-phase contract. |
| Required vs clearable contract regresses in LotForm | User can clear a required expiry date; submit silently fails downstream | `clearable={false}` in `LotForm.svelte` is mandatory; the picker renders no `×` icon and does not silently commit empty. The component's prop signature is the contract. |
| `1900–2100` boundary not enforced in manual input | User types `0001-01-01` or `9999-12-31` and the picker accepts it | Validation rejects out-of-range years on blur and shows the error; year picker disables out-of-range cells. |
| Calendar tab ships all lots to the frontend even when only one month is needed | Performance and memory regression at scale | Documented as a v1 tradeoff. A future slice may add `list_expirations_by_day({ from, to })`; the slice notes will track this. v1 caps gracefully because `list_dashboard_lots` already returns only `status = 'active'` rows ordered by `expiry_date ASC`. |
| No frontend test harness — manual verification only | Regression risk between slices | Recorded in the slice notes; covered by the canonical `Engineering safety > tests accompany implementation` requirement's manual-verify clause. |
| Popover positioning clip on small windows or when the trigger is near the bottom edge | Picker becomes unusable | Implement flip-up when `getBoundingClientRect().bottom + popoverHeight > viewport.height`. Out-of-bounds in both directions falls back to a centered overlay. |
| Esc inside the manual text input discards an uncommitted popover interaction unexpectedly | User loses confidence in typed-entry behavior | Esc closes the popover without committing a calendar selection and never rewrites or clears the typed text. If the typed text is invalid, the invalid text remains visible with validation until the user fixes it, blurs, or clears it through an allowed path. |
| Nav entry reorders existing tabs | Existing muscle memory breaks | Calendar is added **between Products and Reports** (per the user's "alongside Dashboard/Products/Reports" framing), not at the end; explicit ordering recorded so design phase doesn't drift. |
| Spec delta on `## Capability: Date input` overlaps with existing `Expiry lots > lot registration` and `Reports > report filters` text | Drift between capability statements | New capability is the canonical place; the two existing requirements get a one-line pointer edit ("see `Date input` for the picker contract"). |
| Future i18n follow-up requires renaming weekday / month strings everywhere | Tedious refactor | All weekday and month labels in `CalendarMonth.svelte` come from a single `const LABELS = { weekdays: [...], months: [...] }` block at the top of the file. Future i18n replaces one constant. |
| The removed `caduxo-date-picker-dismissal` workaround attempt remains in runtime history | Confusion about which fix is canonical | This proposal is canonical for the custom picker path; it must not reuse code from the removed native-wrapper attempt. |

## Rollback

Rollback is straightforward at every layer:

1. **Git revert.** `git revert <merge-sha>` of the slice restores the
   previous native-input state. No schema migration exists, so revert is
   additive-safe.
2. **Calendar tab rollback.** Remove the `"calendar"` arm of `Tab` and the
   corresponding `nav-btn` in `src/App.svelte`. Delete
   `CalendarPage.svelte`. Picker adoptions remain.
3. **Picker rollback.** Swap each `<DatePicker bind:value={...} />` back to
   `<input type="date" bind:value={...} />` in `LotForm.svelte` (with
   `required`) and `ReportsPage.svelte` (without `required`). Restore the
   `label input[type="date"]` CSS selector. Delete `DatePicker.svelte`.
4. **Primitive rollback.** Delete `CalendarMonth.svelte`. Both consumers
   would be reverted already (steps 2 and 3 above), so no consumer is
   broken.
5. **Spec delta rollback.** Remove the new `## Capability: Date input`
   section and the new Calendar requirement; revert the pointer edits
   under `Expiry lots > lot registration` and `Reports > report filters`.
6. **Data integrity.** No backend data is written by this slice. Nothing
   on disk is touched. The Calendar tab reads via `list_dashboard_lots`
   only.

The picker defect (Linux native picker bug) returns after rollback, so
revert should only be considered if the new picker introduces a worse
regression — not on its own merits.

## Success criteria

- `grep -R 'type="date"' src/` returns **zero** matches. This is the
  one-line defect fix.
- A user can open `LotForm.svelte`, focus or click the new picker, navigate
  to any month in 1900–2100, click a day, and the popover closes with the
  expiry date bound to `YYYY-MM-DD`. The `×` clear icon is **not** present.
  Submitting the form with an empty expiry date still triggers the existing
  JS guard (`"Expiry date is required"`).
- A user can open `ReportsPage.svelte`, type `2026-09-30` in the Date from
  field, blur, and the field commits. Typing `2026-13-40` shows a red
  border + inline error and does **not** commit. The `×` clear icon is
  present and clears the field on click.
- A user can press **Tab** into the trigger, **Tab** out (focus moves to
  the next form field, not into the popover); **Esc** closes the popover
  without committing; **Enter** on a focused day emits that day and closes.
  Arrow keys and PageUp/Down work as documented in `## Scope > B`.
- A user can navigate to the new **Calendar** tab in main nav, see the
  current month with today highlighted and selected, see small dots on
  days with at least one expiring lot, and click any day to see the detail
  list. Clicking a lot row navigates into that lot's edit flow.
- The Calendar tab calls `list_dashboard_lots` once on page load or explicit
  refresh, buckets the active rows client-side, and does **not** refetch on
  day selection or month navigation in v1 because the command has no month
  filter parameters.
- `npx svelte-check --workspace . --threshold error` passes.
- `npm run build` passes.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` keeps the same
  baseline as `archive/2026-09-14-caduxo-measurement-unit-options/apply-progress.md`
  (276 pass + 2 pre-existing failures unchanged by this slice; this slice
  touches zero Rust files).
- Manual smoke on Linux/WebKitGTK and Windows/WebView2 covers: picker
  open / select / clear / type-validate / Esc / Enter; Calendar tab open /
  month nav / day click / detail panel / dot badge accuracy.

## Confirmed product decisions (from question round and parent context)

These are the user-confirmed decisions captured in the user's framing for
this slice. They are restated here so the design phase has them in one place:

**Picker:**

1. Replace native date inputs with an in-house custom calendar popup; do
   **not** render `<input type="date">`.
2. Scope all three current date inputs: `LotForm.svelte` expiry date,
   `ReportsPage.svelte` dateFrom and dateTo.
3. Selection closes the popover immediately and emits ISO `YYYY-MM-DD`.
4. Explicit year / month / day navigation. Year range 1900–2100 inclusive.
5. Clear only the optional Reports filters; the LotForm required date does
   **not** offer clear (`clearable={false}`).
6. Manual `YYYY-MM-DD` text input with visual validation; **invalid text
   must not be silently rewritten.**
7. Picker UI is English-only for now (most app UI is English); future i18n
   is a user-preference follow-up.
8. Open through input + calendar button (both paths behave identically).
9. Include a `Today` shortcut.
10. Basic accessible keyboard: Tab enters/leaves, Esc closes, Enter selects
    the focused day. (Arrow keys / PageUp/Down documented in scope.)

**Calendar tab:**
11. Same SDD change adds a main-navigation Calendar tab/page that reuses
    the calendar/date-grid logic where practical.
12. Calendar tab opens at the current month; the current day is highlighted
    and selected by default.
13. It displays product/lot expirations by day (per-day dot badge).
14. Location: main navigation, alongside Dashboard / Products / Reports
    (positioned between Products and Reports).
15. Clicking a day with expirations selects that day and shows a detail
    list of lots/products expiring on that date.

The deliverable is a single SDD change covering all 15 decisions.

## Next steps (after this proposal is approved)

1. Move to `design.md` — concrete shapes for `CalendarMonth.svelte`,
   `DatePicker.svelte`, `CalendarPage.svelte`; CSS variable usage;
   popover flip-up math; manual-input validation rules and error copy;
   keyboard handler table; outside-click + Esc interaction rules.
2. Move to `tasks.md` — ordered work units sized to the chosen delivery
   shape. Manual-frontend verify gap recorded explicitly per the
   canonical `Engineering safety > tests accompany implementation`
   requirement.
3. Spec delta on `openspec/specs/caduxo-expiry-tracker/spec.md`:
   - new `## Capability: Date input` with one requirement,
   - one-line pointer edits under `Expiry lots > lot registration` and
     `Reports > report filters`,
   - new requirement for the Calendar tab (placed under a new capability
     or under `Dashboard`, TBD in design).
4. PRD alignment on `docs/prd.md` (small, in design).
