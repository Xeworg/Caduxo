# Unit definitions and notifications locale plumbing

## Goal

Add optional locale plumbing to unit-definition and notification IPC commands so backend error-boundary localization covers these remaining surfaces.

## Scope

- Add optional `locale` command arguments to unit-definition and notification commands that can surface user-facing backend errors.
- Apply existing `localize_validation`, `localize_business_rule`, `localize_not_found`, `localize_duplicate_field`, and `localize_internal` helpers where appropriate.
- Update TypeScript wrappers to pass the active UI locale by default.
- Preserve backward compatibility by keeping locale optional.
- Run focused Rust and frontend checks.

## Non-goals

- Do not change service behavior or database schema.
- Do not alter notification OS copy or scheduling behavior.
- Do not localize the developer-facing health command.
- Do not touch `.codegraph/`.
- Do not push.

## Tasks

- [x] Task 1: Add optional locale parameters and localization helper chains to unit-definition and notification commands.
- [x] Task 2: Update TypeScript wrappers to pass the active locale.
- [x] Task 3: Run focused backend/frontend checks and fix regressions within scope.
- [x] Task 4: Commit the verified slice.

## Evidence

### Task 1
- `src-tauri/src/commands/unit_definitions.rs`: added optional `locale: Option<String>` to `create_unit_definition`, `rename_unit_definition`, and `apply_unit_review_action`; resolved it with the existing `Locale::parse` fallback pattern; chained `localize_validation`, `localize_business_rule`, `localize_not_found`, `localize_duplicate_field`, and `localize_internal` before `AppError::into`.
- `src-tauri/src/commands/notifications.rs`: added optional `locale: Option<String>` to `list_due_notifications` and `mark_notification_shown`; applied the same localization helper chain before `AppError::into`.
- Read-only/state-shape commands remain unchanged, and health stays developer-facing/out of scope.

### Task 2
- `src/lib/unit_definitions.ts`: added active-locale resolution and forwards locale for `createUnitDefinition`, `renameUnitDefinition`, and `applyUnitReviewAction`; existing callers remain compatible because the parameter is optional.
- `src/lib/notifications.ts`: added active-locale resolution and forwards locale for `listDueNotifications` and `markNotificationShown`; notification OS title/body formatting and scheduling are unchanged.

### Task 3
- Writer observed:
  - `cargo build --lib` from `src-tauri` — clean.
  - `cargo test --lib -- --skip reports` from `src-tauri` — 626 passed, 0 failed, 32 filtered out.
  - `cargo clippy --lib --no-deps` from `src-tauri` — no warnings in edited files; four pre-existing warnings outside the slice.
  - `npx svelte-check --tsconfig ./tsconfig.json --threshold error` — 0 errors, 0 warnings.
- Independent verifier PASS:
  - Confirmed optional locale parameters are only on intended commands and preserve backward compatibility.
  - Confirmed backend helper chains use the resolved locale before `AppError::into`.
  - Confirmed TS wrappers pass active locale by default and existing call-sites still type-check.
  - Confirmed notification OS copy/scheduling and i18n dictionaries are unchanged.
  - Re-ran `cargo build --lib --manifest-path src-tauri/Cargo.toml` — clean.
  - Re-ran `cargo test --lib --manifest-path src-tauri/Cargo.toml -- --skip reports` — 626 passed, 0 failed, 32 filtered out.
  - Re-ran `npx svelte-check --tsconfig ./tsconfig.json --threshold error` — 0 errors, 0 warnings.
- Fixed verifier's cosmetic newline observation in `src-tauri/src/commands/unit_definitions.rs`, `src/lib/unit_definitions.ts`, and `src/lib/notifications.ts`.
- `git diff --check` — clean.

### Task 1 — backend command plumbing

Files touched:

- `src-tauri/src/commands/unit_definitions.rs`
- `src-tauri/src/commands/notifications.rs`

Helper chain pattern mirrors `commands::products` and `commands::expiry_lots`:

```rust
let loc = resolve_locale(locale);
service::…
    .await
    .map_err(|e| localize_validation(e, loc))
    .map_err(|e| localize_business_rule(e, loc))
    .map_err(|e| localize_not_found(e, loc))
    .map_err(|e| localize_duplicate_field(e, loc))
    .map_err(|e| localize_internal(e, loc))
    .map_err(AppError::into)
```

Commands wired (only those whose service calls can surface the matched variants):

| Command | Helpers applied | Reason |
| --- | --- | --- |
| `create_unit_definition` | all five | service returns `Validation` (key/display_name), `DuplicateField` (key), `Infrastructure` |
| `rename_unit_definition` | all five | service returns `Validation` (display_name), `NotFound`, `Infrastructure` |
| `apply_unit_review_action` | all five | service returns `NotFound` (preset id) and `Infrastructure`; the others are no-op passes-through |
| `list_due_notifications` | all five | service returns `Validation` (strict `YYYY-MM-DD`) and `Infrastructure`; the others are no-op passes-through |
| `mark_notification_shown` | all five | service returns `Validation`, `NotFound` (unknown lot), `Infrastructure`; the others are no-op passes-through |

Pure read/state-shape commands kept signature-free: `list_unit_definitions`,
`list_unrecognized_units`, `unit_audit_banner_state`, `dismiss_unit_audit_banner`.
The notification OS message format, scheduling, and developer-facing health
command are untouched.

### Task 2 — frontend wrappers

Files touched:

- `src/lib/unit_definitions.ts`
- `src/lib/notifications.ts`

Both wrappers now import `DEFAULT_LOCALE`, `locale as activeLocale`, and
`type SupportedLocale` from `../i18n/locale.svelte.js`, expose an optional
`locale?: SupportedLocale` parameter on the backend-mutating wrappers, and
forward the resolved locale to the Rust command via the same
`resolveLocale` helper used by `src/lib/csv.ts` and `src/lib/expiry_lots.ts`:

```ts
function resolveLocale(locale?: SupportedLocale): SupportedLocale {
   if (locale) return locale;
   const current = activeLocale?.current;
   return (current ?? DEFAULT_LOCALE) as SupportedLocale;
}
```

Wrappers updated:

| Wrapper | New signature |
| --- | --- |
| `createUnitDefinition` | `(input, locale?: SupportedLocale)` |
| `renameUnitDefinition` | `(input, locale?: SupportedLocale)` |
| `applyUnitReviewAction` | `(action, locale?: SupportedLocale)` |
| `listDueNotifications` | `(today?, locale?: SupportedLocale)` |
| `markNotificationShown` | `(input, locale?: SupportedLocale)` |

Pure read-only wrappers (`listUnitDefinitions`, `listUnrecognizedUnits`,
`unitAuditBannerState`, `dismissUnitAuditBanner`) keep their no-arg
signatures because their service calls do not surface user-facing
backend errors.

### Focused validation (Tasks 1 & 2 evidence)

- `cargo build --lib` — finished cleanly.
- `cargo test --lib -- --skip reports` — 626 passed, 0 failed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib db::repositories::settings::tests::get_settings_language_unconfigured_for_unsupported_value` — 1 passed, 0 failed (re-run after a superseded automated failure notice).
- `cargo clippy --lib --no-deps` — no new warnings from the edited files;
  the 4 warnings still reported live in `services/reports.rs`,
  `db/repositories/products.rs`, and `services/csv_io.rs`, all pre-existing
  on the parent commit (`76281ab`).
- `npx svelte-check --tsconfig ./tsconfig.json --threshold error` —
  `0 errors and 0 warnings`.

### Task 4
- Commit: `b80252d feat: pass locale to unit and notification commands`.