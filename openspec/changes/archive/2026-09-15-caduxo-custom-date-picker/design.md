# Design: caduxo-custom-date-picker

## Executive summary

This design locks the implementation shape for the in-house custom date picker plus the new main-navigation Calendar tab, both inside a single SDD change. The defect fix is **mechanical and non-negotiable**: no `<input type="date">` remains in `src/` after this slice — the wrapper from `caduxo-date-picker-dismissal` is superseded and its pattern must not be re-introduced. The feature delivery is **architecturally unified**: one presentational `CalendarMonth.svelte` primitive is composed by both the popover-based `DatePicker.svelte` and the full-page `CalendarPage.svelte`, so keyboard ergonomics, year-range enforcement (1900–2100), visual styling, and date math live in exactly one place. The Calendar tab reuses the already-registered `list_dashboard_lots` Tauri command and buckets active rows client-side; no backend changes ship in this slice, and `cargo test --lib` is expected to remain at the 276-pass / 2-pre-existing-failure baseline. The user-approved review budget for this change is **3,000 changed lines**; the implementation must respect that budget and keep the design surface coherent enough that one coherent PR can review the picker swap, the new Calendar tab, the spec delta, and the PRD alignment together.

## Confirmed scope boundaries (locked from proposal)

| # | Decision | Source | Status |
|---|----------|--------|--------|
| D1 | No `<input type="date">` rendered anywhere; trigger is `<input type="text">` or `<button>` | proposal §Intent + decision 1 | locked |
| D2 | Three sites: `LotForm.svelte` expiry, `ReportsPage.svelte` dateFrom + dateTo | proposal §Scope C | locked |
| D3 | Selection closes popover immediately and emits ISO `YYYY-MM-DD` | proposal §Scope B | locked |
| D4 | Explicit year / month / day nav; year range 1900–2100 inclusive | proposal §Scope A/B | locked |
| D5 | `clearable` prop (default `true`); `LotForm` passes `clearable={false}` | proposal §Scope B + decision 5 | locked |
| D6 | Manual `YYYY-MM-DD` text input; **no silent rewrite** of typed text | proposal §Scope B + decision 6 | locked |
| D7 | English-only UI labels in v1 | proposal §Scope A + decision 7 | locked |
| D8 | Single `<input type="text">` doubles as trigger; calendar-icon button is adjacent | proposal §Scope B + decision 8 | locked |
| D9 | `Today` shortcut inside popover; disabled outside min/max | proposal §Scope B + decision 9 | locked |
| D10 | Basic keyboard: Tab/Esc/Enter; arrows + PageUp/Down included | proposal §Scope B + decision 10 | locked |
| D11 | New main-nav Calendar tab reusing the same primitive | proposal §Intent + decision 11 | locked |
| D12 | Calendar opens at current month; today highlighted and selected | proposal §Scope D + decision 12 | locked |
| D13 | Per-day dot badge shows expirations | proposal §Scope D + decision 13 | locked |
| D14 | Nav order: Dashboard / Stores / Products / **Calendar** / Reports / Import / Backup | proposal §Scope D + decision 14 | locked |
| D15 | Clicking a day opens a detail panel listing lots + products for that date | proposal §Scope D + decision 15 | locked |

These 15 decisions are the contract for `tasks.md` and `apply`. No new product question round is required before `tasks.md`; the proposal's product question round already settled every open decision surfaced by `explore.md §5`.

## Architecture decision

The slice is **frontend-only and additive**, mirroring the layering used by the most recent archived slice (`2026-09-13-caduxo-create-product-upc`). It targets the same workspace: Rust backend in `src-tauri/` (untouched), Svelte frontend in `src/` (three new components + three host edits + nav wiring), OpenSpec artifacts (spec delta + this design + tasks + apply-progress), and PRD alignment.

The architectural commitment is **single-source-of-truth for the month grid**. `CalendarMonth.svelte` is the only component that knows about weekday alignment, leap-year math, year-range clamping, day-cell rendering, and keyboard navigation within the grid. `DatePicker.svelte` and `CalendarPage.svelte` are both consumers — they compose `<CalendarMonth>` and add only the surface they own (typed input + popover for the picker; data fetch + day-detail panel for the Calendar page).

```text
                       ┌──────────────────────────────┐
                       │   src-tauri/list_dashboard_  │
                       │   lots  (unchanged)          │
                       └──────────────┬───────────────┘
                                      │  active lots
                                      ▼
   ┌─────────────────────┐   composes   ┌──────────────────────┐
   │ DatePicker.svelte   │────────────▶│ CalendarMonth.svelte │
   │ (typed text input + │              │ (view + events)      │
   │  popover wrapper)   │              │                      │
   └──────────┬──────────┘              └──────────┬───────────┘
              │                                     ▲
              │ used by                             │
              ▼                                     │ composes
   ┌─────────────────────┐                          │
   │ LotForm.svelte      │   ┌──────────────────────┴───┐
   │ ReportsPage.svelte  │   │ CalendarPage.svelte       │
   │  (host sites)       │   │ (page-level shell + day-  │
   │                     │   │  detail panel)            │
   └─────────────────────┘   └──────────────┬───────────┘
                                            │
                                            ▼
                                ┌──────────────────────┐
                                │ src/App.svelte       │
                                │ (Tab union + nav-btn)│
                                └──────────────────────┘
```

Cross-cutting rules (locked):

