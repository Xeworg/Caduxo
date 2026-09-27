# ODD Feature Task: Scanner UX Improvements

**Feature:** Scanner UX Improvements
**Task File:** `odd/tasks/scanner-ux-improvements.md`
**Baseline:** `main @ 0c84f93`
**Branch:** `feat/scanner-ux-improvements`
**Status:** In progress — Tasks 2.1–2.7c implemented; 2.7d–2.8 pending
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
- **Archive service exists in domain/repo but is not yet exposed as a Tauri command** — this is a gap to close before UI can function.
- **Unarchive service does not exist** — command, service method, and repo support are missing.
- **No API endpoint returns the archived unit list** — `list` and `all` commands exist but exclude archived entries.
- Task 2.6 backend scope: expose archive as Tauri command, implement unarchive lifecycle (command + service + repo), add archived-list API (or parameter) so the UI can show archived entries for resolution/historical reference.
- Task 2.6 UI scope: build management UI using the completed backend surface.
- `lot_movements.reason` is currently text and contains legacy values; preserve it as the historical label/snapshot.
- Stock-out choices are currently represented by closed movement kinds in frontend, Rust domain logic, and SQLite constraints. Do not turn these accounting kinds into arbitrary catalog values.
- Existing catalogs use `archived_at`; avoid redundant `is_active` state.
- Existing V14 legacy-prefixed values that cannot be mapped to a seeded reason remain snapshot-only (`exit_reason_id` NULL).

## Implementation Checklist

### Task 2.1: FEFO Contextual Help
- [x] Add translated tooltip explaining FEFO (First Expired, First Out).
- [x] Add one concise dynamic explanation for the currently selected policy (user chose dynamic help).
- [x] Keep the existing general section explanation; selected-policy text adds specific behavior without replacing it.

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
- [x] Apply field-appropriate constraints: positive integer quantities, positive decimal quantities, nonnegative movement adjustment, and nonnegative integer alert days (zero allowed).
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

**Status: complete**

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

**Status: complete**

**Verification evidence:**
- Worker: `npm run check`, `npm run i18n:generate`, `git diff --check` passed.
- Independent verification: `npm run build` passed.
- UI reuses existing backend surface (list, add, rename, archive, unarchive).
- Archived units excluded from active selectors; available for historical resolution.
- Stable IDs, key, and kind not editable.
- i18n parity maintained; no manual browser/E2E harness.

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
- [ ] Replace closed enum selector in stock-out registration UI with dynamic catalog dropdown (active reasons only).
- [ ] Display selected reason label during registration confirmation.
- [ ] Update movement history display to show reason label (from catalog when available, snapshot fallback otherwise).
- [ ] Archived reasons render as plain label text in history (no selection affordance).
- [ ] Preserve `movement_kind` display in history (accounting context); reason is descriptive metadata.

##### Task 2.7e: Catalog Management UI
- [ ] Build reason catalog management page (list active, list archived).
- [ ] Create reason: display name input, read-only movement_kind selector (required, locked after creation).
- [ ] Edit display name (in-place or modal); movement_kind selection not editable post-creation.
- [ ] Archive/unarchive toggle with confirmation.
- [ ] Archived reasons excluded from stock-out registration dropdown.
- [ ] Follow existing Task 2.6 UI patterns for consistency.

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

1. Task 2.1 FEFO help — complete (tooltip + dynamic selected-policy description; `npm run i18n:generate`, `npm run check` passed).
2. Task 2.2 submitted scan preservation — complete (dedicated `submittedScan` state; Registration displays exact submitted scan + SKU; resets clear it; unknown result unchanged; `npm run check` and `git diff --check` passed).
3. Task 2.3 Active Store panel — complete (compact panel with multi-store selector retained; responsive behavior based on actual component/context; explanatory body screen-reader available; `npm run check`, `npm run build`, `git diff --check` passed).
4. Task 2.4 DatePicker — complete (DatePicker captures input event text, protects drafts from reactive overwrite; valid/calendar/clear paths resolve state; LotForm uses local date formatting; `npm run check`, `npm run build`, `git diff --check` passed).
5. Task 2.5 numeric constraints — complete (Input forwards min/max/step; unit-appropriate constraints in consumers; alert-day validates whole 0..3650 with localized EN/ES error before payload; `npm run check`, `npm run build`, `npm run i18n:generate`, `git diff --check` passed; independent verifier confirmed code, no E2E harness).
6a. Task 2.6a unit catalog backend — complete (`cargo test`, `cargo check`, `cargo fmt` passed; independent verifier confirmed `cargo test`, `cargo check` passed; no schema migration; no hard deletes).
6b. Task 2.6b unit catalog UI — complete (uses existing backend; supports add/rename/archive/restore/list; presets included in archive subject to reference guard; stable IDs/key/kind not edited; i18n parity; `npm run check`, `npm run i18n:generate`, `npm run build`, `git diff --check` passed; no manual browser/E2E).
7. Task 2.7 reason catalog — **architecture resolved**; subtasks:
   - 2.7a migration + schema + seed + backfill — **complete** (V20 tests 13 passed, migration suite 74 passed; TEXT stable IDs, seven seeded stock-out categories excluding sale, nullable exit_reason_id FK, reason snapshot unchanged, limited legacy backfill snapshot-only; worker + independent verifier confirmed; no code edits/commit)
   - 2.7b catalog repository/service/command CRUD — **complete** (independent verification: full Rust library suite 823 passed; stale migration-count assertions fixed)
   - 2.7c movement DTO/service snapshot + validations — **complete** (independent verification: full Rust library suite 829 passed; `cargo check` and `cargo fmt -- --check` passed)
   - 2.7d replace stock-out selector + history display — pending
   - 2.7e catalog management UI — pending
8. Task 2.8 shared catalog UX — pending.

Each task must be completed with proportionate tests and evidence before closure. Create a separate Conventional Commit for each verified work unit on this feature branch. Do not publish without explicit user authorization.
