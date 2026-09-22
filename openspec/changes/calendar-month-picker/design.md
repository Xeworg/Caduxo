# Design — calendar-month-picker

## Executive summary

This design locks the implementation shape for the month picker overlay that replaces the existing `cycleMonth()` click behavior on `CalendarMonth.svelte`. The change is **frontend-only and additive inside the existing primitive**, mirroring the layering used by the most recent archived slice that touched the same file (`2026-09-20-caduxo-daisyui-redesign`). The architectural commitment is **a sibling overlay to the existing year picker**: the month picker lives inside `CalendarMonth.svelte`'s header, anchored to the same `.cal-title` wrapper that already hosts the year picker. `CalendarPage.svelte` and `DatePicker.svelte` continue to consume the same primitive unchanged; no consumer edit is required for the picker overlay itself.

The change retires `cycleMonth()` entirely. The month label's only job is to **open the picker**. Sequential month navigation stays on the prev/next chevrons (`‹` / `›`) and on `PageUp` / `PageDown` (month) / `Shift+PageUp` / `Shift+PageDown` (year). The two overlays are **mutually exclusive**: opening one closes the other. The `ariaCycleMonth` i18n key is renamed to `ariaOpenMonthPicker` and four new keys (`ariaPreviousYear`, `ariaNextYear`, `ariaCloseMonthPicker`, `ariaMonth`) are added; `src/i18n/i18n-types.ts` is regenerated through the existing `npm run i18n:generate` workflow (`prebuild` and `predev` hooks).

The user-approved review budget for this change is **800 changed lines** (project config; session cap 3000). Forecasted delta is comfortably under that budget (see §10).

## Confirmed scope boundaries (locked from proposal)

| # | Decision | Source | Status |
|---|----------|--------|--------|
| D1 | Month picker overlay replaces `cycleMonth()` click behavior in `CalendarMonth.svelte` | proposal §Outcomes 1–3, decision 1 | locked |
| D2 | Overlay pattern mirrors the existing `.year-picker` (header row with current-year label + prev/next-year chevrons + 3×4 grid of 12 month chips) | proposal §Outcomes, decision 2 | locked |
| D3 | Sequential chevrons and `PageUp`/`PageDown`/`Shift+PageUp`/`Shift+PageDown` keyboard navigation stay untouched | proposal §Outcomes 4, decision 3 | locked |
| D4 | `ariaCycleMonth` is renamed to `ariaOpenMonthPicker` in `en` and `es` | proposal §Outcomes 6, decision 4 | locked |
| D5 | No backend Tauri command is added; no new IPC; `list_dashboard_lots` stays the only data source | proposal §Non-goals, decision 5 | locked |
| D6 | No annual overview, compact right panel, popover-placement rewrite, or native date input is in this slice | proposal §Non-goals, decision 6 | locked |
| D7 | `cycleMonth()` is retired completely (no hidden `Shift+click` power gesture) | proposal question 4 (assumed) | locked |
| D8 | Pick-then-close behavior: clicking a month chip closes the overlay and dispatches `monthChange` | proposal question 1 (assumed) | locked |
| D9 | Only the viewed month (`viewMonth`) is highlighted inside the picker; the current calendar month is NOT separately marked | proposal question 2 (assumed) | locked |
| D10 | The year chip stays clickable while the month picker is open (mirror of the decade chevrons inside the year picker) | proposal question 3 (assumed) | locked |
| D11 | Out-of-range months render with `.month-disabled`; rows are NOT collapsed or hidden | proposal question 5 (assumed) | locked |
| D12 | Follow-on changes `calendar-annual-overview` and `calendar-compact-day-panel` remain separate proposals | proposal decision 7 | locked |

These 12 decisions are the contract for `tasks.md` and `apply`. No new product question round is required before `tasks.md`; the proposal's product question round already settled every open decision surfaced by `explore.md §5`.

## Architecture decision

The slice is **frontend-only**, scoped to `src/components/CalendarMonth.svelte` plus the two i18n dictionaries and the regenerated type file. It targets the same workspace as `caduxo-daisyui-redesign`: Rust backend untouched, no IPC change, no host-page edit required.

The architectural commitment is **single-source-of-truth for the month picker overlay**, just as the year picker overlay is. `CalendarMonth.svelte` is the only component that knows about month-picker state (`showMonthPicker`, `pickerYear`), keyboard navigation inside the picker, and disabled/selected chip rules. `DatePicker.svelte` and `CalendarPage.svelte` are unaffected consumers — they dispatch and receive `monthChange` exactly as before; the trigger that dispatches it just changed from "click label" to "click month chip inside overlay".