- **No portal.** Both consumers position the grid using absolute positioning + `getBoundingClientRect` (popover) or normal page flow (Calendar page). This is acceptable because the codebase has no popover primitive and the proposal explicitly chose not to introduce one in v1.
- **No CSS framework.** Component-scoped `<style>` blocks only; reuse the existing tokens (`#2563eb`, `#1e293b`, `#f1f5f9`, etc.) already used by `LotForm`, `ReportsPage`, and `DashboardPage`. No Tailwind, no shared theme file.
- **No new Tauri command.** `CalendarPage` calls the existing `listDashboardLots({ store_id: null, location_id: null, preset: null, urgency: null })` exactly once on page load (and again on explicit refresh); day clicks and month navigation do not refetch. The repository already returns active lots ordered by `expiry_date ASC`, which is the correct shape for client-side bucketing.
- **No new frontend test harness.** `strictTdd: false` in `openspec/config.yaml`. Manual smoke on Linux/WebKitGTK + Windows/WebView2 is the verify gate, matching the canonical spec's `Engineering safety > tests accompany implementation` accepted posture for frontend changes.
- **No i18n.** A single `LABELS = { weekdays, months }` constant at the top of `CalendarMonth.svelte` is the future i18n extraction point.

## Component design

### `CalendarMonth.svelte` (new, presentational)

The only component that renders a month grid. Pure view + events; owns no input, no popover, no I/O.

#### Props

| Prop | Type | Required | Default | Notes |
|------|------|----------|---------|-------|
| `viewYear` | `number` | yes | — | 1900–2100 inclusive |
| `viewMonth` | `number` | yes | — | 1–12 |
| `selectedDate` | `string \| null` | yes | — | ISO `YYYY-MM-DD` or `null` for no selection |
| `todayDate` | `string` | yes | — | ISO `YYYY-MM-DD`; visual highlight only |
| `dayBadges` | `Record<string, number> \| undefined` | no | `undefined` | `YYYY-MM-DD → count`; when `> 0` a dot renders under the day number |
| `minDate` | `string` | no | `"1900-01-01"` | inclusive |
| `maxDate` | `string` | no | `"2100-12-31"` | inclusive |
| `ariaLabel` | `string` | no | `"Calendar"` | applied to the grid container |

Props use Svelte 5 `export let` (consistent with the rest of the codebase). Numeric inputs are clamped on `onMount` and on every reactive update.

#### Events

| Event | Payload | Trigger |
|-------|---------|---------|
| `daySelect` | `string` (ISO `YYYY-MM-DD`) | click or Enter on a day cell |
| `monthChange` | `{ year: number, month: number }` | prev/next chevron or month-label cycle |
| `viewYearChange` | `number` | year picker selection |
| `viewMonthChange` | `number` | month cycle (separate event so parent can persist if needed) |

Events are dispatched via `createEventDispatcher` (Svelte 5 idiom already in use across `LotForm.svelte`, `ReportsPage.svelte`, etc.). The component does **not** mutate its own `viewYear` / `viewMonth` on prev/next chevron clicks; the parent owns navigation state. This keeps the picker popover and the Calendar page in control of when to commit a month change.

#### Layout

```text
┌────────────────────────────────────────────────┐
│ [<]   [ Month   Year ▼ ]   [>]                │  ← header
├────────────────────────────────────────────────┤
│ Sun  Mon  Tue  Wed  Thu  Fri  Sat              │  ← weekday row
├────────────────────────────────────────────────┤
│                                                │
│   [  ] [  ] [  ] [  ] [ 1 ] [ 2 ] [ 3 ]       │
│   [ 4] [ 5] [ 6] [ 7] [ 8] [ 9] [10]          │  ← 6×7 day grid
│   ...                                          │
│                                                │
└────────────────────────────────────────────────┘
```

- **Header:** `‹` chevron, clickable month label (cycles month on click), year chip (opens decade-grid popover), `›` chevron. Header buttons are real `<button>` elements with `aria-label`.
- **Weekday row:** `Sun Mon Tue Wed Thu Fri Sat` from a single `LABELS.weekdays` constant.
- **Day cells:** 42 cells (6 rows × 7 columns). Leading/trailing cells outside the current month are rendered as muted blanks (no number, no click). Cells with a day number are `<button>`s with `aria-label="March 14, 2026"` and `aria-pressed={selected}`.
- **Year picker:** when the year chip is clicked, the header swaps to a 12-year decade grid (4×3). Year chips outside `[1900, 2100]` are disabled. `‹ decade` / `decade ›` chevrons step the decade by 10. Selecting a year emits `viewYearChange` and closes the decade popover. The decade popover is implemented as a `<div>` inside the same calendar container; no nested portal needed.
- **Footer:** optional slot via `<slot name="footer" />` used by `DatePicker` to inject the `Today` shortcut. `CalendarPage` does not use the slot.

#### Visual states (CSS classes)

| State | Class | Trigger |
|-------|-------|---------|
| Today highlight | `.day-today` | `cellIso === todayDate` (distinct from selected) |
| Selected | `.day-selected` | `cellIso === selectedDate` (uses `aria-pressed=true`) |
| Disabled | `.day-disabled` | `cellIso < minDate` or `cellIso > maxDate`; `disabled` attr on the button |
| Has badge | `.day-badge` | `dayBadges?.[cellIso] > 0`; renders a 6px dot under the number |
| Outside month | `.day-outside` | cell is padding from prev/next month; not interactive |
| Weekend | `.day-weekend` | Saturday/Sunday; muted color, still interactive |

Saturday/Sunday is determined by `Date#getDay()` returning `0` (Sunday) or `6` (Saturday). The visual muting is decorative only.

#### Locale + labels (single source)

```ts
const LABELS = {
  weekdays: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
  months: [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
  ],
} as const;
```

This block lives at the top of the file. A future i18n slice replaces this constant. All header text and `aria-label`s derive from this constant.

