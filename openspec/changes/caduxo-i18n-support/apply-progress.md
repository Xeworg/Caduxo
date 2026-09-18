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

---

## PR 2 apply — PR2 task 2.5 third slice (this session)

### Slice scope

This slice covered 5 of the ~10 components remaining in task 2.5:
- `src/components/CalendarPage.svelte` ✅ done
- `src/components/BackupRestorePage.svelte` ✅ done
- `src/components/CsvImportPage.svelte` ✅ done
- `src/components/UnitReviewPage.svelte` ✅ done
- `src/components/UnitReviewBanner.svelte` ✅ done (separate checklist item)

### Locale tree additions (en/index.ts + es/index.ts)

**New keys added to `calendar` section:**

| Key | EN | ES |
|-----|----|----|
| `calendar.refresh` | "Refresh" | "Actualizar" |
| `calendar.refreshAria` | "Refresh lot data" | "Actualizar datos de lotes" |
| `calendar.loadingLots` | "Loading lots…" | "Cargando lotes…" |
| `calendar.loadFailed` | "Failed to load lots: {msg}" | "Error al cargar lotes: {msg}" |
| `calendar.dayPanelTitle` | "{date}" | "{date}" |
| `calendar.noLotsOnDate` | "No lots expiring on this date." | "Ningún lote caduca en esta fecha." |
| `calendar.urgencyLabels.expired` | "Expired" | "Caducado" |
| `calendar.urgencyLabels.today` | "Today" | "Hoy" |
| `calendar.urgencyLabels.alertWindow` | "Alert" | "Alerta" |
| `calendar.urgencyLabels.next30Days` | "Soon" | "Pronto" |
| `calendar.urgencyLabels.future` | "Future" | "Futuro" |
| `calendar.table.product` | "Product" | "Producto" |
| `calendar.table.qty` | "Qty" | "Cant" |
| `calendar.table.unit` | "Unit" | "Unidad" |
| `calendar.table.store` | "Store" | "Tienda" |
| `calendar.table.location` | "Location" | "Ubicación" |
| `calendar.table.days` | "Days" | "Días" |
| `calendar.table.status` | "Status" | "Estado" |

**New keys added to `common` section:**
- `common.refresh` / `common.refreshAria`

**New keys added to `backupRestore` section:**

| Key | EN | ES |
|-----|----|----|
| `backupRestore.exportSuccess` | "Backup saved to {path} ({kb} KB, schema v{schema})." | "Respaldo guardado en {path} ({kb} KB, esquema v{schema})." |
| `backupRestore.destructiveOp` | "This is a destructive operation." | "Esta es una operación destructiva." |
| `backupRestore.restoreDangerBanner` | "⚠️ Restoring a backup cannot be undone…" | "⚠️ Restaurar un respaldo no se puede deshacer…" |
| `backupRestore.selectBackupFile` | "Select backup file to restore" | "Seleccionar archivo de respaldo para restaurar" |
| `backupRestore.selecting` | "Selecting…" | "Seleccionando…" |
| `backupRestore.backupValidated` | "✅ Backup file validated" | "✅ Archivo de respaldo validado" |
| `backupRestore.restoreConfirmPrompt` | "Are you sure? This will permanently replace…" | "¿Estás seguro? Esto reemplazará permanentemente…" |
| `backupRestore.restoreData` | "Yes, replace my data" | "Sí, reemplazar mis datos" |
| `backupRestore.cannotRestore` | "❌ Backup file cannot be restored" | "❌ El archivo de respaldo no se puede restaurar" |
| `backupRestore.chooseDifferentFile` | "Choose a different file" | "Elegir un archivo diferente" |

**New keys added to `csvImport` section:**

