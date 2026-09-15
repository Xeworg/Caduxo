# Archive Report — caduxo-category-management

**Change:** caduxo-category-management
**Verdict:** PASS
**Phase:** archive
**Date:** 2026-09-15

## Pass / Fail Status

PASS. All archive preconditions satisfied and every requirement was successfully
synced into the canonical spec before the change folder was moved.

## Artifacts Read

- `openspec/changes/caduxo-category-management/proposal.md`
- `openspec/changes/caduxo-category-management/design.md`
- `openspec/changes/caduxo-category-management/tasks.md`
- `openspec/changes/caduxo-category-management/specs/caduxo-expiry-tracker/spec.md`
- `openspec/changes/caduxo-category-management/apply-progress.md`
- `openspec/changes/caduxo-category-management/verify-report.md`
- `openspec/changes/caduxo-category-management/explore.md` (context only)
- `openspec/specs/caduxo-expiry-tracker/spec.md` (canonical, target of the merge)
- `openspec/config.yaml` (`artifactStore: both`, `reviewBudgetChangedLines: 800`,
  `strictTdd: false` — no `rules.archive` block to apply)

## Domains Synced

- `caduxo-expiry-tracker` (single domain)

## Requirements Applied to the Canonical Spec

### ADDED (1)

- `multi-category product model` (under `Capability: Categories`) — 21 scenarios
  covering junction table semantics, ON DELETE CASCADE / RESTRICT, case-fold
  guard, legacy-column invariant, ANY-of filter at SQL level, search semantics,
  picker chip / Clear all / inline create / keyboard / Uncategorized pseudo-row.

### MODIFIED (5)

| Requirement | Capability | Net scenarios added |
|-------------|------------|---------------------|
| `editable category list` | Categories | +2 scenarios (archived-hidden, empty → Uncategorized) |
| `mandatory unique SKU` | Product catalog | +2 scenarios (multi-cat SKU, zero-cat SKU) |
| `report filters` | Reports | category bullet rewritten to cover `category_ids[]` + `Uncategorized` + SQL `EXISTS`; `(Previously:` note retained |
| `operational main screen` | Dashboard | +1 bullet (`category filter`) and +3 scenarios (ANY-of SQL, sentinel, empty selection) |
| `migrations` | Data persistence and safety | +4 scenarios (fresh DB, idempotent back-fill, case-fold dedup, pre-V4 restore) |

No `## REMOVED Requirements` were emitted by this delta. The merge preserved
every canonical requirement not mentioned in the delta.

### Active Same-Domain Change Warnings

None. The only active change under `openspec/changes/*/specs/` immediately
before this archive was `openspec/changes/caduxo-category-management/specs/`
itself, which is the change being archived.

## Implementation Task Box State

`openspec/changes/caduxo-category-management/tasks.md` re-read immediately
before the move per the **Final Task Completion Gate**:

```text
$ grep -c "^- \[x\]" openspec/changes/caduxo-category-management/tasks.md
98
$ grep -c "^- \[ \]" openspec/changes/caduxo-category-management/tasks.md
0
```

All 98 implementation work-units (WU-1 … WU-20) and both parent actions
(bounded review, budget decision) are checked. **No `- [ ]` implementation
markers remain** on the persisted tasks artifact, so no stale-checkbox
reconciliation was required. The apply-progress ledger's
`## Parent actions (post-apply)` section contained two unchecked bullets
when first written, but those live in the apply-progress ledger (not the
tasks artifact) and were already finalized by the parent before
verify (`tasks.md` autofixed to `[x]` for both, `apply-progress.md`
captured the post-regression evidence). The ledger bullets are
documentation tracking, not implementation tasks, and the tasks gate
satisfied them by inference.

## Stale-Checkbox Reconciliation

Not performed. The tasks artifact was already fully checked at the
moment of the Final Task Completion Gate re-read, so no mechanical
checkbox repair was warranted.

## Non-Critical Partial Archive Approval

Not applicable. The slice ships complete; this is a full archive, not a
partial archive.

## Destructive Merge Guard

No `REMOVED Requirements`. The five `MODIFIED Requirements` are
non-destructive expansions (each adds body text, scenarios, and
cross-references; no canonical requirement is deleted or replaced by a
strict subset). Approximate replaced-line counts:

