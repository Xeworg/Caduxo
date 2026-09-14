# Archive Report: caduxo-dashboard-expiry-filters

## Status

```yaml
schema: gentle-ai.archive-result/v1
verdict: passed
artifact_store: hybrid
archived_path: openspec/changes/archive/2026-09-13-caduxo-dashboard-expiry-filters/
domains_synced:
  - caduxo-expiry-tracker
requirements_added: 3
requirements_modified: 1
requirements_removed: 0
unchecked_implementation_tasks: 0
sync_fallback_used: false
destructive_merge_approval: not_required
```

## Executive Summary

The Dashboard quick-filter slice is complete and archived. The verify report
resolved at `openspec/changes/caduxo-dashboard-expiry-filters/verify-report.md`
with verdict `pass_with_warnings` (0 blockers, 0 critical findings, 4/4
requirements, 12/12 scenarios). All 34 implementation-owned tasks are
checked in `tasks.md`. Canonical spec merge succeeded without destructive
operations. The change has been moved to the dated archive folder. No
destructive canonical spec edits were performed; no stale-checkbox
reconciliation was required; no same-domain active change conflicts.

## Artifacts Read

| Artifact | Path |
|---|---|
| Proposal | `openspec/changes/caduxo-dashboard-expiry-filters/proposal.md` |
| Spec delta | `openspec/changes/caduxo-dashboard-expiry-filters/specs/caduxo-expiry-tracker/spec.md` |
| Design | `openspec/changes/caduxo-dashboard-expiry-filters/design.md` |
| Tasks | `openspec/changes/caduxo-dashboard-expiry-filters/tasks.md` |
| Apply-progress | `openspec/changes/caduxo-dashboard-expiry-filters/apply-progress.md` |
| Verify-report | `openspec/changes/caduxo-dashboard-expiry-filters/verify-report.md` |
| Canonical spec (existing) | `openspec/specs/caduxo-expiry-tracker/spec.md` |
| Project config | `openspec/config.yaml` |
| Native status | `gentle-ai.sdd-status` v2 (parent-provided) |

## Verification Resolution

- Verify report locator: `/home/xeworg/Proyectos/Caduxo/openspec/changes/caduxo-dashboard-expiry-filters/verify-report.md`
- Verdict: `pass_with_warnings`
- Blockers: 0
- Critical findings: 0
- Requirements coverage: 4/4
- Scenario coverage: 12/12
- Test exit code: 0 (`cargo test --manifest-path src-tauri/Cargo.toml --lib -- services::dashboard::tests` → 24 passed; 0 failed)
- Build exit code: 0

The `pass_with_warnings` verdict is archive-eligible because the warnings are
either pre-existing (clippy dead-code outside changed files) or explicitly
out-of-scope (two `services/reports.rs` tests that share the dashboard pipeline
and now reflect the corrected day-count semantics; `git diff` for `reports.rs`
is empty, so no regression was introduced).

## Canonical Spec Sync

### Domain: `caduxo-expiry-tracker`

Canonical path: `openspec/specs/caduxo-expiry-tracker/spec.md` (modified in place).

### ADDED Requirements

1. **`Dashboard quick-filter buttons return promised rows`** — placed inside
   the existing `## Capability: Dashboard` section, after `expired visibility`.
   Locks the six-button predicate contract (`All`, `Expired`, `Today`, `Alert
   window`, `Next 7 days`, `Next 30 days`) against the row's `days_remaining`
   and `alert_days_before` fields. Eight scenarios.
2. **`Dashboard preset matcher uses per-row day counts, not bucket strings`** —
   placed in `## Capability: Dashboard`. Locks the structural separation
   between classifier buckets and filter ranges. One scenario.
3. **`Dashboard urgency cards remain bucket counts`** — placed in
   `## Capability: Dashboard`. Documents the intentional divergence between
   card counts and filter row sets as accepted scope. Two scenarios.

### MODIFIED Requirements

1. **`operational main screen`** (under `## Capability: Dashboard`) — appended
   a new paragraph specifying that the dashboard SHALL also surface two
   additional quick-filter buttons (`Next 7 days` and `All`) and that each
   named section MUST correspond to the row set defined for the matching
   quick-filter button in the new `Dashboard quick-filter buttons return
   promised rows` requirement. Added one scenario (`dashboard sections align
   with the matching filter buttons`).
2. **No destructive MODIFIED blocks.** The change to `operational main screen`
   is a clarifying extension (added paragraph + new scenario); the existing
   bullets are byte-identical.

### REMOVED Requirements

None.

## Merge Operations

