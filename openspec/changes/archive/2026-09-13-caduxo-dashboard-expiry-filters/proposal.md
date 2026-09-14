# Proposal: Fix Dashboard Expiry Filter Buttons

## Summary

The six quick-filter buttons on the Dashboard (All, Expired, Today, Alert window, Next 7 days, Next 30 days) currently filter by **urgency-bucket strings** computed during row enrichment, which makes three of them return the wrong rows. The first implementation confirmed the backend matcher bug, and manual smoke testing exposed two additional Dashboard-only integration defects that prevented the corrected filters from being observable: the Svelte reactive reload block did not depend on the selected quick-filter preset, and Serde's digit-containing enum names expected `next7_days` / `next30_days` instead of the frontend contract `next_7_days` / `next_30_days`. This change keeps the same product scope — Dashboard quick filters only — and fixes the service matcher plus the minimal frontend reload and DTO serde bindings required for the buttons to work end-to-end. No schema changes, no repository SQL changes.

## Problem

The Dashboard's quick-filter bar drives `list_dashboard_lots({ preset })`, which goes through `matches_preset` in `src-tauri/src/services/dashboard.rs`. The current matcher keys off the `urgency` string produced by `classify_urgency_with_alert`. Because that classifier is a **single-bucket** classifier (each row falls into exactly one of `expired | today | alert_window | next_30_days | future`), the buttons that should overlap as ranges end up mutually exclusive, and one button is flat-out wrong:

| Button | Current matcher | What it actually returns | What it should return |
|---|---|---|---|
| All | `true` | All active lots | All active lots — correct |
| Expired | `urgency == "expired"` | Lots where `today > expiry_date` | Lots where `expiry_date < today` — correct |
| Today | `urgency == "today"` | Lots where `expiry_date == today` | Lots where `expiry_date == today` — correct |
| Alert window | `urgency == "alert_window"` | Lots classified `AlertWindow` only — i.e. `alert_days_before < 30 AND days_remaining ≤ alert_days_before`. Lots with `alert_days_before ≥ 30` are classified `next_30_days` and **never appear** here, even though they are inside their own configured window. Today rows are excluded even though `0 ≤ alert_days_before`. | Non-expired lots inside the configured alert window: `days_remaining ≥ 0 ∧ alert_days_before > 0 ∧ days_remaining ≤ alert_days_before` |
| Next 7 days | `urgency == "today" \|\| urgency == "next_30_days"` | **All lots with 0..30 days remaining**, regardless of whether the day count is ≤ 7. This is the most user-visible bug — the button claims "Next 7 days" but shows everything up to a month. | Lots where `0 ≤ days_remaining ≤ 7` |
| Next 30 days | `urgency == "next_30_days"` | Only lots classified `next_30_days`. Lots classified `alert_window` (because their `alert_days_before < 30`) are excluded even though they expire within 30 days. | Lots where `0 ≤ days_remaining ≤ 30` |

Concrete failure a user can reproduce today:

1. Create a product with `default_alert_days_before = 30`.
2. Register a lot expiring in 10 days with `alert_days_before = 30` (the default).
3. Click **Alert window** → the lot is missing, even though it is well inside its own 30-day alert window.
4. Click **Next 7 days** → the lot is included (correct), but so is a second lot expiring in 25 days (incorrect).

The bug is one Rust function. The UI is already correct.

## Goals

- Make every Dashboard quick-filter button return exactly the rows its label promises.
- Express filter logic in `days_remaining` and `alert_days_before`, not in bucket-classification strings.
- Keep the urgency cards on the Dashboard as bucket counts in this change (out of scope to realign).
- Keep the existing active-lot behavior inherited from the repository (`status = 'active'`).
- Keep schema and repository untouched.
- Allow only the minimal Dashboard frontend reload wiring and DTO serde rename needed for the existing quick-filter contract to work end-to-end.
- Update or extend unit tests in `dashboard.rs` to lock the new semantics.

## Non-goals

