# Proposal — calendar-month-picker

## Change metadata

- **Change ID**: `calendar-month-picker`
- **Domain**: `caduxo-expiry-tracker` (single-domain project)
- **Artifact store**: `both` (per session preflight; persisted to OpenSpec and
  Engram topic key `sdd/calendar-month-picker/proposal`)
- **Review budget**: 800 changed lines (project configuration)
- **Delivery strategy**: `auto-chain` (deferred until chaining is selected);
  any phase whose forecast exceeds the 800-line project budget MUST trigger
  `deliveryStrategy: ask-on-risk` before `tasks.md`.
- **Chain strategy**: `deferred` — no chained split planned at proposal time
- **Strict TDD**: `false` (project default; `CalendarMonth.svelte` and
  `DatePicker.svelte` have no unit tests today, and the project does not
  require TDD for UI primitives)
- **Execution mode**: `interactive` — this phase completes only `proposal`;
  `spec`, `design`, and `tasks` must wait for explicit user approval
- **Skill resolution**: `paths-injected` (gentle-ai skill + SDD proposal
  contract from the parent addendum; no SDD-proposal-specific skill was
  indexed in the registry)

## Problem statement

Caduxo reuses a single `CalendarMonth.svelte` primitive in two places: as
the Calendar tab on the main page (`CalendarPage.svelte`) and as the
popover body of the form-field `DatePicker.svelte`. Today, clicking the
month label in the header of that primitive dispatches `monthChange` for
`viewMonth + 1` (with year rollover at December), i.e. it **cycles** the
visible month forward by one instead of opening any picker:

```text
// src/components/CalendarMonth.svelte (current behavior)
function cycleMonth() {
  let m = viewMonth + 1;
  let y = viewYear;
  if (m > 12) { m = 1; y += 1; }
  dispatch("monthChange", { year: y, month: m });
}
```

This is confusing for users because:

1. The label is rendered as a `Button` (visually a clickable affordance),
   yet its click effect is "advance by one month", not "pick a month". The
   affordance and the effect do not match, which is exactly the user
   confusion reported during exploration.
2. The existing year chip right next to it does open a real picker (a 4×3
   grid of years inside the same header), so users naturally expect the
   month label to behave the same way: open a month picker.
3. To reach a far month (e.g. from January to October) the user has to
   click the month label eleven times or click the next-month chevron
   eleven times. The selective `DatePicker` use case has no fast-path
   forward.

The fix is to turn the month label into a real **month picker trigger**
that mirrors the existing year-picker pattern: click → overlay opens →
click a month → overlay closes and the new (year, month) is dispatched.
Sequential chevron navigation, keyboard month navigation, and the year
picker overlay stay untouched so the change is purely additive on top of
the existing primitive.

## Outcomes (success criteria)

After this change ships, a Caduxo user can:

1. Click the month label inside any `CalendarMonth` instance (Calendar
   tab or `DatePicker` popover) and see a **month picker overlay** that
   lists all 12 months, not advance by one month.
2. Click a month inside the picker to jump directly to it: the overlay
   closes, the calendar header and day grid both update to the chosen
   month, and the day-detail panel (`CalendarPage`) or the popover
   preview (`DatePicker`) reflects the new view.
3. Reach a far month in **one click**, not twelve.
4. Still use the existing prev/next chevrons (`‹` / `›`) and the keyboard
   shortcuts `PageUp` / `PageDown` (month) and `Shift+PageUp` /
   `Shift+PageDown` (year) for sequential navigation — they keep the
   primitive's previous behavior and the existing aria labels
   (`ariaPreviousMonth`, `ariaNextMonth`) untouched.
5. Open and close the month picker with the keyboard: `Enter` / `Space`
   on the month label opens it, `Esc` closes it (no selection change if
   cancelled), `←` / `→` / `↑` / `↓` move focus inside the 3×4 month
   grid, `Enter` selects and closes, `Tab` returns to the day grid.
6. See a screen-reader-friendly label change: the existing
   `ariaCycleMonth` aria-label ("Cycle month" / "Cambiar mes") becomes
   `ariaOpenMonthPicker` ("Open month picker" / "Abrir selector de mes"),
   matching the year-picker's `ariaOpenYearPicker` naming.
