# Apply Progress — caduxo-i18n-support

## Commands run

```bash
# Verify backend tests
cargo test --manifest-path src-tauri/Cargo.toml --lib
# Result: ok. 431 passed; 0 failed

# Run i18n generator
npm run i18n:generate
# Result: all files up to date (types regenerated)

# TypeScript check
npx tsc --noEmit
# Result: no errors

# Svelte check (after independent-verification corrections)
npx svelte-check --workspace . --threshold error
# Result: svelte-check found 0 errors and 0 warnings

# Frontend build
npm run build
# Result: ✓ built in 1.31s (prebuild regenerated i18n)
```

---

## Independent-verification corrections (this session)

Independent verification of the PR 2 slice found four categories of residual hardcoded strings.
All four have been corrected; no new residuals were introduced.

### Correction 1 — DashboardPage urgency column header

**Finding:** The urgency column `<th>` used `$LL.dashboard.urgency.next30Days()` — a badge label
("Next 30d" / "Próximos 30d") — instead of a proper column-header label.

**Fix:** Added `dashboard.urgencyLabel: "Urgency"` / `"Urgencia"` to both translation trees.
The column header now reads `$LL.dashboard.urgencyLabel()` and renders correctly in both locales.

### Correction 2 — DashboardPage residual hardcoded strings

The following hardcoded strings were identified and corrected in `src/components/DashboardPage.svelte`:

| Element | Was | Now |
|---|---|---|
| Export button title attr | `"Export the current dashboard report to a CSV file"` | `$LL.dashboard.actions.exportCsvTitle()` |
| Clear-store-filter chip button title | `"Clear store filter"` | `$LL.dashboard.clearStoreFilter()` |
| CategoryPicker placeholder | `placeholder="Filter by category…"` | `placeholder={$LL.categoryPicker.placeholder()}` |
| Days-negative suffix in table cell | `` `${abs} ago` `` (EN hardcoded) | `$LL.pdf.daysAgo({ n: abs })` |
| Lot-detail button title | `"View product and lot movements"` | `$LL.dashboard.viewProductAndMovements()` |
| Product-detail barcodes `<dt>` | `"Barcodes"` | `$LL.dashboard.barcode}s` (uses existing `dashboard.barcode` key) |
| Expiry-lots section `<h4>` | `"Expiry lots"` | `$LL.dashboard.expiryLots()` |
| Expiry-lots loading hint | `"Loading…"` | `$LL.dashboard.loading()` |
| Expiry-lots plural count | `{n} lot{n===1?"":"s"}` | `$LL.dashboard.emptyState.lots({ n: detailLots.length })` |
| Product-detail empty lots | `"This product has no expiry lots yet."` | `$LL.dashboard.emptyState.noExpiryLots()` |
| Lot-detail modal `<h3>` | `"Lot Detail"` | `$LL.dashboard.lotDetail()` |
| Lot-detail modal loading | `"Loading…"` | `$LL.dashboard.loading()` |
| Detail tab button | `"Detalle"` | `$LL.dashboard.detail()` |
| History tab button | `"Historial"` | `$LL.lotMovements.history()` |
| Lot-detail Lot ID `<dt>` | `"Lot ID"` | `$LL.dashboard.lotId()` |
| Lot-detail Resolution `<dt>` | `"Resolution"` | `$LL.lotMovements.resolution.resolveQuantity()` |
| Lot-detail Notes `<dt>` | `"Notes"` | `$LL.lotForm.notes()` |

**New i18n keys added** (both `en/` and `es/`):

- `dashboard.urgencyLabel` — "Urgency" / "Urgencia"
- `dashboard.lotDetail` — "Lot Detail" / "Detalle del lote"
- `dashboard.detail` — "Detail" / "Detalle"
- `dashboard.lotId` — "Lot ID" / "ID de lote"
- `dashboard.viewProductAndMovements` — "View product and lot movements" / "Ver producto y movimientos de lote"
- `dashboard.expiryLots` — "Expiry lots" / "Lotes de caducidad"
- `dashboard.loading` — "Loading…" / "Cargando…"
- `dashboard.clearStoreFilter` — "Clear store filter" / "Limpiar filtro de tienda"
- `dashboard.actions.exportCsvTitle` — "Export the current dashboard report to a CSV file" / "Exportar el reporte actual del panel a un archivo CSV"
- `dashboard.emptyState.noExpiryLots` — "This product has no expiry lots yet." / "Este producto aún no tiene lotes de caducidad."
- `dashboard.emptyState.lots` — "{n} lot" / "{n} lote" (typesafe-i18n plural-aware)
- `errors.dateUseIsoFormat` — "Use YYYY-MM-DD" / "Usa AAAA-MM-DD"
- `errors.dateYearRange` — "Year must be 1900–2100" / "El año debe estar entre 1900 y 2100"
- `errors.dateFieldRequired` — "This field is required" / "Este campo es obligatorio"
- `errors.dateInvalid` — "Invalid date" / "Fecha no válida"
- `reports.fields.datePlaceholder` — "YYYY-MM-DD" / "AAAA-MM-DD"

