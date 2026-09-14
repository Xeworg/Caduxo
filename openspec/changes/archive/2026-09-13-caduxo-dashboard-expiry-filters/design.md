# Design: Dashboard Expiry Filter Buttons

Scope: Dashboard quick-filter semantics only. Surgical service matcher change plus the two minimal Dashboard-only integration fixes proven necessary by manual smoke testing: reload when the quick-filter preset changes, and deserialize digit-containing preset enum names from the existing frontend contract. No repository, schema, classifier, or urgency-card changes.

## Architecture decision

The fix stays inside `src-tauri/src/services/dashboard.rs`. The matcher moves from
a bucket-string equality check to a per-row range predicate over the already-
enriched row. This keeps the existing layering intact:

```
DashboardPage.svelte    (CHANGED: reactive reload observes store/location/preset)
  └─► commands/dashboard.rs    (thin Tauri adapter — unchanged)
        └─► services/dashboard.rs::get_dashboard
              ├─ DashboardPreset serde       (CHANGED: next_7/next_30 wire names)
              ├─ repo::list_dashboard_lots   (unchanged: WHERE status='active')
              ├─ enrich_row                  (unchanged: sets urgency + days_remaining)
              ├─ count_by_urgency            (unchanged: bucket counts for cards)
              ├─ matches_preset              (CHANGED: row-based predicate)
              └─ urgency_rank + expiry_date sort (unchanged)
```

Dependency direction is preserved: domain (`classify_urgency_with_alert`) still
owns classification; the matcher only consumes the row's already-computed
`days_remaining` and `alert_days_before`. Changing the classifier in the future
MUST NOT change filter behavior (spec requirement: matcher uses per-row day
counts, not bucket strings).

## Affected files

| File | Change |
|---|---|
| `src-tauri/src/services/dashboard.rs` | Rewrite `matches_preset` signature + arms; update the single call site in `get_dashboard`; rewrite the six existing matcher unit tests and add matcher coverage for the corrected ranges. |
| `src/components/DashboardPage.svelte` | Make the existing reactive reload block depend on `selectedStoreId`, `selectedLocationId`, and `activePreset` so changing quick filters reloads table data. |
| `src-tauri/src/dto/dashboard.rs` | Add explicit serde renames for `DashboardPreset::Next7Days` and `DashboardPreset::Next30Days` to accept `next_7_days` and `next_30_days`. |
| Anything else | None. |

The diff is intentionally limited to the Dashboard quick-filter path. Repository, schema, classifier, row DTO shape, labels, and urgency-card behavior are untouched.

## Minimal Rust change

### Matcher signature update

```rust
// BEFORE
fn matches_preset(urgency: &str, preset: Option<DashboardPreset>) -> bool

// AFTER
fn matches_preset(row: &DashboardLotRow, preset: Option<DashboardPreset>) -> bool
```

The function remains private to `services::dashboard.rs`. Its single caller is
the `into_iter().filter(...)` in `get_dashboard`. The call site changes from
`matches_preset(&row.urgency, preset)` to `matches_preset(row, preset)`. The
`Option<DashboardPreset>` shape is unchanged, so the `filters.urgency` →
`DashboardPreset` fallback that feeds it stays valid.

### Predicate arms

```rust
fn matches_preset(row: &DashboardLotRow, preset: Option<DashboardPreset>) -> bool {
    match preset {
        None | Some(DashboardPreset::All)                                  => true,
        Some(DashboardPreset::Expired)                                     => row.days_remaining < 0,
        Some(DashboardPreset::Today)                                       => row.days_remaining == 0,
        Some(DashboardPreset::AlertWindow)                                 => {
            row.days_remaining >= 0
                && row.alert_days_before > 0
                && row.days_remaining <= row.alert_days_before as i64
        }
        Some(DashboardPreset::Next7Days)                                   => {
            row.days_remaining >= 0 && row.days_remaining <= 7
        }
        Some(DashboardPreset::Next30Days)                                  => {
            row.days_remaining >= 0 && row.days_remaining <= 30
        }
    }
}
```

Notes:

- `row.alert_days_before` is `i32`; the cast to `i64` is well-defined for any
  realistic value and for `i32::MAX`. `row.days_remaining` is `i64` already.