7. Get help text in both English and Spanish for every new affordance
   (open, close, previous year, next year, month cell) through the
   existing `LL.calendar.*` i18n surface, regenerating
   `src/i18n/i18n-types.ts` so the type system stays in lockstep with
   `en` and `es`.

## Scope (in scope)

- Replace `cycleMonth()` in `src/components/CalendarMonth.svelte` with a
  `month-picker` overlay that mirrors the existing `.year-picker`
  pattern: a header row with the current year (read-only label, not a
  picker) plus previous-year / next-year chevrons, and a 3×4 grid of 12
  month chips below.
- Update the header markup so the month label button now opens the month
  picker on click and on `Enter` / `Space`; the existing chevrons (`‹` /
  `›` for prev/next month) keep calling `prevMonth()` / `nextMonth()`.
- Mutually exclusive overlay state: opening the month picker closes the
  year picker if it was open, and vice versa. Both can no longer be open
  at the same time, so focus management and outside-click handling stay
  simple.
- Keyboard surface inside the month picker:
  - `Enter` / `Space` on a month chip → select the month and close.
  - `←` / `→` / `↑` / `↓` → move focus through the 3×4 grid.
  - `Esc` → close without changing the month.
- Visual differentiation inside the month picker:
  - The chip for the **current** `viewMonth` uses `.month-selected` (same
    visual contract as `.year-selected`).
  - Out-of-range months (those that would land outside
    `[minDate, maxDate]`) render with `.month-disabled` (mirrors
    `.year-disabled`).
- i18n surface changes:
  - Rename `LL.calendar.ariaCycleMonth` → `LL.calendar.ariaOpenMonthPicker`
    in both `src/i18n/en/index.ts` and `src/i18n/es/index.ts`.
  - Add new keys under `LL.calendar.*`:
    - `ariaPreviousYear` ("Previous year" / "Año anterior"),
    - `ariaNextYear` ("Next year" / "Año siguiente"),
    - `ariaCloseMonthPicker` ("Close month picker" / "Cerrar selector de mes"),
    - `ariaMonth` ("{month} {year}" / same pattern, reuses
      `MONTH_NAMES`).
  - Regenerate `src/i18n/i18n-types.ts` so the keys stay type-checked
    across `en` and `es`.
- Spec edits to `openspec/specs/caduxo-expiry-tracker/spec.md`:
  - Replace the two `ariaCycleMonth` references inside the "Calendar
    tab" requirement (occurrences at lines 1235 and 2749–2750) with
    `ariaOpenMonthPicker`.
  - Update the tab-order wording inside the same requirement so the
    month label is described as a **trigger for a month picker** rather
    than a `cycleMonth` button.
  - Add a new Scenario under the "Calendar tab" capability:
    "clicking the month label opens a month picker grid that lists all
    12 months and dispatches `monthChange` on selection".

## Non-goals

This change does NOT:

- Add a new backend Tauri command — the existing `list_dashboard_lots`
  payload is the only data source, no new IPC.
- Replace the calendar primitive with a native `<input type="date">` —
  the existing spec already forbids that.
- Touch the popover positioning logic in `DatePicker.svelte`
  (`placePopover()`-style anchors); the month picker is anchored inside
  the existing `CalendarMonth.svelte` header, not at the popover level.
- Redesign the Calendar tab as a full annual overview or 12-month grid
  — that is the follow-on change `calendar-annual-overview`.
- Reorganize the day-detail panel on the right side of the Calendar tab
  into a more compact form — that is the follow-on change
  `calendar-compact-day-panel`.
- Refactor `CalendarMonth.svelte` into a generic multi-mode component
  beyond what the month picker needs.
- Add new locales beyond `en` / `es`.
- Add unit tests for `CalendarMonth.svelte` — the project default is
  `strictTDD: false`, there is no test harness for these components
  today, and the proposal stays under the review budget without them.
  Manual smoke on both Calendar tab and `DatePicker` covers the slice.

## Confirmed product decisions