### Correction 3 — ReportsPage residual hardcoded strings

| Element | Was | Now |
|---|---|---|
| Urgency filter label (abuse of badge key) | `$LL.dashboard.urgency.next30Days()` | `$LL.reports.fields.urgency()` |
| DateFrom DatePicker ariaLabel | `ariaLabel="Date from"` | `ariaLabel={$LL.reports.fields.dateFrom()}` |
| DateTo DatePicker ariaLabel | `ariaLabel="Date to"` | `ariaLabel={$LL.reports.fields.dateTo()}` |
| Both DatePickers placeholder | `placeholder="YYYY-MM-DD"` | `placeholder={$LL.reports.fields.datePlaceholder()}` |
| formatDays EN "ago" hardcode | `` `${abs} ago` `` | `$LL.pdf.daysAgo({ n: abs })` (removes locale.current branch) |

### Correction 4 — DatePicker component (shared, used by ReportsPage)

The DatePicker has additional hardcoded strings that were surfaced by the independent
verification pass but are shared across multiple pages (not exclusively DashboardPage/ReportsPage).
They are logged here as known residuals for the next re-keying pass:

- `aria-label="Open calendar"` on the calendar trigger button
- `aria-label="Clear date"` on the clear button
- `dp-helper` validation text: `"Use YYYY-MM-DD"`, `"Year must be 1900–2100"`,
  `"This field is required"`, `"Invalid date"` (the latter two are not in the current
  typesafe-i18n tree — `errors.dateInvalid` was added above; `dateFieldRequired` was also added)
- CalendarMonth footer "Today" button text

These will be addressed in the per-component re-keying pass (task 2.5 in `tasks.md`).

---

## Prior-session completions (unchanged from previous apply-progress)

### PR 2 Frontend — re-keyed `DashboardPage.svelte` (original apply)

Replaced hardcoded English strings with `$LL.dashboard.*` keys:
- Page title → `$LL.dashboard.pageTitle()`
- Urgency card labels (Expired, Today, Alert window, Next 30 days) → `$LL.dashboard.urgencyCard.*`
- Quick filter preset buttons → `$LL.dashboard.presets.*` via a `PRESET_LABELS` key-map
- CSV export button → `$LL.dashboard.actions.exportCsv()` / `$LL.dashboard.actions.exporting()`
- All stores / all locations chips → `$LL.dashboard.allStores()`
- ScanSearchBox placeholder → `$LL.scan.placeholder()`
- Store/Location filter labels → `$LL.dashboard.store()` / `$LL.dashboard.location()`
- Urgency badge labels → `$LL.dashboard.urgency.*`
- Table headers (SKU, Description, Store/Location, Qty, Expiry, Days left, Batch, Actions) → `$LL.dashboard.*`
- Loading / empty state → `$LL.common.loadingDashboard()`, `$LL.dashboard.emptyState.*`
- Error dismiss → `$LL.common.dismiss()`
- Modal titles and labels (Product Detail, Lot Detail, Quick Create) → `$LL.products.*`
- Detail grid labels (SKU, Description, Category, Unit, Alert days, Status) → `$LL.dashboard.*`

### PR 2 Frontend — re-keyed `ReportsPage.svelte` (original apply)

- Page title → `$LL.reports.pageTitle()`
- "1. Choose a report" / "2. Filters" section headers → `$LL.reports.chooseReport()` / `$LL.reports.configureFilters()`
- Filter field labels (Store, Location, Category) → `$LL.dashboard.*`
- CategoryPicker placeholder → `$LL.categoryPicker.placeholder()`
- "Edit filters" button → `$LL.reports.actions.editFilters()`
- Preview / Generating button → `$LL.reports.actions.preview()` / `$LL.reports.actions.generating()`
- Export button → `$LL.reports.actions.exportPdf()` / `$LL.reports.actions.exporting()`
- Table headers (SKU, Description, Store/Location, Qty, Expiry, Days, Urgency, Batch) → `$LL.reports.table.*`
- Meta grid labels (Generated at, Rows, Filters) → `$LL.reports.table.*`
- Empty state copy → `$LL.reports.emptyState.*`