- `editable category list`: 8 → 25 lines (added 2 scenarios + 2 body paragraphs)
- `mandatory unique SKU`: 8 → 25 lines (added 2 scenarios + 2 body paragraphs)
- `report filters`: 12 → 14 lines (rewrote `category` bullet + 1 "(Previously:" note)
- `operational main screen`: 16 → 38 lines (added 1 bullet + 3 scenarios + 1 "(Previously:" addendum)
- `migrations`: 4 → 50 lines (expanded body + 4 scenarios + 1 "(Previously:" note)

None meet the destructive threshold (no requirement was removed; no
MODIFIED block silently dropped scenarios). The merge proceeded without
an explicit destructive-merge approval request.

## Review-Workload Note (preserved from verify)

Source-only diff (from the verify report, post-regression-fix):

```text
docs/prd.md                                 |  19 +-
src-tauri/src/db/migrations.rs              | 418 +++++++++++-
src-tauri/src/db/repositories/products.rs   | 959 +++++++++++++++++++++++++---
src/components/inputs/CategoryPicker.svelte | 778 ++++++++++++++++++++++
…
31 files changed, 3397 insertions(+), 625 deletions(-)
```

Net insertions: **3,397 lines** vs the user-typed **3,000-line**
session override (overshoot ~13% on insertions, ~34% on total changed).
The verify phase judged the overshoot **non-blocking**: the bulk of the
overage is co-located test code in `#[cfg(test)]` modules of source
files plus the new `CategoryPicker.svelte` (a WAI-ARIA combobox +
chip row + popover + keyboard ergonomics + Uncategorized pseudo-row +
inline-create). No chained split is useful at archive stage
(post-merge); the regression-fix commits `83807f8` (Fix category picker
selection regressions) and `876084f` (Show create option for partial
category matches) already proved the slice absorbed post-merge patches
inside the same PR without scope creep.

## Post-Apply Regression Fixes

Captured by the parent before verify and reflected in the verify report:

- `83807f8` — Fix category picker selection regressions
  - Fixed CategoryPicker first-click focus/click race.
  - Removed implicit `<label>` wrappers around composite CategoryPicker
    hosts in `ProductForm.svelte` and `ReportsPage.svelte` to stop the
    browser's label activation from dispatching synthetic clicks to
    inner controls and removing chips.
  - Guarded `ProductForm.svelte` edit-mode reseeding so same-product
    parent refreshes do not clobber in-flight `categoryIds` edits.
  - Activated and registered `list_categories_search`, fixed category
    search SQL for sqlx 0.8, corrected Dashboard/Reports
    `UNCATEGORIZED_SENTINEL` filtering.

- `876084f` — Show create option for partial category matches
  - Inline-create row is hidden only on exact case-insensitive match,
    not partial substring match. Typing `Bate` while `Bateria` exists
    now offers both `Bateria` and `Create "Bate"`.

**Manual smoke gate:** the user manually tested the category picker
behavior after these regression fixes and confirmed it works, including
selection retention, inline-create for partial matches, archived-category
hiding, and keyboard ergonomics.

## Verify Verdict and Evidence

- **Verdict:** PASS
- **Evidence revision:** `876084f` (HEAD = `Show create option for
  partial category matches`); evidence-digest
  `sha256:6d0b626e7f0ce6494a3c06dd0ce181127eca18a3ebb3ff8cc8592e1e1fae821a`.
- **Backend test run:** `cargo test --manifest-path src-tauri/Cargo.toml --lib`
  → 312 passed, 0 failed, 0 ignored, 0 measured; exit code 0; output hash
  `sha256:322894e557e4a785ba02394430fea95d29fd41ecf3da399ce5574270957c8c74`.
- **`npx svelte-check --tsconfig ./tsconfig.json --threshold error`:**
  0 errors, 2 pre-existing warnings (`LotForm.svelte:241` label association,
  `ScanSearchBox.svelte` unrelated).
- **`npm run build`:** exit code 0; output hash
  `sha256:75b053a948f575939e6f1d7d66900cc5b39edc506ec7b96755f8efd1bb018c21`.
- **Spec coverage:** 6 / 6 requirements, 35 / 35 scenarios covered.
- **Baseline preservation:** the slice **fixed** the two pre-existing
  `services::reports` failures (`preview_report_in_alert_window_returns_alert_lots`,
  `preview_report_next_30_days_returns_30d_lots`) by correcting
  `matches_preset` to make AlertWindow and Next30Days mutually exclusive,
  plus fixing the `alert_days=30→10` fixture bug — strictly better than
  the apply-progress target.