- The `>= 0` guards on `Today`, `AlertWindow`, `Next7Days`, and `Next30Days`
  ensure expired rows fall out of every range except `Expired` and `All`,
  matching the spec scenario "Negative `days_remaining` falls out of every
  range except Expired and All".
- `alert_days_before > 0` guards the alert window: lots with no configured
  window are not inside any window.

### What does NOT change

- `enrich_row`: still clamps parse failures to `days_remaining = i64::MAX` and
  `urgency = "future"`; such rows fall out of every range filter except `All`,
  identical to today.
- `count_by_urgency`: still bucket-counts for the urgency cards.
- `urgency_rank` + `expiry_date ASC` sort: still most urgent first within each
  filtered list.
- The `filters.urgency`-string → `DashboardPreset` fallback: still feeds
  `preset` before `matches_preset` is called; the new matcher accepts the same
  enum value.
- The repository: still `WHERE el.status = 'active'` ordered by `expiry_date ASC`.
- The Dashboard UI contract: same button labels/order/active styling; only the existing reload block gains explicit dependencies on store, location, and preset.
- The Dashboard preset wire contract: still `next_7_days` / `next_30_days`; Rust now explicitly accepts those spellings.

## Test strategy

All tests live in the existing `#[cfg(test)] mod tests` block in
`services/dashboard.rs`. They are pure-Rust unit tests on `enrich_row`,
`matches_preset`, `urgency_rank`, and `count_by_urgency`. No new harness, no
DB, no frontend.

The matcher signature change forces every existing `matches_preset_*` test to
update. The classifier tests (`enrich_row_*`, `urgency_rank_*`,
`count_by_urgency_*`) stay byte-identical.

### Test helper

Add a small builder that constructs a row with chosen `days_remaining` and
`alert_days_before` directly, so the matcher tests do not depend on the
current calendar date:

```rust
fn make_predicate_row(days_remaining: i64, alert_days_before: i32) -> DashboardLotRow {
    DashboardLotRow {
        // ...other fields defaulted, matching make_row() shape...
        urgency: String::new(),        // matcher MUST NOT depend on this
        days_remaining,
        alert_days_before,
        ..make_row("2099-01-01", alert_days_before) // expiry irrelevant; matcher ignores it
    }
}
```

The existing `make_row(expiry_date, alert_days)` stays for `enrich_row_*` tests
because those DO depend on the calendar date.

### Test list

Rewrite (signature change forces it; assert correct predicate for the
documented semantics):

| Test | Inputs (`days_remaining`, `alert_days_before`) | Preset | Expected |
|---|---|---|---|
| `matches_preset_none_returns_true` | `(_, _)` for each urgency value | `None` | `true` |
| `matches_preset_all_returns_true` | `(_, _)` for each urgency value | `All` | `true` |
| `matches_preset_expired` | `(-1, 30)`, `(0, 30)`, `(5, 30)` | `Expired` | `t, f, f` |
| `matches_preset_today` | `(-1, 30)`, `(0, 30)`, `(1, 30)` | `Today` | `f, t, f` |

Add (locking the new semantics + the user-reproducible bug):

| Test | Inputs | Preset | Expected | Locks |
|---|---|---|---|---|
| `matches_preset_alert_window_includes_today` | `(0, 14)` | `AlertWindow` | `true` | today rows in alert window when `alert_days_before > 0` |
| `matches_preset_alert_window_excludes_expired` | `(-1, 14)` | `AlertWindow` | `false` | expired excluded |
| `matches_preset_alert_window_excludes_outside_window` | `(30, 14)` | `AlertWindow` | `false` | outside `[0, alert_days_before]` excluded |
| `matches_preset_alert_window_excludes_zero_alert` | `(5, 0)` | `AlertWindow` | `false` | `alert_days_before <= 0` excluded |
| `matches_preset_alert_window_includes_high_alert` | `(25, 30)` | `AlertWindow` | `true` | the user-reproducible bug fix |
| `matches_preset_next_7_days_excludes_8_to_30` | `(8, 30)`, `(25, 30)` | `Next7Days` | `f, f` | cap at 7 days inclusive |
| `matches_preset_next_30_days_includes_alert_bucket` | `(20, 14)` | `Next30Days` | `true` | alert-window-classified lots appear under Next 30 days |
| `matches_preset_next_30_days_excludes_31_plus` | `(31, 30)` | `Next30Days` | `false` | cap at 30 days inclusive |