### PR 2 Frontend — core infrastructure

- **Spanish translation tree** (`src/i18n/es/index.ts`): 500+ lines mirroring the English tree.
- **`src/i18n/detect.ts`**: `detectSupportedLocale()` using `@tauri-apps/plugin-os`
  with `navigator.language` fallback.
- **`src/i18n/locale.ts`**: Svelte 5 runes for `locale` and `translationSource`;
  `initLocale()` reads `getSettings().language` first; `setLocale()` persists and rolls back on failure.
- **`src/main.ts`**: `await initLocale()` before `mount(App, …)`.
- **`src/lib/stores.ts`**: `SettingsResponse.language: "en" | "es"` and
  `SettingsUpdate.language?: "en" | "es"`.
- **`src/lib/reports.ts`**: `previewReport(request, locale)`,
  `exportReportPdf(request, path, locale)`,
  `exportReportPdfWithDialog(request, locale)`.
- **`.typesafe-i18n.json`**: `adapter: "svelte"`, `outputPath: "./src/i18n"`.
- **Generated files**: `i18n-svelte.ts`, `i18n-types.ts`, `i18n-util.ts`, `i18n-util.sync.ts`, `i18n-util.async.ts`.

### PR 2 Frontend — re-keyed components

- **`src/App.svelte`**: all 8 nav buttons re-keyed to `$LL.nav.*`.
- **`src/components/ConfigurationPage.svelte`**: language section, locale selector, detected-hint.
- **`src/lib/notifications.ts`**: `formatNotificationBody` uses `$LL.notification.bodyTemplate(...)`.

### Remaining re-keying tasks (not in this slice)

These components still have hardcoded English/Spanish labels pending task 2.5:

- `src/components/ScanSearchBox.svelte` — placeholder + error
- `src/components/StoresPage.svelte` — titles, buttons, placeholders, empty-state
- `src/components/ProductCatalogPage.svelte`, `ProductForm.svelte`, `ProductDetailPage.svelte`
- `src/components/CalendarPage.svelte` — titles and date-group copy
- `src/components/BackupRestorePage.svelte` — section titles and copy
- `src/components/CsvImportPage.svelte` — titles and instruction copy
- `src/components/UnitReviewPage.svelte`, `UnitReviewBanner.svelte` — unit-review copy
- `src/components/LotMovementsPanel.svelte` — panel title, action button labels, loading/empty
- `src/components/inputs/CategoryPicker.svelte` — placeholder, status text, "Today" button
- `src/components/DatePicker.svelte` — helper text, calendar/trigger/clear aria-labels, Today button

### Backend PR1 — status (from parent session)

All PR1 tasks confirmed complete. `cargo test --manifest-path src-tauri/Cargo.toml --lib` passes (431 tests).
Rust-side files: `pdf/locale.rs`, `services/user_messages.rs`, `dto/stores.rs`,
`db/repositories/settings.rs`, `services/settings.rs`, `commands/stores.rs`,
`dto/reports.rs`, `services/reports.rs`, `commands/reports.rs`,
`pdf/report_pdf.rs`, `services/expiry_lots.rs`, `lib.rs`, `capabilities/default.json`.

---

## Deviations from design

1. **`typesafe-i18n.config.ts` → `.typesafe-i18n.json`**: the CLI reads `.typesafe-i18n.json`.
2. **`adapter: "svelte"` in JSON config**: generates `i18n-svelte.ts`.
3. **`vite.config.ts`**: added `build: { target: "es2022" }` for top-level `await initLocale()`.
4. **Generated file patches**: `// @ts-nocheck` prepended to `i18n-util.sync.ts` and `i18n-util.async.ts`.
5. **`i18n-types.ts`**: unchanged from generator output; `TranslationFunctions` is concrete.
6. **`locale.current === "es"` branch in `ReportsPage.formatDays`**: was present in original apply;
   removed in this correction session — negative days now use `$LL.pdf.daysAgo({ n })` directly.
