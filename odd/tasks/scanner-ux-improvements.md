# ODD Feature Task: Scanner UX Improvements

**Feature:** Scanner UX Improvements
**Task File:** `odd/tasks/scanner-ux-improvements.md`
**Baseline:** `main @ 0c84f93`
**Branch:** `feat/scanner-ux-improvements`
**Status:** In progress — Tasks 2.1–2.7d implemented; Task 2.7e in progress; Task 2.8 pending
**Created:** 2026-09-26
**Last Updated:** 2026-09-26

## Problem Statement

The Scanner workflow has operator-facing gaps: unclear FEFO behavior, loss of the original scan value in registration, an oversized Active Store panel, date input synchronization/timezone errors, and insufficient numeric constraints. Unit definitions lack an easy management UI, and stock-out reasons are closed values rather than a business-configurable catalog.

## Explicit User Decisions

The following decisions were made based on approved user experience requirements and must not be altered without explicit re-authorization:

- **Stock-out selector UX:** Replace the closed enum selector with a dynamic catalog dropdown showing active reasons; frontend supplies `exit_reason_id`, backend derives `movement_kind`. The user selects a reason, not a movement kind directly.
- **Reason catalog scope:** Global (not per-company/store), matching the existing catalog tenancy model.
- **Historical preservation:** Existing `reason` text is immutable snapshot; catalog changes never rewrite movement history.
- **No physical deletion:** Archived entries remain resolvable in historical data.

## Product Decisions

1. Unit definitions are a manageable catalog: add, edit display name, archive; no physical deletion. Existing database IDs and quantity semantics remain stable. Changing `key` or `kind` needs a separately designed replacement/versioning flow.
2. Stock-out reasons are a global user-managed catalog: seed useful defaults, allow businesses to add and edit reasons, and soft-archive them. Stable IDs and historical meaning must be preserved.
3. User-configurable reason catalog entries map to a closed internal `movement_kind` category. The visible stock-out selector is replaced by a catalog selector; the frontend supplies the reason ID and the backend derives the corresponding `movement_kind`. Reasons are descriptive metadata and must not alter stock-calculation semantics.
4. Preserve existing `lot_movements.reason` text as an immutable historical snapshot and add a stable nullable `exit_reason_id` reference. Never rewrite old movement snapshots when catalog labels change or reasons are archived.
5. The app currently has no organization/company tenancy model; catalogs such as units and categories are global. Exit reasons follow that existing global scope.
6. No hard deletes. Archived catalog entries are unavailable for new selection but remain resolvable in historical data.

## Verified Architecture Findings

- `unit_definitions` table exists and is seeded with defaults.
- Backend services for create and rename-display-name exist.
- Archive, unarchive, and archived-list services/commands have been implemented as part of Task 2.6a.
- Unit definitions use `archived_at`; unarchive lifecycle is exposed with a duplicate-key guard.
- A dedicated archived-unit list API supports management UI and historical resolution.
- Task 2.6 backend scope is complete: expose archive, implement unarchive lifecycle, and list archived units.
- Task 2.6 UI scope: build management UI using the completed backend surface.
- `lot_movements.reason` is currently text and contains legacy values; preserve it as the historical label/snapshot.
- Stock-out choices are currently represented by closed movement kinds in frontend, Rust domain logic, and SQLite constraints. Do not turn these accounting kinds into arbitrary catalog values.
- Existing catalogs use `archived_at`; avoid redundant `is_active` state.
- Existing V14 legacy-prefixed values that cannot be mapped to a seeded reason remain snapshot-only (`exit_reason_id` NULL).

## Implementation Checklist

### Task 2.1: FEFO Contextual Help
- [ ] Add translated tooltip explaining FEFO (First Expired, First Out), accessible by pointer and keyboard.
- [ ] Add one concise dynamic explanation for the currently selected policy (user chose dynamic help).
- [ ] Keep the existing general section explanation; selected-policy text adds specific behavior without replacing it.

**Status: complete — independently verified.** FEFO tooltip is pointer/keyboard accessible; selected-policy description updates dynamically and preserves the general description. Independent `npm run check`, Svelte warning-threshold check, i18n generation, build, and `git diff --check` passed. Work-unit commit: `5d87d5c` (combined with Task 2.6b due to the shared ConfigurationPage surface).

