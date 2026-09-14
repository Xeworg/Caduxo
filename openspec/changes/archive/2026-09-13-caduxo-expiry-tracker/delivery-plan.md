# Delivery Plan: Caduxo Expiry Tracker

## Decision

Use a **Feature Branch Chain** with a draft/no-merge tracker branch for the complete MVP.

Why:

- The full SDD task set has 106 implementation tasks and will exceed the 400 changed-line interactive review budget.
- Several slices depend on a shared Tauri/Rust/SQLite foundation before they are independently useful.
- The feature should integrate as a coherent MVP before merging to the main branch.

## Dependency diagram

```text
tracker/caduxo-expiry-tracker
├─ PR 1 📍 Foundation and persistence skeleton
   └─ PR 2 Database schema and migrations
      └─ PR 3 First-run setup and stores
         └─ PR 4 Product catalog, categories, and barcodes
            └─ PR 5 Expiry lots and resolution events
               └─ PR 6 Dashboard and scanner workflow
                  └─ PR 7 Local notifications
                     └─ PR 8 CSV import/export
                        └─ PR 9 Reports and PDF export
                           └─ PR 10 Backup, restore, packaging, and MVP verification
```

Each child PR should target the previous branch. The tracker PR stays draft/no-merge until all child PRs are reviewed and integrated.

## Cross-slice engineering standard

Every slice that adds or changes behavior must add or update tests in the same slice. If useful coverage is not practical yet, the slice must state the reason and the follow-up task. Structured logging is part of the foundation and should be used in later backend operations without logging sensitive business data such as product names, SKU/barcode values, imported row contents, or free-form notes.

## Review budget

Target budget: **400 changed lines per slice**.

This budget controls slicing only. Do not compress code, remove useful tests, or reduce documentation quality to fit the budget.

Some slices may still exceed 400 lines because Tauri/Rust/Svelte setup, generated lockfiles, migrations, and UI scaffolding can be bulky. If a cohesive split cannot stay under budget after one honest slicing pass, mark that PR with `size:exception` and explain why.

## Slice 1 — Foundation and persistence skeleton

### Scope

Establish the runnable desktop app skeleton and backend persistence boundaries without implementing product workflows.

### Architecture guardrail

Use the modular layered architecture from `design.md`: thin Tauri commands, application services, pure domain modules, SQLite repositories, DTOs, shared errors, app state, and structured logging. Do not implement business rules directly in Svelte components, Tauri command handlers, or repository SQL modules.

### Included tasks

From `tasks.md`:

- Initialize Tauri v2 project with Svelte + TypeScript.
- Configure Rust workspace and app metadata.
- Establish modular layered backend structure: commands, services, domain, repositories, DTOs, shared errors, and app state.
- Add SQLite support with `sqlx`.
- Add database migration runner.
- Add app data directory resolution for Windows/Linux.
- Add basic error handling shape shared by Tauri commands.
- Add structured Rust logging with safe app-local log output.
- Add baseline Rust test harness for backend/domain/persistence code.

### Out of scope

- Full schema tables beyond a minimal migration harness smoke test.
- Product, store, lot, dashboard, notification, CSV, report, backup, or packaging features.
- Visual polish beyond a basic app shell.

### Verification

- Install/build dependencies reproducibly.
- Run TypeScript check/build if configured.
- Run Rust compile/check if configured.
- Verify the app can initialize its app data path and database connection in development.
- Verify logs are created in the app-local log directory and avoid sensitive business data by default.
- Run the baseline Rust test command and frontend check/test command if configured.

### Risk

Medium line-count risk because project scaffolding and lockfile changes can be large.

## Slice 2 — Database schema and migrations

### Scope

Create the durable SQLite schema and indexes required by the MVP.

### Included tasks

From `tasks.md` section 2:

- Create migrations for `stores`, `store_locations`, `categories`, `products`, `product_barcodes`, `expiry_lots`, `lot_resolution_events`, `notification_log`, and `app_settings`.
- Add indexes for SKU, barcode, expiry date, store/date, product lots, and store locations.