| #   | Decision                                                                                                          | Source                                                                              |
| --- | ----------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| 1   | First slice is a month picker overlay that replaces `cycleMonth` click behavior                                  | Exploration conclusion (this change is the recommended first slice of the broader calendar usability work) |
| 2   | The month picker mirrors the year-picker overlay pattern already present in `CalendarMonth.svelte`               | Exploration conclusion                                                               |
| 3   | Existing chevrons and keyboard month navigation (`PageUp` / `PageDown`, `Shift+PageUp` / `Shift+PageDown`) are preserved as-is | Exploration conclusion                                                               |
| 4   | `ariaCycleMonth` is renamed to `ariaOpenMonthPicker` in both `en` and `es`                                         | Exploration conclusion                                                               |
| 5   | No backend Tauri command is added in this slice                                                                   | Exploration non-goal                                                                 |
| 6   | No annual overview, compact right panel, popover placement rewrite, or native date input are in this slice        | Exploration non-goal                                                                 |
| 7   | Follow-on changes (`calendar-annual-overview`, `calendar-compact-day-panel`) remain separate proposals           | Exploration conclusion                                                               |

## Proposed capabilities

This proposal modifies one capability and adds one new piece of behavior
on top of the existing calendar primitive.

### Modified capability: Calendar primitive (`CalendarMonth.svelte`)

- The header's month label button stops calling `cycleMonth()` and
  instead toggles a `showMonthPicker` state local to the component.
- When `showMonthPicker` is true, the header renders a `.month-picker`
  overlay under the same anchor as the existing `.year-picker` overlay:
  previous-year chevron, current-year label, next-year chevron, 3×4
  month chip grid.
- Clicking a month chip dispatches `monthChange { year: pickerYear,
  month }` and sets `showMonthPicker = false`.
- `showYearPicker` and `showMonthPicker` are mutually exclusive: opening
  one closes the other, so only one overlay is rendered at a time.
- The day grid, weekday header, badge dots, today highlight, selection
  ring, focused ring, footer slot (`<slot name="footer" />`), aria
  label, and disabled-day handling are unchanged.
- Keyboard handler `onKeydown` extends its existing `if (showYearPicker)`
  early-return so it also handles the month picker: only `Esc` closes
  when an overlay is open, mirroring the year-picker's existing behavior.

### Modified capability: i18n (`src/i18n/en/index.ts`, `src/i18n/es/index.ts`)

- `calendar.ariaCycleMonth` is renamed to `calendar.ariaOpenMonthPicker`
  with new English / Spanish strings that match the year-picker wording.
- New keys added under `calendar.*` for the month-picker's previous-year,
  next-year, close, and per-month aria labels.
- `src/i18n/i18n-types.ts` is regenerated by the existing
  `npm run i18n:types` (or equivalent) workflow.

### Modified capability: Calendar tab spec

- The "Calendar tab" requirement in
  `openspec/specs/caduxo-expiry-tracker/spec.md` is updated in both
  occurrences (lines 1227–1290 and 2721–2838) to reflect the new
  month-picker trigger, the renamed aria label, and the new scenario
  "clicking the month label opens a month picker grid".

## Risks and considerations

- **Cross-consumer surface**: `CalendarMonth.svelte` is consumed by both
  `CalendarPage.svelte` and `DatePicker.svelte`. Both consumers already
  listen for `monthChange` and `viewYearChange`, so the dispatch surface
  is unchanged. The risk is that the month picker overlay's z-index,
  focus management, and outside-click handling need to behave
  identically inside the popover context (`DatePicker` wraps the
  primitive inside `dp-popover`). Mitigated by anchoring the overlay
  inside the primitive's own header the same way `.year-picker` already
  does, and by keeping the mutual-exclusion rule with `showYearPicker`.
- **Rename of `ariaCycleMonth`**: this is a soft break for any future
  screen-reader automation that targets the label by its old name. The
  spec edit replaces both occurrences in the same change so the contract
  stays coherent. The dispatch event `monthChange` is unchanged, so any
  consumer that listens to the event is unaffected.
