# Archive Report: caduxo-measurement-unit-options

## Status

```yaml
schema: gentle-ai.archive-result/v1
verdict: passed
artifact_store: hybrid
archived_path: openspec/changes/archive/2026-09-14-caduxo-measurement-unit-options/
domains_synced:
  - caduxo-expiry-tracker
requirements_added: 1
requirements_modified: 1
requirements_removed: 0
unchecked_implementation_tasks: 0
sync_fallback_used: true
destructive_merge_approval: not_required
evidence_revision: sha256:3922090dd99aa63a63f4a8edb02859fbb37746e1161157451a01bb94446cef23
```

## Executive Summary

The unit-catalog + integer/decimal classification slice is complete and archived. The verify report resolved at `openspec/changes/caduxo-measurement-unit-options/verify-report.md` with verdict `pass` (0 blockers, 0 critical findings, 2/2 requirements, 7/7 scenarios). All 55 implementation-owned tasks are checked in `tasks.md`. A targeted cargo test rerun (137 tests across `migrations::tests`, `services::unit_definitions`, `services::unit_audit`, `services::expiry_lots`, `services::products`, `services::csv_io`, and `pdf`) is GREEN; `npm run build` is clean and `npx tsc --noEmit` reports 0 errors. The parent-owned post-fix `ProductForm` close-button for the inline custom-unit menu is in place; the user reports full manual smoke passed. Canonical spec merge succeeded (additive for the unit-catalog requirement, MODIFIED for `lot registration`); no destructive canonical spec edits were performed; no stale-checkbox reconciliation was required; no same-domain active change conflicts. The change has been moved to the dated archive folder.

## Artifacts Read

| Artifact | Path |
|---|---|
| Proposal | `openspec/changes/caduxo-measurement-unit-options/proposal.md` |
| Spec delta | `openspec/changes/caduxo-measurement-unit-options/specs/caduxo-expiry-tracker/spec.md` |
| Design | `openspec/changes/caduxo-measurement-unit-options/design.md` |
| Tasks | `openspec/changes/caduxo-measurement-unit-options/tasks.md` |
| Apply-progress | `openspec/changes/caduxo-measurement-unit-options/apply-progress.md` |
| Verify-report | `openspec/changes/caduxo-measurement-unit-options/verify-report.md` |
| Canonical spec (existing, modified) | `openspec/specs/caduxo-expiry-tracker/spec.md` |
| Project config | `openspec/config.yaml` |
| Native status | `gentle-ai.sdd-status` v2 (parent-provided) |

## Verification Resolution

- Verify report locator: `/home/xeworg/Proyectos/Caduxo/openspec/changes/caduxo-measurement-unit-options/verify-report.md`
- Verdict: `pass`
- Blockers: 0
- Critical findings: 0
- Requirements coverage: 2/2 (`unit catalog and integer/decimal classification` + `lot registration`)
- Scenario coverage: 7/7 (`product picks a preset unit`, `product creates a custom unit inline`, `integer unit drives LotForm quantity input`, `decimal unit drives LotForm quantity input`, `unrecognized legacy units are surfaced without rewriting`, `banner dismissal persists until a new unrecognized value appears`, `CSV preview flags unknown units without blocking the import`)
- Test exit code: 0 (per-module cargo invocations: 21 + 11 + 7 + 21 + 31 + 29 + 17 = 137 tests pass)
- Build exit code: 0 (`npm run build` built in ~2 s)
- Type check exit code: 0 (`npx tsc --noEmit` reports 0 errors)

## Canonical Spec Sync

### Domain: `caduxo-expiry-tracker`

Canonical path: `openspec/specs/caduxo-expiry-tracker/spec.md` (modified in place).

### ADDED Requirements

1. **`unit catalog and integer/decimal classification`** — placed inside the existing `## Capability: Product catalog` section. Locks the typed catalog (mass/volume/count preset families), the `key` / `display_name` / `kind = integer|decimal` contract, the inline-custom-unit creation path from the ProductForm, the LotForm quantity input rules (`step`/`min`), the `default_unit_id` FK + `unit_type` derived column on `products`, the read-only-chip behavior for the lot `unit` field, the migration audit that surfaces (without rewriting) unrecognized legacy units, the dismissal-sticky banner signature, the `UnknownUnit { raw_value, suggested_keys }` CSV preview variant. Seven scenarios covering preset selection, inline custom creation, integer input rules, decimal input rules, unrecognized legacy units, banner dismissal persistence, and CSV preview warn-and-continue.