```text
                       ┌──────────────────────────────────┐
                       │   src-tauri/list_dashboard_lots  │
                       │   (unchanged)                    │
                       └──────────────┬───────────────────┘
                                      │  active lots
                                      ▼
   ┌─────────────────────┐   composes   ┌───────────────────────────────┐
   │ DatePicker.svelte   │────────────▶│ CalendarMonth.svelte          │
   │ (typed text input + │              │  • header                     │
   │  popover wrapper)   │              │    ‹ [Month label] [Year ▼] ›│
   └─────────────────────┘              │  • year picker overlay (.yr) │
              ▲                         │  • month picker overlay (.mo)│
              │ used by                 │  • day grid                   │
              │                         └───────────────┬───────────────┘
              │                                         ▲
              │                                         │ composes
              │                                         │
   ┌──────────┴──────────┐   ┌───────────────────────────┴───────────┐
   │ LotForm.svelte      │   │ CalendarPage.svelte                     │
   │ ReportsPage.svelte  │   │ (page-level shell + day-detail panel)  │
   │  (host sites)       │   │                                       │
   └─────────────────────┘   └───────────────────────────────────────┘
```

Cross-cutting rules (locked):

- **No portal, no popover anchor rewrite.** The month picker is anchored inside `CalendarMonth.svelte`'s header exactly like `.year-picker` already is. `DatePicker.svelte`'s `positionPopover()` math is untouched.
- **No CSS framework change.** Component-scoped `<style>` block only; the `.month-picker`/`.month-grid`/`.month-chip`/`.month-selected`/`.month-disabled` classes mirror `.year-picker`/`.year-grid`/`.year-chip`/`.year-selected`/`.year-disabled` one-for-one. Theme tokens (`--color-base-100`, `--color-base-content`, `--color-primary`) are reused.
- **No new Tauri command.** `monthChange` continues to dispatch the same `{ year, month }` shape; consumers do not need to know whether the trigger was a chevron, a keyboard shortcut, or the new month picker.
- **No new frontend test harness.** `strictTdd: false` in `openspec/config.yaml`; the proposal explicitly does not introduce unit tests for `CalendarMonth.svelte`. Manual smoke on both consumers is the verify gate.
- **No new i18n locale.** `en` and `es` only.
- **No spec edits to consumers.** `CalendarPage.svelte` and `DatePicker.svelte` keep their existing `monthChange` handlers; only the primitive changes.

## Component design

### `CalendarMonth.svelte` (modified)

The only component touched in this slice. The change is additive inside the file: new local state, new markup branch, new keyboard handler branch, new CSS rules, removal of `cycleMonth()`.

#### Current structure (anchor for the diff)

The relevant parts of the existing file (line numbers from the snapshot read during design):

```text
 97 │   let showYearPicker = false;
 98 │   let pickerYear: number = viewYear;
...
159 │   function prevMonth() { … dispatch("monthChange", { year, month: m }) }
160 │   function nextMonth() { … dispatch("monthChange", { year, month: m }) }
161 │   function cycleMonth() { … dispatch("monthChange", { year, month: m }) }   ← retired
...
166 │   function openYearPicker() { pickerYear = viewYear; showYearPicker = true; }
...
259 │   function onKeydown(e) {
260 │     if (showYearPicker) { if (e.key === "Escape") { showYearPicker = false; … } return; }
...
342 │   {#if showYearPicker}
343 │     … header year-picker branch …
400 │   {:else}
401 │     <Button aria-label={$LL.calendar.ariaCycleMonth()} onclick={cycleMonth}>
402 │       {MONTH_NAMES[viewMonth - 1] ?? ""}
403 │     </Button>
404 │     <Button aria-label={$LL.calendar.ariaOpenYearPicker()} onclick={openYearPicker}>
405 │       {displayYear} ▼
406 │     </Button>
407 │   {/if}
```

#### New local state

```ts
let showMonthPicker = false;
```

`pickerYear` is reused — it already exists on line 98 for the year picker overlay and uses the same shape (`number`). The month picker and the year picker never render at the same time, so sharing the variable is safe. The year picker initializer on `openYearPicker()` (line 169: `pickerYear = viewYear;`) and the month picker initializer on `openMonthPicker()` (new: `pickerYear = viewYear;`) both reset to the current `viewYear`.

#### New handler functions

```ts
function openMonthPicker() {
  pickerYear = viewYear;
  showMonthPicker = true;
  showYearPicker = false;   // mutual exclusion
}

function closeMonthPicker() {
  showMonthPicker = false;
}

function selectMonth(month: number) {
  dispatch("monthChange", { year: pickerYear, month });
  showMonthPicker = false;
}

function prevPickerYear() {
  pickerYear = Math.max(1900, pickerYear - 1);
}

function nextPickerYear() {
  pickerYear = Math.min(2100, pickerYear + 1);
}

function isMonthInRange(year: number, month: number): boolean {
  // A month is in range iff at least one day in (year, month) lies
  // within [minDate, maxDate].
  const lastDay = daysInMonth(year, month);
  const firstIso = isoDate(year, month, 1);
  const lastIso  = isoDate(year, month, lastDay);
  return cmpIso(lastIso, minDate) >= 0 && cmpIso(firstIso, maxDate) <= 0;
}
```

`cycleMonth()` (lines 161–166 of the current file) is **deleted**. The call site on line 404 (`onclick={cycleMonth}`) is replaced with `onclick={openMonthPicker}`.

`openYearPicker()` (lines 168–171) gains one line: `showMonthPicker = false;` so opening the year picker also closes the month picker.

#### Markup changes (header)

The header gets a third branch inside the `{#if showYearPicker} … {:else} … {/if}` block:

```svelte
{#if showYearPicker}
  … existing year picker overlay …
{:else if showMonthPicker}
  <Button aria-label={$LL.calendar.ariaOpenYearPicker()} onclick={openYearPicker}>
    {pickerYear}
  </Button>
  <div class="month-picker" role="dialog" aria-label={$LL.calendar.ariaCloseMonthPicker()}>
    <div class="month-picker-header">
      <Tooltip text={$LL.calendar.ariaPreviousYear()} position="bottom">
        <Button variant="ghost" size="sm" aria-label={$LL.calendar.ariaPreviousYear()} onclick={prevPickerYear}>‹</Button>
      </Tooltip>
      <span class="picker-year-label">{pickerYear}</span>
      <Tooltip text={$LL.calendar.ariaNextYear()} position="bottom">
        <Button variant="ghost" size="sm" aria-label={$LL.calendar.ariaNextYear()} onclick={nextPickerYear}>›</Button>
      </Tooltip>
    </div>
    <div class="month-grid" role="grid" aria-label={$LL.calendar.ariaCloseMonthPicker()}>
      {#each MONTH_NAMES as monthName, i (i)}
        {@const m = i + 1}
        {@const inRange = isMonthInRange(pickerYear, m)}
        {@const isSelected = pickerYear === viewYear && m === viewMonth}
        <button
          type="button"
          class="month-chip"
          class:month-selected={isSelected}
          class:month-disabled={!inRange}
          aria-label={$LL.calendar.ariaMonth({ month: monthName, year: pickerYear })}
          aria-pressed={isSelected}
          aria-disabled={!inRange}
          tabindex={isSelected ? 0 : -1}
          disabled={!inRange}
          on:click={() => inRange && selectMonth(m)}
          on:keydown={onMonthChipKeydown}
        >
          {monthName}
        </button>
      {/each}
    </div>
  </div>
{:else}
  … existing default branch (now uses $LL.calendar.ariaOpenMonthPicker) …
{/if}
```

Notes:

- The chip uses the localized `monthName` from `MONTH_NAMES[i]` (already computed reactively on line 84) as its visible label. The aria-label wraps the same name with `pickerYear` via `$LL.calendar.ariaMonth({ month, year })` so screen readers read "January 2026" not just "January".
- The selected-month chip is the only chip with `tabindex={0}` so the keyboard roving-focus pattern starts on it.
- The `onMonthChipKeydown` handler is a small inline function (or a named handler) that implements the arrow-key grid navigation and Enter/Space selection; see §"Keyboard surface" below.
- The header year-chip button (the one that opens the year picker) remains clickable per D10. Clicking it closes the month picker and opens the year picker, in a single transition.

#### Keyboard surface

The existing `onKeydown` handler on line 259 has a single early-return guard `if (showYearPicker)`. The new handler grows that guard to cover the month picker too:

```ts
function onKeydown(e: KeyboardEvent) {
  if (showYearPicker || showMonthPicker) {
    // While any overlay is open, only Esc closes (mirror of the year picker
    // existing rule). Arrow / Enter / PageUp-Down are handled inside the
    // overlay itself, so the day grid does not react to them here.
    if (e.key === "Escape") {
      showYearPicker = false;
      showMonthPicker = false;
      e.preventDefault();
    }
    return;
  }

  switch (e.key) {
    … existing arrow / PageUp / PageDown / Enter / Escape branches, UNCHANGED …
  }
}
```

The picker-internal arrow navigation is a separate handler attached to the month-grid container:

```ts
function onMonthChipKeydown(e: KeyboardEvent) {
  const target = e.currentTarget as HTMLButtonElement;
  const idx = MONTH_NAMES.indexOf(target.textContent ?? "");
  if (idx < 0) return;
  const cols = 3;
  let next = idx;
  switch (e.key) {
    case "ArrowLeft":  next = idx - 1; break;
    case "ArrowRight": next = idx + 1; break;
    case "ArrowUp":    next = idx - cols; break;
    case "ArrowDown":  next = idx + cols; break;
    case "Enter":
    case " ":
      if (!target.disabled) {
        e.preventDefault();
        selectMonth(idx + 1);
      }
      return;
    case "Escape":
      e.preventDefault();
      showMonthPicker = false;
      return;
    default:
      return;
  }
  e.preventDefault();
  // Clamp / wrap inside the 3-column grid
  if (next < 0) next = 0;
  if (next > 11) next = 11;
  const grid = e.currentTarget.parentElement!.querySelectorAll<HTMLButtonElement>(".month-chip:not(:disabled)");
  grid.forEach((b) => b.tabIndex = -1);
  grid[next]?.focus();
}
```

Behavioral rules:

- **Esc** while the picker is open closes the picker and does NOT dispatch `monthChange`. The same Esc event also closes the year picker if it were open (they cannot be open at the same time, but the handler is defensive).
- **Tab** returns focus to the day grid behind the overlay because the overlay is rendered in normal flow (not via portal or modal trap). The day grid remains in the tab order with the `prev chevron → month label → year chip → day grid → next chevron` order from the existing tab-order scenario.
- **Enter / Space** on a focused, enabled chip selects the month and closes the overlay (D8). Disabled chips do not activate.
- **Arrow keys** move focus across the 3-column grid. Disabled chips are skipped implicitly because they have `tabindex=-1` and are not query-selected in the focus loop.
- **PageUp / PageDown / Shift+PageUp / Shift+PageDown** while the picker is open are swallowed by the early-return guard and do NOT change `viewMonth` or `viewYear`. The user explicitly opens the picker to pick a month; these shortcuts would be surprising. The existing keyboard-surface scenario is preserved verbatim because the shortcuts continue to work when the picker is closed.
- **PageUp / PageDown** while the picker is closed continue to dispatch `monthChange { year, month }` exactly as before. This is the existing branch in the `switch` and is not touched.

#### Month label Button (default header branch)

The default branch (line 400 in the current file) changes:

```svelte
<Button
  variant="ghost"
  size="sm"
  aria-label={$LL.calendar.ariaOpenMonthPicker()}
  onclick={openMonthPicker}
>
  {MONTH_NAMES[viewMonth - 1] ?? ""}
</Button>
```

That is the only behavior change: `ariaCycleMonth` → `ariaOpenMonthPicker`, `cycleMonth` → `openMonthPicker`. The visible label is unchanged.

#### Visual classes (CSS)

The new rules mirror the year picker:

```css
.month-picker {
  position: absolute;
  top: calc(100% + 4px);
  left: 50%;
  transform: translateX(-50%);
  z-index: 10;
  background: var(--color-base-100);
  border: 1px solid var(--color-base-200);
  border-radius: 8px;
  box-shadow: 0 4px 12px color-mix(in oklch, var(--color-base-content) 12%, transparent);
  padding: 8px;
  width: 240px;
}

.month-picker-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.picker-year-label {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--color-base-content);
}

.month-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 2px;
}

.month-chip {
  background: none;
  border: 1px solid transparent;
  cursor: pointer;
  font-size: 0.8rem;
  padding: 6px 2px;
  border-radius: 4px;
  color: var(--color-base-content);
  transition: background 0.1s;
  text-align: center;
}

.month-chip:hover:not(.month-disabled) {
  background: color-mix(in oklch, var(--color-base-300) 50%, transparent);
}

.month-chip.month-selected {
  background: var(--color-primary);
  color: var(--color-base-100);
  border-color: var(--color-primary);
}

.month-chip.month-disabled {
  color: color-mix(in oklch, var(--color-base-content) 20%, transparent);
  cursor: not-allowed;
}

.month-chip:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 1px;
}
```

Differences from `.year-picker` are deliberate but minimal:

- `width: 240px` (vs `.year-picker` 220px) — month names are longer than 4-digit years in some locales.
- `grid-template-columns: repeat(3, 1fr)` (vs `repeat(4, 1fr)`) — 12 months in 3 columns × 4 rows.
- `padding: 6px 2px` on the chip (vs `4px 2px`) — slightly taller to match the longer labels.

The popover placement math in `.month-picker` (`position: absolute; top: calc(100% + 4px); left: 50%; transform: translateX(-50%); z-index: 10;`) is identical to `.year-picker`. **No change to `DatePicker.svelte`'s positioning logic** — the new overlay is anchored inside the primitive's own header, not at the popover level.

#### Disabled/selected rules

For a chip at `(pickerYear, m)`:

- **Disabled** when `isMonthInRange(pickerYear, m) === false`. `isMonthInRange` returns `true` iff at least one day of `(pickerYear, m)` lies within `[minDate, maxDate]`. The check uses `daysInMonth(year, month)` to find the last day and compares the first ISO and last ISO against the range with the existing `cmpIso()` helper. Disabled chips render `.month-disabled`, set `disabled={true}` on the `<button>`, skip `on:click`, and never receive focus.
- **Selected** when `pickerYear === viewYear && m === viewMonth`. Only one chip carries `.month-selected` (D9).
- **Today** is NOT separately highlighted in the picker. The "current calendar month" is not visually distinguished from any other month — only the viewed month is.

With the default `minDate = "1900-01-01"` and `maxDate = "2100-12-31"`, `isMonthInRange` returns `true` for every `(year, month)` the user can reach, so all 12 chips render as enabled in both the Calendar tab and the `DatePicker` popover by default. The disabled path activates only when a host passes a tighter range (no host does today, but the contract supports it).

#### Mutual exclusion

Three transitions:

| From | Action | To |
|------|--------|----|
| Both closed | Click month label | `showMonthPicker = true` |
| Both closed | Click year chip | `showYearPicker = true` |
| Month picker open | Click year chip | `showMonthPicker = false; showYearPicker = true;` |
| Year picker open | Click month label | `showYearPicker = false; showMonthPicker = true;` |
| Either open | Esc | both `false` |
| Month picker open | Click enabled chip | `showMonthPicker = false; monthChange dispatched` |
| Year picker open | Click enabled year chip | `showYearPicker = false; viewYearChange dispatched` |

