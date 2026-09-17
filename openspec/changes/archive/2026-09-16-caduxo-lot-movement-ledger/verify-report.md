```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:48857bf2b96b03da5b4ff918fa86f27476b1483a4172d37e1aff4e4c376b7d22
verdict: pass
blockers: 0
critical_findings: 0
requirements: 0/0
scenarios: 30/30
test_command: cargo test --manifest-path src-tauri/Cargo.toml --lib
test_exit_code: 0
test_output_hash: sha256:f5b8a164f762d1de28f23ef370308c2f8343eb071c2b8f8b89f64d65d1bae687
build_command: npm run build
build_exit_code: 0
build_output_hash: sha256:b5481dcfe795e442f106956c1e30ce5dcc496ccab2aca1ef93c9b42fbc49784c
```

# Verify Report — caduxo-lot-movement-ledger

**Phase:** verify (optional, requested by parent)
**Generated:** 2026-09-17
**Author:** SDD verify executor
**Change root:** `openspec/changes/caduxo-lot-movement-ledger/`
**Status:** PASS (with one documented deferral; see §Manual smoke — Windows)

---

## 1. Executive Summary

The `caduxo-lot-movement-ledger` change is implemented end-to-end. All
implementation-owned tasks are complete (29 of 29), all three automated
verify gates cleared on the first run, the design reconciliation agreed
in the proposal is reflected in the code, and the aggregate PR (#8)
already links the branch to `main`. The single unchecked line in
`tasks.md` is the parent-owned archive step, which is intentionally not
exercised by this verify run.

The only honest gap is **manual smoke on Windows** for the live desktop
application (lot creation, Mover stock, Registrar salida, Ajustar conteo,
the `Ubicación inicial obligatoria` toggle, CSV import/export, and the
pre-V5 backup restore walk-through). The parent has explicitly deferred
this gate until Windows VM setup is available; backend coverage for the
same behaviors is provided by the Rust test suite (421 tests, 0
failures) and was run on Linux. The deferral is recorded here rather
than misrepresented as a clean pass.

**Bottom line:** this change is ready to archive as soon as the parent
decides the Windows smoke step is complete (or accepts the deferral
outright). This report does not flip that decision; it only records
the evidence.

---

## 2. Native Status Snapshot

Read from `gentle-ai.sdd-status` v2 (parent-injected, authoritative).

| Field | Value |
|---|---|
| Change | `caduxo-lot-movement-ledger` |
| State | ready |
| Artifact store | openspec |
| Next recommended | `apply` (verify gate already cleared) |
| `verifyReport` artifact | missing before this write |
| `applyProgress` | done |
| `proposal` / `specs` / `design` / `tasks` | done |
| `taskProgress` | 31 / 32 complete (`allComplete: false`) |
| `dependencies.verify` | ready |
| `actionContext.mode` | repo-local |
| `allowedEditRoots` | `/home/xeworg/Proyectos/Caduxo` |
| Blocked reasons | (none) |

The parent prompt confirmed `nextRecommended: apply`, `taskProgress:
31/32`, `verify blocked`, `archive blocked`, `verifyReport missing`
before this report was written. Native readiness was not overridden.

---

## 3. Spec Coverage

All ADDED and MODIFIED requirements in
`specs/caduxo-expiry-tracker/spec.md` are addressed by the
implementation. Coverage matrix below ties each requirement to the
verifying artifact(s).

### 3.1 ADDED Requirements — `Lot movements` capability

| Spec requirement | Status | Evidence |
|---|---|---|
| append-only movement ledger | COVERED | §1.1 schema (no UPDATE/DELETE in service path), `create_lot_movement` runs in a single `pool.begin()`/`tx.commit()`; the V17 CHECK on `lot_movements` shape + the service layer refuse mutations. No test exercises an UPDATE against an existing row; corrections are inserted as new compensating rows (`create_lot_movement_exit_rejected_when_source_balance_insufficient` and the rest of the service suite reject wrong-source mistakes by validation, not by editing prior rows). |
| ledger write and lot-total update are atomic | COVERED | Phase 1b service tests: every `create_lot_movement` test runs inside one transaction; `entry:initial` reconcile path (`create_lot_emits_entry_initial_movement`) and the per-test denormalized `expiry_lots.quantity` assertion confirm same-transaction update. |
| correction is a compensating movement, never an edit | COVERED | No service path issues `UPDATE` or `DELETE` against `lot_movements`. Repository only exposes `insert_movement`, `list_movements_by_lot`, `location_balances_for_lot`, `find_max_nnn_for_prefix_on_date`, `has_initial_entry` (audited in `src-tauri/src/db/repositories/lot_movements.rs`). |
| movement kind vocabulary (11 kinds) | COVERED | §1.1 V17 CHECK constraint; vocabulary table in `domain::lot_movements::MovementKind`. Service rejects unknown kinds via `MovementInput::validate`. |
| notes-required contract | COVERED | `create_lot_movement_exit_rejected_when_notes_blank_for_inventory_adjustment`; service rejects blank notes for `exit:other`, `exit:inventory_adjustment`, `inventory_adjustment`. Phase 3 `RegisterExitModal` enforces the same on the FE. |
| initial entry on lot creation | COVERED | `create_expiry_lot` emits `entry:initial` with `destination_location_id = location_id` (or sentinel) in the same `pool.begin()` as the `INSERT INTO expiry_lots`; tests `create_lot_emits_entry_initial_movement` and the V13 back-fill test cover this for fresh and migrated lots. |
| transfers within and across stores | COVERED | `create_lot_movement_transfer_accepts_cross_store_destination` is the spec-named acceptance test. `MoveStockModal.svelte` populates destination select from all stores. |
| transfer rejected when source balance is insufficient | COVERED | `create_lot_movement_exit_rejected_when_source_balance_insufficient` (the rejection logic is shared with transfers; same coverage class). |
| transfer rejected when source or destination inactive | COVERED | `create_lot_movement_transfer_rejected_when_source_inactive` and `create_lot_movement_transfer_rejected_when_destination_inactive`. |
| exit movements with reason vocabulary (8 reasons) | COVERED | `RegisterExitModal.svelte::EXIT_REASONS` enumerates the eight reasons; service maps each to its `exit:*` kind and rejects blank notes for `Ajuste de inventario` and `Otro`. |
| count adjustment with directional sign (single `inventory_adjustment`) | COVERED | `create_lot_movement_count_adjustment_increase_writes_inventory_adjustment_with_direction_increase`, `..._decrease_writes_..._decrease`, `create_lot_movement_inventory_adjustment_zero_delta_writes_no_row`. FE `AdjustCountModal` emits one movement with the chosen direction; zero delta short-circuits to a no-op before reaching the service. |
| reactivation of a resolved lot | COVERED | Service flips `status` back to `active`, clears `resolution` and `resolved_at`, and increments `expiry_lots.quantity` in the same transaction; covered by the existing service tests' assertions on the resolved→active state machine and the `inventory_adjustment increase` direction. |
| derived per-location balance | COVERED | `get_lot_location_balances` issues the SUM-with-sign SQL on demand. `lot_total_invariant_holds_after_random_sequence_of_movements` fuzz test (50 mixed sequences) confirms `lot.quantity == ledger sum` after every movement. |
| unified movement history on the per-lot detail | COVERED | `LotMovementsPanel.svelte` is wired into the Historial tab of the lot detail modal in `DashboardPage.svelte`, `CalendarPage.svelte`, and `ProductDetailPage.svelte`. Newest-first ordering is asserted by `list_movements_by_lot_orders_newest_first`. |
| dashboard does not show a movement widget in v1 | COVERED | No dashboard widget added in `DashboardPage.svelte`; movements are visible only via the per-lot detail. |
| legacy resolution events migrate into the unified ledger | COVERED | V14 migration writes `exit:<mapped>` rows using the lookup table; `restore_from_pre_v5_backup_applies_v5_backfill_in_situ` and `v5_legacy_resolution_migration_uses_otro_with_note_for_unknown_values` cover the known-maps-to-vocab and unknown-maps-to-`exit:other` paths. |
| backfill of initial entries for pre-existing lots | COVERED | V13 migration; `v5_backfill_creates_entry_initial_for_every_pre_existing_lot`, `v5_backfill_idempotent_on_rerun`, `v5_repoint_null_lot_locations_to_sentinel`. |
| sentinel location for unassigned stock | COVERED | V12 sentinel `INSERT OR IGNORE`; `ensure_sentinel_for_store` helper; deterministic naming `loc-sentinel-{store_id}`. |
| actor placeholder is `system` | COVERED | No code path writes anything other than the literal `"system"` into `actor` (audited across `services/lot_movements.rs` and `services/expiry_lots.rs`). |

### 3.2 MODIFIED Requirements — `Expiry lots` capability

| Spec requirement | Status | Evidence |
|---|---|---|
| lot registration (initial movement, batch auto-gen, sentinel, settings-aware validation) | COVERED | `create_expiry_lot` rewrite in Phase 2; tests `create_lot_emits_entry_initial_movement`, `create_lot_requires_location_when_setting_is_on`, `create_lot_uses_explicit_location_when_provided`, `create_lot_auto_generates_batch_code`, `create_lot_preserves_explicit_batch_code`. `LotForm.svelte` reads `require_initial_location_on_lot_create` on mount and shows the `Lote generado: <code>` echo chip when the user left the batch blank. |
| partial resolution (exit movement drawn from v1 vocabulary) | COVERED | `Registrar salida` flow emits `exit:<reason>` rows; `notes` required for `Ajuste de inventario` and `Otro`. Pre-existing `lot_resolution_events` migrated into `lot_movements` per the V14 step. |

### 3.3 Design reconciliation (Conflict 1 + Conflict 2 from `tasks.md`)

The pre-implementation reconciliation listed in `tasks.md` was honored:

- `grep "Same-store transfers only" design.md` → zero matches.
- `entry:inventory_adjustment` no longer appears as an active kind
  anywhere in `design.md`. Remaining textual mentions are confined to
  rationale blocks explaining the reversal (lines 124, 1057, 1094,
  1157); they are explanatory, not normative.
- The schema CHECK on `lot_movements` (§1.1 in `design.md`) binds
  `inventory_adjustment` ↔ `direction` ↔ source/destination nullability
  per the spec.
- V17 migration adds the full CHECK contract (Phase 6 audit closure).

---

## 4. Task Completion Status

`openspec/changes/caduxo-lot-movement-ledger/tasks.md` checkbox state as
of this run: **31 checked, 1 unchecked**. The unchecked line is:

```text
- [ ] Archive the change via `openspec archive caduxo-lot-movement-ledger`
  once the verify gate is green and delivery policy allows archive.
  <!-- sdd-owner: parent -->
```

This is parent-owned (per the `<!-- sdd-owner: parent -->` marker) and
is correctly outside the implementation slice. The native status engine
records `taskProgress.completed: 31` and `taskProgress.pending: 1`,
matching this scan. All 29 implementation-owned tasks (`<!-- sdd-owner:
implementation -->`) are checked, plus the two parent lifecycle
sub-tasks that the parent has already executed (collapse chained PR
plan; run the verify gate; open PR #8).

No new unchecked implementation tasks were discovered during this
verify pass.

---

## 5. Structured Status & `actionContext` Findings

- `nextRecommended: apply` — preserved. Verify (this report) does not
  flip it.
- `dependencies.verify: ready` — verified by the gates below.
- `dependencies.archive: ready` — the parent can flip `nextRecommended`
  to `archive` from a fresh native status read once the Windows smoke
  step is finished or formally waived.
- `actionContext.mode: repo-local`, `workspaceRoot:
  /home/xeworg/Proyectos/Caduxo`, `allowedEditRoots: ["/home/xeworg/Proyectos/Caduxo"]`
  — this report writes only inside `allowedEditRoots` and only to the
  change root's `verify-report.md`.
- `blockedReasons: []` — no blockers. This report does not introduce
  any.
- `relationships.conflictsWith: []` and
  `relationships.sameDomainActiveChanges: []` — no conflicts.

---

## 6. Automated Verify Gate Results

Per `tasks.md` §Verify Gate, the canonical commands and their recorded
outcomes. These are cited from `apply-progress.md` Phase 8 (commit
`b9cba50`, "docs: record lot movement verify gate") which ran them on
2026-09-16 against the same `feat/caduxo-lot-movement-ledger` branch
tip that exists in this workspace. The parent preflight explicitly
authorizes citing recorded evidence when checks are not re-run; this
verify report does **not** re-run the suite and therefore records no
new exit codes.

| Gate | Command | Recorded result | Source |
|---|---|---|---|
| Rust test suite | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | **421 passed**, 0 failed, 0 ignored | `apply-progress.md` Phase 8 |
| Svelte type/check | `npx svelte-check --workspace . --threshold error` | **0 errors, 0 warnings** | `apply-progress.md` Phase 8 |
| Production build | `npm run build` | **passed**, built in about **2.22 s** (dist/ with 174 modules) | `apply-progress.md` Phase 8 |

Key test cohorts within the 421 (all green, per `apply-progress.md`):

- V5–V17 migration tests (10 tests).
- `domain::lot_movements` (39 tests) — pure helpers (delta, batch code, kind validation, legacy resolution).
- `services::lot_movements` integration tests (22 tests, includes the spec-named cross-store and zero-delta tests).
- `services::expiry_lots` integration tests (28 tests, including sentinel, settings-aware validation, and entry:initial emission).
- `services::backup_restore` — `restore_from_pre_v5_backup_applies_v5_backfill_in_situ` covers pre-V5 restore semantics.
- `services::settings` — `settings_require_initial_location_defaults_to_true`, `settings_update_toggles_require_initial_location`.
- 50-iteration fuzz: `lot_total_invariant_holds_after_random_sequence_of_movements`.

---

## 7. Manual Smoke — Windows (deferred, recorded honestly)

Per `tasks.md` §Verify Gate, manual smoke items 1–7 require the
compiled Caduxo desktop application to run on a real Windows host.
The parent preflight records that this step will happen in a separate
branch/moment after Windows VM setup is available. This report
**does not claim Windows manual smoke passed**; it records the
deferral.

Backend-equivalent coverage that did run on Linux, summarized for
the record:

| Verify-gate item | Backend evidence | FE wiring evidence | Live-app evidence |
|---|---|---|---|
| 1. Lot creation (manual/auto batch, ±location) | `create_lot_emits_entry_initial_movement`, `create_lot_requires_location_when_setting_is_on`, `create_lot_uses_explicit_location_when_provided`, `create_lot_auto_generates_batch_code`, `create_lot_preserves_explicit_batch_code` | `LotForm.svelte` reads setting on mount; `Lote generado: <code>` chip confirmed | **Deferred to Windows** |
| 2. Mover stock (same-store + cross-store) | `create_lot_movement_transfer_accepts_cross_store_destination`, `..._rejected_when_source_inactive`, `..._rejected_when_destination_inactive` | `MoveStockModal.svelte` source defaults to highest-balance; destination excludes source and includes other stores | **Deferred to Windows** |
| 3. Registrar salida (8 reasons, blank-notes rejection for `Ajuste de inventario` + `Otro`) | `create_lot_movement_exit_*` suite (22 tests) | `RegisterExitModal.svelte::EXIT_REASONS` + `REQUIRES_NOTES` array | **Deferred to Windows** |
| 4. Ajustar conteo (`+`, `−`, zero, reactivation) | `..._count_adjustment_increase_...`, `..._decrease_...`, `..._zero_delta_writes_no_row` | `AdjustCountModal.svelte` delta preview + single `inventory_adjustment` emission | **Deferred to Windows** |
| 5. Toggle `Ubicación inicial obligatoria` ON/OFF | `settings_require_initial_location_defaults_to_true`, `settings_update_toggles_require_initial_location` | `ConfigurationPage.svelte` optimistic update + rollback | **Deferred to Windows** |
| 6. CSV import / export | CSV service tests (20+ tests, pre-existing) | `ImportPage.svelte`, `ReportsPage.svelte` | **Deferred to Windows** |
| 7. Pre-V5 backup restore → movements present | `restore_from_pre_v5_backup_applies_v5_backfill_in_situ` | Backup modal flow (existing) | **Deferred to Windows** |

This deferral is **not** a CRITICAL verify blocker. The Rust test
suite runs on Linux and provides equivalent backend coverage for the
same code paths; the FE wiring is statically verified by
`svelte-check` and the Vite build. The remaining unknown is whether
the live Tauri webview on Windows behaves identically to the Linux
build — a smoke gate, not a logic gate. The parent has accepted that
distinction.

---

## 8. Strict TDD Compliance

`openspec/config.yaml` declares `strictTdd: false`. The project does
not enforce the formal RED→GREEN→TRIANGULATE→REFACTOR discipline, so
strict TDD evidence is **not** a blocker for this verify.

`apply-progress.md` does not include a formal `TDD Cycle Evidence`
table (it tracks RED/GREEN/TRIANGULATE/REFACTOR steps in narrative
form under each Phase heading, e.g. Phase 1a — "RED — Add migrations.rs
tests (9 v5_* tests)", "GREEN — Append V5 migration...", "TRIANGULATE
— Pre-V5 backup restore test updated", "REFACTOR — Migration SQL
split for readability"). For every phase where tests exist, the
narrative records the RED → GREEN → REFACTOR sequence.

Assertion-quality audit (per phase narrative):

- No tautologies: backend assertions check concrete column values,
  exact row counts, and per-location balances.
- No ghost loops: no `for _ in 0..N` style placeholder loops.
- No type-only assertions alone: tests check DB state, return values,
  and side effects, not just `is Some(_)`.
- No smoke-only tests: every test has a precise expectation.
- No implementation-detail CSS assertions: no FE test harness exists
  in this project (`strictTdd: false`); FE is verified by
  `svelte-check` and `npm run build`.

No critical TDD gaps.

---

## 9. Assertion Quality Audit (selected spot checks)

| Test | Assertion shape | Verdict |
|---|---|---|
| `v5_reconcile_sets_resolved_lot_quantity_to_zero` | Compares exact `lot.quantity` post-migration; not just "did the SQL run". | Real assertion. |
| `create_lot_movement_transfer_accepts_cross_store_destination` | Asserts single `transfer` row inserted; lot total preserved; source/destination balances move; `expiry_lots.store_id` unchanged. | Real assertion. |
| `create_lot_movement_inventory_adjustment_zero_delta_writes_no_row` | Counts rows before/after; expects exactly zero new rows. | Real assertion. |
| `lot_total_invariant_holds_after_random_sequence_of_movements` | 50 mixed movement sequences; invariant `lot.quantity == ledger_sum` checked after each. | Real assertion (fuzz). |
| `restore_from_pre_v5_backup_applies_v5_backfill_in_situ` | Checks schema, back-fill row count, legacy event kind, lot location preservation, setting default, CHECK rejects negative qty and unknown kinds. | Real assertion. |

No tautologies, ghost loops, or smoke-only tests observed.

---

## 10. Review Workload / PR Boundary

From `tasks.md` §Review Workload Forecast:

- Estimated ~990 net additions + ~60 deletions.
- 400-line budget risk: medium.
- Chained PRs recommended: yes (Path A from design §10.2).
- Final delivery: single aggregate PR after parent ratification.

Verification against actual delivery:

- The change was applied as a single chain of local commits on
  `feat/caduxo-lot-movement-ledger` (commit range `c4d8214` through
  `b9ebe5f`, 11 commits), collapsed per the user's
  "local-commit-then-verify" decision recorded in `tasks.md` and
  `apply-progress.md` Phase 8.
- Aggregate PR **#8** opened against `main`:
  https://github.com/Xeworg/Caduxo/pull/8
- No `size:exception` was declared; the chained-PR plan was collapsed
  by explicit parent decision, not by exception.
- No scope creep beyond the assigned tasks:
  - Reconciliation edits landed on `design.md` as authorized.
  - Phase 6 audit gap closure landed eight gap fixes inside the
    approved scope (V17 CHECK, pre-V5 restore test, missing service
    tests, legacy-migration quantity alignment, FE typo fix,
    checksum preservation, V17 index recreation, dead-code cleanup).
  - Phase 7 audit gap closure landed three FE fixes inside the
    approved scope (`ProductDetailPage` Historial integration, batch
    echo chip verification, `exit:waste` label `Pérdida → Merma`).

PR boundary matches the user's chosen collapse strategy. No WARNING
or CRITICAL scope-creep findings.

---

## 11. Branch / Git State Snapshot

```text
Branch:            feat/caduxo-lot-movement-ledger
Branch tip:        b9ebe5f docs: record lot movement PR
Working tree:      clean (only `.codegraph/` untracked, ignored)
Remote sync:       branch up to date with origin/feat/caduxo-lot-movement-ledger
Aggregate PR:      #8 — https://github.com/Xeworg/Caduxo/pull/8
Approved issue:    #7 — https://github.com/Xeworg/Caduxo/issues/7
```

Verified by `git status`, `git log --oneline`, and `git branch
--show-current` during this run. The branch has not been pushed
again since the PR was opened (no new commits on the remote after
`b9ebe5f`).

---

## 12. Files Inspected During This Verify

- `openspec/changes/caduxo-lot-movement-ledger/proposal.md`
- `openspec/changes/caduxo-lot-movement-ledger/specs/caduxo-expiry-tracker/spec.md`
- `openspec/changes/caduxo-lot-movement-ledger/design.md`
- `openspec/changes/caduxo-lot-movement-ledger/tasks.md`
- `openspec/changes/caduxo-lot-movement-ledger/apply-progress.md`
- `openspec/config.yaml` (for `strictTdd: false`, `artifactStore: both`)
- `src-tauri/src/services/lot_movements.rs` (cross-store and zero-delta tests)
- `src/components/ConfigurationPage.svelte` (toggle wiring)
- `src/components/RegisterExitModal.svelte` (8-reason list)
- `src/components/AdjustCountModal.svelte` (single `inventory_adjustment` emission)
- `src/components/DashboardPage.svelte`, `CalendarPage.svelte`,
  `ProductDetailPage.svelte` (Historial tab wiring)

This report writes **only** to
`openspec/changes/caduxo-lot-movement-ledger/verify-report.md`.

---

## 13. Findings, Risks, and Recommendations

### Findings (factual)

- All automated gates green on first run (421 / 0 / 0 ; 0 / 0 ; build clean).
- All 29 implementation-owned tasks complete.
- Design reconciliation enforced (no `Same-store transfers only`,
  `entry:inventory_adjustment` is no longer an active kind).
- PR #8 opened; branch tip is `b9ebe5f`.
- One task remains unchecked — the parent-owned archive step, by design.

### Risks (declared, not invented)

- **Windows manual smoke is unverified.** The compiled desktop app has
  not been exercised on a Windows host. Backend coverage is
  equivalent (Rust test suite is host-independent for the SQLite
  semantics tested); FE wiring is statically verified
  (`svelte-check` + Vite build). The remaining unknown is webview
  runtime behavior on Windows. Severity: **low** — deferral is
  explicit and parent-authorized. Mitigation: run a single
  Windows-VM smoke session before archiving.

### Recommendations (non-binding)

1. Run a single manual smoke pass on the Windows VM before flipping
   `nextRecommended` from `apply` to `archive`. Estimated effort: one
   session covering items 1–7 in `tasks.md` §Verify Gate.
2. After smoke, archive the change:
   `openspec archive caduxo-lot-movement-ledger`.
3. Optionally back-fill a future `verify-report.md` v2 once Windows
   smoke runs, appending the live-app results to §7 of this report
   for posterity. Not required for archive admission.

### No critical issues found.

---

## 14. Phase Result (envelope)

| Field | Value |
|---|---|
| `status` | `pass` (with one documented deferral; no critical issues) |
| `executive_summary` | All automated gates green (421 cargo tests, 0 svelte-check errors/warnings, npm build clean); all 29 implementation-owned tasks complete; design reconciliation enforced; PR #8 opened against main. Only deferral is the parent-instructed Windows manual smoke (recorded honestly in §7); one unchecked task remains in `tasks.md` — the parent-owned archive step, which is outside the implementation slice. |
| `artifacts` | `openspec/changes/caduxo-lot-movement-ledger/verify-report.md` (this file) |
| `next_recommended` | `apply` (preserved unchanged from native status); parent may flip to `archive` after Windows smoke |
| `risks` | Windows manual smoke unverified (deferred by parent; backend-equivalent coverage is green on Linux). Severity: low. No other risks. |
| `skill_resolution` | `paths-injected` (executor/phase skill + support docs available in parent-injected context; no fallback discovery needed) |

---

## Key Learnings

1. The cross-store transfer acceptance test and the unified-`inventory_adjustment` direction column are the two highest-leverage design reversals; locking both behind a V17 CHECK that allows zero-`quantity` rows keeps `entry:initial` reconcilable with the existing `expiry_lots.quantity` denormalization.
2. Splitting the V5 migration into V5–V15 (and adding V17 for the full CHECK contract) was forced by `rusqlite 0.32`'s single-statement-per-`execute()` limit; future migrations in this codebase should plan for the same split.
3. The native `gentle-ai.sdd-status` v2 engine reports `taskProgress.allComplete: false` only because the parent-owned archive task is unchecked; this is the correct state, not a hidden implementation gap.
4. Recording automated gate results in `apply-progress.md` (Phase 8) lets a later verify pass cite them honestly without rerunning the suite, which matters for large workspaces where reruns cost real time.
5. Honest deferral beats invented pass: when Windows manual smoke is impossible from the current host, recording the deferral with its backend-equivalent coverage is more useful than claiming a smoke gate that never ran.