# Tasks: Dashboard Expiry Filter Buttons

## Review Workload Forecast

|Field|Value|
|-|-|
| Estimated changed lines | ~190–260 (Rust matcher/tests + 2-line DTO serde fix + small Svelte reactive reload fix) |
| 3000-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | ask-on-risk |
| Chain strategy | single-pr |

```text
Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: single-pr
3000-line budget risk: Low
```

Forecast rationale:

- The original planned matcher change touches `src-tauri/src/services/dashboard.rs`; manual smoke testing proved two minimal Dashboard-only integration fixes are also required: explicit serde renames in `src-tauri/src/dto/dashboard.rs` and reactive reload dependencies in `src/components/DashboardPage.svelte`.
- Production code delta is small: matcher signature/body/call site, two enum attributes, and a small Svelte dependency block.
- Test delta dominates: ≈1 helper (`make_predicate_row`, ≈10 lines) + 6 rewrites + 11 new tests (≈140–180 lines grouped under `#[cfg(test)]`).
- Worst-case total well under the 3000-line review budget. No integration points, no migrations, no generated artifacts.
- Session SDD choice `delivery_strategy=ask-on-risk` does not require a chain decision here — the forecast is unambiguously low risk. If during implementation the diff exceeds ~300 changed lines, the implementer pauses and asks before continuing.

## Source-of-truth references

- Proposal: `openspec/changes/caduxo-dashboard-expiry-filters/proposal.md`
- Spec delta: `openspec/changes/caduxo-dashboard-expiry-filters/specs/caduxo-expiry-tracker/spec.md`
- Design: `openspec/changes/caduxo-dashboard-expiry-filters/design.md`
- Files under change: `src-tauri/src/services/dashboard.rs`, `src-tauri/src/dto/dashboard.rs`, `src/components/DashboardPage.svelte`
- Untouched by this slice (verify at the end): `src/lib/dashboard.ts`, `src-tauri/src/db/repositories/dashboard.rs`, `src-tauri/src/domain/expiry_status.rs`.

## Sequencing note (strict TDD)

`openspec/config.yaml` sets `strictTdd: false`, but the user requested "strict TDD evidence expectations" for this slice. Tasks below follow RED → GREEN → TRIANGULATE → REFACTOR ordering:

- **RED** writes failing tests (the matcher signature change forces a compile-time fail on every existing `matches_preset_*` test, plus runtime fails on the new assertion rows).
- **GREEN** makes the matcher pass by changing signature + arms + the single call site.
- **TRIANGULATE** runs the wider test module + lib + clippy to confirm classifier/sort/counter tests stay green.
- **REFACTOR** cleans up the matcher body and test helper without changing behavior.

## Tasks

### Setup (read-only)

- [x] Confirm runtime changes are limited to the Dashboard quick-filter path (`src-tauri/src/services/dashboard.rs`, `src-tauri/src/dto/dashboard.rs`, `src/components/DashboardPage.svelte`) and that `design.md` §"Affected files" matches reality. <!-- sdd-owner: implementation -->
- [x] Record the baseline test command output for `cargo test --manifest-path src-tauri/Cargo.toml --lib -- services::dashboard::tests` so we can diff against GREEN output. <!-- sdd-owner: implementation -->

### RED — write the failing tests first