### Task 2.2: Preserve Submitted Scan Value
- [x] Preserve the trimmed submitted scan in dedicated UI state before `scanInput` is cleared.
- [x] In matched-product Registration (`lot_match` and `product_match`), display the submitted scan value and the resolved product SKU directly below it.
- [x] Remove display dependence on the lot's/first lot's `batch_code`; unknown-scan behavior remains unchanged.
- [x] Clear the retained scan state wherever the corresponding registration/resolution context is reset; preserve it across ordinary mode switches while the result remains visible.

### Task 2.3: Simplify Active Store Panel
- [x] Compact panel while retaining store selector.
- [x] Implement responsive behavior based on actual component/context, not an unsupported single-store assumption.
- [x] Verify multi-store selector remains usable.

**Note:** Multi-store Listbox/card retained; explanatory body remains screen-reader available. `npm run check`, `npm run build`, `git diff --check` passed.

### Task 2.4: Expiry Date Input
- [x] Fix manual DatePicker text synchronization with the appropriate input event.
- [x] Generate local calendar dates without UTC ISO slicing errors.

### Task 2.5: Numeric Constraints
- [x] Forward `min`, `max`, and `step` through the Input primitive.
- [x] Apply field-appropriate constraints: positive integer quantities, positive decimal quantities, nonnegative movement adjustment, and nonnegative integer alert days (zero allowed). DistributionEditor now uses `min=1` for integer units and `min=0.01` for decimal units.
- [x] Validate alert days as a nonnegative integer; preserve documented backend fallback behavior unless evidence supports changing it.

**Implementation evidence:** Input primitive now forwards `min`, `max`, `step` attributes. Consumers receive unit-appropriate constraints (positive integer/decimal for quantities, nonnegative for adjustments and alert days). Raw alert-day field validates whole numbers 0–3650 inclusive with localized EN/ES error message before payload submission; backend fallback preserved. `npm run check`, `npm run build`, `npm run i18n:generate`, `git diff --check` passed. Independent verifier confirmed code; no E2E harness.

### Task 2.6: Unit Catalog Management

#### Task 2.6a: Backend Exposure & Lifecycle Completion
- [x] Expose existing archive service as a Tauri command (archive unit by ID).
- [x] Implement unarchive command, service method, and repository support for unit definitions.
- [x] Add API surface to list archived units (dedicated list endpoint or `list` with `include_archived` parameter) for historical reference and UI.
- [x] Verify archived units are excluded from active product/unit selectors but remain resolvable in historical movement views.
- [x] Preserve no-hard-delete invariant; archived entries remain with `archived_at` timestamp.
- [x] No schema migration required; existing `archived_at` column reused.
- [x] No hard deletes; archive sets `archived_at`, unarchive clears it.

**Status: complete — independently verified.**

**Verification evidence:** Full Rust library suite passed (829 tests); `cargo check --lib`, `cargo check --bin caduxo`, `cargo check --all-targets`, and `cargo fmt -- --check` passed. Five lifecycle/list tests passed.

**Verification evidence:**
- Worker: `cargo test`, `cargo check`, `cargo fmt` passed.
- Independent verifier: `cargo test`, `cargo check` passed.
- No schema migration performed; existing `unit_definitions.archived_at` column reused.
- No physical deletion paths added.

#### Task 2.6b: Configuration Management UI
- [x] Build unit management UI reusing existing schema, seeds, and completed backend operations.
- [x] UI surfaces: list active units, add unit (name/key/kind), rename display name, archive/unarchive toggle.
- [x] Expose archived entries in a way that supports historical resolution without polluting active choice lists.
- [x] Do not expose ID mutation, key changes, or kind changes in place.

**Status: complete — independently verified.**

**Verification evidence:**
- Independent verification: `npm run check` (0 errors/warnings), `npx svelte-check --tsconfig ./tsconfig.json --threshold warning` (0/0), `npm run i18n:generate` (up to date, no generated changes), `npm run build`, and `git diff --check` passed.
- UI reuses existing backend surface (list, add, rename, archive, unarchive); backend commands are registered.
- Archived units are excluded from active selectors and available through the archived/history surface.
- Stable IDs, key, and kind are not editable after creation.
- No manual browser/E2E harness.
- Work-unit commit: `5d87d5c` (combined with Task 2.1 due to the shared ConfigurationPage surface).

### Task 2.7: Global User-Managed Stock-Out Reasons

**Architecture Resolved — Implementation Pending**