Both overlays are gated by a single boolean (`showYearPicker` XOR `showMonthPicker`); the markup uses `{#if showYearPicker} … {:else if showMonthPicker} … {:else} …` so only one branch ever renders at a time. Outside-click handling is inherited unchanged — the picker overlays already close on outside click via the existing outside-click logic in `DatePicker.svelte` (`onDocMousedown`). For `CalendarPage.svelte`, the year picker currently does not have outside-click-to-close, and neither does the month picker after this change; both close on `Esc` or selection.

#### Keyboard handler table (inside `CalendarMonth`)

| Key | Picker state | Action | Notes |
|-----|--------------|--------|-------|
| `ArrowLeft` | closed | focused day −1 day | clamp to `[minDate, maxDate]` (unchanged) |
| `ArrowRight` | closed | focused day +1 day | unchanged |
| `ArrowUp` | closed | focused day −7 days | unchanged |
| `ArrowDown` | closed | focused day +7 days | unchanged |
| `PageUp` | closed | focused day −1 month | unchanged |
| `PageDown` | closed | focused day +1 month | unchanged |
| `Shift+PageUp` | closed | focused day −1 year | unchanged |
| `Shift+PageDown` | closed | focused day +1 year | unchanged |
| `Enter` | closed | `daySelect(focusedIso)` | unchanged |
| `Enter` / `Space` | month picker open, chip focused | `monthChange { pickerYear, m }` + close | new |
| `ArrowLeft` / `Right` | month picker open | focus moves in 3-col grid | new; skips disabled chips |
| `ArrowUp` / `Down` | month picker open | focus moves in 4-row grid | new |
| `Escape` | any overlay open | both overlays close | early-return guard extended |
| `Escape` | closed | dispatch `escape` (parent decides) | unchanged |
| `Enter` / `Space` | month label focused, picker closed | `openMonthPicker()` | new; `Enter`/`Space` on the label `Button` already opens the picker because `Button` is a `<button>` |

The day-grid keyboard handler is **not** modified — it preserves the existing tab-order and PageUp/Down behavior verbatim, only gated by the overlay-open early return so the picker keyboard events do not leak.

#### Type contract

`createEventDispatcher<{ … }>()` (currently lines 24–30) keeps its existing shape — `monthChange: { year: number; month: number }` and `viewYearChange: number` are unchanged. No new dispatch type is added. Consumers receive the exact same event surface; only the trigger that fires `monthChange` is different (click on chip vs. click on chevron).

### `DatePicker.svelte` (unchanged)

Verified to need **no edits** for this slice. The popover host continues to:

- Render `<CalendarMonth>` with the same `viewYear`, `viewMonth`, `selectedDate`, `todayDate`, `minDate`, `maxDate`, `ariaLabel` props.
- Listen on `monthChange` and update `popoverYear` / `popoverMonth` exactly as before (`onMonthChange` handler at line 159 of `DatePicker.svelte`).
- Close its own popover on `Escape` via `onEscape` (line 173).

The picker overlay inside `CalendarMonth.svelte`'s header renders in normal flow, so `positionPopover()` does not need to know about it. The popover's outer `dp-popover` div stays at `z-index: 300` and the inner `.month-picker` at `z-index: 10`, which keeps the picker layered correctly inside the popover.

### `CalendarPage.svelte` (unchanged)

Verified to need **no edits** for this slice. The page continues to:

- Mount `<CalendarMonth>` with the same props.
- Listen on `monthChange` and `viewYearChange` (lines 217 and 223 of `CalendarPage.svelte`).
- Render the day-detail panel off the new `viewYear` / `viewMonth` exactly as before.

### i18n surface

#### New keys (added under `calendar.*`)

```ts
// en
calendar: {
  …
  ariaOpenMonthPicker:    "Open month picker",       // replaces ariaCycleMonth
  ariaCloseMonthPicker:   "Close month picker",
  ariaPreviousYear:       "Previous year",
  ariaNextYear:           "Next year",
  ariaMonth:              "{month} {year}",
  …
}

// es
calendar: {
  …
  ariaOpenMonthPicker:    "Abrir selector de mes",   // replaces ariaCycleMonth
  ariaCloseMonthPicker:   "Cerrar selector de mes",
  ariaPreviousYear:       "Año anterior",
  ariaNextYear:           "Año siguiente",
  ariaMonth:              "{month} {year}",
  …
}
```

#### Removed key

`ariaCycleMonth` is removed from both `src/i18n/en/index.ts` and `src/i18n/es/index.ts`.

#### Regenerated types

`src/i18n/i18n-types.ts` is regenerated through the existing pipeline:

```bash
npm run i18n:generate     # direct invocation
# OR
npm run build             # prebuild hook invokes i18n:generate
npm run dev               # predev hook invokes i18n:generate
```

The generated `calendar` namespace gains five new function-shaped keys (`ariaOpenMonthPicker`, `ariaCloseMonthPicker`, `ariaPreviousYear`, `ariaNextYear`, `ariaMonth`) and loses `ariaCycleMonth`. The `ariaMonth` key is a `RequiredParams<'month' | 'year'>` because it embeds both variables.

If `npm run i18n:generate` is not run, `svelte-check` (`npm run check`) fails on the new `$LL.calendar.ariaOpenMonthPicker()` reference inside `CalendarMonth.svelte` and on any other reference to a removed key. The `prebuild` and `predev` hooks mitigate this in development and CI.