#### Day-grid computation

A pure helper inside the component (no export):

```ts
function buildMonthGrid(year: number, month: number): { iso: string; day: number }[] {
  // Returns 42 cells, leading blanks encoded as { iso: "", day: 0 }.
  // - first-of-month weekday (0..6) determines leading blanks
  // - last-of-month day determines trailing blanks
  // - each cell is a local-date YYYY-MM-DD built from year/month/day parts
  //   (no UTC conversion — keeps the picker in user-local calendar semantics)
}
```

The helper is used both by the picker popover and by `CalendarPage`. Putting it in `CalendarMonth` keeps the implementation single-sourced; `CalendarPage` reuses it via composition (renders the cells, just arranges them differently if needed). For v1, `CalendarPage` renders the same `<CalendarMonth>` and feeds the same `dayBadges` prop, so no extraction to a shared `.ts` module is required in this slice. If a future slice needs the math outside of `CalendarMonth`, that helper is the documented extraction point.

#### Keyboard handler table (inside `CalendarMonth`)

| Key | Action | Notes |
|-----|--------|-------|
| `ArrowLeft` | focused day −1 day | clamp to `[minDate, maxDate]` |
| `ArrowRight` | focused day +1 day | clamp |
| `ArrowUp` | focused day −7 days | clamp |
| `ArrowDown` | focused day +7 days | clamp |
| `PageUp` | focused day −1 month | clamp + same-day-of-month when target month is shorter |
| `PageDown` | focused day +1 month | same clamping |
| `Shift+PageUp` | focused day −1 year | clamp |
| `Shift+PageDown` | focused day +1 year | clamp |
| `Enter` | emit `daySelect(focusedIso)` | parent decides whether to close popover |
| `Escape` | re-dispatched as `escape` event | parent decides whether to close popover |

The component tracks a `focusedIso` internal state. On `onMount` it focuses the cell for `selectedDate` if set, else the cell for `todayDate` if visible in the current view, else the first cell of the visible month. When `viewYear` / `viewMonth` change, focus re-evaluates and tries to land on the same day-of-month in the new view, falling back to `todayDate`.

The keyboard handler attaches to the grid container via `on:keydown` and uses `event.key` matching (no `keyCode`). Modifier checks use `event.shiftKey`.

### `DatePicker.svelte` (new, composite)

Wraps `<CalendarMonth>` inside a popover attached to a text input + calendar icon button. Owns the trigger, the typed text, validation, and popover placement.

#### Props

| Prop | Type | Default | Notes |
|------|------|---------|-------|
| `value` | `string` | — | bound via `bind:value`; `""` for blank |
| `clearable` | `boolean` | `true` | when `false`, no `×` icon |
| `minDate` | `string` | `"1900-01-01"` | passed to `CalendarMonth` |
| `maxDate` | `string` | `"2100-12-31"` | passed to `CalendarMonth` |
| `ariaLabel` | `string` | `"Date"` | applied to the trigger input |
| `id` | `string \| undefined` | — | for label `for=` association |
| `placeholder` | `string` | `"YYYY-MM-DD"` | shown when `value === ""` |
| `name` | `string \| undefined` | — | for form-submission parity even though the element is `type="text"` |
| `todayDate` | `string` | — | required; hosts pass `todayAsIso()` to keep tests deterministic |

`value` is two-way bound via Svelte's `export let value` + a `value` setter that updates the local `displayText` only when validation accepts the change. This is the explicit non-rewrite contract: the **input element's text** and the **bound value** are decoupled.

#### Local state

```ts
let displayText = value ?? "";     // what's rendered in <input>
let isOpen = false;                // popover open state
let isInvalid = false;             // visual red border + helper text
let triggerEl: HTMLDivElement;     // bound ref for popover positioning
let popoverEl: HTMLDivElement;     // bound ref for outside-click detection
let inputEl: HTMLInputElement;     // bound ref for focus + selection
```

The component does **not** mutate `value` while the user is typing. On every input event it re-validates `displayText`:

1. If empty → `isInvalid = false`, `value` stays as-is (last good value, or `""` if user started blank).
2. If `displayText` parses to ISO `YYYY-MM-DD` inside `[minDate, maxDate]` → `isInvalid = false`, `value = parsedIso`.
3. Otherwise → `isInvalid = true`, `value` is **not** updated. The `displayText` remains as typed.

On `blur`:

- If `displayText === ""` and `clearable` → `value = ""`, `isInvalid = false`.
- If `displayText` is valid → commit the same as during typing.
- If `displayText` is invalid → `isInvalid = true`, `value` left at last good value (or `""`). **No silent rewrite.**

#### Manual-input validation rules

| Input shape | Action | Visual |
|-------------|--------|--------|
| `""` (empty) | `value` unchanged; `isInvalid = false` if `clearable`, else `isInvalid = true` (LotForm cannot be empty by spec) | normal border |
| `"2026-09-30"` (valid ISO in range) | `value = "2026-09-30"`; `isInvalid = false` | normal border |
| `"2026-13-40"` (invalid) | `value` unchanged; `isInvalid = true` | red border + helper text "Use YYYY-MM-DD" |
| `"2026-02-30"` (invalid day for month) | `value` unchanged; `isInvalid = true` | same as above |
| `"1899-12-31"` (out of range) | `value` unchanged; `isInvalid = true` | helper text "Year must be 1900–2100" |
| `"2101-01-01"` (out of range) | `value` unchanged; `isInvalid = true` | same |
| Partial typing (`"2026-0"`) | `isInvalid = true` only after blur; during typing the partial text is kept visible | no visual during typing unless user blurs |

Validation uses a single helper:

```ts
export function parseIsoDate(input: string, minDate = "1900-01-01", maxDate = "2100-12-31"):
  | { ok: true; iso: string }
  | { ok: false; reason: "shape" | "calendar" | "range" };
```

This helper lives in `DatePicker.svelte` (private export) in v1. `CalendarMonth` does **not** validate dates — it trusts its `minDate` / `maxDate` props and disables out-of-range cells at render time.

#### Popover positioning

Absolute-positioned relative to the trigger wrapper. The picker is a **non-modal floating popover** — no overlay, no focus trap, no `aria-modal`. Tab leaves the popover naturally.

```ts
function positionPopover() {
  if (!isOpen || !triggerEl || !popoverEl) return;
  const rect = triggerEl.getBoundingClientRect();
  const popHeight = popoverEl.offsetHeight;
  const popWidth = popoverEl.offsetWidth;
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  // prefer-below with flip-up
  const spaceBelow = vh - rect.bottom;
  const spaceAbove = rect.top;
  const placeBelow = spaceBelow >= popHeight + 8 || spaceBelow >= spaceAbove;
  popoverEl.style.top = placeBelow
    ? `${rect.bottom + 4}px`
    : `${rect.top - popHeight - 4}px`;
  // horizontal: align to trigger left, clamp to viewport
  const left = Math.max(8, Math.min(rect.left, vw - popWidth - 8));
  popoverEl.style.left = `${left}px`;
  popoverEl.style.position = "fixed"; // fixed + viewport coords avoids scroll-jump
}
```

Position recalculation is triggered on:

- `isOpen` flipping to `true`.
- `viewYear` / `viewMonth` changes (year picker swap may change popover height).
- `window` `resize` (debounced 100ms) and `scroll` (debounced 100ms).

Out-of-bounds in both directions (very small viewport) keeps the popover at the `placeBelow` coordinate with horizontal clamping; the existing modal pattern in `DashboardPage.svelte` (`.modal-overlay` covering the viewport) is **not** reused because the picker is non-modal.

#### Outside-click + Esc interaction

- **Outside click** (`document` `mousedown` listener attached while open): if `event.target` is not inside `popoverEl` or `triggerEl`, close the popover. The listener is removed on close / `onDestroy`.
- **Esc**: closes the popover **without committing a calendar selection**. The text input's `displayText` and the bound `value` are both left untouched (still subject to the blur-validation rules above when the user blurs).
- **Click on a day**: emits `daySelect(iso)` via `CalendarMonth`'s dispatcher; the picker handler sets `value = iso`, `displayText = iso`, `isInvalid = false`, `isOpen = false`.

#### Trigger composition

```html
<div class="dp-trigger" bind:this={triggerEl}>
  <input
    bind:this={inputEl}
    type="text"
    inputmode="numeric"
    autocomplete="off"
    autocapitalize="off"
    spellcheck="false"
    {id}
    {name}
    {placeholder}
    aria-label={ariaLabel}
    aria-invalid={isInvalid}
    aria-haspopup="dialog"
    aria-expanded={isOpen}
    class:dp-invalid={isInvalid}
    value={displayText}
    on:focus={openPopover}
    on:input={onInput}
    on:blur={onBlur}
    on:keydown={onInputKeydown}
  />
  <button
    type="button"
    class="dp-icon"
    aria-label="Open calendar"
    tabindex="0"
    on:click={togglePopover}
  >📅</button>
  {#if clearable && value !== ""}
    <button
      type="button"
      class="dp-clear"
      aria-label="Clear date"
      on:click={onClear}
    >✕</button>
  {/if}
</div>

{#if isOpen}
  <div
    bind:this={popoverEl}
    class="dp-popover"
    role="dialog"
    aria-label={`${ariaLabel} calendar`}
  >
    <CalendarMonth
      viewYear={popoverYear}
      viewMonth={popoverMonth}
      selectedDate={value || null}
      todayDate={todayDate}
      {minDate}
      {maxDate}
      ariaLabel={ariaLabel}
      on:daySelect={onDaySelect}
      on:monthChange={(e) => { popoverYear = e.detail.year; popoverMonth = e.detail.month; }}
      on:viewYearChange={(e) => { popoverYear = e.detail; }}
    >
      <svelte:fragment slot="footer">
        <button type="button" class="dp-today" on:click={onToday}>
          Today
        </button>
      </svelte:fragment>
    </CalendarMonth>
  </div>
{/if}
```

`popoverYear` / `popoverMonth` are internal state initialised from `value ?? todayDate` (parsed as a `Date` and decomposed into year/month). They are not bound to `value` directly — selecting a day commits and resets them, while clicking prev/next chevrons in the popover updates them without committing.

#### `Today` shortcut

```ts
function onToday() {
  if (!isWithin(todayDate, minDate, maxDate)) return; // disabled in template
  value = todayDate;
  displayText = todayDate;
  isInvalid = false;
  isOpen = false;
}
```

The `Today` button is rendered with `disabled` when `todayDate < minDate || todayDate > maxDate`. In practice this never fires (the defaults are 1900–2100), but the prop contract is honoured.

#### Clear (`×`) button

Rendered only when `clearable === true && value !== ""`. Click handler:

```ts
function onClear() {
  value = "";
  displayText = "";
  isInvalid = false;
  isOpen = false;
}
```

When `clearable === false`, the `×` is **not** in the DOM at all. The user can still delete text from the input down to empty; on blur with empty text, the picker shows `isInvalid = true` (LotForm `required` semantics: the parent JS guard then blocks submit).

### `CalendarPage.svelte` (new, page-level)

Top-level tab that reuses `<CalendarMonth>` and a per-day bucketed view.