- No frontend behavior or layout changes beyond making the existing dashboard reload block depend on the selected store, location, and quick-filter preset.
- No DTO shape changes to `DashboardFilters`, `DashboardPreset`, `DashboardLotRow`, `DashboardResponse`, or `UrgencyCounts`; only explicit serde renames for `DashboardPreset::Next7Days` and `DashboardPreset::Next30Days` to preserve the existing frontend contract.
- No SQL changes. The repository already returns active lots ordered by `expiry_date ASC`; that is sufficient.
- No changes to `domain::expiry_status`. The classifier is intentionally a single-bucket classifier and stays that way. Filter ranges are a UI/service-layer concept and live in `matches_preset`.
- No changes to the urgency cards at the top of the Dashboard. They continue to show bucket counts (Expired / Today / AlertWindow / Next30Days). Realignment with filter semantics is a separate follow-up.
- No changes to the Reports page, CSV export wiring, PDF export, i18n strings, notifications, scanner, quick-create flow, or any other surface.
- No new test harness. Existing in-file unit tests in `dashboard.rs` are updated and extended; no end-to-end harness is introduced.
- No performance work. The matcher still runs in-memory after `enrich_row`. Filter cost is O(n) over active lots, identical to today.

## Proposed solution

### Backend: rewrite `matches_preset` against `days_remaining`

In `src-tauri/src/services/dashboard.rs`, change the matcher from a string-equality check to a range check against the already-enriched row. Concretely:

- Replace the signature `fn matches_preset(urgency: &str, preset: Option<DashboardPreset>) -> bool` with `fn matches_preset(row: &DashboardLotRow, preset: Option<DashboardPreset>) -> bool`.
- Replace the arms with the resolved semantics:

  | Preset | Predicate |
  |---|---|
  | `None` / `All` | `true` (repository already filtered to `status = 'active'`) |
  | `Expired` | `row.days_remaining < 0` |
  | `Today` | `row.days_remaining == 0` |
  | `AlertWindow` | `row.days_remaining >= 0 && row.alert_days_before > 0 && row.days_remaining <= row.alert_days_before as i64` |
  | `Next7Days` | `row.days_remaining >= 0 && row.days_remaining <= 7` |
  | `Next30Days` | `row.days_remaining >= 0 && row.days_remaining <= 30` |

- Update the single caller in `get_dashboard` to pass `row` instead of `&row.urgency`.
- The preset fallback in `get_dashboard` that maps `filters.urgency` strings (`"expired"`, `"today"`, `"alert_window"`, `"next_7_days"`, `"next_30_days"`) to `DashboardPreset` values stays in place — it is a separate, unchanged entry point and still works.
- The urgency-card counts (`count_by_urgency` and the `UrgencyCounts` payload) are unchanged. They keep counting by `Urgency` bucket.
- The sort step (`urgency_rank` + `expiry_date ASC`) is unchanged. The default sort is still urgency-first.

### Minimal frontend reload correction

Manual smoke testing showed that changing the active quick-filter preset updated button state but did not reload the table. The reactive block in `src/components/DashboardPage.svelte` only referenced `loading`, so Svelte had no dependency on `activePreset`, `selectedStoreId`, or `selectedLocationId`. The fix is intentionally tiny: reference those three values inside the existing reload block before calling `loadDashboard()`. Button labels, ordering, active styling, and data contracts stay unchanged.

`src/components/DashboardPage.svelte` and `src/lib/dashboard.ts` already define:

- `PRESET_LABELS` mapping `all → "All"`, `expired → "Expired"`, `today → "Today"`, `alert_window → "Alert window"`, `next_7_days → "Next 7 days"`, `next_30_days → "Next 30 days"`.
- `PRESET_ORDER` listing the six buttons in that order.
- `DashboardFilters.preset: DashboardPreset` passed straight through to the Tauri command.

None of this needs to change. The buttons send the right enum values today; the Rust service matcher and reload wiring were the broken parts.

### Minimal DTO serde correction

Manual smoke testing also showed Tauri argument errors for `next_7_days` and `next_30_days`. Root cause: `#[serde(rename_all = "snake_case")]` maps Rust variants with digits to `next7_days` and `next30_days`, while the frontend contract and fallback strings use `next_7_days` and `next_30_days`. The fix adds explicit serde renames to those two enum variants only. No DTO fields or wire types change.

### Why no repository change

- `DashboardFilters`, `DashboardPreset`, `DashboardLotRow`, `DashboardResponse`, `UrgencyCounts` are already the right shape. `days_remaining: i64` and `alert_days_before: i32` are already on `DashboardLotRow` and are populated by `enrich_row`. No new fields.
- The repository already filters `WHERE el.status = 'active'` and orders by `expiry_date ASC`. The "All" preset continues to inherit active-only behavior without any SQL change.

### Test changes

The existing unit tests in `dashboard.rs` are rewritten to lock the new matcher behavior. The signature change forces them all to update.

