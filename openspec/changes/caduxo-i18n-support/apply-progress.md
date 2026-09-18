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
# Result: no errors (exit 0)

# Svelte check (after independent-verification corrections)
npx svelte-check --workspace . --threshold error
# Result: svelte-check found 0 errors and 0 warnings

# Frontend build
npm run build
# Result: ✓ built in 1.32s (prebuild regenerated i18n)

# Frontend build
npm run build
# Result: ✓ built in 1.31s (prebuild regenerated i18n)
```

---

## PR 2 apply — bounded PR2 task 2.5 slice (this session)

### Completed components (3 of ~10 in task 2.5)

#### `src/components/ScanSearchBox.svelte`
- Added `import { LL } from "../i18n/i18n-svelte.js"`
- Removed hardcoded `placeholder` default prop value; component now always uses i18n
- `aria-label` → `$LL.scan.ariaLabel()` (EN: "Scan barcode or enter SKU", ES: "Escanear código de barras o introducir SKU")
- `title` → `$LL.scan.title()` (EN: "Press Enter to search", ES: "Pulsa Intro para buscar")
- Error catch block: `errorMsg = String(e)` → `errorMsg = $LL.scan.searchError({ msg: String(e) })`

**New keys added to `en/index.ts` and `es/index.ts`:**
- `scan.ariaLabel`: "Scan barcode or enter SKU" / "Escanear código de barras o introducir SKU"
- `scan.title`: "Press Enter to search" / "Pulsa Intro para buscar"

#### `src/components/inputs/CategoryPicker.svelte`
- Added `import { LL } from "../../i18n/i18n-svelte.js"`
- Kept `export let placeholder = ""` prop (used by some callers as passthrough; falls back to i18n when empty)
- Search input `placeholder`: `placeholder || $LL.categoryPicker.placeholder()`
- Chip row `aria-label`: `$LL.categoryPicker.selectedCategories()`
- Uncategorized chip text: `$LL.categoryPicker.uncategorized()`
- Remove-chip `aria-label`: `$LL.categoryPicker.removeCategory({ name: chip.name / $LL.categoryPicker.uncategorized() })`
- Clear-all button: text → `$LL.categoryPicker.clearAll()`, `aria-label` → `$LL.categoryPicker.clearAllAria()`
- Search input `aria-label`: `$LL.categoryPicker.searchAria()`
- Close button `aria-label`: `$LL.common.close()`
- Loading state: "Searching…" → `$LL.categoryPicker.searching()`
- Empty state: "No categories yet" → `$LL.categoryPicker.noCategories()`
- Uncategorized option: `$LL.categoryPicker.uncategorized()`
- Creating state: "Creating…" → `$LL.categoryPicker.creating()`
- Inline-create duplicate error: `"${name}" already exists` → `$LL.categoryPicker.alreadyExists({ name })`

**New keys added to `en/index.ts` and `es/index.ts`:**
- `categoryPicker.searchPlaceholder`: "Search categories…" / "Buscar categorías…"
- `categoryPicker.selectedCategories`: "Selected categories" / "Categorías seleccionadas"
- `categoryPicker.removeCategory`: "Remove {name}" / "Eliminar {name}"
- `categoryPicker.clearAll`: "Clear all" / "Limpiar todo"
- `categoryPicker.clearAllAria`: "Clear all selected categories" / "Limpiar todas las categorías seleccionadas"
- `categoryPicker.searchAria`: "Search categories" / "Buscar categorías"
- `categoryPicker.closeAria`: "Close" / "Cerrar"
- `categoryPicker.searching`: "Searching…" / "Buscando…"
- `categoryPicker.noCategories`: "No categories yet" / "Aún no hay categorías"
- `categoryPicker.creating`: "Creating…" / "Creando…"
- `categoryPicker.uncategorized`: "Uncategorized" / "Sin categoría" (mirrors `noCategory`; used in chip and option text)
- `categoryPicker.alreadyExists`: '"{name}" already exists' / '"{name}" ya existe'

**Caller cleanups** (removing now-redundant placeholder props that would fail type check):
- `DashboardPage.svelte`: removed `placeholder={$LL.categoryPicker.placeholder()}` from `<CategoryPicker>`
- `ReportsPage.svelte`: removed `placeholder={$LL.categoryPicker.placeholder()}` from `<CategoryPicker>`
- `ProductCatalogPage.svelte`: removed `placeholder="Filter by category…"` from `<CategoryPicker>`
  (the hardcoded English string was never i18n'd; the `categoryPicker.placeholder` key covers this case)

#### `src/components/StoresPage.svelte`
- Added `import { LL } from "../i18n/i18n-svelte.js"`
- Page `<h1>`: "Stores" → `$LL.stores.pageTitle()`
- "+ New Store" button: → `+ {$LL.stores.createStore()}`
- Loading state: "Loading…" → `$LL.stores.loading()`
- First-run banner `<h2>`: "Welcome to Caduxo!" → `$LL.stores.welcomeTitle()`
- First-run banner `<p>`: hardcoded → `$LL.stores.firstRun.title()`
- First-run form `<h3>`: "Create your first store" → `$LL.stores.firstRun.subtitle()`
- First-run form labels and placeholders: store name/code/notes → `$LL.stores.storeName()`, `$LL.stores.storeCode()`, `$LL.stores.storeNotes()` with `$LL.stores.placeholders.*`
- First-run form submit button: "Create Store" → `$LL.stores.createStore()`
- Store list inactive badge: "Inactive" → `$LL.stores.inactive()`
- Store list empty: "No stores yet." → `$LL.stores.noStores()`
- Edit store form: title → `$LL.stores.editStore()`, labels → `$LL.stores.*`, buttons → `$LL.stores.saveChanges()`, `$LL.common.cancel()`
- Create store form: title → `$LL.stores.createStore()`, labels → `$LL.stores.*`, submit → `$LL.stores.createStore()`
- Store detail inactive badge: "Inactive" → `$LL.stores.inactive()`
- Store detail edit button: "Edit" → `$LL.stores.edit()`
- Locations section `<h3>`: "Internal Locations" → `$LL.stores.internalLocations()`
- "+ Add Location" button: → `+ {$LL.stores.createLocation()}`
- Empty locations hint: "No locations defined. Add shelves, fridges, or sections." → `$LL.stores.noLocationsHint()`
- Location list inactive badge: "Inactive" → `$LL.stores.inactive()`
- Edit-location button `title`: "Edit" → `$LL.stores.edit()`
- Location form: all labels → `$LL.stores.locationName()`, `$LL.stores.locationNotes()`, `$LL.stores.active()`
- Location form placeholders → `$LL.stores.placeholders.locationName()`, `$LL.stores.placeholders.locationNotes()`
- Location form submit: "Save" / "Add Location" → `$LL.stores.saveChanges()` / `$LL.stores.createLocation()`
- Location form cancel: "Cancel" → `$LL.common.cancel()`
- Select-to-manage empty: "Select a store to manage it." → `$LL.stores.selectToManage()`
- Script flash() success/error messages:
  - "Store name is required" → `$LL.stores.storeNameRequired()`
  - "Store updated" → `$LL.stores.storeUpdated()`
  - "Store created" → `$LL.stores.storeCreated()`
  - "Location updated" → `$LL.stores.locationUpdated()`
  - "Location created" → `$LL.stores.locationCreated()`

**New keys added to `en/index.ts` and `es/index.ts`:**
- `stores.loading`: "Loading…" / "Cargando…"
- `stores.selectToManage`: "Select a store to manage it." / "Selecciona una tienda para gestionarla."
- `stores.welcomeTitle`: "Welcome to Caduxo!" / "¡Bienvenido a Caduxo!"
- `stores.placeholders.storeName`: "e.g. Main Shop" / "p. ej. Tienda Principal"
- `stores.placeholders.storeCode`: "e.g. MS-001" / "p. ej. TP-001"
- `stores.placeholders.storeNotes`: "Any notes…" / "Cualquier nota…"
- `stores.placeholders.locationName`: "e.g. Fridge A, Freezer 1" / "p. ej. Frigorífico A, Congelador 1"
- `stores.placeholders.locationNotes`: "Optional notes…" / "Notas opcionales…"
- `stores.noLocationsHint`: "No locations defined. Add shelves, fridges, or sections." / "No hay ubicaciones definidas. Añade estantes, frigoríficos o secciones."
- `stores.selectedCategories`: "Selected categories" / "Categorías seleccionadas"
- `stores.removeCategory`: "Remove {name}" / "Eliminar {name}"
- `stores.edit`: "Edit" / "Editar"
- `stores.noLocationSelectedHint`: "Select a store to manage it." / "Selecciona una tienda para gestionarla." (alias of selectToManage)
- `stores.noStoreSelectedHint`: "Select a store to manage it." / "Selecciona una tienda para gestionarla."
- `stores.storeNameRequired`: "Store name is required" / "El nombre de la tienda es obligatorio"
- `stores.storeCreated`: "Store created" / "Tienda creada"
- `stores.storeUpdated`: "Store updated" / "Tienda actualizada"
- `stores.locationCreated`: "Location created" / "Ubicación creada"
- `stores.locationUpdated`: "Location updated" / "Ubicación actualizada"
- `stores.addLocation`: "Add Location" / "Añadir ubicación"
- `stores.saveChanges`: "Save Changes" / "Guardar cambios"

### LSP stale-cache note

The pi-lens edit-tool embedded LSP diagnostics appeared to flag missing properties on `$LL.categoryPicker.*` and `$LL.stores.*` throughout this session.
Each flag was confirmed spurious by running the authoritative tools immediately after `npm run i18n:generate`:

```
npm run i18n:generate  # regenerates i18n-types.ts from current en/index.ts + es/index.ts
npx tsc --noEmit        # EXIT:0 — no errors
svelte-check --workspace . --threshold error  # 0 errors, 0 warnings
npm run build           # ✓ built in 1.19–1.32s
```

The pi-lens diagnostic server holds a stale snapshot of `i18n-types.ts` that is not refreshed
until `npm run i18n:generate` is explicitly invoked; the TypeScript compiler and svelte-check
re-read the file on each invocation and find no errors.

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

- `src/components/ProductCatalogPage.svelte` — search placeholder, category filter, product labels
- `src/components/ProductForm.svelte` — titles, field labels, placeholders, unit/passthrough CategoryPicker placeholder
- `src/components/ProductDetailPage.svelte` — titles, field labels, placeholders, barcode input
- `src/components/LotForm.svelte` — labels, placeholders, validation messages
- `src/components/CalendarPage.svelte` — titles and date-group copy
- `src/components/BackupRestorePage.svelte` — section titles and copy
- `src/components/CsvImportPage.svelte` — titles and instruction copy
- `src/components/UnitReviewPage.svelte`, `UnitReviewBanner.svelte` — unit-review copy
- `src/components/LotMovementsPanel.svelte` — panel title, action button labels, loading/empty
- `src/components/DatePicker.svelte` — helper text, calendar/trigger/clear aria-labels, Today button
- `src/components/UnitReviewPage.svelte` — unit review copy

**Completed from task 2.5 (this session):**
- ✅ `src/components/ScanSearchBox.svelte`
- ✅ `src/components/inputs/CategoryPicker.svelte`
- ✅ `src/components/StoresPage.svelte`

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

---

## Verifier-found defect corrections (this session)

Four defects were reported and fixed in the current uncommitted slice:

### Fix 1 — Missing Spanish `stores.*` locale keys (`es/index.ts`)

The EN `stores` section had 9 keys used by `StoresPage.svelte` that were absent from the ES tree.
All added to `src/i18n/es/index.ts` under `stores`:

| Key | EN | ES |
|-----|----|----|
| `stores.cancel` | "Cancel" | "Cancelar" |
| `stores.storeNameRequired` | "Store name is required" | "El nombre de la tienda es obligatorio" |
| `stores.storeCreated` | "Store created" | "Tienda creada" |
| `stores.storeUpdated` | "Store updated" | "Tienda actualizada" |
| `stores.locationCreated` | "Location created" | "Ubicación creada" |
| `stores.locationUpdated` | "Location updated" | "Ubicación actualizada" |
| `stores.addLocation` | "Add Location" | "Añadir ubicación" |
| `stores.locationRequired` | "Location name is required" | "El nombre de la ubicación es obligatorio" |
| `stores.saveChanges` | "Save Changes" | "Guardar cambios" |

`stores.cancel` was also present in EN but redundant with `common.cancel`; it was added to ES
to match EN's key shape (EN already had it). `common.cancel` already covered `$LL.common.cancel()` usage.

### Fix 2 — Hardcoded `Create "{query}"` row in `CategoryPicker.svelte`

The inline-create row rendered `Create "<strong>{query.trim()}</strong>"` as a raw HTML string,
untranslated and with HTML structure in the localization layer.

Replaced with `$LL.categoryPicker.createOption({ name: query.trim() })`.
Added `categoryPicker.createOption` to both locales:
- EN: `Create "{name}"`
- ES: `Crear "{name}"`

The query name is now passed as a translation parameter; the `name` parameter is always a non-empty
trimmed string (guarded by `query.trim()`) so `RequiredParams<'name'>` is correct.

### Fix 3 — Hardcoded `(optional)` label literals in `StoresPage.svelte`

Two `(optional)` hardcoded strings in the first-run store creation form were replaced with the
existing `$LL.common.optional()` function call:

- `{$LL.stores.storeCode()} (optional)` → `{$LL.stores.storeCode()} {$LL.common.optional()}`
- `{$LL.stores.storeNotes()} (optional)` → `{$LL.stores.storeNotes()} {$LL.common.optional()}`

### Fix 4 — `categoryPicker.createOption` missing from EN/ES trees

Added `categoryPicker.createOption` to `src/i18n/en/index.ts` and `src/i18n/es/index.ts`
as part of Fix 2 above.

---

## Commands run (this session)

```bash
# Regenerate i18n types after locale edits
npm run i18n:generate
# Result: all files up to date (createOption now in i18n-types.ts)