#### Props

None — this is a self-contained page-level component imported by `App.svelte`.

#### State

```ts
let loading = true;
let errorMsg = "";
let lots: DashboardLotRow[] = [];                // populated from listDashboardLots
let viewYear: number;                            // initialised from today
let viewMonth: number;                           // initialised from today
let selectedDate: string = todayIso();           // YYYY-MM-DD of today
const todayDate: string = todayIso();
let detailLoading = false;
let showLotDetail = false;                       // reuses LotForm pattern
let detailLot: ExpiryLotResponse | null = null;
```

`viewYear` / `viewMonth` are initialised from `new Date().getFullYear()` / `getMonth() + 1` at component instantiation. They are mutated by `CalendarMonth`'s `monthChange` and `viewYearChange` events. Refetching on month change is **not** done in v1 (per proposal §Spec delta alignment note + the already-filtered `list_dashboard_lots` payload).

#### Data flow

```text
onMount
  └─► listDashboardLots({ store_id: null, location_id: null, preset: null, urgency: null })
        └─► DashboardResponse { counts, lots }
              └─► lots filtered to status === "active" (already true in payload)
                    └─► dayBadges = lots.reduce(
                          (acc, l) => {
                            const k = l.expiry_date;     // YYYY-MM-DD
                            acc[k] = (acc[k] ?? 0) + 1;
                            return acc;
                          },
                          {} as Record<string, number>,
                        )
```

Day-detail panel computes the rows for the selected day from `lots`:

```ts
$: dayRows = lots.filter((l) => l.expiry_date === selectedDate);
$: dayBadges = lots.reduce(/* as above */, {});  // recomputed when lots change
```

The bucket map is reactive on `lots`; the day-detail list is reactive on `selectedDate` + `lots`. Both are cheap (n < ~500 in v1; the slice notes record the future `list_expirations_by_day` follow-up as documented scope expansion if lot volumes grow).

#### Layout

```text
┌─ CalendarPage ──────────────────────────────────────────────┐
│  [<]   [ Month  Year ▼ ]   [>]                              │
│  Sun  Mon  Tue  Wed  Thu  Fri  Sat                           │
│  ┌──┬──┬──┬──┬──┬──┬──┐                                     │
│  │ 1│ 2│ 3│ 4│ 5│ 6│ 7│                                     │
│  ├──┼──┼──┼──┼──┼──┼──┤                                     │
│  │ 8│ 9│10│11│12│13│14│                                     │
│  └──┴──┴──┴──┴──┴──┴──┘                                     │
│                                                              │
│  Day detail — 2026-03-14 (3 lots)                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ Product   │ Qty │ Unit │ Store   │ Loc │ Days │ Stat │    │
│  ├─────────────────────────────────────────────────────┤    │
│  │ SKU-001   │  10 │ kg   │ Store A │ —   │   0  │ today│    │
│  │ SKU-002   │   3 │ L    │ Store A │ Frg │  -3  │ exp. │    │
│  └─────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

- The grid uses `<CalendarMonth dayBadges={dayBadges}>` directly. The grid occupies the page's main column.
- The day-detail panel sits below the grid (single-column on narrow viewports) or alongside on wider ones (`grid-template-columns: minmax(320px, 1fr) 320px` breakpoint). For v1, the simplest layout is single-column (grid + panel stacked) to keep the slice focused.
- Empty-day message: when `dayRows.length === 0`, the panel shows `No expirations on {formatDate(selectedDate)}`.
- Lot row click: opens an inline lot edit overlay reusing the existing `LotForm` (create + edit modes). v1 uses the same overlay pattern as `DashboardPage.svelte::showLotDetail` (modal with `LotForm` inside). This avoids inventing a new modal primitive.
- Refresh button: small button next to the month label, triggers `loadLots()` again. Used when the user suspects new data; v1 does not auto-refresh.

#### Lifecycle

```ts
onMount(async () => {
  try {
    const data = await listDashboardLots({});
    lots = data.lots;
  } catch (e) {
    errorMsg = String(e);
  } finally {
    loading = false;
  }
});
```

No setInterval; no reactive polling. The slice note about a future in-app refresh trigger is recorded as out of scope.

#### Keyboard navigation

Inherits the entire `<CalendarMonth>` keyboard surface. Tab order on the page:

1. Prev-month chevron
2. Month label (button)
3. Year chip (button → opens decade grid)
4. Prev-year chevron (in decade view)
5. Next-year chevron (in decade view)
6. Day grid (focusable container, Tab into first cell)
7. Next-month chevron
8. Refresh button
9. Day-detail panel (each lot row is a `<button>`)

This matches proposal §Scope D's tab order requirement.

### Host adoptions (locked from proposal §Scope C)

| # | File | Change |
|---|------|--------|
| H1 | `src/components/LotForm.svelte` | Replace `<input type="date" bind:value={expiryDate} required>` at lines 244–249 with `<DatePicker bind:value={expiryDate} clearable={false} ariaLabel="Expiry date" id="lot-expiry" />`. Remove `label input[type="date"]` selector at line 343. Keep `required` semantics in JS guard (`if (!expiryDate) errorMsg = "Expiry date is required";` at line 119). |
| H2 | `src/components/ReportsPage.svelte` | Replace `<input type="date" bind:value={dateFrom} placeholder="YYYY-MM-DD">` at lines 367–373 with `<DatePicker bind:value={dateFrom} ariaLabel="Date from" placeholder="YYYY-MM-DD" />`. `clearable` defaults to `true`. |
| H3 | `src/components/ReportsPage.svelte` | Replace `<input type="date" bind:value={dateTo} placeholder="YYYY-MM-DD">` at lines 376–382 with `<DatePicker bind:value={dateTo} ariaLabel="Date to" placeholder="YYYY-MM-DD" />`. |

#### Nav wiring (`src/App.svelte`)

```diff
- type Tab = "dashboard" | "stores" | "products" | "reports" | "import" | "backup";
+ type Tab = "dashboard" | "stores" | "products" | "calendar" | "reports" | "import" | "backup";

  let activeTab: Tab = "stores";