- [x] Add a `make_predicate_row(days_remaining: i64, alert_days_before: i32) -> DashboardLotRow` helper inside `mod tests` in `src-tauri/src/services/dashboard.rs` (≈10 lines; mirrors `make_row`'s field shape and sets `urgency = String::new()` so the matcher cannot accidentally read it). Place it next to `make_row` (line ≈195). <!-- sdd-owner: implementation -->
- [x] Rewrite `matches_preset_none_returns_true` (currently lines 249–254) to call the new signature `matches_preset(&row, None)` and assert `true` across five urgency values by varying `make_predicate_row(days_remaining, alert_days_before)` inputs. <!-- sdd-owner: implementation -->
- [x] Rewrite `matches_preset_all_returns_true` (currently lines 257–261) to call `matches_preset(&row, Some(DashboardPreset::All))` and assert `true` across the same five urgency values. <!-- sdd-owner: implementation -->
- [x] Rewrite `matches_preset_expired` (currently lines 264–271) to use `make_predicate_row` with inputs `(-1, 30)`, `(0, 30)`, `(5, 30)` and assert `true, false, false`. <!-- sdd-owner: implementation -->
- [x] Rewrite `matches_preset_today` (currently lines 274–281) to use `make_predicate_row` with inputs `(-1, 30)`, `(0, 30)`, `(1, 30)` and assert `false, true, false`. <!-- sdd-owner: implementation -->
- [x] Rewrite `matches_preset_next7days` (currently lines 284–292) to assert the new range predicate: `make_predicate_row(_, 30)` with `(0, _)`, `(7, _)`, `(8, _)` against `DashboardPreset::Next7Days` returns `true, true, false`; `(25, _)` returns `false`. <!-- sdd-owner: implementation -->
- [x] Rewrite `matches_preset_next30days` (currently lines 296–303) to assert the new range predicate: `make_predicate_row(_, _)` with `(0, 30)`, `(20, 14)`, `(30, 30)`, `(31, 30)` against `DashboardPreset::Next30Days` returns `true, true, true, false`. <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_alert_window_includes_today` after the rewritten matcher tests: `make_predicate_row(0, 14)` against `DashboardPreset::AlertWindow` returns `true`. Locks the resolved decision "today rows DO appear in Alert window when `alert_days_before > 0`". <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_alert_window_excludes_expired`: `make_predicate_row(-1, 14)` against `AlertWindow` returns `false`. <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_alert_window_excludes_outside_window`: `make_predicate_row(30, 14)` against `AlertWindow` returns `false` (outside `[0, alert_days_before]`). <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_alert_window_excludes_zero_alert`: `make_predicate_row(5, 0)` against `AlertWindow` returns `false` (no configured window). <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_alert_window_includes_high_alert`: `make_predicate_row(25, 30)` against `AlertWindow` returns `true` — this is the user-reproducible bug from `proposal.md` §"Concrete failure a user can reproduce today". <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_next_7_days_includes_seven`: `make_predicate_row(7, 30)` against `Next7Days` returns `true` (upper-bound inclusive anchor). <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_next_7_days_excludes_8_to_30`: `make_predicate_row(8, 30)` and `make_predicate_row(25, 30)` against `Next7Days` both return `false`. <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_next_30_days_includes_alert_bucket`: `make_predicate_row(20, 14)` (which classifies as `alert_window` urgency bucket) against `Next30Days` returns `true`. Locks the spec scenario "Next 30 days includes alert-window lots that expire within 30 days". <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_next_30_days_includes_thirty`: `make_predicate_row(30, 30)` against `Next30Days` returns `true` (upper-bound inclusive anchor). <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_next_30_days_excludes_31_plus`: `make_predicate_row(31, 30)` against `Next30Days` returns `false`. <!-- sdd-owner: implementation -->
- [x] Add `matches_preset_negative_falls_out_of_non_expired_ranges`: `make_predicate_row(-3, 30)` against `Today`, `AlertWindow`, `Next7Days`, `Next30Days` returns `false` for all four. Locks the spec scenario "Negative days_remaining falls out of every range except Expired and All". <!-- sdd-owner: implementation -->
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml --lib -- services::dashboard::tests` and confirm the build fails with a signature-mismatch error on `matches_preset(&str, _)` and that every new assertion row fails for the expected reason (compile fail or runtime fail on the predicate). Paste the failing output in the slice notes as RED evidence. <!-- sdd-owner: implementation -->

### GREEN — make the matcher pass

- [x] Rewrite the `matches_preset` signature in `src-tauri/src/services/dashboard.rs` (currently line 75) from `fn matches_preset(urgency: &str, preset: Option<DashboardPreset>) -> bool` to `fn matches_preset(row: &DashboardLotRow, preset: Option<DashboardPreset>) -> bool`. <!-- sdd-owner: implementation -->
- [x] Replace the matcher arms with the range predicates from `design.md` §"Predicate arms": `None | Some(All) => true`; `Expired => row.days_remaining < 0`; `Today => row.days_remaining == 0`; `AlertWindow => row.days_remaining >= 0 && row.alert_days_before > 0 && row.days_remaining <= row.alert_days_before as i64`; `Next7Days => row.days_remaining >= 0 && row.days_remaining <= 7`; `Next30Days => row.days_remaining >= 0 && row.days_remaining <= 30`. <!-- sdd-owner: implementation -->
- [x] Update the single call site in `get_dashboard` (currently `services/dashboard.rs` line 132) from `.filter(|row| matches_preset(&row.urgency, preset))` to `.filter(|row| matches_preset(row, preset))`. Do NOT touch the `filters.urgency`-string → `DashboardPreset` fallback above the call (it still feeds the same `preset` argument). <!-- sdd-owner: implementation -->
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml --lib -- services::dashboard::tests` and confirm all 17 matcher tests pass with zero compile errors. Paste the GREEN output (test count + duration) in the slice notes as GREEN evidence. <!-- sdd-owner: implementation -->

### TRIANGULATE — confirm the rest of the module is unaffected

- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml --lib` and confirm the four `enrich_row_*` tests, `urgency_rank_order`, `count_by_urgency_empty`, and `count_by_urgency_sums_correctly` still pass unchanged. Paste the test summary in the slice notes. <!-- sdd-owner: implementation -->
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml --lib -- services::dashboard::tests` once more and confirm the full module green. <!-- sdd-owner: implementation -->
- [x] Run `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --tests -- -D warnings` and record that it fails on pre-existing accumulated warnings/dead code; confirm the slice introduces no new clippy finding in the changed code path. <!-- sdd-owner: implementation -->
- [x] Confirm the `filters.urgency`-string → `DashboardPreset` fallback in `get_dashboard` (lines ≈117–128) still resolves each accepted snake_case string to the right enum value by reading the block once and noting it in the slice notes. No code change expected. <!-- sdd-owner: implementation -->

### REFACTOR — clean up without changing behavior

- [x] Add a one-line doc comment above the rewritten `matches_preset` describing that it filters by per-row `days_remaining` + `alert_days_before` and MUST NOT read `row.urgency`. <!-- sdd-owner: implementation -->
- [x] If the `make_predicate_row` helper duplicates most of `make_row`, extract a single private builder and reuse it from both helpers. If duplication stays under 8 lines, leave it for a follow-up — behaviour preservation is the only acceptance bar. <!-- sdd-owner: implementation -->
- [x] Re-run `cargo test --manifest-path src-tauri/Cargo.toml --lib -- services::dashboard::tests`, `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --tests -- -D warnings`, and `npm run build` after the cleanup; paste outputs and pre-existing clippy caveat in the slice notes. <!-- sdd-owner: implementation -->
- [x] `git diff --stat` against `main` and confirm: (a) changed runtime files are limited to `src-tauri/src/services/dashboard.rs`, `src-tauri/src/dto/dashboard.rs`, and `src/components/DashboardPage.svelte`, (b) total changed lines ≤ 300, (c) no entry under `src-tauri/src/db/repositories/dashboard.rs`, `src-tauri/src/domain/expiry_status.rs`, or `src/lib/dashboard.ts`. The DTO/Svelte entries are accepted because manual smoke testing proved they are required for the Dashboard quick-filter contract. <!-- sdd-owner: implementation -->

### Success-criteria check (per `proposal.md` §"Success criteria")

- [x] Walk the 12 bullets in `proposal.md` §"Success criteria" one by one in the slice notes, naming the test (or code line) that locks each bullet. This is the human-readable review aid. <!-- sdd-owner: implementation -->

### Parent-owned lifecycle notes (not implementation checkboxes)

- Open the PR against `main` after SDD verify/archive decisions, with a body that links to `proposal.md`, `specs/caduxo-expiry-tracker/spec.md`, and `design.md`, and lists the RED + GREEN + TRIANGULATE + clippy + frontend build outputs from the slice notes. <!-- sdd-owner: parent -->
- Run a bounded review pass on the PR (size: ~190–260 lines, three Dashboard-scope files). Confirm the architecture diagram in `design.md` §"Architecture decision" still matches the diff and that no file outside `src-tauri/src/services/dashboard.rs`, `src-tauri/src/dto/dashboard.rs`, and `src/components/DashboardPage.svelte` was touched. <!-- sdd-owner: parent -->
- After PR merge, archive the change by moving `openspec/changes/caduxo-dashboard-expiry-filters/` into `openspec/changes/archive/` per the project's OpenSpec workflow. <!-- sdd-owner: parent -->