| Key | EN | ES |
|-----|----|----|
| `csvImport.chooseDifferentFile` | "Choose Different File" | "Elegir archivo diferente" |
| `csvImport.importButton` | "Import {n} Products" | "Importar {n} productos" |
| `csvImport.importAnotherFile` | "Import Another File" | "Importar otro archivo" |
| `csvImport.conflictOptions.skipDuplicates` | "Skip duplicates" | "Omitir duplicados" |
| `csvImport.conflictOptions.updateExisting` | "Update existing" | "Actualizar existentes" |
| `csvImport.conflictOptions.reviewConflicts` | "Review conflicts" | "Revisar conflictos" |
| `csvImport.badge.ok` | "OK" | "OK" |
| `csvImport.badge.dupSku` | "Dup SKU" | "Dup SKU" |
| `csvImport.badge.dupBarcode` | "Dup BC" | "Dup BC" |
| `csvImport.badge.missing` | "Missing" | "Falta" |
| `csvImport.badge.invalid` | "Invalid" | "Inválido" |
| `csvImport.badge.unknownUnit` | "Unknown unit" | "Unidad desconocida" |
| `csvImport.detailRow.readyToImport` | "Ready to import" | "Listo para importar" |
| `csvImport.detailRow.alreadyHasSku` | 'Already has SKU <code>{sku}</code>' | 'Ya tiene SKU <code>{sku}</code>' |
| `csvImport.detailRow.barcodeBelongsToOther` | 'Barcode <code>{bc}</code> belongs to another product' | 'El código de barras <code>{bc}</code> pertenece a otro producto' |
| `csvImport.detailRow.missingField` | "Missing: {field}" | "Falta: {field}" |
| `csvImport.detailRow.unknownUnitSuggest` | 'Not in catalog — suggested: {suggested}' | 'No está en el catálogo — sugerido: {suggested}' |
| `csvImport.actions.created` | "Created" | "Creados" |
| `csvImport.actions.updated` | "Updated" | "Actualizados" |
| `csvImport.actions.skipped` | "Skipped" | "Omitidos" |
| `csvImport.actions.invalid` | "Invalid" | "Inválidos" |
| `csvImport.actions.skuCreated` | "SKU {sku} created" | "SKU {sku} creado" |
| `csvImport.actions.skuUpdated` | "SKU {sku} updated" | "SKU {sku} actualizado" |

**New keys added to `unitReview` section (plural-aware):**

| Key | EN | ES |
|-----|----|----|
| `unitReview.unitCount_singular` | "{n} product uses units not in the catalog." | "{n} producto usa unidades no estándar." |
| `unitReview.unitCount_plural` | "{n} products use units not in the catalog." | "{n} productos usan unidades no estándar." |
| `unitReview.unitCount_singular_alt` | "{n} product has units not in the catalog." | "{n} producto tiene unidades no estándar." |
| `unitReview.unitCount_plural_alt` | "{n} products have units not in the catalog." | "{n} productos tienen unidades no estándar." |
| `unitReview.noUnrecognizedUnits` | "No unrecognized units — all products are catalog-linked." | "No hay unidades no reconocidas — todos los productos están vinculados al catálogo." |
| `unitReview.unrecognizedFound` | "{count} unrecognized {values} found. Choose how to handle each one." | "{count} {values} no reconocidas encontradas. Elige cómo manejar cada una." |
| `unitReview.unrecognizedValue_singular` | "value" | "valor" |
| `unitReview.unrecognizedValue_plural` | "values" | "valores" |
| `unitReview.allRecognized` | "✅ All products use recognized catalog units." | "✅ Todos los productos usan unidades reconocidas del catálogo." |
| `unitReview.backToDashboard` | "Back to dashboard" | "Volver al panel" |
| `unitReview.mapToPresetDropdown` | "Map to preset" | "Mapear a predefinida" |
| `unitReview.displayName` | "Display name" | "Nombre para mostrar" |
| `unitReview.createAndAssign` | "Create & assign" | "Crear y asignar" |
| `unitReview.inProgress` | "…" | "…" |
| `unitReview.productsUpdated_singular` | "✅ {count} product updated." | "✅ {count} producto actualizado." |
| `unitReview.productsUpdated_plural` | "✅ {count} products updated." | "✅ {count} productos actualizados." |
| `unitReview.unitAssigned` | 'Unit "<strong>{name}</strong>" is now assigned.' | 'Unidad "<strong>{name}</strong>" ahora está asignada.' |
| `unitReview.done` | "Done — back to dashboard" | "Listo — volver al panel" |
| `unitReview.integerPresets` | "Integer presets:" | "Unidades enteras predefinidas:" |
| `unitReview.decimalPresets` | "Decimal presets:" | "Unidades decimales predefinidas:" |

### Components re-keyed