+ import CalendarPage from "./components/CalendarPage.svelte";
```

Add a `<button class="nav-btn" class:active={activeTab === "calendar"} on:click={() => (activeTab = "calendar")}>Calendar</button>` between the `Products` and `Reports` buttons.

Add `{:else if activeTab === "calendar"} <CalendarPage />` to the view switch.

Initial `activeTab` stays `"stores"` (existing default). The Calendar tab is not auto-selected on first run; it follows the same opt-in pattern as Reports.

### Data flow summary

```text
User input → DatePicker input event
              ├─ parseIsoDate(displayText, minDate, maxDate)
              ├─ ok → value = iso, isInvalid = false
              └─ bad → value unchanged, isInvalid = true

User clicks day in popover → CalendarMonth daySelect(iso)
              └─ DatePicker: value = iso, displayText = iso, isOpen = false
                    └─ host: bind:value receives iso
                          ├─ LotForm: stored in expiryDate; submitted as expiry_date
                          └─ ReportsPage: stored in dateFrom/dateTo; passed to buildFilters() via .trim()||null

User opens Calendar tab → CalendarPage.onMount
              └─ listDashboardLots({}) → lots
                    └─ dayBadges computed (reduce by expiry_date)
                          └─ CalendarMonth renders grid with dot badges
                                └─ click day → selectedDate = iso
                                      └─ dayRows = lots.filter(...)
                                            └─ row click → opens LotForm overlay (existing pattern)
```

No new Tauri commands. No new SQL. No new Rust modules. No new migrations.

### Backend surface

**No backend changes in this slice.** The proposal's `Out of scope` section is explicit and the design respects it.

`cargo test --lib` is expected to remain at the archived baseline: **276 passed + 2 pre-existing failures** (`services::reports::tests::preview_report_in_alert_window_returns_alert_lots`, `services::reports::tests::preview_report_next_30_days_returns_30d_lots`) unchanged by this slice, per the `2026-09-14-caduxo-measurement-unit-options` apply-progress evidence.

### File changes (locked)

| File | Status | Approx LOC delta | Rationale |
|------|--------|------------------|-----------|
| `src/components/CalendarMonth.svelte` | new | 220–280 | presentational grid + year picker + footer slot + keyboard handlers + CSS |
| `src/components/DatePicker.svelte` | new | 260–360 | trigger composition + popover positioning + validation + clear/Today + CSS |
| `src/components/CalendarPage.svelte` | new | 250–380 | page shell + onMount fetch + dayBadges reducer + day-detail table + CSS |
| `src/components/LotForm.svelte` | edit | +12 / −10 | picker adoption + JS guard unchanged + remove `input[type="date"]` selector |
| `src/components/ReportsPage.svelte` | edit | +20 / −14 | two picker adoptions; existing `.filter-field input` selector still matches |
| `src/App.svelte` | edit | +6 / −1 | `Tab` union + nav-btn + view arm |
| `openspec/specs/caduxo-expiry-tracker/spec.md` | edit | +50 / −5 | new `## Capability: Date input` requirement + new Calendar requirement + pointer edits in two existing requirements |
| `openspec/changes/caduxo-custom-date-picker/{design.md,tasks.md,apply-progress.md,verify-report.md,archive-report.md}` | new | varies | SDD artifacts |
| `docs/prd.md` | edit | +25 / −5 | add Calendar tab section; align picker references |
| **Total estimate** | — | **~850–1,100 LOC** | comfortable inside the **3,000-line** user-approved budget |

The lower bound reuses existing CSS variables with minimal new tokens. The upper bound assumes a polished popover with full a11y attributes, decade-grid transitions, and rich day-detail table copy. The 3,000-line budget leaves ample headroom for design tweaks during apply without triggering `size:exception`.

### Tests

#### Automated