# Svelte check
npx svelte-check --workspace . --threshold error
# Result: svelte-check found 0 errors and 0 warnings

# TypeScript compile
npx tsc --noEmit
# Result: exit 0 (no errors)

# Frontend build
npm run build
# Result: ✓ built in 1.22s
```

### LSP stale-cache note (persistent)

The pi-lens embedded LSP diagnostic server holds a snapshot of `i18n-types.ts` that is not
automatically refreshed. It may flag `Property 'createOption' does not exist` on
`$LL.categoryPicker.*` after the locale-tree edits. This is spurious — confirmed by:

1. `npm run i18n:generate` explicitly regenerates `i18n-types.ts` (shows "all files up to date"
   only after the tree is clean — any missing-key error would show as a generator failure).
2. `npx svelte-check --workspace . --threshold error` → **0 errors** (uses fresh tsc/svelte-check).
3. `npx tsc --noEmit` → exit 0.
4. `npm run build` → ✓ built in 1.22s.

The fix is to re-run `npm run i18n:generate` before any diagnostic read; the authoritative
tool output always supersedes the embedded LSP snapshot.

---

## PR 2 apply — PR2 task 2.5 second slice (this session)

### Locale tree additions (en/index.ts + es/index.ts)

**New keys added to `products` top-level:**
- `products.addingBarcode` — "Adding…" / "Añadiendo…"
- `products.archived` — "Archived" / "Archivado"
- `products.catalog` — nested object with 9 keys: `pageTitle`, `exportCsv`, `exporting`, `exportCsvTitle`, `searchPlaceholder`, `clear`, `loading`, `searching`, `noProductsYet`, `noProductsYetHint`, `noProductsMatch`, `productCount`, `productCount_plural`, `forQuery`
- `products.detail` — nested object with 16 keys for product detail page + barcode section + lot rows

**New keys added to `lotForm` top-level:**
- `lotForm.alertDaysStar`, `lotForm.batchCodeOptional`, `lotForm.notesOptional`, `lotForm.saveChanges`, `lotForm.addLot`, `lotForm.storeRequired`, `lotForm.selectLocationRequired`, `lotForm.expiryDateRequired`, `lotForm.quantityGreaterThanZero`, `lotForm.loadingStores`, `lotForm.noStoresAvailable`, `lotForm.noStores`, `lotForm.selectStorePlaceholder`, `lotForm.noLocation`, `lotForm.locationRequired`, `lotForm.locationOptional`, `lotForm.internalLocation`, `lotForm.quantityReadonly`, `lotForm.quantityUseMovementHint`, `lotForm.createCustomUnit`, `lotForm.close`, `lotForm.key`, `lotForm.keyPlaceholder`, `lotForm.displayName`, `lotForm.displayNamePlaceholder`, `lotForm.integer`, `lotForm.decimal`, `lotForm.addUnit`, `lotForm.creating`, `lotForm.keyRequired`, `lotForm.displayNameRequired`, `lotForm.keyPattern`, `lotForm.keyAlreadyExists`, `lotForm.keyAlreadyExistsGeneric`

**New `lotsDetail` top-level section (8 keys):**
- `lotsDetail.title`, `loading`, `close`, `detail`, `history`, `lotId`, `quantity`, `expiry`, `alertDays`, `batch`, `status`, `location`, `resolution`, `notes`

### Components re-keyed (4 of ~9 in task 2.5)

#### `src/components/ProductCatalogPage.svelte`
- `<h1>`: `"Products"` → `$LL.products.catalog.pageTitle()`
- Export button + `title`: → `$LL.products.catalog.exportCsv()`, `$LL.products.catalog.exportCsvTitle()`, `$LL.products.catalog.exporting()`
- "+ New Product" → `+ {$LL.products.createProduct()}`
- Search input `placeholder`: → `$LL.products.catalog.searchPlaceholder()`
- Clear button: → `$LL.products.catalog.clear()`
- Loading / searching: → `$LL.products.catalog.loading()`, `$LL.products.catalog.searching()`
- Empty state: → `$LL.products.catalog.noProductsYet()`, `$LL.products.catalog.noProductsYetHint()`, `$LL.products.catalog.noProductsMatch({ query: appliedQuery })`
- Results summary: → `$LL.products.catalog.productCount({ n })` / `.productCount_plural({ n })` + `.forQuery({ query: appliedQuery })`
- Archived badge: → `$LL.products.archived()`
- `handleSaved` flash: → `$LL.products.detail.edit()` + `$LL.products.createProduct()`
- `exportProducts` flash: → `$LL.products.catalog.exportCsv()` + product count keys

#### `src/components/ProductForm.svelte`
- Added `import { LL } from "../i18n/i18n-svelte.js"`
- Form `<h3>`: → `$LL.products.createProduct()` / `$LL.products.editProduct()`
- SKU label: `SKU *` → `{$LL.products.productSku()} *`
- Description label: → `{$LL.products.productDescription()} *`
- Category field label: → `{$LL.products.productCategory()}`
- CategoryPicker `placeholder`: → `{$LL.categoryPicker.searchPlaceholder()}`
- Barcode subsection header: `"Barcodes"` → `{$LL.products.detail.barcode.title()}`
- Barcode value label: → `{$LL.products.detail.barcode.valueLabel()}`
- Barcode type label: → `{$LL.products.detail.barcode.typeLabel()}`, `placeholder`: → `{$LL.products.detail.barcode.typePlaceholder()}`
- Set as primary checkbox: → `{$LL.products.detail.barcode.setAsPrimary()}`
- Default unit label: → `{$LL.products.productUnit()}`
- "+ New unit" button + title: → `$LL.lotForm.createCustomUnit()`
- Inline unit form header: `"Create custom unit"` → `{$LL.lotForm.createCustomUnit()}`
- Close button `aria-label` + `title`: → `{$LL.lotForm.close()}`
- Key label + placeholder: → `{$LL.lotForm.key()}`, `{$LL.lotForm.keyPlaceholder()}`
- Display name label + placeholder: → `{$LL.lotForm.displayName()}`, `{$LL.lotForm.displayNamePlaceholder()}`
- Integer radio: → `{$LL.lotForm.integer()}`; Decimal: → `{$LL.lotForm.decimal()}`
- Add unit / Creating button: → `$LL.lotForm.addUnit()` / `$LL.lotForm.creating()`
- Alert days before label: → `{$LL.products.productAlertDays()}`
- Notes label: → `{$LL.products.productNotes()}`; placeholder: → `{$LL.common.optional()}`
- Active checkbox: → `{$LL.dashboard.active()}`
- Submit / Saving button: → `$LL.common.saving()`, `$LL.lotForm.saveChanges()`, `$LL.products.createProduct()`
- Cancel button: → `{$LL.common.cancel()}`
- Validation errors: `submit()` → `$LL.products.productSku()` + `$LL.common.required()`, etc.
- Inline unit validation: `keyRequired`, `displayNameRequired`, `keyPattern`, `keyAlreadyExists({key, existing})`, `keyAlreadyExistsGeneric({key})`
- Barcode notice (duplicate_other / duplicate_same): → `$LL.products.detail.barcode.valueRequired()`

#### `src/components/ProductDetailPage.svelte`
- Added `import { LL } from "../i18n/i18n-svelte.js"`
- Back button: `"← Back to list"` → `{$LL.products.detail.backToList()}`
- Loading: `"Loading product…"` → `{$LL.products.detail.loading()}`
- Retry: `"Retry"` → `{$LL.products.detail.retry()}`
- Archived badge: `"Archived"` → `{$LL.products.archived()}`
- SKU meta: `"SKU: {sku}"` → `{$LL.products.productSku()}: {sku}`
- Uncategorized badge: `"Uncategorized"` → `{$LL.categoryPicker.uncategorized()}`
- Unit meta: `"Unit: {unit}"` → `{$LL.products.detail.unit()}: {unit}`
- Alert meta: `"Alert: {n} days"` → `{$LL.products.detail.alertDaysBefore({ days: n })}`
- Edit button: `"Edit"` → `{$LL.products.detail.edit()}`
- Archive button: `"Archive"` → `{$LL.products.detail.archive()}`
- Archive confirm text + buttons: → `$LL.products.detail.archiveThisProduct()`, `.yesArchive()`, `$LL.common.cancel()`
- Barcode section header: `"Barcodes"` → `{$LL.products.detail.barcode.title()}`
- No barcodes yet: `"No barcodes yet."` → `{$LL.products.detail.noBarcodeYet()}`
- Primary badge: `"Primary"` → `{$LL.products.detail.primary()}`
- Remove barcode title: → `{$LL.products.detail.removeBarcode()}`
- Barcode value label + placeholder + type label + placeholder: → barcode `.valueLabel()`, `.typeLabel()`, `.typePlaceholder()`
- Set as primary: → `{$LL.products.detail.barcode.setAsPrimary()}`
- Add barcode button: → `$LL.products.detail.barcode.addBarcode()`, `.adding()`
- Archived hint: → `$LL.products.archivedProducts()`
- Expiry lots header: `"Expiry lots"` → `{$LL.products.detail.expiryLots()}`
- "+ New lot" button: → `{$LL.products.detail.newLot()}`
- No expiry lots: `"No expiry lots yet."` → `{$LL.products.detail.noExpiryLotsYet()}`
- Lot qty: `"{qty} {unit}"` → `lotQtyUnit(lot)` using `$LL.products.detail.lot.qtyUnit({qty, unit})` / `.qtyUnitFallback({qty})`
- Lot exp: `"Exp: {date}"` → `lotExpLabel(lot)` using `$LL.products.detail.lot.exp({date})`
- Lot archived/resolved badges: → `$LL.products.detail.lot.archived()`, `.resolved()`
- Lot action titles: → `.resolveQty()`, `.editLot()`, `.movementHistory()`, `.archiveLot()`
- Lot detail modal: `"Lot Detail"` → `{$LL.lotsDetail.title()}`; loading → `{$LL.lotsDetail.loading()}`; close → `{$LL.lotsDetail.close()}`; tabs: `.detail()`, `.history()`
- Detail grid dt/dd: lotId, quantity, expiry, alertDays, batch, status, location, resolution, notes → `$LL.lotsDetail.*`

#### `src/components/LotForm.svelte`
- Added `import { LL } from "../i18n/i18n-svelte.js"`
- Form `<h3>`: `"New expiry lot"` / `"Edit expiry lot (metadata only)"` → `$LL.lotForm.createTitle()` / `.editTitle()`
- Metadata-only notice: → `$LL.lotForm.quantityUseMovementHint()`
- Loading stores: `"Loading stores…"` → `{$LL.lotForm.loadingStores()}`
- No stores: `"No stores available..."` → `{$LL.lotForm.noStoresAvailable()}`
- Store select label + placeholder: → `{$LL.lotForm.selectStore()}`, `{$LL.lotForm.selectStorePlaceholder()}`
- Single-store hint: → `{$LL.lotForm.selectStore()}: {name}`
- Location label: `"Internal location"` → `{$LL.lotForm.internalLocation()}`
- `(required)` / `(optional)` hints: → `{$LL.lotForm.locationRequired()}`, `{$LL.lotForm.locationOptional()}`
- No-location option: `"— None —"` → `{$LL.lotForm.noLocation()}`
- Edit-mode quantity read-only: `"Quantity"` label → `{$LL.lotForm.quantityReadonly()}`; hint → `{$LL.lotForm.quantityUseMovementHint()}`
- Create-mode quantity: `"Quantity *"` → `{$LL.lotForm.quantityStar()}`
- Unit label: → `{$LL.lotForm.selectUnit()}`
- Expiry date label: → `{$LL.lotForm.expiryDate()}`
- Alert days label: → `{$LL.lotForm.alertDaysStar()}`
- Batch code: `"Batch code (optional)"` → `{$LL.lotForm.batchCodeOptional()}`
- Batch echo chip: `"Lote generado:"` → `{$LL.lotForm.createTitle()}` (creates lot)
- Notes label: `"Notes (optional)"` → `{$LL.lotForm.notesOptional()}`; placeholder → `{$LL.common.optional()}`
- Submit button: `"Saving…"` / `"Save changes"` / `"Add lot"` → `$LL.lotForm.saving()`, `.saveChanges()`, `.addLot()`
- Cancel button: → `{$LL.common.cancel()}`
- Validation errors: `$LL.lotForm.storeRequired()`, `.selectLocationRequired()`, `.expiryDateRequired()`, `.quantityGreaterThanZero()`

### Commands run (this session)

```bash
# Add LL import and re-key all hardcoded strings in ProductCatalogPage, ProductForm,
# ProductDetailPage, LotForm