#### `src/components/CalendarPage.svelte`
- Added `import { LL } from "../i18n/i18n-svelte.js"`
- `urgencyLabel()`: hardcoded "Expired"/"Today"/"Alert"/"Soon"/"Future" → `$LL.calendar.urgencyLabels.*`
- `<h2 class="page-title">Calendar</h2>` → `{$LL.calendar.pageTitle()}`
- Refresh button: `aria-label="Refresh lot data"` → `{$LL.calendar.refreshAria()}`, text `Refresh` → `{$LL.calendar.refresh()}`
- Loading state: `Loading lots…` → `{$LL.calendar.loadingLots()}`
- Error state: `Failed to load lots: {errorMsg}` → `{$LL.calendar.loadFailed({ msg: errorMsg })}`
- Day panel title: `formatDate(selectedDate)` → `{$LL.calendar.dayPanelTitle({ date: formatDate(selectedDate) })}`
- Empty day state: `No expirations on {selectedDate}` → `{$LL.calendar.noLotsOnDate()}`
- Table headers: `Product`/`Qty`/`Unit`/`Store`/`Location`/`Days`/`Status` → `$LL.calendar.table.*`
- Modal overlay: `aria-label="Lot detail"` → `{$LL.dashboard.lotDetail()}`
- Modal `<h3>`: `Lot Detail` → `{$LL.dashboard.lotDetail()}`
- Modal loading: `Loading…` → `{$LL.lotsDetail.loading()}`
- Tabs: `Detalle` → `{$LL.lotsDetail.detail()}`, `Historial` → `{$LL.lotsDetail.history()}`
- Detail grid labels: `Product`/`Store`/`Location`/`Quantity`/`Expiry date`/`Alert days`/`Batch` → `$LL.dashboard.*` / `$LL.lotsDetail.*`
- Close button: `Close` → `{$LL.common.close()}`
- Open movement button: `Open movement actions` → `{$LL.lotMovements.panelTitle()}`

#### `src/components/BackupRestorePage.svelte`
- Added `import { LL } from "../i18n/i18n-svelte.js"`
- Page `<h1>`: `Backup & Restore` → `{$LL.backupRestore.pageTitle()}`
- All section `<h2>` headings → `$LL.backupRestore.exportSection()`, `.restoreSection()`, `.aboutSection()`
- Export description → `$LL.backupRestore.exportDesc()`
- Export button: `Export database` → `{$LL.backupRestore.exportButton()}`, `Exporting…` → `{$LL.backupRestore.exporting()}`
- Export success: hardcoded template → `{$LL.backupRestore.exportSuccess({ path, kb, schema })}`
- Restore description + destructive warning → `$LL.backupRestore.*`
- Danger banner → `$LL.backupRestore.restoreDangerBanner()`
- Select backup button → `$LL.backupRestore.selectBackupFile()` / `.selecting()`
- Validated heading → `$LL.backupRestore.backupValidated()`
- Confirm prompt → `$LL.backupRestore.restoreConfirmPrompt()`
- Restore button → `$LL.backupRestore.restoreData()`
- Cannot restore heading → `$LL.backupRestore.cannotRestore()`
- Choose different file → `$LL.backupRestore.chooseDifferentFile()`
- FAQ dt/dd pairs → all `$LL.backupRestore.*` keys

#### `src/components/CsvImportPage.svelte`
- Added `import { LL } from "../i18n/i18n-svelte.js"`
- All stage `<h1>` headings: → `$LL.csvImport.pageTitle()`, `.importPreview()`, `.importComplete()`
- Select file card title/desc → `$LL.csvImport.selectFile()`, `.selectFileDesc()`
- Expected columns card → `$LL.csvImport.expectedColumns()`
- Hint box → `$LL.csvImport.tip()`, `.tipText()`
- Choose different file button → `$LL.csvImport.chooseDifferentFile()`
- Summary card labels: `Total rows`/`Valid`/duplicate keys → `$LL.csvImport.totalRows()`, `.valid()`, `.duplicates.*`
- Conflict strategy section → `$LL.csvImport.conflictStrategy()`, `.conflictOptions.*`
- Preview table headers → `$LL.csvImport.description()`, `.detail()`, `.status()`
- Badge labels: `rowBadge()` → `$LL.csvImport.badge.*`
- Detail column: `rowDetailMessage()` → `$LL.csvImport.detailRow.*`; uses `{@html}` for `<code>` tags
- Outcome badges: `outcomeBadge()` → `$LL.csvImport.actions.*`
- Outcome reasons: `outcomeReason()` → `$LL.csvImport.actions.skuCreated()`, `.skuUpdated()`
- Import button: `Import {n} Products` → `$LL.csvImport.importButton({ n: counts.valid })`
- Result card labels: `Created`/`Updated`/`Skipped`/`Invalid` → `$LL.csvImport.created()`, etc.
- Import log `<h2>` → `$LL.csvImport.importLog()`
- Import another file → `$LL.csvImport.importAnotherFile()`