### MODIFIED Requirements

1. **`lot registration`** (under `## Capability: Expiry lots`) — `Required lot fields > unit` extended with the catalog-driven resolution contract (catalog `display_name` → legacy `products.default_unit` → seeded `units` preset `display_name` "Unidades") and the read-only-chip vs editable-input split. Closing `(Previously: ...)` note replaces the older short note.

### REMOVED Requirements

None.

## Merge Operations

- No new canonical spec file was created.
- The existing canonical spec at `openspec/specs/caduxo-expiry-tracker/spec.md` was updated in place via two targeted edits:
  - Full replacement of the `unit catalog and integer/decimal classification` requirement block (the apply-phase merge had landed a trimmed 5-scenario subset; the archive-time sync fallback brought the canonical up to the change-spec's full 7 scenarios, including the previously missing `decimal unit drives LotForm quantity input` and `banner dismissal persists until a new unrecognized value appears` scenarios, and tightened the normative wording to use RFC 2119 `SHALL`).
  - Full replacement of the `lot registration` requirement block (replaced the truncated `- unit` bullet with the catalog-driven resolution contract from the MODIFIED delta; replaced the older short note with the `(Previously: ...)` clarification from the delta).
- No REMOVED operations were performed. The two MODIFIED additions (two new scenarios for the unit-catalog requirement) are append-only; the lot registration edit replaces a stale block with the change-spec's authoritative version.

## Same-Domain Active Changes

None. Search across `openspec/changes/` confirmed no other active change under `specs/caduxo-expiry-tracker/spec.md`. Prior `caduxo-expiry-tracker`, `caduxo-create-product-upc`, and `caduxo-dashboard-expiry-filters` entries are already in `openspec/changes/archive/`.

## Unchecked Implementation Tasks

```bash
$ grep -E '^\s*- \[ \]' openspec/changes/caduxo-measurement-unit-options/tasks.md
(no output)
```

All 55 implementation-owned tasks are checked (`- [x]`). No unchecked implementation task markers remain. No stale-checkbox reconciliation was performed or required. The four lifecycle-gate tasks (bounded review, `sdd-apply`, `sdd-verify`, `sdd-archive`) at the bottom of `tasks.md` are parent-owned and are intentionally outside the implementation-owned count.

## Sync Fallback

Sync fallback was **used**. The apply phase performed an initial partial merge (5 of 7 scenarios for the unit-catalog requirement, and a trimmed lot registration note rather than the delta's full `unit` field contract). This archive step completed the merge to bring the canonical up to the change-spec's full content. The parent's instruction "merge verified spec delta into canonical spec if needed" explicitly authorized archive-time sync fallback; verification alone is not sufficient, but the parent's direct invocation of merge behavior covers the fallback authority for this slice.

The final canonical now matches the change spec verbatim:

- 7 of 7 scenarios for the unit-catalog requirement (with normative `SHALL` wording).
- The full MODIFIED `lot registration` block (catalog-driven `unit` resolution contract + `(Previously: ...)` note).

No drift was introduced; no canonical requirement was removed or shrunk.

## Action Context Findings

From native `gentle-ai.sdd-status` v2:

- `artifactStore: openspec` (status projection) vs `hybrid` (parent preflight contract). The hybrid preflight contract governs persistence; the filesystem sync + memory save were both performed.
- `actionContext.mode: repo-local`
- `workspaceRoot: /home/xeworg/Proyectos/Caduxo`
- `allowedEditRoots: ["/home/xeworg/Proyectos/Caduxo"]` — all archive operations (canonical spec edit at `openspec/specs/caduxo-expiry-tracker/spec.md`, change-folder move to `openspec/changes/archive/2026-09-14-caduxo-measurement-unit-options/`) fell inside this root.
- `taskProgress: 55/55 complete; allComplete: true`
- `dependencies.archive: ready`
- `remediationState.required: false`
- `blockedReasons: []`

No status blockers; archive was permitted.

## Destructive Merge Approval

Not required. No REMOVED requirements, no large MODIFIED blocks (both edits are append-only / in-place replacements of the matching requirement blocks). The unit-catalog edit adds two scenarios; the lot registration edit replaces a stale block with the authoritative delta.

## Archived Path

The active change folder was moved to:

```text
openspec/changes/caduxo-measurement-unit-options/
  -> openspec/changes/archive/2026-09-14-caduxo-measurement-unit-options/
```

Audit trail preserved (no active artifacts deleted).

## Memory Persistence

For `hybrid` mode, the archive report is also saved to Engram memory under `sdd/caduxo-measurement-unit-options/archive-report` with `type: architecture` for traceability. The observation records:

- proposal/spec/design/tasks/verify-report paths
- canonical spec merge operations (unit-catalog ADDED with 7 scenarios, lot registration MODIFIED)
- archived path
- verdict (`passed`)
- evidence revision `sha256:3922090dd99aa63a63f4a8edb02859fbb37746e1161157451a01bb94446cef23`
- follow-up open item: lot-level quantity enforcement (out-of-scope for this slice)

## Risks and Out-of-Scope Findings (Carried Forward)

1. **Non-blocking Svelte a11y warning** at `LotForm.svelte:235:20` (`a11y_label_has_associated_control` on the read-only `unit-chip` span inside its label). Intentional; build succeeds; the read-only chip has no input binding. Follow-up if the project wants a strict a11y pass.
2. **Frontend component harness gap** — the canonical spec's `Engineering safety > tests accompany implementation` requirement has an accepted post-MVP posture; the six manual-frontend flows (ProductForm datalist + inline unit happy path, inline unit error path, LotForm input shape for both `unit_type` values, banner re-appears on new unrecognized values, PDF `{qty} {display_name}` rendering) are recorded as a backlog item in `apply-progress.md` §"Section Q" and remain open for a future Svelte harness slice.
3. **Lot-level quantity enforcement deferred** — `2.5 pcs` against an `integer` product is a documented gap; the catalog `kind` is presentation-only for quantity input in this slice. This is the explicit follow-up SDD captured in `apply-progress.md` §"Section R" and design.md §"Out of scope".
4. **Two pre-existing report-service test failures outside this slice** — `services::reports::tests::preview_report_in_alert_window_returns_alert_lots` and `services::reports::tests::preview_report_next_30_days_returns_30d_lots` were failing before this change (confirmed by `git stash` in the apply session). They share the dashboard pipeline and were tracked in the dashboard-filters archive. No regression was introduced.
5. **Pre-existing clippy baseline** — `cargo clippy --all-targets -- -D warnings` reports 20 lib + 15 test binary warnings; all match the pre-existing baseline. The one fix in this slice (`#[allow(dead_code)]` on `LeaveForLater { raw_value }`) reduced the count to match the baseline exactly. No new clippy warnings were introduced.
6. **Manual-frontend close-button fix** — the parent patched `src/components/ProductForm.svelte` (`resetInlineUnit` helper + `inline-unit-close` button, lines 498-510 / 871-885) after the previous verify attempt. The fix is in the working tree; the user's manual smoke passed after the fix.

## Next Recommended

- Follow-up SDD for lot-level quantity enforcement (rejects `2.5 pcs` against `unit_type = integer`, etc.) — out-of-scope for this archive.
- Follow-up Svelte component harness slice to convert the six manual-frontend flows into automated component tests — out-of-scope for this archive.
- Parent-owned lifecycle: PR, review, and merge remain with the orchestrator. Archive is the final SDD step for this slice.

## Skill Resolution

- `paths-injected`: parent preflight supplied the SDD executor skill contract inline; no fallback registry lookup was performed.

## Key Learnings

1. The apply-phase partial sync (5 of 7 scenarios; trimmed lot registration note) would have left the canonical drifting from the change spec and the verify evidence; the archive-time sync fallback is the right place to complete the merge rather than let drift enter archive.
2. Reading the change spec and canonical side-by-side before merge (rather than relying on `apply-progress.md` alone) caught the trimmed canonical that the apply log described as "complete".
3. Bounded per-module cargo invocations finish in well under a second each and give a complete per-slice signal without the harness-stall risk of a single full-suite run.
