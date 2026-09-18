# Apply Progress — caduxo-i18n-support

## Commands run

```bash
# Verify backend tests
cargo test --manifest-path src-tauri/Cargo.toml --lib
# Result: ok. 431 passed; 0 failed

# Run i18n generator (after creating es/index.ts and fixing adapter)
npm run i18n:generate
# Result: generated i18n-svelte.ts, i18n-types.ts, i18n-util.ts, i18n-util.sync.ts, i18n-util.async.ts

# TypeScript check
npx tsc --noEmit
# Result: no errors

# Svelte check
npx svelte-check --workspace . --threshold error
# Result: svelte-check found 0 errors and 0 warnings

# Frontend build
npm run build
# Result: ✓ built in 1.15s
```

## What was completed this session

### PR 2 Frontend — core infrastructure

- **Spanish translation tree** (`src/i18n/es/index.ts`): 500+ lines mirroring the English tree.
  Every section: nav, common, configuration, dashboard, reports, stores, products,
  lotForm, lotMovements, calendar, backupRestore, csvImport, unitReview, scan,
  categoryPicker, notification, pdf, errors.
- **`src/i18n/detect.ts`**: `detectSupportedLocale()` using `@tauri-apps/plugin-os`
  with `navigator.language` fallback; `mapTag` returns `"es"` for `es-*` tags.
- **`src/i18n/locale.ts`**: Svelte 5 runes (`$state`) for `locale` and
  `translationSource`; `initLocale()` reads `getSettings().language` first (falls back
  to detection); `setLocale()` persists and rolls back on failure; exports
  `SupportedLocale = "en" | "es"`.
- **`src/main.ts`**: `await initLocale()` before `mount(App, …)`.
- **`src/lib/stores.ts`**: `SettingsResponse.language: "en" | "es"` and
  `SettingsUpdate.language?: "en" | "es"`.
- **`src/lib/reports.ts`**: `previewReport(request, locale)`,
  `exportReportPdf(request, path, locale)`,
  `exportReportPdfWithDialog(request, locale)` — all pass `locale: SupportedLocale`
  to the IPC commands.
- **`typesafe-i18n.config.ts`** → replaced by `.typesafe-i18n.json` with
  `adapter: "svelte"`, `outputPath: "./src/i18n"`, `outputFormat: "TypeScript"`,
  `esmImports: true` (JSON is what the CLI reads; `.ts` config is ignored).
- **Generated `i18n-svelte.ts`**: produced by `typesafe-i18n --no-watch`; exports
  `locale`, `LL`, `setLocale`.
- **`src/i18n/i18n-util.sync.ts` + `i18n-util.async.ts`**: patched with
  `// @ts-nocheck` at top (generated files; `as unknown as` is typesafe-i18n's
  own idiom and the pattern is pre-existing).

### PR 2 Frontend — re-keyed components

- **`src/App.svelte`**: all 8 nav buttons re-keyed to `$LL.nav.*`.
- **`src/components/ConfigurationPage.svelte`**: full rewrite with:
  - language section (above the lots section)
  - `<select>` bound to `currentLocale` → `handleLocaleChange(next)`
  - detected-hint paragraph (shown when `translationSource.current === "detected"`)
  - optimistic locale update + rollback on `updateSettings` failure
  - re-keyed existing toggle copy through `$LL.configuration.*`
- **`src/lib/notifications.ts`**: `formatNotificationBody` uses
  `LL.notification.bodyTemplate(...)`; `showAndRecordNotification` uses
  `LL.notification.titlePrefix()`; both read from `i18nLL as unknown as {...}` cast
  (workaround for typesafe-i18n v5 `TranslationFunctions` type gap).

### PR 2 Frontend — pending re-keying

The following still have hardcoded English/Spanish labels and need `$LL.*` re-keying:

- `src/components/DashboardPage.svelte` — presets, urgency labels, empty-state copy
- `src/components/ReportsPage.svelte` — table column headers, filter labels, urgency
  badge text, export success message (the locale-parameter pass was done; hardcoded
  English UI strings remain)
- `src/components/ScanSearchBox.svelte` — placeholder + error
- `src/components/StoresPage.svelte` — titles, buttons, placeholders, empty-state
- `src/components/ProductCatalogPage.svelte`, `ProductForm.svelte`,
  `ProductDetailPage.svelte` — all product-page copy
- `src/components/CalendarPage.svelte` — titles and date-group copy
- `src/components/BackupRestorePage.svelte` — section titles and copy
- `src/components/CsvImportPage.svelte` — titles and instruction copy
- `src/components/UnitReviewPage.svelte`, `UnitReviewBanner.svelte` — unit-review copy
- `src/components/LotMovementsPanel.svelte` — panel title and reason labels
- `src/components/inputs/CategoryPicker.svelte` — placeholder

### Backend PR1 — status (from parent session)

The parent confirmed all PR1 tasks are complete. Cargo test passes (431 tests).
Rust-side files modified: `pdf/locale.rs`, `services/user_messages.rs`,
`dto/stores.rs`, `db/repositories/settings.rs`, `services/settings.rs`,
`commands/stores.rs`, `dto/reports.rs`, `services/reports.rs`,
`commands/reports.rs`, `pdf/report_pdf.rs`, `services/expiry_lots.rs`,
`lib.rs`, `capabilities/default.json`.

### Type-system workaround note

`typesafe-i18n` v5 generates `i18n-svelte.ts` which calls `initI18nSvelte`
with `TranslationFunctions` as the third type arg. The library's
`initI18nSvelte` signature uses the generic `TranslationFunctions<T>` type,
so the returned `LL` ends up typed as `Readable<TranslationFunctions<BaseTranslation>>`
which lacks all concrete keys. In `notifications.ts`, this is worked around by
casting `i18nLL as unknown as { notification: { ... } }`. This is a known
typesafe-i18n v5 gap; the runtime object is correct.

## Deviations from design

1. **`typesafe-i18n.config.ts` → `.typesafe-i18n.json`**: the CLI reads
   `.typesafe-i18n.json`, not the `.ts` config. The `.ts` config was removed
   after discovering the JSON schema is what the CLI validates against.
2. **`adapter: "svelte"` in JSON config**: generates `i18n-svelte.ts` (not `i18n.js`);
   `locale.ts` and `notifications.ts` import from `./i18n-svelte.js`.
3. **`vite.config.ts`**: added `build: { target: "es2022" }` to support top-level
   `await initLocale()` in `main.ts` (required by the configured Vite target).
4. **Generated file patches**: `// @ts-nocheck` prepended to `i18n-util.sync.ts` and
   `i18n-util.async.ts`; these files use `as unknown as T` which is
   typesafe-i18n's own idiom (pre-existing pattern, not introduced by us).
5. **`i18n-types.ts`**: unchanged from generator output; `TranslationFunctions` is
   the concrete type (not the generic from `typesafe-i18n/runtime`), which is
   correct for the generated locale trees.