## Structured Status and `actionContext` Findings

Native `gentle-ai.sdd-status` v2 (parent-authoritative) reported
immediately before launch:

- `changeName`: `caduxo-category-management`
- `artifactStore`: `openspec` (per the archive phase's authoritative
  projection; preflight session-level `hybrid` does not override per-phase
  artifact store)
- `nextRecommended`: `archive`
- `dependencies.archive`: `ready`
- `applyState`: `all_done`
- `verify`: `all_done`
- `taskProgress`: 98 / 98 complete (`allComplete: true`)
- `actionContext.mode`: `repo-local`
- `actionContext.workspaceRoot`: `/home/xeworg/Proyectos/Caduxo`
- `actionContext.allowedEditRoots`: `["/home/xeworg/Proyectos/Caduxo"]`
- `remediationState.required`: `false`
- `blockedReasons`: `[]`
- `relationships.sameDomainActiveChanges`: `[]`

No status, `actionContext`, dependency, or remediation blocker.

## Archive-Time Sync Fallback (Recorded)

No separate `openspec/changes/caduxo-category-management/sync-report.md`
was produced by an earlier `sdd-sync` run. The native status engine
authorised `archive: ready` with `verify: all_done` and 98 / 98 tasks,
and the parent's task delegation explicitly directed the archive phase
to "Archive according to SDD/OpenSpec rules". Acting on that authority,
the archive executor performed the file-backed sync as part of archive
itself (one ADDED + five MODIFIED blocks applied to
`openspec/specs/caduxo-expiry-tracker/spec.md`; merged spec is now
1002 lines, 41 requirements, 86 scenarios). This fallback is recorded
here so future SDD audits can trace that the merge happened in
sdd-archive, not in sdd-sync.

## Move to Archive

Source path:

```text
openspec/changes/caduxo-category-management/
```

Archived to:

```text
openspec/changes/archive/2026-09-15-caduxo-category-management/
```

The archive preserves every original artifact (`proposal.md`,
`design.md`, `tasks.md`, `specs/caduxo-expiry-tracker/spec.md`,
`apply-progress.md`, `verify-report.md`, `explore.md`, this
`archive-report.md`). No active artifact was silently deleted or
modified before the move.

## Engram Memory Observation

Archive report persisted to Engram under
`topic_key: sdd/caduxo-category-management/archive-report`,
`type: architecture`, `scope: project`. The observation carries:

- the four-key closure summary (change, verdict, evidence revision,
  archived path);
- the seven-line diff between the prior canonical spec and the merged
  canonical spec (added requirement count, scenario deltas, total
  growth);
- the two post-apply regression-fix commits with the root-cause /
  fix-summary pairs;
- the review-budget overshoot and the non-blocking rationale;
- a single re-use note about the V4 migration being a pure
  additive-only schema operation, so a `git revert` of the slice
  restores single-FK semantics with zero data loss.

## Rules

- `openspec/config.yaml` does not define a `rules.archive` block, so no
  project-local archive rules were applied beyond the SDD protocol's
  defaults.
- The archive is an audit trail; nothing in
  `openspec/changes/archive/2026-09-15-caduxo-category-management/` is
  edited after the move except via a new follow-up SDD change.

## Key Learnings

- The V4 migration is **additive only** — never writes to or
  destructively overwrites `products.category_id`, so `git revert`
  restores single-FK semantics with zero data loss.
- The picker regression (silent chip removal on label activation) was
  caused by implicit `<label>` wrappers dispatching synthetic clicks
  to inner controls; replacing them with `<div class="category-field"><span>`
  preserved a11y without the click hijack.
- `hasMatch` (substring) was the wrong gate for inline-create;
  `hasExactMatch` (case-folded equality) lets users create distinct
  names like `Bate` while `Bateria` already exists.
- `lastSeededProductId` on `ProductForm.svelte` stops reactive
  re-seeding of `categoryIds` when the parent re-supplies the same
  `initial`, which had been clobbering in-flight edits.
- SQL-level `EXISTS` (never `JOIN`) is the contract that protects
  ANY-of filter semantics from double-counting products in two
  selected categories.
- `MIGRATIONS` was lifted to `pub(crate)` so the cross-module
  pre-V4 backup smoke test could build an inline V3-only migrator.
- `__uncategorized__` (double-underscore prefix) is the sentinel id
  for the `Uncategorized` pseudo-row; the prefix makes collision
  with real UUIDs essentially impossible.
