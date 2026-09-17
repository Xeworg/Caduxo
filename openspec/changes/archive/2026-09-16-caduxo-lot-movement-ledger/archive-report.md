# Archive Report — caduxo-lot-movement-ledger

**Change:** caduxo-lot-movement-ledger
**Verdict:** PASS
**Phase:** archive
**Date:** 2026-09-16

## Pass / Fail Status

PASS. All archive preconditions satisfied. The single-domain delta was composed into the canonical spec, the destructive-merge guard is clean (no REMOVED requirements and both MODIFIED blocks are non-destructive expansions), the implementation-owned task boxes are all checked, and the verify report (pass, 30/30 scenarios) was preserved unchanged.

## Artifacts Read

- `openspec/changes/caduxo-lot-movement-ledger/proposal.md`
- `openspec/changes/caduxo-lot-movement-ledger/specs/caduxo-expiry-tracker/spec.md`
- `openspec/changes/caduxo-lot-movement-ledger/design.md`
- `openspec/changes/caduxo-lot-movement-ledger/tasks.md`
- `openspec/changes/caduxo-lot-movement-ledger/apply-progress.md`
- `openspec/changes/caduxo-lot-movement-ledger/verify-report.md`
- `openspec/changes/caduxo-lot-movement-ledger/explore.md` (context only)
- `openspec/changes/caduxo-lot-movement-ledger/research.md` (context only)
- `openspec/specs/caduxo-expiry-tracker/spec.md` (canonical, target of the merge; pre-write snapshot inspected)
- `openspec/config.yaml` (project artifact store `both`; session override `openspec`; `reviewBudgetChangedLines` 800, session override 400; `strictTdd: false`; no `rules.archive` or `rules.sync` block)
- `openspec/changes/archive/` (collision check: no prior `2026-09-16-caduxo-lot-movement-ledger` destination)

## Domains Synced

- `caduxo-expiry-tracker` (single domain)

## Native Status Read

Read from `gentle-ai.sdd-status` v2 (parent-injected, authoritative):

| Field | Value |
|---|---|
| `changeName` | `caduxo-lot-movement-ledger` |
| `state` | ready |
| `artifactStore` | openspec |
| `taskProgress` | 32 / 32 complete (`allComplete: true`) |
| `dependencies.archive` | ready |
| `applyState` | all_done |
| `actionContext.mode` | repo-local |
| `actionContext.workspaceRoot` | `/home/xeworg/Proyectos/Caduxo` |
| `actionContext.allowedEditRoots` | `["/home/xeworg/Proyectos/Caduxo"]` |
| `nextRecommended` | archive |
| `relationships.sameDomainActiveChanges` | `[]` |
| `blockedReasons` | `[]` |

The state machine authorized the archive phase. No destructive confirmation was requested by the parent (none required: see §Destructive Merge Guard).

## Resume Prior Composition

Pre-write inspection confirmed all 13 ADDED requirements under the new `Lot movements` capability and both MODIFIED requirements (`lot registration`, `partial resolution`) had no matching canonical counterparts yet. **All operations are pending → were applied as-is.** No partial application, no replay, no already-applied effects to reconcile. The strict delta helper is honored: ADDED requirements appended; MODIFIED requirements replaced in full.

## Requirements Applied to the Canonical Spec

### ADDED (13) — new `Capability: Lot movements`

| # | Requirement | Scenarios added |
|---|---|---|
| 1 | `append-only movement ledger` | 2 (atomic update; compensating correction) |
| 2 | `movement kind vocabulary` | 1 (vocabulary + kind/source/destination contract) |
| 3 | `initial entry on lot creation` | 1 (atomic lot + initial movement) |
| 4 | `transfers within and across stores` | 3 (same-store, cross-store, source-balance insufficient) |
| 5 | `exit movements with reason vocabulary` | 3 (optional note; Otro requires note; Ajuste de inventario requires note) |
| 6 | `count adjustment with directional sign` | 4 (increase, decrease, zero delta no-op, blank note rejected) |
| 7 | `reactivation of a resolved lot via a compensating increase` | 1 (resolved → active via count increase) |
| 8 | `derived per-location balance` | 1 (full-ledger reflection) |
| 9 | `unified movement history on the per-lot detail` | 2 (Historial panel content; dashboard no widget) |
| 10 | `legacy resolution events migrate into the unified ledger` | 2 (known vocab; unknown → Otro + note) |
| 11 | `backfill of initial entries for pre-existing lots` | 2 (existing lot; NULL → sentinel) |
| 12 | `sentinel location for unassigned stock` | 1 (lot creation with setting off uses sentinel) |
| 13 | `actor placeholder is \`system\`` | 1 (actor recorded as `system`) |