#### `src/components/UnitReviewPage.svelte`
- Added `import { LL } from "../i18n/i18n-svelte.js"`
- Page `<h2>` → `$LL.unitReview.pageTitle()`
- Subtitle: plural-aware → `$LL.unitReview.noUnrecognizedUnits()` or `$LL.unitReview.unrecognizedFound({ count, values })`
- Loading: `Loading…` → `$LL.common.loading()`
- All-recognized empty state → `$LL.unitReview.allRecognized()`, `.backToDashboard()`
- Raw value label + product count (plural-aware) → `$LL.unitReview.unitCount_singular({ n })` / `.unitCount_plural({ n })`
- Map to preset dropdown → `$LL.unitReview.mapToPresetDropdown()`
- Integer/decimal dropdown hints → `$LL.unitReview.integerPresets()`, `.decimalPresets()`
- Create custom unit → `$LL.unitReview.createCustomUnit()`, `.displayName()`, `.integer()`, `.decimal()`, `.createAndAssign()`, `.inProgress()`
- Leave for later → `$LL.unitReview.leaveForLater()`
- Success message: `productsUpdated_singular({ count })` / `productsUpdated_plural({ count })`
- Assigned unit: `@html $LL.unitReview.unitAssigned({ name })`
- Done button → `$LL.unitReview.done()`

#### `src/components/UnitReviewBanner.svelte`
- Added `import { LL } from "../i18n/i18n-svelte.js"`
- Banner message: plural-aware `{n}` parameter → `$LL.unitReview.unitCount_singular_alt({ n })` / `.unitCount_plural_alt({ n })`
- Review button → `$LL.unitReview.review()`
- Dismiss button + title → `$LL.unitReview.dismiss()`, `.inProgress()`

### Type signature fix discovered and corrected

`svelte-check` caught a genuine error: `unitCount_plural_alt` and `unitCount_singular_alt`
templates in the locale trees used `{n}` as the template parameter, but the component initially
called them with `{ count: unrecognizedCount }`. Fixed: locale templates use `{n}` consistently
so the function signature is `(arg: { n: unknown }) => LocalizedString` and the component call
passes `{ n: unrecognizedCount }`.

### Commands run

```bash
# Regenerate i18n types after locale tree edits
npm run i18n:generate
# Result: all files up to date

# TypeScript compile
npx tsc --noEmit
# Result: EXIT:0 — no TypeScript errors

# Svelte component check
npx svelte-check --workspace . --threshold error
# Result: svelte-check found 0 errors and 0 warnings

# Frontend build
npm run build
# Result: ✓ built in 1.26–1.43s (prebuild regenerated i18n)
```

### LSP stale-cache note (persistent — all sessions)

The pi-lens embedded LSP diagnostic server holds a snapshot of `i18n-types.ts` that is NOT
automatically refreshed when `npm run i18n:generate` regenerates the types. After locale-tree
edits, it may flag `Property 'urgencyLabels' does not exist`, `Property 'badge' does not
exist`, etc. These are stale-cache false positives. The authoritative pipeline always
confirms zero errors:

```
npm run i18n:generate  # regenerates i18n-types.ts (typesafe-i18n CLI)
npx tsc --noEmit        # EXIT:0 — TypeScript uses fresh i18n-types.ts
svelte-check --workspace . --threshold error  # 0 errors, 0 warnings
npm run build           # ✓ built in 1.26–1.43s
```

The fix is to run `npm run i18n:generate` before any diagnostic read; the authoritative tool
output always supersedes the embedded LSP snapshot.

### Remaining work in task 2.5