A small positive set also stays to anchor the upper/lower bounds:

| Test | Inputs | Preset | Expected |
|---|---|---|---|
| `matches_preset_next_7_days_includes_seven` | `(7, 30)` | `Next7Days` | `true` |
| `matches_preset_next_30_days_includes_thirty` | `(30, 30)` | `Next30Days` | `true` |
| `matches_preset_negative_falls_out_of_non_expired_ranges` | `(-3, 30)` | `Today` / `AlertWindow` / `Next7Days` / `Next30Days` | `false` for all four |

### Tests that stay unchanged

- `enrich_row_sets_expired`, `enrich_row_sets_today`, `enrich_row_sets_next_30_days`, `enrich_row_sets_future` — classifier untouched.
- `urgency_rank_order` — sort untouched.
- `count_by_urgency_empty`, `count_by_urgency_sums_correctly` — bucket counter untouched.

## Verification commands

Project-standard Rust command (per `docs/packaging.md` and the archived
`caduxo-expiry-tracker` verify report):

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Plus a targeted run during the slice to keep iteration tight:

```bash
# Matcher unit tests only
cargo test --manifest-path src-tauri/Cargo.toml --lib -- \
  services::dashboard::tests::matches_preset

# Whole dashboard service module
cargo test --manifest-path src-tauri/Cargo.toml --lib -- \
  services::dashboard::tests
```

Lint baseline (no new warnings expected):

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --lib --tests
```

Frontend verification is required because `DashboardPage.svelte` is touched:

```bash
npm run build
```

## Non-goals

- No frontend layout/contract changes. `src/lib/dashboard.ts`, button labels, ordering, active styling, and disabled states stay byte-identical; only `DashboardPage.svelte` reload dependencies change.
- No DTO shape changes. `DashboardFilters`, `DashboardLotRow`, `DashboardResponse`, and `UrgencyCounts` stay byte-identical; `DashboardPreset` only gains explicit serde renames for the two digit-containing variants.
- No SQL changes. `db/repositories/dashboard.rs::list_dashboard_lots` stays
  byte-identical (`WHERE el.status = 'active'` ordered by `expiry_date ASC`).
- No classifier changes. `domain::expiry_status::classify_urgency_with_alert`
  stays a single-bucket classifier. Filter ranges are a service-layer concept
  and live in `matches_preset`.
- No urgency-card realignment. The four cards continue to show bucket counts
  (`Expired` / `Today` / `AlertWindow` / `Next30Days`). Card counts are allowed
  to diverge from filter row counts by design; realignment is a separate
  follow-up.
- No CSV export, PDF export, Reports page, scanner, notifications, i18n,
  quick-create, store/location/category UI, or other surface changes.
- No new test harness. Existing in-file unit tests are updated and extended;
  no end-to-end harness is introduced.
- No performance work. Filter cost remains O(n) over active lots after
  `enrich_row`, identical to today.

## Risks specific to design

| Risk | Mitigation in this design |
|---|---|
| `alert_days_before as i64` cast edge case | Documented above; safe for any realistic value and `i32::MAX`. |
| The `filters.urgency` fallback path now feeds a matcher that ignores the urgency string | Documented above; the fallback only builds `Option<DashboardPreset>` and is null-safe. The string never reaches the matcher. |
| Existing tests that documented the buggy behavior | The rewrite is mandatory; the slice updates them in the same change. Skipping them would lock the bug in. |
| `urgency` strings still flow in `DashboardLotRow` and badges | Unchanged. Badges, CSS classes, and the urgency-card counts continue to use bucket labels. Only the matcher stops using them as filter keys. |

## Rollback

`git revert` of the slice. No SQL migration, no DTO change, no frontend change.
Reverting only `src-tauri/src/services/dashboard.rs` restores the prior
behavior with a clean no-op diff against the rest of the tree.

## Open design decisions

None. The proposal's question round resolved all product decisions:

- Today rows DO appear in Alert window when `days_remaining = 0` and
  `alert_days_before > 0`.
- Urgency cards stay as bucket counts (intentional divergence from filter row
  counts; accepted scope).
- `alert_days_before <= 0` lots are excluded from Alert window.
- "All = all active lots" inherits the repository's `status = 'active'`.
- Scope is Dashboard only.