#### Domain Model

**New table: `stock_out_reasons`**
- `id` TEXT PRIMARY KEY — stable identifier (matches existing catalog convention), never reused after archival
- `display_name` TEXT NOT NULL — user-facing label, editable after creation
- `movement_kind` TEXT NOT NULL — the authoritative closed enum value; selected at catalog entry creation, not user-editable post-creation
- `sort_order` INTEGER — optional display ordering hint
- `archived_at` TIMESTAMP NULL — NULL = active, non-NULL = archived; follows existing `archived_at` convention
- `created_at` TIMESTAMP NOT NULL
- `updated_at` TIMESTAMP NOT NULL

**Schema change: `lot_movements`**
- Add `exit_reason_id` TEXT NULLABLE FK → `stock_out_reasons(id)` — stable reason reference
- Preserve existing `reason TEXT` column as immutable historical snapshot (never updated by catalog changes)

**Business rules:**
- `movement_kind` enum remains closed and authoritative for stock accounting.
- Each catalog entry maps to exactly one `movement_kind`; this mapping is frozen after entry creation (changing the label does not change the mapped kind).
- Backend derives and validates `movement_kind` from the selected `exit_reason_id` at movement creation time; frontend never sets `movement_kind` directly.
- Archived reasons are excluded from new selection dropdowns but remain resolvable in movement history.
- No physical deletion; archival preserves referential integrity for historical queries.
- Sale flow is out of scope; exit_reason_id applies only to stock-out movements.

#### Seed Strategy

Seed one catalog entry per existing stock-out `movement_kind` with clear, user-friendly labels derived from the existing domain vocabulary. Do not seed legacy V14-prefixed values.

#### Migration Scope

- Create `stock_out_reasons` table with seed data.
- Add `exit_reason_id` column to `lot_movements`.
- Backfill `exit_reason_id` where legacy `reason` text maps unambiguously to a seeded reason.
- Leave unmappable legacy-prefixed values as snapshot-only: `exit_reason_id` = NULL, `reason` preserved as-is.
- Migration must be idempotent; rollback tested.

#### Subtask Split

##### Task 2.7a: Migration + Schema + Seed + Backfill
- [x] Create `stock_out_reasons` table migration (up/down, idempotent).
- [x] Define `id` as TEXT PRIMARY KEY — stable identifier, never reused after archival.
- [x] Define `display_name`, `movement_kind`, `sort_order`, `archived_at`, `created_at`, `updated_at` columns matching the domain model.
- [x] Add `exit_reason_id` TEXT NULLABLE FK → `stock_out_reasons(id)` to `lot_movements`.
- [x] Preserve existing `lot_movements.reason` TEXT column as immutable historical snapshot.
- [x] Seed one catalog entry per existing stock-out `movement_kind` with clear, user-friendly labels. Do not seed legacy V14-prefixed values.
- [x] Backfill `exit_reason_id` where legacy `reason` text maps unambiguously to a seeded reason.
- [x] Leave unmappable legacy-prefixed values as snapshot-only: `exit_reason_id` = NULL, `reason` preserved as-is.
- [x] No physical deletion paths.
- [x] Migration idempotent; rollback tested under supported SQLite strategy.
- [x] Unit tests for backfill mapping logic.

**Status: complete**

**Verification evidence:**
- Worker: V20 stock-out reason tests 13 passed, migration suite 74 passed, `cargo check`, `cargo fmt` passed.
- Independent verifier: V20 13 tests passed, migration suite 74 passed, `cargo check` passed.
- No code edits or commits performed by agent; verification only.
- Schema: TEXT primary key stable IDs, seven seeded stock-out categories (excluding sale), nullable `lot_movements.exit_reason_id` FK.
- Reason snapshot preserved unchanged; unmappable legacy-prefixed values remain snapshot-only (`exit_reason_id` = NULL).

##### Task 2.7b: Catalog Repository + Service + Tauri Commands
- [x] Implement `stock_out_reasons` repository: list (active only), list_all (including archived), get_by_id, create, update_display_name, archive, unarchive.
- [x] Archive sets `archived_at`, unarchive clears it; no hard deletes.
- [x] Implement service layer with business logic: validate create inputs, enforce movement_kind enum closure, enforce no-kind-change post-creation.
- [x] Expose catalog operations as Tauri commands: list_reasons, list_all_reasons, get_reason, create_reason, update_reason_name, archive_reason, unarchive_reason.
- [x] No hard-delete commands.
- [x] Unit tests for repository and service logic covering happy path, archive/unarchive, and kind-immutability.