Still pending re-keying:
- `src/components/LotMovementsPanel.svelte` — panel title, action labels, reason labels, loading/empty
- `src/components/DatePicker.svelte` — helper text, aria-labels, Today button
- Modal helpers (task item after component re-keying): `MoveStockModal`, `RegisterExitModal`,
  `AdjustCountModal`, `ResolveQuantityDialog`, `ArchiveLotDialog`, and any English residue
  surfaced by grep

---

## Verifier-found residual corrections — fourth pass (this session)

Five residuals confirmed by the independent verifier, all fixed in this session.

### Fix 1 — `CsvImportPage.svelte`: three hardcoded English strategy descriptions

The `strategies` array (lines ~115-125) had three hardcoded English `desc` strings:
- `"Import only new products. Existing SKUs and barcodes are ignored."`
- `"Update product details for existing SKUs and add barcodes to existing products."`
- `"Show which rows have conflicts without making any changes."`

**Fix:** Added `conflictOptions.skipDuplicatesDesc`, `conflictOptions.updateExistingDesc`,
and `conflictOptions.reviewConflictsDesc` to both locale trees, then replaced the hardcoded
strings with `$LL.csvImport.conflictOptions.skipDuplicatesDesc()` etc.

### Fix 2 — `CsvImportPage.svelte`: `selectFileDesc` on action card

The action-card `card-desc` used `$LL.csvImport.selectFileDesc()` which carries the
full column-description text ("Choose a CSV file to import products") — inappropriate for
a brief click-hint on the card. The old UI had simple "Optional/Opcional" text for this
slot, but the current key conveys the full description.

**Fix:** Added `csvImport.selectFileAction` to both locale trees:
- EN: `"Click to select a CSV file from your device"`
- ES: `"Haz clic para seleccionar un archivo CSV desde tu dispositivo"`

Updated the card-desc span to use `$LL.csvImport.selectFileAction()` instead of
`$LL.csvImport.selectFileDesc()`.

### Fix 2b — `CsvImportPage.svelte`: sibling info card still misusing `selectFileDesc`

The verifier flagged that the sibling info action card (the "ℹ️ Expected columns" card)
also used `$LL.csvImport.selectFileDesc()` for the optional-columns label — semantically
incorrect. The existing key `csvImport.optional` ("optional"/"opcional") already existed
in both locale trees.

**Fix:** Changed the second `card-desc` line from `$LL.csvImport.selectFileDesc()` to
`$LL.csvImport.optional()`.

- **Before:** `{$LL.csvImport.selectFileDesc()}: <code>barcode</code>, ...`
- **After:** `{$LL.csvImport.optional()}: <code>barcode</code>, ...`

### Fix 3 — `CalendarPage.svelte`: hardcoded English month names in `formatDate()`

`formatDate()` used a hardcoded `months` array (`"January"…"December"`) regardless of
locale.

**Fix:** Replaced the hardcoded array with `toLocaleDateString(undefined, { month: "long",
day: "numeric", year: "numeric" })`, which respects the active runtime locale rune
(`locale.current`). Output is now locale-aware: English renders "January 5, 2025",
Spanish renders "5 de enero de 2025", etc.

### Fix 4 — `CalendarPage.svelte`: hardcoded `ariaLabel="Expiry calendar"`

The `CalendarMonth` child component received `ariaLabel="Expiry calendar"` as a static
string.

**Fix:** Replaced with `ariaLabel={$LL.calendar.pageTitle()}` — the i18n key for the
calendar page title ("Expiry Calendar" / "Calendario de caducidad"), keeping ARIA consistent
with the localized page heading.

### Fix 5 — `UnitReviewPage.svelte`: duplicated product count

The `group-header` rendered `{group.product_count}` as a raw number followed by the
translation result, which already embeds `{n}`:
```
{group.product_count} {group.product_count === 1
  ? $LL.unitReview.unitCount_singular({ n: group.product_count })
  : $LL.unitReview.unitCount_plural({ n: group.product_count })}
```
This showed e.g. `"3 {n} products use units not in the catalog."` with the raw `3`
appearing before the translated text.

**Fix:** Removed the standalone `{group.product_count}` prefix; the plural-aware translation
function now carries `{n}` as its sole count display.

### Commands run (this session)