### Dependencies

- Slice 1 migration runner and database connection.

### Verification

- Run migrations on a fresh database.
- Verify schema constraints and indexes exist.
- Add focused migration/schema tests where practical.
- Include tests for migration idempotency/fresh database setup where practical.

### Risk

Medium. Schema is central; mistakes compound into all later slices.

## Slice 3 — First-run setup and stores

### Scope

Make the app usable for a first store and basic store/local management.

### Included tasks

From sections 3 and 5:

- Implement `is_first_run` command.
- Implement first store creation flow.
- Block expiry lot creation until at least one store exists.
- Remember last selected store in settings.
- Implement store CRUD.
- Implement optional internal location CRUD per store.
- Enforce unique location name per store.
- Build store/local management UI.
- Build optional internal location UI.

### Dependencies

- Slices 1–2.

### Verification

- First run detects no stores.
- Creating first store clears first-run state.
- Store/location constraints behave correctly.
- Last selected store persists.

### Risk

Medium. This combines backend commands and initial UI.

## Slice 4 — Product catalog, categories, and barcodes

### Scope

Implement product identity and search foundation.

### Included tasks

From section 4:

- Product create/update/archive commands.
- Required unique SKU enforcement.
- Category list CRUD.
- Product default alert-days-before with software suggestion of 30 days.
- Barcode add/remove/list commands.
- Barcode uniqueness across products.
- Product search by description, SKU, and barcode.
- Product list UI.
- Product form UI.
- Product detail UI with barcode list and expiry lots placeholder.

### Dependencies

- Slices 1–3.

### Verification

- SKU uniqueness.
- Multiple barcodes per product.
- Barcode cannot be assigned to multiple products.
- Search works by description, SKU, and barcode.

### Risk

High line-count risk because it includes backend and multiple UI screens. Split into backend/UI sub-PRs if needed.

## Slice 5 — Expiry lots and resolution events

### Scope

Track expiring quantities tied to products and stores.

### Included tasks

From section 6:

- Expiry lot create/update/archive commands.
- Pre-fill lot unit from product default when available.
- Pre-fill lot alert days from product default.
- Per-lot alert-days-before override.
- Require store selection only when multiple stores exist.
- Optional internal location and batch code.
- Partial quantity resolution.
- Record partial resolutions in `lot_resolution_events`.
- Lot form UI.
- Resolve quantity UI.

### Dependencies

- Slices 1–4.

### Verification

- Lot defaults derive from product.
- Per-lot overrides persist.
- Partial resolution updates remaining quantity and records event history.
- Store selection rule changes based on store count.

### Risk

High. This is core domain behavior and likely deserves careful tests.

## Slice 6 — Dashboard and scanner workflow

### Scope

Expose daily operational workflow: urgent lots plus scanner/search entry.

### Included tasks

From sections 7 and 8:

- Dashboard query command.
- Compute expired, today, alert-window, and next-30-days groups.
- Urgency cards.
- Prominent expired section.
- Urgent lot table sorted by urgency.
- Store/local filter.
- Quick filters.
- Row actions: view product, edit lot, resolve quantity, report selection.
- Always-visible scan/search input.
- Exact barcode search first, exact SKU second.
- Open product/lot entry flow on match.
- Quick product creation with scanned value pre-filled when no match exists.
- Manual typed SKU/UPC input.

### Dependencies

- Slices 1–5.

### Verification

- Expired and urgent grouping rules.
- Scanner keyboard-wedge flow via typed input + Enter.
- No-match quick creation path.
- Filters and row actions preserve expected navigation.

### Risk

High line-count risk. If needed, split dashboard and scanner into separate child PRs.

## Slice 7 — Local notifications

### Scope

Add local OS notification behavior without cloud or messaging integrations.

### Included tasks

From section 9:

- Notification permission/request flow.
- Due notification query using alert window rules.
- `notification_log` duplicate same-day prevention.
- Startup notification check.
- Periodic notification check while app is open.
- Stop OS notifications after expiry date.
- Keep expired lots prominent on dashboard after notification period ends.

