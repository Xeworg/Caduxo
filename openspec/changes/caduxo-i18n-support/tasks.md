# Tasks — caduxo-i18n-support

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~2240 net (backend ~860, frontend ~1380) — design's per-slice rollup in §12.1 |
| 3000-line budget risk | Low (≈75% of budget; design's two-PR chain recommendation is for review focus, not budget) |
| Chained PRs recommended | Yes — design §12.2 commits to PR 1 backend then PR 2 frontend |
| Suggested split | PR 1 — backend i18n plumbing (settings + PDF locale + IPC delta + user_messages + tauri-plugin-os) → PR 2 — frontend i18n (typesafe-i18n install + trees + bindings + ConfigurationPage selector + notifications + reports wrapper + all component re-keying) |
| Delivery strategy | auto-chain (session preflight); orchestrator is authorized to chain |
| Chain strategy | stacked-to-main (PR 1 lands to `main` first so PR 2's frontend code can resolve the new IPC contract; PR 2 stacks onto PR 1's commit) |

```text
Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
3000-line budget risk: Low
```

The chain-strategy selection is `deferred` in the session preflight and the design names `stacked-to-main` as the recommended pattern. The orchestrator must confirm `stacked-to-main` (or fall back to `feature-branch-chain`) before PR 1 apply.

---

## PR 1 — Backend i18n plumbing (~860 lines, lands first)

### 1.1 Settings backend: `language` field

- [ ] Add `pub language: String` (default `"en"`) to `SettingsResponse` and `pub language: Option<String>` to `SettingsUpdate` in `src-tauri/src/dto/stores.rs`. <!-- sdd-owner: implementation -->
- [ ] Add `get_language_setting(pool) -> Option<String>` and `set_language_setting(pool, value)` helpers in `src-tauri/src/db/repositories/settings.rs`, using the existing `get_setting` / `upsert_setting` key-value functions. <!-- sdd-owner: implementation -->
- [ ] Update `get_settings` in `src-tauri/src/db/repositories/settings.rs` so the response carries `language` with `"en"` fallback when the row is absent, empty, or outside `{en, es}`. <!-- sdd-owner: implementation -->
- [ ] Branch on `input.language` in `services/settings.rs::update_settings` so a partial update of only `language` does not touch `last_selected_store_id` or `require_initial_location_on_lot_create`. <!-- sdd-owner: implementation -->
- [ ] Reject `language` values outside `{en, es}` at the IPC boundary in `src-tauri/src/commands/stores.rs::update_settings`; return a `CommandError::Validation` and leave the persisted row untouched. <!-- sdd-owner: implementation -->
- [ ] Behaviour check — add `#[tokio::test]` cases in `db/repositories/settings.rs`: missing row returns `"en"`; empty string returns `"en"`; `"fr"` returns `"en"`; partial update of only `language` preserves the other two keys; `set_language_setting("fr")` is rejected by the command handler. <!-- sdd-owner: implementation -->

### 1.2 Rust PDF translation module

- [ ] Create `src-tauri/src/pdf/locale.rs` declaring `pub enum Locale { En, Es }`, `impl Locale { pub fn parse(tag: &str) -> Locale }` (primary subtag lower-cased; `es*` → `Es`, anything else → `En`), and `pub enum PdfMessageKey` enumerating every key named in design §7.1. <!-- sdd-owner: implementation -->
- [ ] Add `pub struct PdfMessages` with the fields listed in design §7.1 (titles, header, filter prefixes, columns `[Cow<'static, str>; 8]`, empty notice, days-ago template, footer, brand). <!-- sdd-owner: implementation -->
- [ ] Add `pub fn pdf_messages(locale: Locale) -> PdfMessages` returning the English table for `En` and the spec's Spanish translations for `Es` (header `Generado: {date}  ·  Filas: {n}`, filters `tienda=`/`ubicación=`/`categoría=`/`categorías (N)`/`urgencia=`/`desde=`/`hasta=`, columns `SKU`/`Descripción`/`Tienda / Ubicación`/`Cantidad`/`Caducidad`/`Días`/`Alerta`/`Lote`, empty `Ninguna fila coincide con los filtros actuales.`, days-ago `hace {n} días`, footer `Página {n} de {m}`, brand `Caduxo · Control de caducidades`, title `Reporte Caduxo`). <!-- sdd-owner: implementation -->
- [ ] Add `#[cfg(test)] mod tests` in the same module asserting every `PdfMessageKey` returns a non-empty string for both `En` and `Es` (parity test per design §7.4). <!-- sdd-owner: implementation -->

### 1.3 Thread locale through PDF renderer

- [ ] Change `pdf/report_pdf.rs::render_report` to accept `locale: Locale`; route document title, header line, filter line prefix, brand line, column headers, empty notice, footer, and `format_filters` / `format_days` through `pdf_messages(locale)` instead of static English strings. Column widths and pagination are unchanged. <!-- sdd-owner: implementation -->
- [ ] Change `ReportType::description` in `src-tauri/src/dto/reports.rs` to `fn description(&self, locale: Locale) -> Cow<'static, str>` returning the four spec mappings (`Lots in alert window`/`Lotes en ventana de alerta`, `Expired lots`/`Lotes vencidos`, `Lots expiring in the next 30 days`/`Lotes que vencen en los próximos 30 días`, `Custom filtered report`/`Reporte personalizado filtrado`). <!-- sdd-owner: implementation -->
- [ ] Thread `Locale` through `services/reports.rs::preview_report` and `services/reports.rs::export_report_pdf` so `ReportMetadata.description` is locale-resolved and the PDF render receives `Locale`. <!-- sdd-owner: implementation -->
- [ ] Add `locale: String` parameter to `commands/reports.rs::preview_report` and `commands/reports.rs::export_report_pdf`; call `Locale::parse(&locale)` at the boundary and forward to the service. <!-- sdd-owner: implementation -->
- [ ] Behaviour check — `cargo test --manifest-path src-tauri/Cargo.toml --lib` runs green; specifically assert `Locale::parse("es-MX") == Locale::Es`, `Locale::parse("fr") == Locale::En`, `Locale::parse("") == Locale::En`, and that `pdf_messages(Locale::Es).filter_prefix_store == "tienda="` / `Locale::En` keeps `"store="`. <!-- sdd-owner: implementation -->

### 1.4 User-visible backend error strings

- [ ] Create `src-tauri/src/services/user_messages.rs` declaring `pub enum UserMessage` (variants per design §6.2: `InvalidDateFormat { label, value }`, `InvertedDateRange { from, to }`, `QuantityNonNegative { value }`, `QuantityPositive { value }`, `AlertDaysNegative`, `AlertDaysExceedsMax { max }`, `SkuColumnNotDetected`, `DescriptionColumnNotDetected`) and `pub fn user_message(kind: UserMessage, locale: Locale) -> String` returning the spec's English / Spanish strings. <!-- sdd-owner: implementation -->
- [ ] Add `parse_user_message_kind(msg: &str) -> Option<UserMessage>` in the same module to inverse the well-known messages we own; free-form messages return `None` so they pass through unchanged. <!-- sdd-owner: implementation -->
- [ ] Add a `localize_validation(err: AppError, locale: Locale) -> AppError` helper used by `commands/notifications.rs`, `commands/reports.rs`, and `commands/expiry_lots.rs` to wrap `DomainError::Validation` strings from `parse_strict_date`, `validate_quantity`, `validate_alert_days`, `validate_filters`, and CSV header detection. Developer-only `DomainError::Internal` and tracing logs are NOT routed through `user_message`. <!-- sdd-owner: implementation -->
- [ ] Behaviour check — unit tests asserting `user_message(InvalidDateFormat { label: "notification_date", value: "x" }, Locale::Es)` begins with `Formato de` and the English variant matches the original `parse_strict_date` message verbatim. <!-- sdd-owner: implementation -->

### 1.5 Tauri OS plugin wiring

- [ ] Register `tauri_plugin_os::init()` in `src-tauri/src/lib.rs` `tauri::Builder` chain (next to the existing plugin registrations). <!-- sdd-owner: implementation -->
- [ ] Add `"permissions": ["core:default", "os:default", ...]` (preserve existing permissions and append `os:default`) in `src-tauri/capabilities/default.json`. <!-- sdd-owner: implementation -->
- [ ] Behaviour check — `cargo build --manifest-path src-tauri/Cargo.toml` green; `cargo test --manifest-path src-tauri/Cargo.toml --lib` green. <!-- sdd-owner: implementation -->

### 1.6 PR 1 manual smoke

- [ ] Manual smoke — export a PDF report in Spanish from the existing UI (after temporarily hard-coding `Locale::Es` at the renderer entry point or invoking `export_report_pdf` from a Rust test with `locale = "es"`); confirm in Okular / `pdfinfo` that the filter line reads `Filtros: tienda=… · categoría=…`, the empty notice reads `Ninguna fila coincide con los filtros actuales.` when rows are empty, the days-ago cell renders `hace 3 días` for `days_remaining = -3`, and accents (`á é í ó ú ñ ü`) render with the built-in Helvetica font. Re-run with `Locale::En` and confirm the English baseline is byte-identical to a pre-change export. <!-- sdd-owner: implementation -->

---

## PR 2 — Frontend i18n (~1380 lines, stacks onto PR 1)

### 2.1 Install and configure `typesafe-i18n`

- [x] Add `typesafe-i18n` to `devDependencies` and add scripts `i18n:generate`, `i18n:typesafe`, `prebuild`, `predev` to `package.json` per design §2. <!-- sdd-owner: implementation -->
- [x] Add `@tauri-apps/plugin-os` to `dependencies` in `package.json`. <!-- sdd-owner: implementation -->
- [x] Create `src/i18n/en/index.ts` and `src/i18n/es/index.ts` exporting the full key tree (nav, configuration including `language.*` and `locationRequired.*`, dashboard, reports, scan, stores, products, calendar, backup, csv, unitReview, lotMovements, categoryPicker, notification, common, pdf, errors, modal helpers) — Spanish tree mirrors the English key tree exactly per the spec parity requirement. <!-- sdd-owner: implementation -->
- [x] Run `npm run i18n:generate` once to produce `src/i18n/i18n-svelte.ts` and `src/i18n/i18n-util.ts`; commit the generated output. <!-- sdd-owner: implementation -->

### 2.2 Locale rune and bootstrap

- [x] Create `src/i18n/locale.ts` exporting `type SupportedLocale = "en" | "es"`, `locale` and `translationSource` Svelte 5 `$state` runes, `initLocale()`, and `setLocale(next)` per design §1.3; `initLocale` MUST consult the persisted `app_settings.language` row first, fall back to detection, and update `translationSource.current` to `"manual"` when the row is present and `"detected"` otherwise. `setLocale` MUST roll back the rune and the `typesafe-i18n` runtime on `updateSettings` failure. <!-- sdd-owner: implementation -->
- [x] Create `src/i18n/detect.ts` exporting `detectSupportedLocale()` that calls `@tauri-apps/plugin-os`'s `locale()` first, falls back to `navigator.language`, and maps the primary subtag to `SupportedLocale` per design §11.1. <!-- sdd-owner: implementation -->
- [x] Update `src/main.ts` to `await initLocale()` before `mount(App, …)` so the first paint uses the resolved locale, not the runtime default. <!-- sdd-owner: implementation -->

### 2.3 Frontend DTOs and reports wrapper

- [x] Update `src/lib/stores.ts` `SettingsResponse` and `SettingsUpdate` types to include `language: SupportedLocale` and optional `language?: SupportedLocale`; re-export `SupportedLocale` from `src/i18n/locale.ts`. <!-- sdd-owner: implementation -->
- [x] Update `src/lib/reports.ts` so `previewReport`, `exportReportPdf`, and `exportReportPdfWithDialog` accept `locale: SupportedLocale` and pass it through to the `preview_report` and `export_report_pdf` IPC commands as the new `locale` field. <!-- sdd-owner: implementation -->

### 2.4 Re-key top-level components

- [x] Replace hard-coded nav button labels in `src/App.svelte` with `$LL.nav.*` (`dashboard`, `stores`, `products`, `calendar`, `reports`, `import`, `backup`, `settings`). <!-- sdd-owner: implementation -->
- [x] Re-key `src/components/ConfigurationPage.svelte`: add the new "Idioma" / "Language" section above the existing toggle; render the `<select>` listing `English` and `Español` (bound to `locale.current`); render the `$LL.configuration.language.detectedHint` paragraph above the selector when `translationSource.current === "detected"`; on change call `setLocale(next)` with optimistic update and rollback on `updateSettings` failure; re-key the existing `Configuración` / `Ubicación inicial obligatoria al crear lote` copy through `$LL.configuration.pageTitle` and `$LL.configuration.locationRequired.{label,description}`. <!-- sdd-owner: implementation -->
- [ ] Re-key `src/components/DashboardPage.svelte` presets + banner copy through `$LL.dashboard.*`. <!-- sdd-owner: implementation -->
- [ ] Re-key `src/components/ReportsPage.svelte`: report-type labels, urgency options, success / error messages through `$LL.reports.*`; read `locale.current` from the rune and pass it to `previewReport` and `exportReportPdfWithDialog`. <!-- sdd-owner: implementation -->
- [x] Re-key `src/lib/notifications.ts`: `formatNotificationBody` uses `$LL.notification.bodyTemplate({ qty, expiry_date, location })`; the title is composed as `${$LL.notification.titlePrefix()}${lot.sku}`; `LL` is accessed inside the functions (not at module load). <!-- sdd-owner: implementation -->

### 2.5 Re-key remaining components

- [ ] Re-key `src/components/ScanSearchBox.svelte` placeholder + error through `$LL.scan.*`. <!-- sdd-owner: implementation -->
- [ ] Re-key titles, buttons, placeholders, modals through `$LL.*` in `src/components/StoresPage.svelte`, `src/components/ProductCatalogPage.svelte`, `src/components/ProductForm.svelte`, `src/components/ProductDetailPage.svelte`, `src/components/LotForm.svelte`, `src/components/CalendarPage.svelte`, `src/components/BackupRestorePage.svelte`, `src/components/CsvImportPage.svelte`, `src/components/UnitReviewPage.svelte`, `src/components/LotMovementsPanel.svelte`. <!-- sdd-owner: implementation -->
- [ ] Re-key plural-aware copy in `src/components/UnitReviewBanner.svelte` through `$LL.unitReview.*`. <!-- sdd-owner: implementation -->
- [ ] Re-key placeholder + helper text in `src/components/inputs/CategoryPicker.svelte` through `$LL.categoryPicker.*`. <!-- sdd-owner: implementation -->
- [ ] Re-key modal helpers (`MoveStockModal`, `RegisterExitModal`, `AdjustCountModal`, `ResolveQuantityDialog`, `ArchiveLotDialog`, `LotMovementsPanel` dialogs) and any other user-visible English residue surfaced by `grep -RIn '>[A-Z][a-z]' src/components src/App.svelte | grep -v '\$LL'` after the per-component re-keying pass. <!-- sdd-owner: implementation -->

### 2.6 PR 2 verify gate

- [ ] Run `npx svelte-check --workspace . --threshold error`; resolve any missing-key diagnostics emitted by `typesafe-i18n` (the CLI itself fails the build when a key is absent from `es/index.ts`). <!-- sdd-owner: implementation -->
- [ ] Run `npm run build`; confirm `prebuild` regenerates `src/i18n/i18n.ts` and `vite build` exits green. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — Configuration page: locale selector shows `English` / `Español`; selecting one triggers `updateSettings({ language })` and the dropdown re-renders on success; first run with Spanish OS locale renders `Detectado: Español` above the selector and the hint disappears after a manual pick; restarting the app retains the manual pick; clearing the `language` row in `app_settings` and restarting re-shows the hint. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — PDF: with active locale `es`, export a PDF from the Reports page and verify the PDF body, filter line, column headers, and metadata title render the spec's Spanish strings; with active locale `en`, verify the English baseline is unchanged. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — notifications: trigger a daily alert in both locales and verify the title is `${prefix}${sku}` with the Spanish / English prefix from `$LL.notification.titlePrefix()` and the body uses the Spanish / English `bodyTemplate`; confirm the SKU is visible in both locales. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — error message: submit a lot with an invalid `notification_date` while the active locale is `es` and confirm the inline error begins with `Formato de`; repeat with `en` and confirm the original English message. <!-- sdd-owner: implementation -->

---

## Parent actions (lifecycle gates and review)

- [ ] Before PR 1 apply: confirm the chain strategy is `stacked-to-main` (PR 1 lands to `main`, PR 2 stacks onto PR 1). If `feature-branch-chain` is preferred, update the chain strategy before apply; do not invent an exception without explicit user acceptance. <!-- sdd-owner: parent -->
- [ ] Open PR 1 against `main` with the change description matching design §12.2; merge PR 1 once `cargo test --lib` is green and the manual Spanish PDF smoke (1.6) has been recorded. <!-- sdd-owner: parent -->
- [ ] After PR 1 merges: rebase PR 2 onto `main` (it now contains PR 1's IPC contract), open PR 2 against `main` with the change description matching design §12.2; if the final PR 2 diff exceeds 1500 net changed lines, escalate via `deliveryStrategy: ask-on-risk` per design §12.3 before merging. <!-- sdd-owner: parent -->
- [ ] Start or reuse a bounded review for the change covering: (a) the spec parity requirement between `en.json` and `es.json`, (b) the `pdf_messages` parity test in `src-tauri/src/pdf/locale.rs`, (c) the Configuration page's detected-hint behaviour across restart + manual pick + row-clear, (d) the SKU-visibility invariant in OS notifications, (e) the printpdf accent spot-check in Okular / Preview / Adobe Reader. <!-- sdd-owner: parent -->
- [ ] After both PRs land and review passes: archive the SDD artifacts (`proposal.md`, `specs/...`, `design.md`, this `tasks.md`) per the project's OpenSpec lifecycle; mark the change `applied` and close out the `sdd/caduxo-i18n-support/*` Engram topic keys once the bounded review receipt is filed. <!-- sdd-owner: parent -->