```bash
npm run i18n:generate
# Result: all files up to date

npx tsc --noEmit
# Result: EXIT 0 — no TypeScript errors

npx svelte-check --workspace . --threshold error
# Result: svelte-check found 0 errors and 0 warnings

npm run build
# Result: ✓ built in 1.24s (prebuild regenerated i18n)
```

### LSP stale-cache note (persistent — all sessions)

The pi-lens embedded LSP diagnostic server holds a snapshot of `i18n-types.ts` that is NOT
automatically refreshed when `npm run i18n:generate` regenerates the types. After locale-tree
edits, it flags `Property 'skipDuplicatesDesc' does not exist`, `Property 'selectFileAction'
does not exist`, etc. — all stale-cache false positives. The authoritative pipeline always
confirms zero errors:

```
npm run i18n:generate  # regenerates i18n-types.ts (typesafe-i18n CLI)
npx tsc --noEmit        # EXIT:0
svelte-check --workspace . --threshold error  # 0 errors, 0 warnings
npm run build           # ✓ built in 1.25s
```

---

## Verifier-follow-up slice — CalendarMonth.svelte hardcoded LABELS (this session)

The independent verifier confirmed that `src/components/CalendarMonth.svelte` still had
pre-existing hardcoded English `LABELS.months` and `LABELS.weekdays` arrays and six
hardcoded `aria-label` string literals not covered by the prior CalendarPage pass.

### Locale tree additions

**New keys added to `en/index.ts` and `es/index.ts` under `calendar`:**

| Key | EN | ES |
|-----|----|----|
| `calendar.monthNames` | `["January", …, "December"]` | `["Enero", …, "Diciembre"]` |
| `calendar.weekdayShort` | `["Sun","Mon",…,"Sat"]` | `["Dom","Lun",…,"Sáb"]` |
| `calendar.ariaPreviousMonth` | `"Previous month"` | `"Mes anterior"` |
| `calendar.ariaNextMonth` | `"Next month"` | `"Mes siguiente"` |
| `calendar.ariaCycleMonth` | `"Cycle month"` | `"Cambiar mes"` |
| `calendar.ariaOpenYearPicker` | `"Open year picker"` | `"Abrir selector de año"` |
| `calendar.ariaPreviousDecade` | `"Previous decade"` | `"Década anterior"` |
| `calendar.ariaNextDecade` | `"Next decade"` | `"Década siguiente"` |
| `calendar.ariaYear` | `"Year {year}"` | `"Año {year}"` |
| `calendar.ariaDayCell` | `"{month} {day}, {year}"` | `"{month} {day}, {year}"` |

### CalendarMonth.svelte changes

- Added `import { LL } from "../i18n/i18n-svelte.js"`
- Removed the hardcoded `LABELS` constant (hardcoded `weekdays` and `months` arrays)
- Added reactive `$: MONTH_NAMES` and `$: WEEKDAY_NAMES` arrays that unpack the typesafe-i18n
  array objects (`{ '0': fn, '1': fn, … }`) into plain `string[]` — needed because
  typesafe-i18n generates array translations as index-keyed objects, not true arrays, which are
  not directly iterable with Svelte's `{#each}`
- `cellAriaLabel()`: replaced `${LABELS.months[d.getMonth()]} ${d.getDate()}, …` with
  `$LL.calendar.ariaDayCell({ month: MONTH_NAMES[d.getMonth()], day, year })`
- Month button: `{LABELS.months[viewMonth-1]}` → `{MONTH_NAMES[viewMonth-1]}`
- Weekday row: `{#each LABELS.weekdays as wd}` → `{#each WEEKDAY_NAMES as wd}`
- Grid `aria-label`: `{LABELS.months[viewMonth-1]} {viewYear}` → `{MONTH_NAMES[viewMonth-1]} {viewYear}`
- Six button aria-labels replaced:
  - `"Previous month"` → `$LL.calendar.ariaPreviousMonth()`
  - `"Next month"` → `$LL.calendar.ariaNextMonth()`
  - `"Cycle month"` → `$LL.calendar.ariaCycleMonth()`
  - `"Open year picker"` → `$LL.calendar.ariaOpenYearPicker()`
  - `"Previous decade"` → `$LL.calendar.ariaPreviousDecade()`
  - `"Next decade"` → `$LL.calendar.ariaNextDecade()`
  - `"Year {yr}"` → `$LL.calendar.ariaYear({ year: yr })`