- `npx svelte-check --workspace . --threshold error` — must pass with zero new errors.
- `npm run build` — must pass.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` — must remain at the 276 + 2 baseline; **no new Rust tests required** because the slice is frontend-only.

There is no vitest / playwright / cypress harness in this slice. The canonical spec's `Engineering safety > tests accompany implementation` requirement permits manual-verify for frontend changes; this slice documents that posture in `apply-progress.md` and creates the standing backlog entry for a future Svelte component harness (matching the `2026-09-14-caduxo-measurement-unit-options` precedent).

#### Manual smoke (Linux/WebKitGTK + Windows/WebView2)

| # | Flow | Expected |
|---|------|----------|
| M1 | Open LotForm; trigger expires-input; click any day in current month | popover closes; `expiryDate` committed; no `×` icon visible |
| M2 | Open LotForm; focus text input; type `2026-09-30`; blur | `expiryDate` commits; form submits; no red border |
| M3 | Open LotForm; type `2026-13-40`; blur | red border + helper text "Use YYYY-MM-DD"; `expiryDate` unchanged; form submit blocked by JS guard if forced |
| M4 | Open LotForm; type `1899-12-31`; blur | red border + helper text "Year must be 1900–2100" |
| M5 | Open LotForm; click `Today` button in popover | `expiryDate = today`; popover closes |
| M6 | Open LotForm; press `Tab` from trigger; `Tab` again | focus moves to next form field (Alert days), not into popover |
| M7 | Open LotForm; open popover; press `Esc` | popover closes; no calendar selection committed; typed text intact |
| M8 | Open LotForm; open popover; `ArrowLeft` from a focused day | focus moves −1 day, clamped to 1900–2100 |
| M9 | Open LotForm; open popover; press `Enter` on a focused day | that day commits; popover closes |
| M10 | Open ReportsPage; click `Date from`; type `2026-09-30`; blur | `dateFrom` committed; preview filters respect it |
| M11 | Open ReportsPage; click `×` on `Date from` | `dateFrom` clears; buildFilters returns `null` for date_from |
| M12 | Open Calendar tab in main nav | current month visible; today highlighted + selected |
| M13 | Calendar tab; click a day with 3+ expirations | day-detail panel lists the 3+ rows |
| M14 | Calendar tab; click a day with no expirations | day-detail panel shows "No expirations on YYYY-MM-DD" |
| M15 | Calendar tab; click prev-month chevron twice | grid moves two months back; no backend call |
| M16 | Calendar tab; click a lot row in day-detail | opens lot edit overlay (existing pattern) |
| M17 | `grep -R 'type="date"' src/` | returns zero matches |
| M18 | Year picker (in any consumer); click year chip | decade grid shows 12 years; `‹ decade` / `decade ›` work; out-of-range years disabled |
| M19 | Calendar tab; month nav from March → February in non-leap year (e.g., 2025 → 2026 → Feb) | Feb 29 not rendered; cell counts correct (28 days) |
| M20 | Linux/WebKitGTK; open picker | **does not** trigger the GTK native picker (this is the defect fix) |

### Spec delta (locked plan for `apply-progress`)

The apply phase edits `openspec/specs/caduxo-expiry-tracker/spec.md` exactly as follows:

1. **Append** a new `## Capability: Date input` section with one requirement covering:
   - The picker is an in-house Svelte component, not a native `<input type="date">`.
   - ISO `YYYY-MM-DD` in/out; year range 1900–2100 inclusive.
   - Selection closes the popover and emits the value.
   - Manual `YYYY-MM-DD` text input is supported with visual validation and **no silent rewrite**.
   - `clearable` prop contract (default `true`; `LotForm` passes `false`).
   - Basic keyboard ergonomics (Tab / Esc / Enter; arrows + PageUp/Down included).

2. **Edit** `## Capability: Expiry lots > ### Requirement: lot registration` to add a one-line pointer after the field list: "The `expiry date` field uses the `Date input` picker with `clearable={false}`."

3. **Edit** `## Capability: Reports > ### Requirement: report filters` to add a one-line pointer after the filter list: "The `date range` fields use the `Date input` picker (clearable, optional)."

4. **Append** a new requirement under a new `## Capability: Calendar` section (preferred over nesting under `Dashboard` because the page is a top-level nav entry, not a dashboard widget):
   - Opens at the current month; today is highlighted and selected by default.
   - Per-day expirations are visible as dot badges; the count is shown in the day-detail panel.
   - The day-detail panel lists lots + products for the selected date and lets the user open the lot edit flow.

The apply phase records each edit in `apply-progress.md` with a paste of the before/after block (matching the measurement-unit-options precedent). The verify phase grep-confirms no `<input type="date">` and no occurrence of the removed CSS selector.

### Rollout

Single-slice delivery, **no chained PR split**:

- The user explicitly approved a 3,000-line review budget and asked for picker + Calendar tab in one coherent slice.
- The shared primitive split (`CalendarMonth` used by both consumers) keeps the diff structurally reviewable even at ~1,000 LOC.
- The slice ships three new components + three host edits + one nav edit + spec delta + PRD alignment. The implementation tasks are listed in `tasks.md` (separate artifact) in this order:
  1. Build `CalendarMonth.svelte` (primitive, no consumers yet).
  2. Build `DatePicker.svelte` using `CalendarMonth`.
  3. Build `CalendarPage.svelte` using `CalendarMonth`.
  4. Adopt in `LotForm.svelte` (delete `input[type="date"]`, mount `<DatePicker>`).
  5. Adopt in `ReportsPage.svelte` (delete two `input[type="date"]`, mount two `<DatePicker>`).
  6. Wire nav in `src/App.svelte`.
  7. Apply spec delta to `openspec/specs/caduxo-expiry-tracker/spec.md`.
  8. Align `docs/prd.md` Calendar section + picker references.
  9. Run `npx svelte-check --workspace . --threshold error`.
  10. Run `npm run build`.
  11. Record manual smoke evidence (M1–M20 above) in `apply-progress.md`.
  12. Verify `cargo test --lib` baseline unchanged.

Tasks 1–3 land together because the primitive drives both consumers; tasks 4–6 are mechanical adoptions once the primitive + composite are green. Tasks 7–8 are spec/doc alignment that happens during the same apply pass.

### Risks