**Status: complete**

**Verification evidence:**
- Worker: 34 service/domain tests passed, `cargo check`, `cargo fmt` passed.
- Independent verifier: full Rust library suite 823 passed, 0 failed.
- V20 exposed two stale `backup_restore` migration count assertions; corrected tests/comments and verified full suite.
- Locale errors fixed.
- Archive/restore guards verified.
- No code edits or commits performed by agent; verification only.
- Domain/DTO/repo/service/Tauri command APIs implemented; 7 stock-out kinds excluding sale.

##### Task 2.7c: Movement DTO/Service Snapshot + Validation
- [x] Adapt existing `LotMovementCreate` input to accept `exit_reason_id` for stock-out paths (do not introduce a new `StockOutMovementInput` type).
- [x] For stock-out movements: service layer derives `movement_kind` from the reason catalog entry at movement creation; UI wiring is Task 2.7d.
- [x] For sales and non-stockout paths: `LotMovementCreate` keeps its existing behavior when `exit_reason_id` is omitted.
- [x] Validate: reason exists, reason is not archived, reason's `movement_kind` is a valid stock-out category.
- [x] Store both `exit_reason_id` and `reason` snapshot in the movement record.
- [x] Movement history queries return the immutable `reason` snapshot and `exit_reason_id` alongside `movement_kind`.
- [x] Unit/integration tests for snapshot preservation and kind derivation.
- [x] Sale flow unchanged; non-stockout paths unchanged.

**Status: complete — independently verified.**

**Verification evidence:** `cargo check --lib`, `cargo check --bin caduxo`, `cargo check --all-targets`, `cargo fmt -- --check`, and full `cargo test --lib` passed (829 tests). Six focused 2.7c integration tests passed. Existing unrelated compiler warnings remain.

##### Task 2.7d: Replace Stock-Out Selector + History Display
- [x] Replace closed enum selectors in both Scanner stock-out mode and RegisterExitModal with dynamic active catalog reasons.
- [x] Display the selected reason in stock-out confirmation; submit `exit_reason_id` and let backend derive `movement_kind`.
- [x] Movement history shows the accounting `movement_kind` and immutable reason snapshot separately.
- [x] Archived reasons remain plain snapshot text in history and are unavailable in new selection.
- [x] Preserve sale and non-stockout flows; notes requirements follow the selected reason's movement kind.
- [x] Show loading/errors and provide retry for catalog-load failures.

**Status: complete — independently verified.**

**Verification evidence:** `npm run check` (0 errors/warnings), `npx svelte-check --tsconfig ./tsconfig.json --threshold warning` (0/0), `npm run i18n:generate`, `npm run build`, and `git diff --check` passed. Independent review confirmed both stock-out surfaces submit `exit_reason_id`, sale remains unchanged, notes rules match backend, and movement history renders `movement_kind` plus snapshot. Rust full library suite passed 829/829; focused stock-out reason tests passed 34 and exit-reason tests passed 4. No browser/E2E harness.

##### Task 2.7e: Catalog Management UI
- [x] Build reason catalog management UI in `ConfigurationPage.svelte`, following the verified Task 2.6b catalog pattern (list active and archived).
- [x] Create reason with display name and a closed `movement_kind` selector; movement kind is immutable after creation.
- [x] Edit display name only; never expose ID or movement-kind edits.
- [x] Archive/unarchive with confirmation; archived reasons remain excluded from stock-out registration dropdowns.
- [x] Add a dedicated localized `stockOutReasons` namespace in EN/ES for management UI and the seven closed kinds; do not label the catalog through `lotMovements` UI namespace.
- [x] Omit a “preset” badge: the current DTO does not distinguish seeded from custom entries.
- [ ] Keep Rust formatting changes and other pre-existing dirty files outside this work unit.

**Status: implementation independently verified; work-unit commit pending.** Independent verification caught and the implementation corrected two runtime defects: `loadReasonsCatalog()` was missing from `onMount`, and localized movement-kind dictionary keys used underscores instead of the backend `exit:` wire values. Final verification passed: `npm run check` (0 errors/warnings), `npx svelte-check --tsconfig ./tsconfig.json --threshold warning` (0/0), `npm run i18n:generate` (up to date, no mutation), `npm run build`, and `git diff --check`. No browser/E2E harness.