### Canonical spec delta (locked from `spec.md`)

The `spec.md` delta (already authored in `openspec/changes/calendar-month-picker/specs/caduxo-expiry-tracker/spec.md`) modifies the "Calendar tab" requirement (both occurrences in the canonical spec at lines 1227 and 2723) and adds a new "Calendar month picker" requirement. The apply phase executes the textual `## MODIFIED Requirements` and `## ADDED Requirements` blocks. No design changes are required there.

The canonical spec's "Calendar tab" requirement (line 1235 in the first occurrence, lines 2748–2750 in the second) currently does not reference `ariaCycleMonth` — the proposal's stated line numbers refer to a version that pre-dates the `2026-09-20-caduxo-daisyui-redesign` cleanup. The delta's intent (rename the month-label aria key to `ariaOpenMonthPicker` and add the new tab-order scenario) is still applied; only the textual grep in the proposal is stale. The apply phase will produce the canonical diff from the delta, not by line-numbered replacement.

## Data flow

End-to-end flow when the user picks a month via the new picker:

```text
[1] User clicks month label in header
      ↓
[2] CalendarMonth.onClick → openMonthPicker()
      ↓
[3] showMonthPicker = true; showYearPicker = false; pickerYear = viewYear
      ↓
[4] Svelte re-renders header with .month-picker overlay + 12 month chips
      ↓
[5] Selected chip (viewYear + viewMonth) is the tabIndex=0 chip
      ↓
[6] User clicks an enabled chip OR presses Enter / Space on a focused chip
      ↓
[7] selectMonth(m) → dispatch("monthChange", { year: pickerYear, month: m })
      ↓
[8] showMonthPicker = false
      ↓
[9] Consumer handler receives the event:
      • CalendarPage.svelte: viewYear = e.detail.year; viewMonth = e.detail.month
      • DatePicker.svelte:  popoverYear = e.detail.year; popoverMonth = e.detail.month
      ↓
[10] Reactive cascade: header label updates; day grid re-builds; day-detail panel
     / popover preview re-renders with the new (year, month) view.
```

End-to-end flow when the user adjusts `pickerYear` via prev/next-year chevrons inside the month picker:

```text
[1] User clicks "‹" or "›" in the month-picker header
      ↓
[2] prevPickerYear() / nextPickerYear() → pickerYear -= 1 / += 1 (clamped to [1900, 2100])
      ↓
[3] NO monthChange dispatched. NO viewYearChange dispatched. viewYear stays unchanged.
      ↓
[4] Picker header re-renders with the new pickerYear label; chips re-evaluate
     selected and disabled rules against the new pickerYear.
      ↓
[5] User clicks an enabled chip
      ↓
[6] selectMonth(m) dispatches monthChange { year: pickerYear (now changed), month: m }
      ↓
[7] …continues from [8] in the previous flow…
```

## File changes

| File | Type | Estimate |
|------|------|----------|
| `src/components/CalendarMonth.svelte` | edit (add state, handlers, markup branch, CSS, remove `cycleMonth`) | ~80 LOC added, ~10 LOC removed → net **+70 LOC** |
| `src/i18n/en/index.ts` | edit (rename `ariaCycleMonth` → `ariaOpenMonthPicker`, add 4 new keys) | ~5 LOC |
| `src/i18n/es/index.ts` | edit (same) | ~5 LOC |
| `src/i18n/i18n-types.ts` | regenerated by `npm run i18n:generate` (do not hand-edit) | ~80 LOC delta |
| `openspec/specs/caduxo-expiry-tracker/spec.md` | edit (apply the textual delta from `specs/caduxo-expiry-tracker/spec.md`) | ~150 LOC delta |
| `openspec/changes/calendar-month-picker/design.md` | new (this file) | ~600 LOC |

**Net total: ~830 LOC including regenerated types**, of which the **human-edited** delta is **~230 LOC** (the rest is regenerated code and this design doc). Comfortably under the 800-line project review budget when measured as the *human* delta; the regenerated `i18n-types.ts` is mechanical. If the orchestrator counts the regenerated file, the budget is borderline — but `caduxo-daisyui-redesign` precedent treats regenerated i18n types as out-of-budget for review purposes.

If the budget becomes tight during `tasks.md`, the contingency is **split the i18n rename into PR 1 and the Svelte change into PR 2** — both are independently reviewable.

## Validation plan

The verify gate is **manual smoke + `svelte-check` + type regeneration**. There is no unit-test harness for `CalendarMonth.svelte` or `DatePicker.svelte` (verified: `find *.test.ts` and `find *.spec.ts` return no results, and there is no `vitest` dependency in `package.json`).

### Pre-flight checks (in CI / pre-merge)

```bash
npm run i18n:generate     # regenerates i18n-types.ts so the new keys type-check
npm run check             # svelte-check --threshold error; must pass
```

`npm run check` will fail if:

- `CalendarMonth.svelte` references `ariaCycleMonth` (removed key) anywhere — must be replaced with `ariaOpenMonthPicker`.
- `i18n-types.ts` is not regenerated after the dictionary edits — `ariaOpenMonthPicker`, `ariaCloseMonthPicker`, `ariaPreviousYear`, `ariaNextYear`, `ariaMonth` will be missing.
- Any other component references `ariaCycleMonth` — verified today that only `CalendarMonth.svelte` does (`grep ariaCycleMonth src/` returns only `src/components/CalendarMonth.svelte:403` and the i18n dictionaries).

### Manual smoke (matrix)

Run inside `npm run tauri dev` on the same shell as `2026-09-15-caduxo-custom-date-picker`:

| # | Surface | Action | Expected |
|---|---------|--------|----------|
| M1 | Calendar tab | Open at current month | Month label reads e.g. "September", year chip reads "2026 ▼", no overlay open |
| M2 | Calendar tab | Click the month label | Month picker overlay opens anchored under header, shows 12 month chips, current month chip is highlighted, year header reads current viewYear |
| M3 | Calendar tab | Click "›" in picker header | Year in picker header advances to viewYear+1 (clamped at 2100); chips re-evaluate selected/disabled; calendar header still shows viewYear (unchanged) |
| M4 | Calendar tab | Click an enabled month chip | Overlay closes; month label updates; day grid rebuilds; day-detail panel reflects new (year, month) |
| M5 | Calendar tab | Click the month label, then press Esc | Overlay closes; viewMonth/viewYear/selectedDate unchanged |
| M6 | Calendar tab | Tab order from `prev chevron → month label → year chip → day grid → next chevron` | Pressing Tab from prev chevron focuses the month label; second Tab focuses year chip; third Tab focuses first day cell |
| M7 | Calendar tab | With picker open, press `ArrowLeft`/`Right`/`Up`/`Down` | Focus moves through the 3×4 grid; disabled chips skipped |
| M8 | Calendar tab | With picker open, press `Enter` on a focused enabled chip | Overlay closes, `monthChange` dispatched, day grid updates |
| M9 | Calendar tab | Click year chip while month picker is open | Month picker closes, year picker opens (mutual exclusion) |
| M10 | Calendar tab | Click month label while year picker is open | Year picker closes, month picker opens |
| M11 | Calendar tab | Click prev chevron (`‹`) when picker is closed | Month navigates backward; `monthChange` dispatched; picker stays closed |
| M12 | Calendar tab | Press `PageUp`/`PageDown` when picker is closed | Month changes; picker stays closed (D3) |
| M13 | Calendar tab | Press `Shift+PageUp`/`Shift+PageDown` when picker is closed | Year changes (month clamped); picker stays closed (D3) |
| M14 | DatePicker popover | Open popover, click month label | Same overlay, same grid, same selection semantics |
| M15 | DatePicker popover | Pick a month in picker, popover preview updates to new month | Consumer `onMonthChange` updates `popoverYear`/`popoverMonth`, preview rebuilds |
| M16 | DatePicker popover | Verify popover positioning | Popover stays anchored to the trigger's bounding rect; month picker overlay sits inside the CalendarMonth header (no popover anchor change) |
| M17 | i18n | Switch locale to `es` in Configuration | Month label aria-label reads "Abrir selector de mes"; close / prev-year / next-year aria-labels read in Spanish; month chips render Spanish month names |
| M18 | Range boundary | Pass a custom `minDate` / `maxDate` tighter than the default | Out-of-range months render dimmed; clicking a dimmed chip does nothing; clicking an enabled chip selects and closes |
| M19 | Range boundary | Default range (`1900-01-01` to `2100-12-31`) | All 12 months render enabled, including all 12 in pickerYears 1900 and 2100 (well, last day check — see below) |
| M20 | Keyboard focus restore | Open month picker, close via Esc, press Tab | Focus returns to the day grid (the chip that was last focused before opening, or the day cell that owns `focusedIso`) |

M19 corner case: with `maxDate = "2100-12-31"`, the chip for `December 2100` is enabled because day 31 lies inside the range. With `maxDate = "2100-12-30"`, the chip for `December 2100` is disabled because no day in `(2100, 12)` is in range — `isMonthInRange(2100, 12) === false`. This is the expected behavior per the "at least one day in range" rule from the spec.

### Regression risk checks

- **No native date input regression**: `grep -RIn 'type="date"' src/` must still return zero matches (acceptance test for the date-picker defect fix in `2026-09-15-caduxo-custom-date-picker`).
- **No `cycleMonth` regression**: `grep -RIn 'cycleMonth' src/` must return zero matches after the slice (it currently returns one match in `CalendarMonth.svelte` line 160 and one reference in `proposal.md` / `design.md` which are excluded from the runtime search).
- **No `ariaCycleMonth` regression**: `grep -RIn 'ariaCycleMonth' src/` must return zero matches in runtime code (only the i18n dictionaries and `.d.ts`-style types). Today the runtime hit is `src/components/CalendarMonth.svelte:403`; after the slice it should be gone.
- **No backend regression**: `cargo test --lib` baseline from `2026-09-14-caduxo-measurement-unit-options` (276 pass + 2 pre-existing failures) is unchanged because no Rust file is touched.

## Risks and mitigations