Total scenarios added by the new capability: **24**.

### MODIFIED (2) — under `Capability: Expiry lots`

| Requirement | Prior shape | New shape | Scenarios added |
|---|---|---|---|
| `lot registration` | One-paragraph requirement + required/conditional/optional field list + expiry-date picker clause; **0 scenarios** | Multi-bullet requirement extending contract with `entry:initial` emission, batch-code auto-generation rules, `require_initial_location_on_lot_create` setting behavior, preserved prior required/conditional/optional fields | **4** (auto-generated batch; manual batch verbatim; toggle-off uses sentinel; toggle-on rejected) |
| `partial resolution` | One-paragraph requirement + 1 scenario (`partial quantity resolved`) | Multi-bullet requirement realizing the resolution as a single `exit:*` movement drawn from the v1 vocabulary, with notes-required contract and legacy-table migration; the prior `partial quantity resolved` scenario is replaced by `partial exit with reason preserves the remaining quantity` which covers the same behavior under the new vocabulary | **2** (partial exit reason; legacy data migration). Net change: 1 prior scenario removed, 2 added (+1) |

Total scenarios added by MODIFIED blocks: **6** (net +1 vs. prior scenarios that lived under `partial resolution`).

### REMOVED (0)

No `## REMOVED Requirements` was emitted by this delta. No canonical requirement was deleted.

### Total scenario delta

`+30` scenarios added to the canonical spec (24 ADDED + 6 MODIFIED), `0` requirements removed, `1` capability added (`Lot movements`). This matches the verify report's `gentle-ai.verify-result/v1` schema frontmatter (`scenarios: 30/30`).

## Active Same-Domain Change Warnings

None. `gentle-ai.sdd-status` reports `relationships.sameDomainActiveChanges: []`. The only active change under `openspec/changes/*/specs/` immediately before this archive was the change being archived.

## Implementation Task Box State

`openspec/changes/caduxo-lot-movement-ledger/tasks.md` re-read immediately before the move per the **Final Task Completion Gate**:

```text
$ grep -c '^- \[x\]' openspec/changes/caduxo-lot-movement-ledger/tasks.md
32
$ grep -cE '^- \[ \]' openspec/changes/caduxo-lot-movement-ledger/tasks.md
0
```

All 32 task boxes are checked (`- [x]`). The native status engine independently reports `taskProgress.total: 32`, `taskProgress.completed: 32`, `taskProgress.pending: 0`, `taskProgress.allComplete: true`. The previously-pending `- [ ]` parent-owned archive task is now `- [x]` (the archive procedure ran). **No `- [ ]` markers remain on the persisted tasks artifact**, so no stale-checkbox reconciliation was required, and no CRITICAL verification issue exists in `verify-report.md`.

## Stale-Checkbox Reconciliation

Not performed. The tasks artifact was already fully checked at the moment of the Final Task Completion Gate re-read, so no mechanical checkbox repair was warranted.

## Non-Critical Partial Archive Approval

Not applicable. The slice ships complete; this is a full archive, not a partial archive.

## Destructive Merge Guard

No `REMOVED Requirements`. The two `MODIFIED Requirements` and 13 `ADDED Requirements` are non-destructive additions. Approximate replaced-line counts vs. the pre-write canonical snapshot:

- `lot registration`: 23 lines (no scenarios) → 70 lines (multi-bullet + 4 scenarios + extended retained fields). Net **+47** lines; no canonical scenario deleted.
- `partial resolution`: 11 lines (1 scenario) → 26 lines (4 bullets + 2 scenarios). Net **+15** lines; the prior scenario was *replaced* by a scenario that covers the same user-facing behavior under the new vocabulary (a non-destructive replacement per spec rules; verify report §3.2 confirms coverage of the resolved-behavior contract).
- `Capability: Lot movements` (new): ~310 lines (13 requirements, 24 scenarios).

None of the MODIFIED blocks silently drop scenarios. None request destructive canonical writes. The destructive threshold is not met. Merge proceeded without explicit destructive-merge approval, mirroring the prior `caduxo-category-management` archive precedent.

## Structured Status & `actionContext` Findings

- `nextRecommended: archive` — executed in this run.
- `actionContext.mode: repo-local`; `actionContext.allowedEditRoots: ["/home/xeworg/Proyectos/Caduxo"]` — every write in this archive report stayed inside the repo-local workspace root (`openspec/specs/caduxo-expiry-tracker/spec.md`, `openspec/changes/caduxo-lot-movement-ledger/archive-report.md`, `openspec/changes/archive/2026-09-16-caduxo-lot-movement-ledger/`). No edits outside `allowedEditRoots`.
- `dependencies.archive: ready` — the authoritative state machine authorized this phase.
- `relationships.sameDomainActiveChanges: []` — no collision; archive proceeds.
- `blockedReasons: []` — no blockers; this report does not introduce any.

## Archived Path

```text
openspec/changes/caduxo-lot-movement-ledger/   →   openspec/changes/archive/2026-09-16-caduxo-lot-movement-ledger/
```

The archive directory was present (`total 0`): no collision risk. The change folder was moved via `git mv`-equivalent shell `mv` (full audit trail preserved on disk). All six source artifacts (`proposal.md`, `specs/caduxo-expiry-tracker/spec.md`, `design.md`, `tasks.md`, `apply-progress.md`, `verify-report.md`) plus this `archive-report.md` and the contextual `explore.md`/`research.md` move together; nothing is dropped, nothing is overwritten, and the pre-existing `2026-09-13-caduxo-create-product-upc`, `2026-09-13-caduxo-dashboard-expiry-filters`, `2026-09-13-caduxo-expiry-tracker`, `2026-09-14-caduxo-measurement-unit-options`, `2026-09-15-caduxo-category-management`, and `2026-09-15-caduxo-custom-date-picker` archive folders are untouched.

## Persistence Notes

- `artifactStore: openspec` (session override) → only the filesystem report was written; no Engram memory save performed (artifactStore is `openspec`, not `both` / `hybrid`, so memory persistence is out of scope for this run).
- The native `gentle-ai.sdd-status` v2 projection was the authoritative read for `taskProgress`, `dependencies.archive`, and `actionContext`. No local readiness was recomputed.

## Final-State Facts Preserved (from parent's explicit handoff)

These were recorded by the parent prompt and are preserved verbatim in `apply-progress.md` / `verify-report.md` and now in this archive report. No claim is invented.

- Automated gates passed (on Linux, 2026-09-16): `cargo test --manifest-path src-tauri/Cargo.toml --lib` → 421 passed; `npx svelte-check --workspace . --threshold error` → 0 errors / 0 warnings; `npm run build` → success.
- Windows manual smoke (live-desktop items 1–7) is explicitly deferred by the parent to a future Windows/VM branch or later test cycle. Backend-equivalent coverage is green on Linux (Rust suite). This report does **not** claim Windows manual smoke passed.
- Issue: https://github.com/Xeworg/Caduxo/issues/7
- PR: https://github.com/Xeworg/Caduxo/pull/8
- Verify report frontmatter: `schema: gentle-ai.verify-result/v1`, `verdict: pass`, `blockers: 0`, `critical_findings: 0`, `requirements: 0/0`, `scenarios: 30/30`, test and build exit codes both `0`.

## Files Changed (this archive)