- **Range boundaries**: out-of-range months (those that would land
  outside `[minDate, maxDate]`) render as disabled chips, mirroring the
  year picker's existing `.year-disabled` contract. The default range
  (`1900-01-01` to `2100-12-31`) renders all 12 months enabled, so the
  default Calendar and `DatePicker` views look identical to today.
- **Review budget**: the proposal's touched files are
  `src/components/CalendarMonth.svelte` (~60 LOC), the two i18n files
  (~10 lines of renames and additions), the regenerated
  `i18n-types.ts`, and two scoped edits to `spec.md`. Estimated total
  well under the 800-line project budget, so `ask-on-risk` is not
  expected to trigger here.
- **Follow-on coordination**: the two follow-on changes
  (`calendar-annual-overview`, `calendar-compact-day-panel`) are out of
  scope but referenced explicitly so the orchestrator can schedule
  them separately. The annual-overview change will eventually require a
  larger rewrite of the "Calendar tab" requirement; the scoped edits in
  this proposal keep that rewrite tractable.

## Next phase

After user approval of this proposal, the next phase writes the delta
spec for `caduxo-expiry-tracker` covering:

1. The "Calendar tab" requirement update (both occurrences) with the
   renamed aria label, the new tab-order wording, and the new
   "clicking the month label opens a month picker grid" scenario.
2. A new capability "Calendar month picker" that codifies the overlay,
   keyboard surface, range handling, and mutual exclusion with the year
   picker.

Then `design.md` and `tasks.md` follow once the spec is approved.

## Proposal question round

This proposal is complete on the slices the orchestrator confirmed
(month picker mirrors year picker, preserve chevrons and keyboard
month nav, i18n wording updates, no backend / native input / popover
rewrite). The product questions below are intended to surface edge
cases and decision gaps that are **not** part of the confirmed slice but
matter for the spec phase. If you would rather skip this round and
approve the proposal as written, the spec can be written against the
assumptions listed; if any answer is "no" or "different", I will revise
the proposal before the spec phase.

1. **Pick-then-close vs stay-open behavior.** When the user clicks a
   month chip in the picker, should the overlay close immediately and
   dispatch `monthChange`, or should the overlay stay open so the user
   can compare another month before committing (only closing on outside
   click or `Esc`)? The proposed default is **pick-then-close**, which
   matches the year picker's behavior and keeps the click count low.
   _Assumed default: pick-then-close._
2. **Current-month visual differentiation.** Should the month picker
   highlight only the currently **viewed** month (selected style), or
   also visually distinguish the **today** month if it falls in the
   same year? The year picker only marks the selected year, so the
   proposed default mirrors that and only highlights the viewed month.
   _Assumed default: highlight only `viewMonth`._.
3. **Year chip while the month picker is open.** When the month picker
   is open, the year chip button currently sits next to the picker
   header. Should the year chip stay clickable from inside the month
   picker (so users can jump year-by-year without closing first), or
   should it be disabled while the month picker is open? The proposed
   default is **stay clickable**, mirroring the existing decade
   chevrons inside the year picker (the year label inside the year
   picker overlay is itself a year-picker re-trigger; same shape here).
   _Assumed default: year chip stays clickable._
4. **`cycleMonth` retirement.** Should the old "click the label to
   advance by one month" behavior survive as a hidden power gesture
   (e.g. `Shift+click` on the label), or be retired completely? The
   proposed default is **retire completely**: the label's only job is
   to open the picker, and sequential navigation lives in the chevrons
   and `PageUp` / `PageDown`. _Assumed default: retire completely._
5. **Range boundary UX.** When `minDate` / `maxDate` are tighter than
   the default range (e.g. a `DatePicker` instance restricted to a
   single calendar year), the picker should disable out-of-range
   months. Should we also visually group the disabled months (e.g.
   fade the whole row that contains a disabled month) or just dim the
   individual chips? The proposed default matches the year picker:
   dim each disabled chip individually, keep the row layout intact.
   _Assumed default: dim individual chips, no row-level grouping._

After the first answers, the proposal assumptions list updates and I ask
whether you want corrections or a second question round before the spec
phase.