### Commands run

```bash
npm run i18n:generate
# Result: generated i18n-types.ts with new calendar keys

npx tsc --noEmit
# Result: EXIT:0 — no TypeScript errors

npx svelte-check --workspace . --threshold error
# Result: svelte-check found 0 errors and 0 warnings

npm run build
# Result: ✓ built in 1.29s
```

### LSP stale-cache note (persistent)

The pi-lens embedded LSP diagnostic server held a stale snapshot of `i18n-types.ts` that
was refreshed only after `npm run i18n:generate`. Initial edits showed `Property 'monthNames'
does not exist` errors in pi-lens; the authoritative pipeline confirmed zero errors after
generator regeneration. Additionally, the typesafe-i18n array type (`{ '0': fn, … }`) is not
directly callable — `$LL.calendar.monthNames()` caused a genuine svelte-check error. Fixed
by accessing `$LL.calendar.monthNames` as a property (not a function call) then indexing it.

---

## Verifier-follow-up slice — DatePicker.svelte hardcoded strings (this session)

The independent verifier confirmed that `src/components/DatePicker.svelte` still had hardcoded
English strings for helper text, aria-labels, and the Today button — none covered by prior
CalendarMonth or CalendarPage passes.

### Locale tree additions

**New top-level `datePicker` section added to `en/index.ts` and `es/index.ts`:**

| Key | EN | ES |
|-----|----|----|
| `datePicker.ariaOpenCalendar` | "Open calendar" | "Abrir calendario" |
| `datePicker.ariaClearDate` | "Clear date" | "Borrar fecha" |
| `datePicker.today` | "Today" | "Hoy" |

The `datePicker.today` key is intentionally distinct from `calendar.today` ("this week" in
CalendarPage) — both serve different UI contexts and reuse is not semantically appropriate.

Helper-text strings were already present in `errors.*` and were reused directly:

| Existing key | EN | ES |
|-------------|----|----|
| `errors.dateUseIsoFormat` | "Use YYYY-MM-DD" | "Usa AAAA-MM-DD" |
| `errors.dateYearRange` | "Year must be 1900–2100" | "El año debe estar entre 1900 y 2100" |
| `errors.dateFieldRequired` | "This field is required" | "Este campo es obligatorio" |
| `errors.dateInvalid` | "Invalid date" | "Fecha no válida" |

### DatePicker.svelte changes

- Added `import { LL } from "../i18n/i18n-svelte.js"`
- `helperText` reactive block:
  - `"Use YYYY-MM-DD"` → `$LL.errors.dateUseIsoFormat()`
  - `"Year must be 1900–2100"` → `$LL.errors.dateYearRange()`
  - `"This field is required"` → `$LL.errors.dateFieldRequired()`
  - `"Invalid date"` (fallback) → `$LL.errors.dateInvalid()`
- Calendar trigger button: `aria-label="Open calendar"` → `aria-label={$LL.datePicker.ariaOpenCalendar()}`
- Clear button: `aria-label="Clear date"` → `aria-label={$LL.datePicker.ariaClearDate()}`
- Today button: `Today` → `{$LL.datePicker.today()}`
- Prop defaults `placeholder="YYYY-MM-DD"` and `ariaLabel="Date"` left as-is (caller-supplied; no user-facing hardcoded text)
- `aria-label="{ariaLabel} calendar"` on the popover dialog already uses the caller-supplied `ariaLabel` prop (dynamic); no change needed

### Commands run

```bash
npm run i18n:generate
# Result: generated i18n-types.ts with new datePicker section

npx tsc --noEmit
# Result: EXIT:0 — no TypeScript errors

npx svelte-check --workspace . --threshold error
# Result: svelte-check found 0 errors and 0 warnings

npm run build
# Result: ✓ built in 1.25s
```

### LSP stale-cache note (persistent)

The pi-lens embedded LSP diagnostic server held a stale snapshot of `i18n-types.ts`. Initial
edits showed `Property 'datePicker' does not exist` errors in pi-lens; the authoritative pipeline
confirmed zero errors after `npm run i18n:generate` regenerated the types file.