- `openspec/specs/caduxo-expiry-tracker/spec.md` — composed: `Capability: Lot movements` added (13 ADDED requirements, 24 scenarios); `Capability: Expiry lots > Requirement: lot registration` MODIFIED (multi-bullet extension + 4 scenarios); `Capability: Expiry lots > Requirement: partial resolution` MODIFIED (multi-bullet replacement + 2 scenarios replacing the prior single scenario).
- `openspec/changes/caduxo-lot-movement-ledger/archive-report.md` — this report (written before the folder move per archive rules).
- `openspec/changes/archive/2026-09-16-caduxo-lot-movement-ledger/` — destination of the folder move; preserves `proposal.md`, `specs/caduxo-expiry-tracker/spec.md`, `design.md`, `tasks.md`, `apply-progress.md`, `verify-report.md`, `explore.md`, `research.md`, and the freshly written `archive-report.md`.

No git commit, push, or interactive change lifecycle step was performed. PR #8 remains open against `main` and is unchanged by this archive.

## Phase Result (envelope)

| Field | Value |
|---|---|
| `status` | `pass` |
| `executive_summary` | One new capability (`Lot movements`, 13 requirements, 24 scenarios) added to the canonical spec; two existing requirements (`lot registration`, `partial resolution`) replaced with extended non-destructive versions adding 6 scenarios (+1 net after the partial-resolution scenario replacement). All 32 `tasks.md` checkboxes are `- [x]`; the only `- [ ]` previously reported by the verify gate was the parent-owned archive task, which is now satisfied by this run. No REMOVED requirements, no destructive canonical writes, no CRITICAL verify findings, no `sameDomainActiveChanges` collisions. Automated gates were green on Linux (421 / 0 / 0 errors; npm build clean); Windows manual smoke is explicitly deferred by parent and is not claimed. PR #8 (linked to issue #7) and branch `feat/caduxo-lot-movement-ledger` are unchanged by this archive. |
| `artifacts` | `openspec/changes/archive/2026-09-16-caduxo-lot-movement-ledger/archive-report.md`, `openspec/changes/archive/2026-09-16-caduxo-lot-movement-ledger/{proposal,design,tasks,apply-progress,verify-report}.md`, `openspec/changes/archive/2026-09-16-caduxo-lot-movement-ledger/specs/caduxo-expiry-tracker/spec.md`, `openspec/changes/archive/2026-09-16-caduxo-lot-movement-ledger/{explore,research}.md`, `openspec/specs/caduxo-expiry-tracker/spec.md` (canonical, post-composition). |
| `next_recommended` | none (archive is the terminal SDD phase for this change). Post-archive out-of-band followups, if any, are owned by the parent: reopen the change via a follow-up proposal if Windows manual smoke or a Phase-5 polish slice is requested. |
| `risks` | Windows manual smoke is unverified (parent-deferred; backend-equivalent coverage is green on Linux). Severity: low. No other risks. |
| `skill_resolution` | `paths-injected` (executor/phase skill + support docs available in parent-injected context; no fallback discovery needed) |

## Key Learnings

1. The unified `inventory_adjustment` kind with a single `direction` column (`'increase' | 'decrease'`) collapses what would otherwise be two vocabulary entries into one wire shape; the canonical spec reflects this by carrying the kind exactly once in the `movement kind vocabulary` table and routing all `Ajustar conteo` flows through it.
2. `tasks.md` was already 32/32 checked before the archive began, so the previously-flagged "31 / 32" was strictly the parent-owned archive task itself; no stale-checkbox reconciliation was needed once the archive procedure closed.
3. The destructive-merge guard requires no explicit parent approval when a delta is purely additive (only ADDED + non-destructive MODIFIED + zero REMOVED), which is exactly this change's shape and matches the prior archive precedent.
4. Recording session facts (automated gate results, deferrals, issue/PR links) in the archive report preserves traceability without re-running any gate — important for archival audits because the move invalidates the `openspec/changes/caduxo-lot-movement-ledger/` path for any subsequent reader.
5. The archived folder retains every byte of the delta plus its `verify-report.md` and `apply-progress.md`, so a future reader can verify the change end-to-end without re-running any pipeline; the audit trail is honored.
