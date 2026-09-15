# Archive Report: caduxo-custom-date-picker

> Closure record for the change. The active change folder is moved under
> `openspec/changes/archive/2026-09-15-caduxo-custom-date-picker/` on the same
> commit this report references (PR #4 / `32329c4`). All traceability lives in
> the artefacts preserved alongside this report.

## Status

**archived** — apply + verify + sync clean; the change folder was moved to
the dated archive and the canonical spec already matches the delta from PR #4.

| Field | Value |
|---|---|
| `schemaName` | `gentle-pi.sdd-status` (v2 projection honoured for archive readiness) |
| `changeName` | `caduxo-custom-date-picker` |
| `artifactStore` | `openspec` (native status) / hybrid persistence (parent preflight) |
| `verdict` | `pass_with_warnings` (`blockers: 0`, `critical_findings: 0`) |
| `requirements` | `4/4` (2 ADDED, 2 MODIFIED) |
| `scenarios` | `15/15` (8 Custom date picker + 6 Calendar tab + 1 cross-ref scenario) |
| `tasks` | `17/17` implementation tasks complete |
| `merge` | `32329c4` — PR #4 — 2026-09-15 |
| `archive-path` | `openspec/changes/archive/2026-09-15-caduxo-custom-date-picker/` |
| `committed-by-this-phase` | `false` — no commit created; only folder move on disk |

## Artefacts read

| Artefact | Path | Use |
|---|---|---|
| Proposal | `openspec/changes/caduxo-custom-date-picker/proposal.md` | Context — review budget override (3,000 LOC) and slice scope |
| Spec delta | `openspec/changes/caduxo-custom-date-picker/specs/caduxo-expiry-tracker/spec.md` | Source of ADDED / MODIFIED / REMOVED accounting |
| Design | `openspec/changes/caduxo-custom-date-picker/design.md` | Component contracts, file change table, manual smoke matrix |
| Tasks | `openspec/changes/caduxo-custom-date-picker/tasks.md` | All 17 implementation tasks confirmed `[x]`; no `- [ ]` remained |
| Apply progress | `openspec/changes/caduxo-custom-date-picker/apply-progress.md` | Evidence ledger — svelte-check exit 0, vite build exit 0, M17 `grep -R 'type="date"' src/` zero matches, cargo baseline 276 + 2 pre-existing failures |
| Verify report | `openspec/changes/caduxo-custom-date-picker/verify-report.md` | Gate evidence — `verdict: pass_with_warnings`, `blockers: 0`, `critical_findings: 0`, `requirements: 4/4`, `scenarios: 15/15` |
| Sync report | `openspec/changes/caduxo-custom-date-picker/sync-report.md` | Records that the canonical spec was edited during apply (soft-sync) and already matches the delta after PR #4 — no further file moves required |
| Canonical spec | `openspec/specs/caduxo-expiry-tracker/spec.md` | Post-merge ground truth — confirms both ADDED requirements and both MODIFIED pointers are present at the line numbers reported by sync |

## Domains synced

- `caduxo-expiry-tracker` (1 domain)

This phase did **not** run a separate sync. The sync-report records that the
canonical spec was edited during apply (soft-sync) and already matches the
delta from PR #4 (`+161 / −15` on `openspec/specs/caduxo-expiry-tracker/spec.md`).

## Requirement-level delta accounting

| Operation | Requirement name | Canonical location (post-sync) | Match status |
|---|---|---|---|
| ADDED | `Custom date picker` | `openspec/specs/caduxo-expiry-tracker/spec.md` line 304 (under `## Capability: Date input` at line 302) | matched |
| ADDED | `Calendar tab` | `openspec/specs/caduxo-expiry-tracker/spec.md` line 608 (under `## Capability: Calendar` at line 606) | matched |
| MODIFIED | `lot registration` | `openspec/specs/caduxo-expiry-tracker/spec.md` line 259, with one-line pointer edit at line 281 | matched |
| MODIFIED | `report filters` | `openspec/specs/caduxo-expiry-tracker/spec.md` line 588, with one-line pointer edit at line 596 | matched |
| REMOVED | — | — | none in delta |
| RENAMED | — | — | none in delta |

### Scenario accounting (re-verified)

| Requirement | Scenarios in delta | Scenarios in canonical | Match |
|---|---|---|---|
| `Custom date picker` | 8 (Scenario blocks) | 8 | matched |
| `Calendar tab` | 6 (Scenario blocks) | 6 (continues past the line cap) | matched |

No partial MODIFIED blocks were detected. Both MODIFIED requirements were
applied as whole-block replacements per default OpenSpec delta semantics.

## Active same-domain collisions

**None.** `openspec/changes/` lists only `caduxo-custom-date-picker/` outside
`archive/`. No other active change touches `openspec/specs/caduxo-expiry-tracker/spec.md`.
No ordering decision needed.

## Destructive merge approvals or blockers

**None.**

- No `## REMOVED Requirements` in the delta.
- No large `## MODIFIED Requirements` blocks (both MODIFIEDs are one-line
  pointer edits + one `expiry date` clarification; neither triggers the
  destructive-MODIFIED guard).
- No `## RENAMED Requirements` in the delta.
- The sync helper's destructive approvals are not exercised.

## Unchecked implementation task reconciliation

**None required.** Re-read of `openspec/changes/caduxo-custom-date-picker/tasks.md`
immediately before this archive confirms `grep -nE "^\s*- \[ \]"` returns zero
matches across the 17 implementation tasks. The three "Parent lifecycle gates"
at the end of the file (`<!-- sdd-owner: parent -->`) are also checked but
they are not implementation tasks; this archive is the parent action that
satisfies the third of them ("Parent gate: Archive the change to
`openspec/changes/archive/` after a clean review and capture `archive-report.md`").

No stale-checkbox reconciliation was performed. No `apply-progress.md` plus
`verify-report.md` proof was needed because every checkbox was already in its
correct final state at the persisted tasks artefact.

## Structured status and actionContext findings

| Field | Value |
|---|---|
| `schemaName` | `gentle-ai.sdd-status` (v2) |
| `schemaVersion` | `2` |
| `changeName` | `caduxo-custom-date-picker` |
| `artifactStore` | `openspec` (native) / hybrid persistence via this archive report |
| `actionContext.mode` | `repo-local` |
| `actionContext.workspaceRoot` | `/home/xeworg/Proyectos/Caduxo` |
| `actionContext.allowedEditRoots` | `[/home/xeworg/Proyectos/Caduxo]` |
| `applyState` | `all_done` |
| `dependencies.verify` | `ready` (verify-report clean, merged to main) |
| `dependencies.archive` | `ready` (post-sync-report) |
| `phaseInstructions.archive` | "Archive only when a verify report resolves at that locator (read the file at that path) and every task is complete" — satisfied |

The move target `openspec/changes/archive/2026-09-15-caduxo-custom-date-picker/`
is inside the only `allowedEditRoot` (`/home/xeworg/Proyectos/Caduxo`), so no
actionContext override is needed.

## Manual smoke outcomes (from verify-report)

M1–M16 and M18–M20 are **deferred** to a live Tauri desktop run (Linux/WebKitGTK
provides no headless GUI for picker popover, keyboard, or dot-badge smoke).
M17 — the mechanical defect-fix gate — is **PASSED** (`grep -R 'type="date"' src/`
returns zero matches). This is the only smoke gate that can be exercised
without a desktop environment, and it confirms the core defect fix
structurally.

The Calendar loading hang defensive patches documented in `apply-progress.md`
(Calendar timeout → loadGeneration counter → in-UI diagnostics → independent
watchdog) are out-of-scope feature hardening applied during apply. They are
captured in the merged source and validated by `npm run build` exit 0 and
svelte-check exit 0, but the user-reported "stuck on Loading lots…" symptom
remains an open runtime investigation outside this slice's scope.

## Files written by this phase

| Path | Reason |
|---|---|
| `openspec/changes/caduxo-custom-date-picker/archive-report.md` | This report — closure record before the folder move |
| `openspec/changes/archive/2026-09-15-caduxo-custom-date-picker/` | The moved change folder (preserves `proposal.md`, `specs/`, `design.md`, `tasks.md`, `apply-progress.md`, `verify-report.md`, `sync-report.md`, `archive-report.md`, plus the unrelated `explore.md` discovery scratchpad) |

## Files NOT modified by this phase

- Source code (`src/**`) — untouched per the parent instruction "Source code should not be edited."
- Canonical specs (`openspec/specs/**`) — already match the delta from PR #4.
- `docs/prd.md` — already updated during apply / merged in PR #4.
- `git` history — `git status` is clean on `main` before the move; the folder
  move is an on-disk rename only and is **not** committed by this phase.
- `.codegraph/` (untracked) — untouched.

## `openspec/config.yaml` rules.archive

`openspec/config.yaml` is not present at the repo root. Default archive rules
apply: dated `openspec/changes/archive/YYYY-MM-DD-{change}/` folder,
`archive-report.md` alongside the other change artefacts, no destructive
edits to canonical specs, no commit by the archive phase itself.

## Next recommended phase

**None.** This change is closed. The next SDD activity is parent-driven
(new change proposal for the next slice) or remediation work if a future
bug surfaces from the Calendar loading hang investigation.

## Rules respected

- The verify report was read before any archive move.
- The persisted tasks artefact was re-read before any archive move; no
  unchecked implementation tasks found.
- File-backed sync was already complete (soft-sync during apply, recorded
  in `sync-report.md`); archive did not re-run the sync.
- Audit trail preserved: the archived folder retains every artefact
  (`proposal.md`, `specs/`, `design.md`, `tasks.md`, `apply-progress.md`,
  `verify-report.md`, `sync-report.md`, `archive-report.md`, `explore.md`).
- No child subagents were launched (parent owns delegation).
- No destructive canonical spec edits were attempted (none required).
- No commit was made by this phase.

## Memory observation (hybrid persistence)

This archive report is persisted to the Engram memory provider under
`topic_key: "sdd/caduxo-custom-date-picker/archive-report"` (type:
`architecture`, project: `caduxo`). Memory observation ID is recorded at the
end of this phase's envelope.

## Archived path

`openspec/changes/archive/2026-09-15-caduxo-custom-date-picker/`