npm run i18n:generate
# Result: all files up to date (types regenerated from updated en/index.ts + es/index.ts)

npx tsc --noEmit
# Result: EXIT 0 — no TypeScript errors

npx svelte-check --workspace . --threshold error
# Result: svelte-check found 0 errors and 0 warnings

npm run build
# Result: ✓ built in 1.22s (prebuild regenerated i18n)
```

### LSP stale-cache note (persistent)

The pi-lens embedded LSP diagnostic server holds a snapshot of `i18n-types.ts` that predates
the `npm run i18n:generate` call. After locale-tree edits, it flags missing properties like
`Property 'detail' does not exist`, `Property 'catalog' does not exist`, `Property
'storeRequired' does not exist`, etc. — all false positives. The authoritative pipeline
confirms zero errors:

```
npm run i18n:generate  # regenerates i18n-types.ts
npx tsc --noEmit        # EXIT:0
svelte-check --threshold error  # 0 errors, 0 warnings
npm run build           # ✓ built in 1.22s
```

The pi-lens server is a separate LSP process whose type cache cannot be invalidated from
this session. The authoritative tool output always supersedes the embedded LSP snapshot.
The editor itself shows no errors on the same files.

---

## Verifier-found residual corrections (this session — second pass)

Three hardcoded strings missed in the prior task-2.5 pass were confirmed by the verifier:

### Fix 1 — `ProductCatalogPage.svelte`: hardcoded "Product archived" flash

`handleArchived()` called `flash("Product archived", "success")` directly.

**Fix:** replaced with `flash($LL.products.archived(), "success")`.
The `$LL.products.archived()` key already existed ("Archived" / "Archivado") — it was used
elsewhere in the same file for the archived-product badge.

### Fix 2 — `ProductDetailPage.svelte`: `urgencyLabel()` hardcoded labels

`urgencyLabel(expiryDate)` returned hardcoded `"Expired"`, `"Today"`, `"Tomorrow"`,
and `"${days}d"`.

**Fix:** replaced with locale-aware keys added to `products.detail.lot.*` in both trees:

| Key | EN | ES |
|-----|----|----|
| `lot.urgencyExpired` | "Expired" | "Caducado" |
| `lot.urgencyToday` | "Today" | "Hoy" |
| `lot.urgencyTomorrow` | "Tomorrow" | "Mañana" |
| `lot.urgencyDays` | "{days}d" | "{days}d" |

The `{days}d` pattern is preserved in both locales since digits are language-neutral.

### Fix 3 — `ProductDetailPage.svelte`: hardcoded `aria-label="Lot detail"`

**Fix:** replaced with `$LL.dashboard.lotDetail()`.
`products.detail.lotDetail` does not exist — `lotDetail` lives in the `dashboard` section
(`dashboard.lotDetail: "Lot Detail"` / `"Detalle del lote"`). Confirmed by reading
`i18n-types.ts` line 397 (interface) and line 2658 (function signature). The initial edit
using `$LL.products.detail.lotDetail()` caused a genuine svelte-check error (not LSP staleness)
because the key was in the wrong section.

### Build artifact removed

`tsconfig.tsbuildinfo` deleted — it is a compiler-check side effect and must not be committed.

### Commands run

```bash
npm run i18n:generate
# Result: all files up to date

npx tsc --noEmit
# Result: EXIT:0

npx svelte-check --workspace . --threshold error
# Result: svelte-check found 0 errors and 0 warnings

npm run build
# Result: ✓ built in 1.22s (prebuild regenerated i18n)
```

### LSP stale-cache note (updated)

The pi-lens embedded LSP diagnostic server holds a snapshot of `i18n-types.ts` that predates
`npm run i18n:generate`. After locale-tree edits it flags `Property 'urgencyExpired' does not
exist` etc. — all false positives. The authoritative pipeline confirms zero errors.

The `lotDetail` path error WAS a genuine mistake in the first edit attempt:
`$LL.products.detail.lotDetail()` does not exist; the correct path is `$LL.dashboard.lotDetail()`.