### Task 2.8: Shared Catalog UX
- [ ] Reuse consistent archive confirmation and active/archived presentation where it fits both catalogs.
- [ ] Use `archived_at` as lifecycle state; do not add redundant `is_active`.
- [ ] Verify historical views show preserved snapshot text and archived references clearly.
- [ ] Avoid premature generic catalog infrastructure unless implementation demonstrates meaningful duplication.

## Acceptance Criteria

- FEFO control explains the policy and selected option; tooltip supports pointer and keyboard focus.
- Registration shows the exact submitted scan value with SKU directly beneath; it never substitutes a batch code.
- Active Store panel is compact and selector remains accessible.
- DatePicker manual entry syncs correctly and local calendar dates do not shift due to UTC conversion.
- Numeric inputs forward and apply correct bounds/steps; alert days accept zero and reject invalid negatives/fractions.
- Units can be managed through UI using existing backend; archived units cannot be newly selected and remain available to historical resolution.
- Users can add/edit/archive global stock-out reasons; defaults are seeded; archived reasons cannot be newly selected.
- Reason catalog changes never alter movement kind/accounting semantics or rewrite old movement snapshots.
- Migration preserves movements and historical reason text, maps only unambiguous values, is idempotent, and has tested rollback/recovery expectations under the supported SQLite strategy.

## Non-Goals

- No physical deletion of units or reasons.
- No in-place rewrite of historical unit/reason identity or movement meaning.
- No company/store-specific catalog scope (not supported by current tenancy model).
- No dynamic movement accounting kinds or workflow engine changes.
- No new store creation UI.

## Task Order and Status

1. Task 2.1 FEFO help — **complete, independently verified**; commit `5d87d5c` (shared work unit with 2.6b).
2. Task 2.2 submitted scan preservation — complete; commit `a63b574` also contains Tasks 2.3–2.5 as one cohesive scanner/input UX work unit; code-inspection verifier and frontend checks passed.
3. Task 2.3 Active Store panel — complete; commit `a63b574` (multi-store selector retained; frontend checks/build passed).
4. Task 2.4 DatePicker — complete; commit `a63b574` (input draft handling and local-date generation; frontend checks/build passed).
5. Task 2.5 numeric constraints — complete; commits `88a4223` (positive DistributionEditor min/step) and `a63b574` (Input forwarding, field bounds, localized alert-day validation); frontend checks/build passed; no E2E harness.
6a. Task 2.6a unit catalog backend — complete; commit `42ea9f3` (full Rust library suite 829 passed; checks and formatting passed; no schema migration; no hard deletes).
6b. Task 2.6b unit catalog UI — **complete, independently verified**; commit `5d87d5c` (shared work unit with 2.1). Independent verifier reports no defects; npm checks/build/i18n and Svelte warning-threshold checks pass. Unrelated Rust formatting remains outside this work unit.
7. Task 2.7 reason catalog — **architecture resolved**; subtasks:
   - 2.7a migration + schema + seed + backfill — **complete** (V20 tests 13 passed, migration suite 74 passed; TEXT stable IDs, seven seeded stock-out categories excluding sale, nullable exit_reason_id FK, reason snapshot unchanged, limited legacy backfill snapshot-only; worker + independent verifier confirmed; no code edits/commit)
   - 2.7b catalog repository/service/command CRUD — **complete** (independent verification: full Rust library suite 823 passed; stale migration-count assertions fixed)
   - 2.7c movement DTO/service snapshot + validations — **complete**, commit `30fcd58` (independent verification: full Rust library suite 829 passed; `cargo check` and `cargo fmt -- --check` passed)
   - 2.7d replace stock-out selector + history display — **complete**; commits `68253ab` (client API foundation) and `b37bef3` (selector, history, retry UI)
   - 2.7e catalog management UI — **implemented and independently verified; work-unit commit pending**. Includes active/archived catalog management, create with closed kind selection, rename label only, and archive/restore confirmation. Corrections verified against backend wire kinds; frontend checks/build/i18n generation passed. No browser/E2E harness.
8. Task 2.8 shared catalog UX — pending.

Each task must be completed with proportionate tests and evidence before closure. Create a separate Conventional Commit for each verified work unit on this feature branch. Do not publish without explicit user authorization.