- **Replace** `matches_preset_expired`, `matches_preset_today`, `matches_preset_next7days`, `matches_preset_next30days`, `matches_preset_none_returns_true`, `matches_preset_all_returns_true` so each calls the new signature and asserts the right predicate against a row with a chosen `days_remaining` and `alert_days_before`.
- **Add** `matches_preset_alert_window_includes_today` — row with `days_remaining = 0` and `alert_days_before = 14` is included.
- **Add** `matches_preset_alert_window_excludes_expired` — row with `days_remaining = -1` and `alert_days_before = 14` is excluded.
- **Add** `matches_preset_alert_window_excludes_outside_window` — row with `days_remaining = 30` and `alert_days_before = 14` is excluded.
- **Add** `matches_preset_alert_window_excludes_zero_alert` — row with `days_remaining = 5` and `alert_days_before = 0` is excluded.
- **Add** `matches_preset_alert_window_includes_high_alert` — row with `days_remaining = 25` and `alert_days_before = 30` is included (this is the bug the user can reproduce today).
- **Add** `matches_preset_next_7_days_excludes_8_to_30` — row with `days_remaining = 8` and `alert_days_before = 30` is excluded from Next 7 days.
- **Add** `matches_preset_next_30_days_includes_alert_bucket` — row with `days_remaining = 20` and `alert_days_before = 14` (classifies as `alert_window`) is included in Next 30 days.
- **Add** `matches_preset_next_30_days_excludes_31_plus` — row with `days_remaining = 31` is excluded from Next 30 days.
- Existing `enrich_row_*` tests and `urgency_rank_*`, `count_by_urgency_*` tests stay — they test the classifier and counters, which are unchanged.

The slice notes will list the test commands run (matching the existing project standard in `openspec/specs/caduxo-expiry-tracker/spec.md`).

## Affected areas

| Area | Change |
|---|---|
| `src-tauri/src/services/dashboard.rs` | Rewrite `matches_preset` to use `days_remaining` + `alert_days_before`. Update its single caller in `get_dashboard`. Update / extend the in-file unit tests. |
| `src/components/DashboardPage.svelte` | Minimal reactive dependency fix so store/location/preset changes reload the table. Buttons, labels, ordering, and binding stay. |
| `src/lib/dashboard.ts` | None. Filters and DTOs stay. |
| `src-tauri/src/dto/dashboard.rs` | Add explicit serde renames for `Next7Days` and `Next30Days`; no DTO shape change. |
| `src-tauri/src/db/repositories/dashboard.rs` | None. Active-only filter and ordering stay. |
| `src-tauri/src/domain/expiry_status.rs` | None. Classifier stays a single-bucket classifier. |
| `openspec/specs/caduxo-expiry-tracker/spec.md` | No spec delta in this proposal. The Dashboard capability already names "expired / today / alert window / next 30 days" as sections; the filter math is implementation detail and does not need a requirement change. A follow-up may add a per-button spec requirement later if the project wants it locked at spec level. |
| Reports, CSV export, PDF export, scanner, notifications, i18n, quick-create, store/location/category UI | None. |

## Behavior rules and edge cases

- **All**: returns every active lot regardless of expiry. Repository already enforces `status = 'active'`, so resolved / archived lots never appear. Partial-resolution lots whose `status` is still `active` (e.g. `quantity = 6` after a 4-unit resolution) remain visible — this is current behavior and stays.
- **Expired**: `days_remaining < 0`. A lot with `expiry_date` two days ago returns 2 rows from this filter.
- **Today**: `days_remaining == 0`. Exactly the calendar day.
- **Alert window**:
  - Includes rows with `days_remaining = 0` when `alert_days_before > 0` (resolved decision: "Today rows DO appear in Alert window when `days_remaining = 0` and `alert_days_before > 0`").
  - Excludes expired rows (`days_remaining < 0`).
  - Excludes rows with `alert_days_before <= 0` (resolved decision: no configured window → not in any window).
  - Includes rows with `alert_days_before >= 30` when `days_remaining` is in `[0, alert_days_before]`. This is the case the current code misses.
- **Next 7 days**: `0 ≤ days_remaining ≤ 7`. Overlaps with Alert window and Next 30 days by design (each button is an independent range).
- **Next 30 days**: `0 ≤ days_remaining ≤ 30`. Includes rows whose urgency bucket is `alert_window` (because their `alert_days_before < 30` made them classify that way). Includes Today and Alert window rows that are within 30 days. Excludes 31+ days.
- **Sort order** within any filtered list: most urgent first by bucket (expired → today → alert_window → next_30_days → future), then by `expiry_date ASC` within each group. The matcher does not affect sort.
- **Urgency card counts**: bucket-classification counts. They intentionally differ from filter row counts after this change. This is accepted for the slice.
- **Per-row enrichment edge case**: `enrich_row` already clamps parse failures to `days_remaining = i64::MAX`, `urgency = "future"`. Such rows fall out of every range filter except All, identical to today.