| Risk | Impact | Mitigation in this design |
|------|--------|---------------------------|
| Total LOC exceeds the 3,000-line budget after CSS polish + manual smoke evidence + apply-progress | Would require `size:exception` mid-apply | Estimate ~1,000 LOC with wide tolerance; manual smoke table is compact; apply-progress is text-only |
| Duplicate month-grid logic between `DatePicker` and `CalendarPage` | Maintenance burden; drift | `CalendarMonth` is the single primitive; both consumers render `<CalendarMonth>` directly. No `buildMonthGrid` extraction to a shared module in v1 (overkill for one function) |
| Manual-input validation silently rewrites user text | Violates explicit user constraint; trust loss | `value` only updates on valid parse or `daySelect`; `displayText` is the input's text; `isInvalid` flag is purely visual. Encoded in the contract table above |
| `clearable={false}` regression in LotForm | User can clear required date; submit silently fails | `×` button is conditionally rendered on `clearable && value !== ""`. LotForm passes `clearable={false}`. JS guard still blocks empty submit |
| 1900–2100 boundary not enforced in manual input | User types `0001-01-01`; picker accepts | `parseIsoDate` rejects out-of-range; year picker disables out-of-range cells; chevrons clamp at extremes |
| Calendar tab ships all active lots to frontend even when only one month is needed | Performance regression at scale | Documented as v1 tradeoff; future slice adds `list_expirations_by_day({ from, to })`. v1 is bounded by `status = 'active'` filter already in `list_dashboard_lots` |
| No frontend test harness | Regression between slices | Manual smoke table (M1–M20) recorded in `apply-progress.md`; standing backlog entry for future Svelte harness matches the measurement-unit-options precedent |
| Popover positioning clips on small windows | Picker unusable | `placeBelow` flip-up math; horizontal clamp `Math.max(8, min(left, vw - popWidth - 8))`; fixed-position + viewport coords avoids scroll-jump; resize/scroll listeners reposition |
| Esc inside text input discards popover interaction unexpectedly | User loses confidence | Esc closes popover without committing selection AND without clearing typed text; the typed text is still subject to blur-validation rules |
| Nav entry reorders existing tabs | Muscle-memory break | Calendar added between Products and Reports per user-confirmed ordering; documented in nav order |
| Spec delta drift between `## Capability: Date input` and existing requirement pointer edits | Inconsistent statements | Single canonical home (`Date input`); the two existing requirements get one-line pointer edits only; verify phase reads both sides and greps for the cross-references |
| Future i18n requires renaming weekday/month strings everywhere | Tedious refactor | Single `LABELS = { weekdays, months }` constant at top of `CalendarMonth.svelte` is the documented extraction point |
| External GitHub/community calendar sample is cited or vendored | Violates explicit user constraint | Proposal research note forbids it; this design never imports third-party code; manual implementation only. Recorded in Engram memory as a guarded decision |
| `caduxo-date-picker-dismissal` wrapper pattern re-appears | Defect returns | No `<input type="date">` rendered anywhere in v1; `grep -R 'type="date"' src/` is in the success criteria and the manual smoke list (M17) |

### Rollback

Rollback is layered and additive-safe (no schema, no migrations):

1. `git revert <merge-sha>` restores the previous state.
2. Calendar tab rollback: remove `"calendar"` from `Tab` union, delete the nav button, delete `CalendarPage.svelte`. Picker adoptions remain.
3. Picker rollback: swap each `<DatePicker bind:value={...} />` back to `<input type="date" bind:value={...} />` in `LotForm.svelte` (with `required`) and `ReportsPage.svelte` (without `required`). Restore the `label input[type="date"]` CSS selector in `LotForm.svelte`. Delete `DatePicker.svelte`.
4. Primitive rollback: delete `CalendarMonth.svelte`. Both consumers are already reverted in steps 2–3.
5. Spec delta rollback: remove the new `## Capability: Date input` section and the `## Capability: Calendar` requirement; revert the pointer edits under `Expiry lots > lot registration` and `Reports > report filters`.
6. The native picker defect returns on Linux after rollback — revert is acceptable only if the new picker introduces a worse regression, not on its own merits.

### Post-MVP backlog (recorded for future slices)

- Svelte component test harness (vitest + @testing-library/svelte or Playwright component testing). This is the standing backlog entry the canonical `Engineering safety > tests accompany implementation` requirement already expects.
- `list_expirations_by_day({ from, to })` backend command so the Calendar tab can server-side filter by month.
- i18n of weekday / month labels via the `LABELS` constant extraction point.
- Date-range picker (single calendar with `from` + `to` selection) to replace the two separate pickers in `ReportsPage.svelte`.
- Settings UI for picker preferences (first day of week, locale, weekend colouring).
- Time-of-day / timezone handling if product expiry ever needs it (not in v1; `expiry_date` is date-only).

## Open questions resolved by this design

The proposal surfaced five product questions in `explore.md §5`. Each is now resolved:

| Question | Resolution |
|----------|------------|
| Year-picker shape (decade grid vs. dropdown vs. stepper) | Decade grid with 12-year pages; `‹ decade` / `decade ›` chevrons; clamp 1900–2100 |
| Trigger element (text input + icon vs. button-only) | Single `<input type="text">` doubles as trigger; adjacent calendar-icon button; both paths open the popover identically |
| `Today` shortcut | Yes — `Today` button in popover footer slot; disabled outside min/max |
| Locale | English-only in v1; single `LABELS` constant as future i18n extraction point |
| Keyboard scope | Tab/Esc/Enter at minimum; arrows + PageUp/Down + Shift+PageUp/Down included (standard calendar expectation) |

No further question round is required before `tasks.md`.

## Definition of done

- `grep -R 'type="date"' src/` returns zero matches.
- `npx svelte-check --workspace . --threshold error` exits 0.
- `npm run build` exits 0.
- `cargo test --lib` baseline (276 pass + 2 pre-existing failures) unchanged.
- Manual smoke M1–M20 recorded in `apply-progress.md`.
- Spec delta applied to `openspec/specs/caduxo-expiry-tracker/spec.md` per the four edits above.
- PRD alignment applied to `docs/prd.md` (Calendar section + picker references).
- Total LOC delta stays inside the 3,000-line review budget.
- Single-slice delivery; no chained PR split.
- No `<input type="date">` in the DOM. The picker defect on Linux/WebKitGTK is fixed because the host WebView's native picker is never invoked.
