# Delta for caduxo-expiry-tracker

## ADDED Requirements

### Requirement: Dashboard quick-filter buttons return promised rows

The Dashboard quick-filter bar SHALL provide six buttons in the fixed order `All`, `Expired`, `Today`, `Alert window`, `Next 7 days`, `Next 30 days`. For each button, the dashboard's `lots` array MUST contain exactly the active lots whose enriched row satisfies the corresponding predicate below. The predicate is computed from the row's `days_remaining` (an `i64` day count where `< 0` means expired, `0` means today, `> 0` means future) and `alert_days_before` (an `i32`). The repository continues to enforce `WHERE el.status = 'active'`, so resolved and archived lots are not in scope regardless of preset.

| Button | Predicate on the enriched row |
|---|---|
| `All` | `true` (no additional filter; the row is already active) |
| `Expired` | `row.days_remaining < 0` |
| `Today` | `row.days_remaining == 0` |
| `Alert window` | `row.days_remaining >= 0 && row.alert_days_before > 0 && row.days_remaining <= row.alert_days_before as i64` |
| `Next 7 days` | `row.days_remaining >= 0 && row.days_remaining <= 7` |
| `Next 30 days` | `row.days_remaining >= 0 && row.days_remaining <= 30` |

The predicates are intentionally independent ranges. A row may satisfy zero, one, or several predicates. Each button is its own filter.

#### Scenario: All returns every active lot

- GIVEN active lots with `days_remaining` in `{-2, 0, 5, 31}`
- WHEN the user activates the "All" button
- THEN the dashboard lists every active lot
- AND the row set is identical to the unfiltered repository result for the same store/location filters

#### Scenario: Expired returns only lots past their expiry date

- GIVEN active lots with `days_remaining` in `{-2, 0, 5}`
- WHEN the user activates the "Expired" button
- THEN the dashboard lists only the lot with `days_remaining = -2`
- AND lots with `days_remaining >= 0` are NOT listed

#### Scenario: Today returns only lots expiring today

- GIVEN active lots with `days_remaining` in `{-1, 0, 1}`
- WHEN the user activates the "Today" button
- THEN the dashboard lists only the lot with `days_remaining = 0`

#### Scenario: Alert window includes lots inside their configured window including today

- GIVEN active lots with `(days_remaining, alert_days_before)` of `(0, 14)`, `(10, 30)`, `(25, 30)`, and `(-1, 14)`
- WHEN the user activates the "Alert window" button
- THEN the dashboard lists the lots with `days_remaining` in `{0, 10, 25}`
- AND the expired lot with `days_remaining = -1` is NOT listed
- AND a lot with `days_remaining = 0` and `alert_days_before = 14` IS listed

#### Scenario: Alert window excludes lots with no configured alert window

- GIVEN an active lot with `days_remaining = 5` and `alert_days_before = 0`
- WHEN the user activates the "Alert window" button
- THEN the lot is NOT listed
- AND a lot with `days_remaining = 5` and `alert_days_before = 6` IS listed

#### Scenario: Next 7 days returns lots within seven calendar days inclusive

- GIVEN active lots with `days_remaining` in `{0, 5, 7, 8, 25}`
- WHEN the user activates the "Next 7 days" button
- THEN the dashboard lists only the lots with `days_remaining` in `{0, 5, 7}`
- AND a lot with `days_remaining = 25` is NOT listed even when its `alert_days_before` is `30`

#### Scenario: Next 30 days includes alert-window lots that expire within 30 days

- GIVEN an active lot with `days_remaining = 20` and `alert_days_before = 14` (which classifies as the `alert_window` urgency bucket)
- WHEN the user activates the "Next 30 days" button
- THEN the lot IS listed
- AND a lot with `days_remaining = 31` is NOT listed

#### Scenario: Negative days_remaining falls out of every range except Expired and All

- GIVEN an active lot with `days_remaining = -3`
- WHEN the user activates any of "Today", "Alert window", "Next 7 days", or "Next 30 days"
- THEN the lot is NOT listed under that filter
- AND the lot IS listed under "Expired" and under "All"

### Requirement: Dashboard preset matcher uses per-row day counts, not bucket strings

The Dashboard preset matcher MUST evaluate the row's `days_remaining` and `alert_days_before` fields. The matcher MUST NOT rely on the row's `urgency` bucket string as a filter key. Changing the classifier's bucket boundaries MUST NOT change the rows returned by any quick-filter button.

#### Scenario: matcher signature uses the enriched row, not the urgency string

- GIVEN the matcher implementation
- WHEN it is invoked during `get_dashboard` for each enriched row
- THEN it receives the enriched `DashboardLotRow` and an `Option<DashboardPreset>`
- AND it returns `true` when the row satisfies the predicate for that preset per the requirement above
- AND the result does not depend on the value of `row.urgency`

### Requirement: Dashboard urgency cards remain bucket counts

The four urgency cards on the Dashboard MUST continue to display counts derived from the row's `Urgency` classification (`Expired` / `Today` / `AlertWindow` / `Next30Days`). The card counts are independent of the quick-filter row sets and MAY differ from them.

#### Scenario: card counts are derived from the urgency bucket

- GIVEN an active lot with `days_remaining = 10` and `alert_days_before = 30` (which classifies as the `next_30_days` bucket because `alert_days_before >= 30`)
- WHEN the dashboard loads with no filter active
- THEN the "Next 30 days" card count includes this lot
- AND the "Alert window" card count does NOT include this lot
- AND activating the "Alert window" button still lists this lot because `days_remaining` is in `[0, alert_days_before]`

#### Scenario: card counts may diverge from filter row counts by design

- GIVEN active lots whose `alert_days_before >= 30` classify them as `next_30_days` even when they expire within their own window
- WHEN the dashboard loads
- THEN the "Next 30 days" card count includes those lots
- AND the "Alert window" filter row set also includes those lots
- AND the two numbers are allowed to differ; this divergence is accepted scope and is not a defect

## MODIFIED Requirements

### Requirement: operational main screen

Caduxo shall provide a dashboard focused on expiry actions.

It shall include:

- expired lots
- lots expiring today
- lots inside alert window
- next 30 days
- scan/search field
- store filter

The dashboard SHALL also surface two additional quick-filter buttons in its filter bar: "Next 7 days" and "All". Each named section in the list above MUST correspond to the row set defined for the matching quick-filter button in the `Dashboard quick-filter buttons return promised rows` requirement; activating the matching button MUST return exactly that section's rows.

(Previously: the dashboard listed four sections by name without specifying the exact row set per section or the existence of the "Next 7 days" and "All" buttons.)

#### Scenario: dashboard sections align with the matching filter buttons

- GIVEN the dashboard is loaded with no preset active
- WHEN the user activates each of the six filter buttons in turn
- THEN the row set displayed for "Expired" matches the expired-lots section
- AND the row set displayed for "Today" matches the lots-expiring-today section
- AND the row set displayed for "Alert window" matches the lots-inside-alert-window section
- AND the row set displayed for "Next 30 days" matches the next-30-days section
- AND the row set displayed for "Next 7 days" is the seven-day subset of "Next 30 days"
- AND the row set displayed for "All" is the unfiltered active-lot set

## REMOVED Requirements

None.