## Risks

| Risk | Mitigation |
|---|---|
| The matcher signature change breaks the silent fallback path that maps `filters.urgency` strings to `DashboardPreset` if that path ever depends on `matches_preset`. | It does not. The fallback runs before `matches_preset` and only feeds the `preset` argument. The new matcher still accepts `Option<DashboardPreset>` and is null-safe. |
| Users who have memorized "Alert window card count == Alert window button result" will see them diverge. | Accepted scope decision. The four cards stay bucket-counts. A follow-up could add a help icon or realign the cards. |
| The new matcher casts `alert_days_before as i64`; if a lot ever has `alert_days_before = i32::MAX`, casting is still safe and the comparison is still well-defined. | Documented in the test list. Realistic `alert_days_before` values are positive small integers. |
| Existing in-file tests `matches_preset_next7days`, `matches_preset_next30days`, etc. document the **old** (buggy) behavior. Updating them is mandatory; skipping them would leave the bug locked in. | The slice updates them as part of the same change. Verification step is listed in the success criteria. |
| The CSV export path in `src-tauri/src/services/export.rs` (or equivalent) might call `matches_preset` or have its own filter. If it has its own logic, this fix does not propagate. | Out of scope for this slice. If the export has its own copy, it is left alone here and tracked as a potential follow-up. The Dashboard surface itself is fixed. |
| `urgency` strings ("alert_window", "next_30_days") still flow through the response payload and the UI badges. They remain bucket labels on rows and badges; only the matcher stops using them as filter keys. | No UX change to badges. Row badges and table-row highlights still use bucket labels and CSS classes, which is the current contract. |

## Rollback

The change is localized to a single Rust function and its in-file tests:

- Revert `matches_preset` in `src-tauri/src/services/dashboard.rs` to the bucket-string matcher.
- Revert the single call site in `get_dashboard`.
- Revert the test updates.

No SQL migration. No DTO change. No frontend change. No archive process required beyond a normal `git revert` of the slice. The proposal's working tree never touches the frontend, schema, or DTOs, so a no-op diff is possible by reverting only `dashboard.rs`.

## Success criteria

- `matches_preset` is expressed in terms of `days_remaining` and `alert_days_before`, not urgency-bucket strings.
- A lot expiring in 10 days with `alert_days_before = 30` appears in **Alert window** when clicked.
- A lot expiring in 10 days appears in **Next 7 days** when clicked.
- A lot expiring in 25 days does **not** appear in **Next 7 days** when clicked.
- A lot expiring in 25 days with `alert_days_before = 14` (classifies as `alert_window`) appears in **Next 30 days** when clicked.
- A lot expiring in 31 days does not appear in **Next 30 days**.
- A lot with `alert_days_before = 0` does not appear in **Alert window** even when `days_remaining > 0`.
- A lot with `days_remaining = 0` and `alert_days_before > 0` appears in **Alert window**.
- The six buttons, their order, their labels, and the active styling on `DashboardPage.svelte` are unchanged.
- The repository SQL is unchanged and still returns only `status = 'active'` lots ordered by `expiry_date ASC`.
- The urgency cards still show bucket counts.
- In-file unit tests cover all the bullets above and pass.
- Only the minimal Dashboard frontend reload wiring is modified by the slice; no labels, ordering, active styling, or TypeScript contract changes.

## Confirmed product decisions (from question round)

- **Today rows DO appear in Alert window** when `days_remaining = 0` and `alert_days_before > 0`. The "Alert window" button is "non-expired lots inside the configured alert window", and `0 ≤ alert_days_before` holds for any positive alert window.
- **Urgency cards stay as bucket counts** in this slice. They continue to show `Urgency::Expired / Today / AlertWindow / Next30Days` classification counts, intentionally diverging from the filter row counts. Realignment is a separate follow-up, out of scope here.
- **`alert_days_before <= 0` lots are excluded from Alert window.** A lot with no configured window is not inside any window.
- **"All = all active lots" inherits the existing repository behavior** (`WHERE el.status = 'active'`). No SQL change. Partial-resolution lots whose status is still `active` remain visible, which is current behavior and accepted.
- **Scope: Dashboard only.** Reports, CSV / PDF export wiring, i18n, scanner, notifications, quick-create, store/location/category UI, and broad test-harness work are all explicitly out.