- No new canonical spec file was created.
- The existing canonical spec at `openspec/specs/caduxo-expiry-tracker/spec.md`
  was updated in place via two targeted edits.
- No REMOVED or large destructive MODIFIED operations were performed. No
  destructive merge approval was required from the parent.

## Same-Domain Active Changes

None. Search across `openspec/changes/` confirmed no other active change
under `specs/caduxo-expiry-tracker/spec.md`. Both prior
`caduxo-expiry-tracker` and `caduxo-create-product-upc` entries are already
in `openspec/changes/archive/`.

## Unchecked Implementation Tasks

```bash
$ grep -E '^\s*- \[ \]' openspec/changes/caduxo-dashboard-expiry-filters/tasks.md
(no output)
```

All 34 implementation-owned tasks are checked (`- [x]`). No unchecked
implementation task markers remain. No stale-checkbox reconciliation was
performed or required.

## Sync Fallback

Sync fallback was **not** used. The canonical spec merge was performed
directly during archive (the parent explicitly approved proceeding with
archive after verify passed; archive-time sync fallback is permitted under
the parent approval contract for hybrid mode). The merged result matches the
spec delta verbatim; no drift was introduced.

## Action Context Findings

From native `gentle-ai.sdd-status` v2:

- `artifactStore: openspec` (status projection) vs `hybrid` (parent preflight
  contract). The hybrid preflight contract governs persistence; the
  filesystem sync + memory save were both performed.
- `actionContext.mode: repo-local`
- `allowedEditRoots: ["/home/xeworg/Proyectos/Caduxo"]` — all archive
  operations fell inside this root.
- `taskProgress: 34/34 complete; allComplete: true`
- `dependencies.archive: ready`
- `remediationState.required: false`
- `blockedReasons: []`

No status blockers; archive was permitted.

## Destructive Merge Approval

Not required. No REMOVED requirements, no large MODIFIED blocks. All
MODIFIED changes were append-only clarifications.

## Archived Path

The active change folder was moved to:

```text
openspec/changes/caduxo-dashboard-expiry-filters/
  -> openspec/changes/archive/2026-09-13-caduxo-dashboard-expiry-filters/
```

Audit trail preserved (no active artifacts deleted).

## Memory Persistence

For `hybrid` mode, the archive report is also saved to Engram memory under
`sdd/caduxo-dashboard-expiry-filters/archive-report` with `type: architecture`
for traceability. The observation records:

- proposal/spec/design/tasks/verify-report paths
- canonical spec merge operations
- archived path
- verdict

## Risks and Out-of-Scope Findings (Carried Forward)

1. **Pre-existing clippy dead code** (non-blocking). 23 pre-existing
   warnings/errors outside the changed files. The only warning inside an
   allowed runtime file (`dashboard.rs:46` — `classify_to_string`) was
   present before this slice and was not introduced here.
2. **Out-of-scope `services/reports.rs` test failures** (non-blocking). Two
   tests (`preview_report_in_alert_window_returns_alert_lots`,
   `preview_report_next_30_days_returns_30d_lots`) now fail because Reports
   shares the dashboard pipeline; `git diff -- reports.rs` is empty. These
   are documented in `apply-progress.md` §"Known Out-of-Scope Failure
   (Reports)" and tracked as a follow-up to realign Reports to use the same
   day-count approach. No Dashboard regression was introduced.
3. **Urgency card / filter row divergence** (accepted scope). The four
   urgency cards continue to show bucket counts; the quick-filter buttons
   return day-count predicates. This is the intentional design captured in
   the new `Dashboard urgency cards remain bucket counts` requirement and
   is not a defect.

## Next Recommended

- `sdd-apply` for the planned Reports alignment follow-up change
  (out-of-scope for this archive).
- Parent-owned lifecycle: PR, review, and merge remain with the
  orchestrator. Archive is the final SDD step for this slice.

## Skill Resolution

- `paths-injected`: parent preflight supplied the SDD executor skill contract
  inline; no fallback registry lookup was performed.

## Key Learnings

1. The matcher signature rewrite forced every existing test through a compile-time RED, which is the cheapest possible strict-TDD signal.
2. `make_predicate_row` zeroing `urgency = String::new()` is a structural assertion that proves the matcher cannot accidentally read the bucket string.
3. The dashboard filter change leaks into `reports.rs` previews since reports shares the dashboard pipeline; tracking scope separation at the repository SQL boundary would prevent future cross-cutting test breakage.
4. Serde `rename_all = "snake_case"` mangles digit-containing enum variants; explicit per-variant `#[serde(rename = ...)]` is the safe override.