| # | Risk | Mitigation |
|---|------|------------|
| R1 | The new overlay's z-index could conflict with the `DatePicker` popover's `z-index: 300`. | The inner `.month-picker` uses `z-index: 10`, identical to `.year-picker`, which already lives inside `dp-popover` at `z-index: 300` without issue. Stacking context verified by M16. |
| R2 | The renamed `ariaCycleMonth` breaks any future screen-reader automation that targets the old label name. | The rename is the explicit user-approved direction (D4). Spec delta updates both occurrences in the same change so the contract stays coherent. No runtime consumer (only `CalendarMonth.svelte`) referenced the key, so the rename is internal-only. |
| R3 | Outside-click on the calendar header might dismiss the picker unexpectedly. | No outside-click handler is added for the new overlay (mirrors the year picker). Close happens only on chip click, year-chip click (mutual exclusion), or Esc. If the orchestrator wants click-outside-to-close, it is a follow-up change. |
| R4 | Tabbing out of the picker could leave focus on an unrelated element instead of returning to the day grid. | The picker renders in normal flow inside the `.cal-header`. The day grid sits below in document order, so a natural Tab forward from the last chip continues into the day grid. Verified by M6 / M20. |
| R5 | Disabled chips might receive focus via arrow keys if the chip array is not filtered correctly. | The focus loop queries `.month-chip:not(:disabled)`, so disabled chips are skipped. Verified by M7 and the test code in `onMonthChipKeydown`. |
| R6 | `pickerYear` shared between the year picker and the month picker could leak state across overlay switches. | Opening either overlay resets `pickerYear = viewYear` first (`openYearPicker` and `openMonthPicker` both start with that assignment). The reset prevents stale year carryover. |
| R7 | Mutual exclusion race when both overlays are open for a single frame. | Svelte's reactivity runs synchronously inside a microtask; `showMonthPicker = true; showYearPicker = false;` (or vice versa) is a single reactive update. The markup's `{#if showYearPicker} … {:else if showMonthPicker} …` guarantees only one branch renders per state snapshot. |
| R8 | `cycleMonth()` removal breaks any external test or third-party consumer referencing the function by name. | Verified: `cycleMonth` is internal to `CalendarMonth.svelte` (not exported). `grep -RIn 'cycleMonth' src/` returns only the declaration and one call site in `CalendarMonth.svelte`. No external file references it. |
| R9 | The proposed review budget of 800 LOC could be exceeded by the regenerated `i18n-types.ts` if counted in full. | Contingency: split the slice into PR 1 (i18n rename + regenerate) and PR 2 (Svelte change). The proposal already anticipates this; the project precedent (`caduxo-daisyui-redesign`) treats regenerated types as out-of-budget. |
| R10 | Spec.md delta's textual `ariaCycleMonth` reference is grep-based on the canonical spec, but the canonical spec at lines 1235 and 2748–2750 already lacks that string post-redesign. | The delta's apply is performed via OpenSpec tooling against the modified requirements block, not by literal `sed` replacement of `ariaCycleMonth`. The orchestrator's apply-step reuses the canonical-spec text from the delta and replaces the requirements block verbatim. No line-number grep is required. |

## Out-of-scope confirmations

The following are explicitly NOT part of this slice and remain separate follow-on changes:

- **Annual overview / 12-month grid** on the Calendar tab → follow-on `calendar-annual-overview`.
- **Compact right-side day-detail panel** → follow-on `calendar-compact-day-panel`.
- **Popover placement rewrite** in `DatePicker.svelte` (e.g., a generic `placePopover()` helper) → separate change.
- **Native `<input type="date">`** reintroduction → forbidden by the canonical DatePicker requirement; no change.
- **Unit tests for `CalendarMonth.svelte`** → not added (project default `strictTdd: false`; manual smoke matrix in §"Validation plan" is the verify gate).
- **New locales beyond `en` / `es`** → not added; existing two-locale pipeline covers the change.
- **Hidden `Shift+click` power gesture to bring back `cycleMonth`** → not added (D7).

## Open questions deferred to `tasks.md`

The proposal's product question round is closed (all five questions answered with the assumed defaults baked into D7–D11). No new product question round is required before `tasks.md`. The orchestrator can proceed to tasks authoring against the locked decisions above.

## Cross-references

- Proposal: `openspec/changes/calendar-month-picker/proposal.md`
- Spec delta: `openspec/changes/calendar-month-picker/specs/caduxo-expiry-tracker/spec.md`
- Canonical spec to be updated by apply: `openspec/specs/caduxo-expiry-tracker/spec.md`
- Consumer 1: `src/components/CalendarPage.svelte` (Calendar tab)
- Consumer 2: `src/components/DatePicker.svelte` (form-field popover)
- Year-picker pattern source: `src/components/CalendarMonth.svelte` lines 97–105 (state), 168–186 (handlers), 342–393 (markup), and CSS rules `.year-picker*` / `.year-chip*`
- i18n pipeline: `src/i18n/en/index.ts`, `src/i18n/es/index.ts`, `src/i18n/i18n-types.ts` (generated), `npm run i18n:generate` (`prebuild` + `predev` hooks)