### Dependencies

- Slices 1–6.

### Verification

- Same-day deduplication.
- Notifications run during alert window only.
- Expired lots remain visible even after OS notifications stop.
- Permission denial degrades gracefully.

### Risk

Medium/high due to platform behavior.

## Slice 8 — CSV import/export

### Scope

Support practical product import/export and report row export.

### Included tasks

From section 10:

- CSV file selection flow.
- Header detection.
- Column mapping UI for SKU, description, UPC/barcode.
- Required mapped field validation.
- Import preview with invalid rows and duplicate warnings.
- Conflict strategies: skip, update, review.
- Import products and barcodes.
- Export products CSV.
- Export report rows CSV.

### Dependencies

- Slices 1–6. Report row export can be completed after Slice 9 if needed.

### Verification

- Header and delimiter edge cases where practical.
- Invalid required fields are rejected before import.
- Duplicate SKU/barcode conflicts follow selected strategy.
- Exported CSV can be re-opened by spreadsheet tools.

### Risk

High line-count risk due to mapping UI and validation states.

## Slice 9 — Reports and PDF export

### Scope

Generate operational expiry reports and PDF output.

### Included tasks

From section 11:

- Report data query for in-alert-window report.
- Report data query for expired report.
- Report data query for next-30-days report.
- Custom report filters.
- Report preview UI.
- Rust structured PDF export using `printpdf` or equivalent.
- A4 landscape default.
- Pagination and page numbers.
- Report metadata: type, filters, generated date/time.
- CSV export for report data if not completed in Slice 8.

### Dependencies

- Slices 1–8.

### Verification

- Report query correctness.
- PDF file generation on Linux development environment.
- Pagination with enough rows to span multiple pages.
- Metadata reflects selected filters.

### Risk

High. PDF generation may need `size:exception` or backend/UI split.

## Slice 10 — Backup, restore, packaging, and MVP verification

### Scope

Finish operational safety and release-readiness checks.

### Included tasks

From sections 12–14:

- Database backup export.
- Restore flow with explicit destructive confirmation.
- Validate restored database before replacing active data where practical.
- Document backup/restore behavior in the app.
- Validate development build on Linux.
- Validate production build on Linux.
- Validate Windows build strategy.
- Check Tauri/WebView runtime assumptions for Windows.
- Check Tauri/WebKitGTK assumptions for Linux.
- Document user-level install or portable run options.
- Verify all MVP behaviors listed in section 14.

### Dependencies

- Slices 1–9.

### Verification

- Backup round-trip test with sample data.
- Restore requires explicit destructive confirmation.
- Linux dev and production build evidence.
- Windows strategy documented even if not built locally.
- Final SDD verify pass.

### Risk

Medium/high. Packaging and cross-platform assumptions can create environment-specific issues.

## First `sdd-apply` boundary

Start with **Slice 1 only**.

The apply agent should:

1. Implement the project foundation and persistence skeleton only.
2. Create or update `openspec/changes/caduxo-expiry-tracker/apply-progress.md`.
3. Mark only Slice 1 task checkboxes in `tasks.md` when completed.
4. Record verification evidence.
5. Stop before schema/domain/UI workflow implementation.

## Chain context template for each PR

````markdown
## Chain Context

Tracker: <tracker PR URL>
Previous PR: <previous PR URL or none>
Next PR: <next planned PR or TBD>

```text
tracker/caduxo-expiry-tracker
├─ PR 1 Foundation and persistence skeleton
   └─ PR 2 Database schema and migrations
      └─ PR 3 First-run setup and stores
         └─ ...
```

Current PR: <mark with 📍 in the diagram>

### In scope

- ...

### Out of scope

- ...

### Verification

- ...

````

## Open delivery risks

- The repository appears to have no valid `HEAD` yet, so an initial commit or branch baseline decision is needed before real PR creation.
- Generated scaffolding and lockfiles may exceed the line budget even for Slice 1.
- Several UI-heavy slices may need backend/UI sub-splitting after the first honest slicing pass.
- Windows validation may require a separate machine or CI environment.
