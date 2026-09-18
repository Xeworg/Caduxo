# Design — caduxo-i18n-support

This is the technical design for the i18n change. It settles the binding
shape, generator pipeline, app bootstrap sequence, settings backend changes,
PDF / notification / error-message translation strategy, the `printpdf`
accent-coverage risk, and the review workload forecast against the 3000-line
session budget.

**Decision in one paragraph.** Add `typesafe-i18n` to the Svelte 5 frontend
with a single reactive `locale` rune that calls `setLocale()` and persists
through the existing `app_settings.language` setting. The Rust backend
gains a small `pdf/locale.rs` module that owns every PDF string and a tiny
`user_message(kind, locale)` lookup for the handful of user-visible
`DomainError::Validation` strings. The Tauri IPC commands `preview_report`
and `export_report_pdf` accept a new `locale: &str` parameter. The frontend
passes the active locale on every call. The locale is detected on every
app start (`@tauri-apps/api/os::locale()` in production,
`navigator.language` during `vite dev`) and the persisted row wins after
the first run.

**Out of scope (reaffirmed).** RTL; a third locale; locale-aware number /
currency formatting; locale-aware date format in the PDF; translating
user-entered data; translating PDF document properties beyond the visible
title; first-run locale dialog; per-store / per-user overrides; a new
plugin-store for the preference; localising developer-only
`DomainError::Internal` strings; a custom TTF font for the PDF.

## Quick path

| # | Decision | Where it lives |
|---|----------|----------------|
| 1 | `typesafe-i18n` Svelte 5 binding shape | `src/i18n/locale.ts` + `src/i18n/i18n.ts` (generated) + `$LL` helper in templates |
| 2 | Generator pipeline | `package.json` scripts (`i18n:generate`, `i18n:typesafe`); prebuild hook |
| 3 | App bootstrap order | `src/main.ts` → `initLocale()` → `App.svelte` first render |
| 4 | Persisted vs detected — no extra DB flag | `SettingsResponse.language: String` is the single source; `detection_source` is computed in-memory from the absence of the row, not stored |
| 5 | Settings backend changes | `dto/stores.rs` + `db/repositories/settings.rs` + `services/settings.rs` + `commands/stores.rs` |
| 6 | PDF translation shape | new `pdf/locale.rs` + `pdf_messages(Locale)` + Rust parity test |
| 7 | IPC delta for reports | `commands/reports.rs` accepts `locale: &str`; `services/reports.rs` threads it through |
| 8 | Notification composition | `lib/notifications.ts` reads the active locale rune before composing title/body |
| 9 | `printpdf` accent coverage | spot-check in the integration test; accept Helvetica's WinAnsiEncoding coverage of `á é í ó ú ñ ü` |
| 10 | Forecast and slicing | ~1900 net changed lines; split into two chained PRs (PR 1 backend, PR 2 frontend) — stays under 3000 |

## 1. `typesafe-i18n` Svelte 5 binding shape

The binding has three pieces:

1. **Generated translation tree** — `src/i18n/i18n.ts` and
   `src/i18n/i18n-util.ts` are produced by the `typesafe-i18n` CLI from
   `src/i18n/en/index.ts` and `src/i18n/es/index.ts`. The generated
   `LL` helper is fully typed; referencing a missing key fails
   `svelte-check`.
2. **Reactive locale rune** — `src/i18n/locale.ts` exposes:
   - `initLocale()` — one-shot boot that reads OS / WebView locale,
     consults the persisted `app_settings.language` row, calls
     `setLocale(locale)`, and updates a Svelte 5 `$state` rune.
   - `setLocale(locale: SupportedLocale)` — calls the runtime's
     `setLocale(locale)`, updates the rune, and (when called from the
     Configuration page) persists via `updateSettings({ language })`.
   - `locale` — the rune itself; components that need to branch on the
     active locale (rare) bind to it.
   - `translationSource` — a derived rune that is `detected` when the
     persisted row was absent at boot and `manual` after the first
     successful `updateSettings({ language })`. **It is not persisted**;
     it is reconstructed from the boot path every launch. See §4 for the
     persisted-vs-detected distinction.
3. **Template binding** — components reference `$LL.nav.dashboard`,
   `$LL.configuration.section.lots`, `$LL.notification.title`, etc.
   `typesafe-i18n` provides a Svelte-compatible helper so `$LL.foo.bar()`
   re-evaluates when `setLocale()` runs.

### 1.1 Module shape

```text
src/i18n/
├── i18n.ts          # generated, do not edit
├── i18n-util.ts     # generated, do not edit
├── detect.ts        # pure helper: locale tag → SupportedLocale
├── locale.ts        # initLocale, setLocale, locale rune, translationSource
├── en/
│   └── index.ts     # base English tree (source of truth)
└── es/
    └── index.ts     # mirror tree translated to Spanish
```

### 1.2 Why this shape

- The Svelte 5 runes API integrates cleanly with `typesafe-i18n`'s
  `setLocale`; the helper triggers a re-render of every template that
  reads from `$LL`. The team's `startPeriodicNotificationCheck` and the
  Configuration page's optimistic-update pattern are the only two
  reactive surfaces that pre-date this change, and both already work
  with the rune pattern.
- Putting `locale.ts` next to the generated `i18n.ts` keeps the
  import path stable: `import { initLocale, setLocale, locale,
  translationSource } from "./i18n/locale.js"`.
- The single rune (`locale`) is read-only from templates; mutating
  it goes through `setLocale()`, which guarantees that
  `typesafe-i18n` and the rune stay in sync.

### 1.3 What the rune surface looks like

```ts
// src/i18n/locale.ts
import { setLocale as i18nSetLocale } from "./i18n.js"; // generated
import { detectSupportedLocale } from "./detect.js";
import { getSettings, updateSettings } from "../lib/stores.js";

export type SupportedLocale = "en" | "es";

const DEFAULT: SupportedLocale = "en";

/** Active locale; templates branch on this only when necessary. */
export const locale = $state<{ current: SupportedLocale }>({ current: DEFAULT });

/** How the active locale was resolved. Not persisted; see §4. */
export const translationSource = $state<{ current: "detected" | "manual" }>({
  current: "detected",
});

export async function initLocale(): Promise<SupportedLocale> {
  // 1. Try persisted preference.
  try {
    const s = await getSettings();
    if (s.language === "en" || s.language === "es") {
      i18nSetLocale(s.language);
      locale.current = s.language;
      translationSource.current = "manual";
      return s.language;
    }
  } catch {
    // Fall through to detection.
  }

  // 2. Detect from OS / WebView.
  const detected = await detectSupportedLocale();
  i18nSetLocale(detected);
  locale.current = detected;
  translationSource.current = "detected";
  return detected;
}

export async function setLocale(next: SupportedLocale): Promise<void> {
  i18nSetLocale(next);
  locale.current = next;
  translationSource.current = "manual"; // user chose it
  // Persist; on failure, roll back.
  const prev = locale.current;
  try {
    await updateSettings({ language: next });
  } catch (e) {
    i18nSetLocale(prev);
    locale.current = prev;
    translationSource.current = "manual";
    throw e;
  }
}
```

**Rollback note.** The rollback writes back the previous locale, but
`translationSource.current` stays `"manual"` because the user did try
to change it. The persistence error does not flip the source back to
"detected"; the user already knows they tried.

## 2. Generator pipeline

The `typesafe-i18n` CLI lives in the `typesafe-i18n` npm package and is
invoked from `package.json` scripts:

```jsonc
// package.json (additions)
{
  "scripts": {
    "i18n:generate": "typesafe-i18n",
    "i18n:typesafe": "typesafe-i18n --no-watch",
    "prebuild": "npm run i18n:generate",
    "predev": "npm run i18n:generate"
  },
  "devDependencies": {
    "typesafe-i18n": "^5"
  }
}
```

- `i18n:generate` runs the CLI in watch mode for local development.
- `i18n:typesafe` runs a one-shot generation for CI / pre-commit.
- `prebuild` and `predev` guarantee that `src/i18n/i18n.ts` is up to
  date before `vite build` or `vite dev`.
- `typesafe-i18n` reads `src/i18n/en/index.ts` as the base; the CLI
  type-checks every other locale (`es/index.ts`) against the base and
  fails if a key is missing or empty.
- `svelte-check` (run in verify) confirms the generated `LL` helper is
  used consistently and that no template references a key absent from
  the active locale tree.

### 2.1 Where the generator reads from

`typesafe-i18n` expects `src/i18n/<locale>/index.ts` to export a typed
object. The base locale (`en`) is the source of truth. The CLI also
emits `src/i18n/i18n.ts` and `src/i18n/i18n-util.ts`.

```ts
// src/i18n/en/index.ts
import type { BaseTranslation } from "../i18n-util.js";
const en: BaseTranslation = {
  nav: {
    dashboard: "Dashboard",
    stores: "Stores",
    products: "Products",
    calendar: "Calendar",
    reports: "Reports",
    import: "Import",
    backup: "Backup",
    settings: "Configuration",
  },
  configuration: {
    pageTitle: "Configuration",
    section: { lots: "Lots" },
    locationRequired: {
      label: "Require initial location when creating a lot",
      description:
        "When enabled, the lot-creation form requires a location to be selected. " +
        "When disabled, lots may be created without a location (a default location is assigned).",
    },
    language: {
      sectionTitle: "Language",
      label: "Interface language",
      detectedHintEn: "Detected: English",
      detectedHintEs: "Detected: Spanish",
      saving: "Saving…",
      loadErrorPrefix: "Could not load settings: ",
      saveErrorPrefix: "Could not save setting: ",
    },
  },
  notification: {
    titlePrefix: "⚠️ Expiry alert: ",
    bodyTemplate: "{qty} expires on {expiry_date} · {location}",
  },
  common: {
    loading: "Loading…",
    error: "Error",
    cancel: "Cancel",
    save: "Save",
  },
  // …
};
export default en;
```

The Spanish mirror (`src/i18n/es/index.ts`) is structurally identical;
the CLI enforces the mirror.

## 3. Files created and modified

### 3.1 New files (frontend)

| Path | Purpose | Approx. lines |
|------|---------|---------------|
| `src/i18n/en/index.ts` | English base tree | 200 |
| `src/i18n/es/index.ts` | Spanish mirror tree | 200 |
| `src/i18n/i18n.ts` | generated | 100 |
| `src/i18n/i18n-util.ts` | generated | 60 |
| `src/i18n/locale.ts` | initLocale, setLocale, runes | 90 |
| `src/i18n/detect.ts` | tag → SupportedLocale | 40 |

### 3.2 New files (backend)

| Path | Purpose | Approx. lines |
|------|---------|---------------|
| `src-tauri/src/pdf/locale.rs` | `Locale` enum, `PdfMessages`, parity test | 180 |
| `src-tauri/src/services/user_messages.rs` | small `user_message(kind, locale)` lookup | 80 |

### 3.3 Modified files (frontend)

| Path | Change |
|------|--------|
| `package.json` | add `typesafe-i18n` dep + scripts |
| `src/main.ts` | call `initLocale()` before mounting `App` |
| `src/App.svelte` | nav buttons → `$LL.nav.*` |
| `src/components/ConfigurationPage.svelte` | new locale selector section, "Detected" hint, existing toggle copy → `$LL.configuration.*` |
| `src/components/DashboardPage.svelte` | presets + banner copy → `$LL.dashboard.*` |
| `src/components/ReportsPage.svelte` | report-type labels, urgency options, success / error messages → `$LL.reports.*` |
| `src/components/ScanSearchBox.svelte` | placeholder + error → `$LL.scan.*` |
| `src/components/StoresPage.svelte`, `ProductCatalogPage.svelte`, `ProductForm.svelte`, `CalendarPage.svelte`, `BackupRestorePage.svelte`, `CsvImportPage.svelte`, `UnitReviewPage.svelte`, `LotMovementsPanel.svelte` | titles, buttons, placeholders, modals → `$LL.*` |
| `src/components/UnitReviewBanner.svelte` | plural-aware copy → `$LL.unitReview.*` |
| `src/components/inputs/CategoryPicker.svelte` | placeholder → `$LL.categoryPicker.placeholder` |
| `src/lib/notifications.ts` | `formatNotificationBody`, `sendNotification` title → `$LL.notification.*` |
| `src/lib/reports.ts` | pass `locale` to `previewReport` and `exportReportPdf` |
| `src/lib/stores.ts` | `SettingsResponse.language`, `SettingsUpdate.language` |

Approximate total modified frontend: ~700 lines (including the trees).

### 3.4 Modified files (backend)

| Path | Change |
|------|--------|
| `src-tauri/Cargo.toml` | (no new crate — `typesafe-i18n` does not have a Rust counterpart; `printpdf` already present) |
| `src-tauri/src/lib.rs` | register `tauri-plugin-os` for `locale()` access |
| `src-tauri/src/pdf/report_pdf.rs` | thread `Locale` through `render_report`, route all strings through `pdf_messages(locale)` |
| `src-tauri/src/dto/reports.rs` | `ReportType::description(locale: Locale) -> Cow<'static, str>` |
| `src-tauri/src/dto/stores.rs` | add `language: String` to `SettingsResponse`, `Option<String>` to `SettingsUpdate` |
| `src-tauri/src/db/repositories/settings.rs` | `get_language_setting`, `set_language_setting`, default `"en"` |
| `src-tauri/src/services/settings.rs` | branch on `input.language` |
| `src-tauri/src/services/reports.rs` | thread `Locale` through preview / export |
| `src-tauri/src/commands/stores.rs` | reject `language` outside `{en, es}` at the IPC boundary |
| `src-tauri/src/commands/reports.rs` | accept `locale: &str` parameter |
| `src-tauri/src/services/notifications.rs` | route `parse_strict_date` error through `user_message` |
| `src-tauri/src/services/expiry_lots.rs` | route a small set of user-visible `Validation` messages through `user_message` |
| `src-tauri/src/services/reports.rs::optional_date` | thread locale through date-format error |
| `src-tauri/capabilities/default.json` | add `os:default` permission for `os.locale()` |

Approximate total modified backend: ~350 lines (including tests).

## 4. Persisted vs detected — no extra DB flag

The Configuration page renders a "Detected: English / Detected: Spanish"
hint when the active value came from auto-detection. **The hint state is
not stored.** It is reconstructed on every app start:

1. The bootstrap reads `app_settings.language`. If present and in
   `{en, es}`, the value came from a manual override → hint hidden.
2. If the row is absent, empty, or unrecognised, the bootstrap falls
   back to detection → hint visible.

The `translationSource` rune in `src/i18n/locale.ts` is computed from
this branch and is **not persisted**. There is no second DB flag, no
`detected_at` timestamp, no `was_auto` boolean. The hint is purely a
function of "is the persisted row present?".

**Why no extra DB flag.** A second flag would have to be written on
every manual selection and never written on detection, which is the same
information as "row present or not". The single `language` row already
encodes the user's intent; deriving the hint from its presence keeps
the migration story trivial (existing rows are treated as "manual",
because the row is present).

**Edge case.** A user who has never picked a locale manually has no
row → hint shows. They pick a locale → row written → hint hides.
They restart → row is read → hint stays hidden. They delete the row
manually (or the row is removed in a future migration) → next boot
falls back to detection → hint shows again. This matches the spec
scenario `hint reappears if persisted value is cleared`.

## 5. Settings backend changes

### 5.1 DTO (`src-tauri/src/dto/stores.rs`)

```rust
pub struct SettingsResponse {
    pub last_selected_store_id: Option<String>,
    pub require_initial_location_on_lot_create: bool,
    /// Active locale; one of {"en", "es"}. Defaults to "en" when missing.
    pub language: String,
}

pub struct SettingsUpdate {
    pub last_selected_store_id: Option<String>,
    pub require_initial_location_on_lot_create: Option<bool>,
    /// Optional: when present, sets the language preference.
    pub language: Option<String>,
}
```

### 5.2 Repository (`src-tauri/src/db/repositories/settings.rs`)

```rust
pub async fn get_language_setting(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    get_setting(pool, "language").await
}

pub async fn set_language_setting(
    pool: &SqlitePool,
    value: &str,
) -> Result<(), sqlx::Error> {
    upsert_setting(pool, "language", value).await
}
```

`get_settings()` is updated to include `language`. The default is `"en"`
when the row is absent, empty, or unrecognised:

```rust
pub async fn get_settings(pool: &SqlitePool) -> Result<SettingsResponse, sqlx::Error> {
    let last_selected_store_id = get_last_selected_store_id(pool).await?;
    let require_initial_location_on_lot_create =
        get_require_initial_location_on_lot_create(pool).await?;
    let language = match get_language_setting(pool).await? {
        Some(v) if v == "en" || v == "es" => v,
        _ => "en".to_string(),
    };
    Ok(SettingsResponse {
        last_selected_store_id,
        require_initial_location_on_lot_create,
        language,
    })
}
```

### 5.3 Service (`src-tauri/src/services/settings.rs`)

```rust
pub async fn update_settings(
    pool: &DbPool,
    input: SettingsUpdate,
) -> Result<SettingsResponse, AppError> {
    // Existing branches unchanged.
    repo::set_last_selected_store_id(pool, input.last_selected_store_id.as_deref())
        .await
        .map_err(AppError::from)?;
    if let Some(value) = input.require_initial_location_on_lot_create {
        repo::set_require_initial_location_on_lot_create(pool, value)
            .await
            .map_err(AppError::from)?;
    }
    if let Some(value) = input.language {
        repo::set_language_setting(pool, &value)
            .await
            .map_err(AppError::from)?;
    }
    repo::get_settings(pool).await.map_err(AppError::from)
}
```

The `if let Some(value)` keeps the partial-update semantics already in
place — `updateSettings({ language: "es" })` does not touch the other
two keys.

### 5.4 Command (`src-tauri/src/commands/stores.rs`)

```rust
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    input: SettingsUpdate,
) -> Result<SettingsResponse, CommandError> {
    if let Some(value) = input.language.as_deref() {
        if value != "en" && value != "es" {
            return Err(CommandError::Validation {
                message: format!(
                    "language must be one of {{en, es}}, got `{value}`"
                ),
            });
        }
    }
    let pool = state.pool().await;
    settings_service::update_settings(&pool, input)
        .await
        .map_err(AppError::into)
}
```

The IPC handler rejects unsupported values before they reach the
repository. The persisted row is not overwritten on rejection.

### 5.5 Frontend DTO (`src/lib/stores.ts`)

```ts
export interface SettingsResponse {
  last_selected_store_id: string | null;
  require_initial_location_on_lot_create: boolean;
  language: SupportedLocale;
}

export interface SettingsUpdate {
  last_selected_store_id?: string | null;
  require_initial_location_on_lot_create?: boolean;
  language?: SupportedLocale;
}
```

`SupportedLocale` is re-exported from `src/i18n/locale.ts`.

## 6. Backend user-visible errors in scope and translation strategy

### 6.1 Scope

The following backend strings are surfaced verbatim by the UI today and
are in scope for translation. All other `DomainError::Validation` and
`DomainError::Internal` strings stay English.

| Source | English | Spanish |
|--------|---------|---------|
| `parse_strict_date` | `Invalid {label} format: \`{value}\` (expected YYYY-MM-DD)` | `Formato de {label} no válido: \`{value}\` (se esperaba YYYY-MM-DD)` |
| `validate_quantity` (expiry_lots) | `Quantity must be positive, got {qty}` | `La cantidad debe ser positiva, se recibió {qty}` |
| `validate_quantity` (lot_movements) | `Quantity must be non-negative, got {n}` / `Quantity must be positive, got {n}` | `La cantidad debe ser no negativa, se recibió {n}` / `La cantidad debe ser positiva, se recibió {n}` |
| `expiry_lots::create_lot` location pick | `Selecciona una ubicación` (already Spanish) | (unchanged) |
| `validate_filters::optional_date` | `Invalid {label} \`{value}\` (expected YYYY-MM-DD)` | `Formato de {label} no válido: \`{value}\` (se esperaba YYYY-MM-DD)` |
| `validate_filters::inverted_range` | `date_from \`{from}\` must be on or before date_to \`{to}\`` | `date_from \`{from}\` debe ser igual o anterior a date_to \`{to}\`` |
| `validate_alert_days` | `Default alert days before cannot be negative` / `Default alert days exceeds maximum of {n}` | `Los días de alerta predeterminados no pueden ser negativos` / `Los días de alerta predeterminados exceden el máximo de {n}` |
| `validate_sku` / `validate_description` / `validate_name` | (short English strings surfaced by UI) | localized variants |
| `csv_io` header detection | `SKU column not detected. …` / `Description column not detected. …` | `No se detectó la columna SKU. …` / `No se detectó la columna Descripción. …` |

### 6.2 Strategy

A small `services/user_messages.rs` module exposes:

```rust
pub enum UserMessage {
    InvalidDateFormat { label: String, value: String },
    InvertedDateRange { from: String, to: String },
    QuantityNonNegative { value: f64 },
    QuantityPositive { value: f64 },
    AlertDaysNegative,
    AlertDaysExceedsMax { max: i32 },
    SkuColumnNotDetected,
    DescriptionColumnNotDetected,
    // …
}

pub fn user_message(kind: UserMessage, locale: Locale) -> String {
    use Locale::*;
    match (kind, locale) {
        (UserMessage::InvalidDateFormat { label, value }, En) =>
            format!("Invalid {label} format: `{value}` (expected YYYY-MM-DD)"),
        (UserMessage::InvalidDateFormat { label, value }, Es) =>
            format!("Formato de {label} no válido: `{value}` (se esperaba YYYY-MM-DD)"),
        // …
    }
}
```

`Locale` is the same enum used by `pdf/locale.rs`. The
`user_message` function is the **only** place that decides which
Spanish / English string is emitted; services call it with a
`UserMessage` variant + `Locale` and the function owns the format.

### 6.3 How the locale reaches the service

The locale is **not** threaded through every service signature. Instead:

- Commands that surface validation messages from `DomainError::Validation`
  accept a `locale: &str` parameter on the IPC boundary and translate
  at the command layer.
- For `parse_strict_date` (called from `list_due_notifications` and
  `mark_notification_shown`), the date validation error is currently
  emitted as `DomainError::Validation { message }`. The service keeps
  the same shape; the **command** wraps the error with
  `user_message(UserMessage::InvalidDateFormat { label, value }, locale)`
  before returning it. This keeps service signatures stable.

**Why wrap at the command layer.** Threading `Locale` through every
service signature would force a refactor across the codebase. The
`CommandError::Validation { message }` already carries the string; the
command layer is the natural place to translate it because the locale
arrives over IPC exactly there.

The wrap function in `commands/notifications.rs` (and analogous
`commands/reports.rs`, `commands/expiry_lots.rs`) is small:

```rust
fn localize_validation(err: AppError, locale: Locale) -> AppError {
    let AppError::Domain(DomainError::Validation { message }) = err else {
        return err;
    };
    // Best-effort parse; on miss, leave the English message.
    if let Some(kind) = parse_user_message_kind(&message) {
        return AppError::Domain(DomainError::Validation {
            message: user_message(kind, locale),
        });
    }
    AppError::Domain(DomainError::Validation { message })
}
```

`parse_user_message_kind` is the inverse of `user_message` for the
messages we own. For messages we do not own (e.g. a service emits a
free-form English string), the original message passes through
unchanged. Developer-only errors (`DomainError::Internal`, tracing
logs) are never translated.

## 7. Rust PDF translation shape, locale fallback, parity test

### 7.1 Module shape (`src-tauri/src/pdf/locale.rs`)

```rust
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Es,
}

impl Locale {
    pub fn parse(tag: &str) -> Locale {
        // Take the primary subtag (before "-"), lower-case, and map.
        let primary = tag.split('-').next().unwrap_or("").to_ascii_lowercase();
        match primary.as_str() {
            "es" => Locale::Es,
            // "en" and everything else fall back to English.
            _ => Locale::En,
        }
    }
}

/// Keys for every PDF-visible string. Adding a key without populating
/// it for both locales fails the parity test.
#[derive(Debug, Clone, Copy)]
pub enum PdfMessageKey {
    DocumentTitle,             // "Caduxo Report" / "Reporte Caduxo"
    PdfMetadataTitle,          // same as DocumentTitle (PDF metadata Title)
    HeaderGenerated,           // "Generated: {date}  ·  Rows: {n}" / "Generado: {date}  ·  Filas: {n}"
    HeaderFiltersPrefix,       // "Filters:" / "Filtros:"
    FilterPrefixStore,         // "store=" / "tienda="
    FilterPrefixLocation,      // "location=" / "ubicación="
    FilterPrefixCategory,      // "category=" / "categoría="
    FilterPrefixCategoriesN,   // "categories ({n})" / "categorías ({n})"
    FilterPrefixUrgency,       // "urgency=" / "urgencia="
    FilterPrefixFrom,          // "from=" / "desde="
    FilterPrefixTo,            // "to=" / "hasta="
    ColumnSku,                 // "SKU"
    ColumnDescription,         // "Description" / "Descripción"
    ColumnStoreLocation,       // "Store / Location" / "Tienda / Ubicación"
    ColumnQty,                 // "Qty" / "Cantidad"
    ColumnExpiry,              // "Expiry" / "Caducidad"
    ColumnDays,                // "Days" / "Días"
    ColumnAlert,               // "Alert" / "Alerta"
    ColumnBatch,               // "Batch" / "Lote"
    EmptyNotice,               // "No rows match the current report filters." / "Ninguna fila coincide con los filtros actuales."
    DaysAgoTemplate,           // "{n} ago" / "hace {n} días"
    FooterPageOf,              // "Page {n} of {m}" / "Página {n} de {m}"
    Brand,                     // "Caduxo · Expiry Tracker" / "Caduxo · Control de caducidades"
}

pub struct PdfMessages {
    pub document_title: Cow<'static, str>,
    pub pdf_metadata_title: Cow<'static, str>,
    pub header_generated: Cow<'static, str>,
    pub header_filters_prefix: Cow<'static, str>,
    pub filter_prefix_store: Cow<'static, str>,
    pub filter_prefix_location: Cow<'static, str>,
    pub filter_prefix_category: Cow<'static, str>,
    pub filter_prefix_categories_n: Cow<'static, str>,
    pub filter_prefix_urgency: Cow<'static, str>,
    pub filter_prefix_from: Cow<'static, str>,
    pub filter_prefix_to: Cow<'static, str>,
    pub columns: [Cow<'static, str>; 8], // SKU..Batch
    pub empty_notice: Cow<'static, str>,
    pub days_ago_template: Cow<'static, str>,
    pub footer_page_of: Cow<'static, str>,
    pub brand: Cow<'static, str>,
}

pub fn pdf_messages(locale: Locale) -> PdfMessages { /* match Locale */ }
```

`pdf_messages(Locale::En)` returns all English strings, `Locale::Es`
returns Spanish. `Cow<'static, str>` lets the static English strings
sit in `.rodata` while the dynamic days-ago / category-count templates
allocate.

### 7.2 Locale fallback

`Locale::parse("es-MX")` returns `Locale::Es`. `Locale::parse("fr")`
returns `Locale::En`. `Locale::parse("")` returns `Locale::En`. This is
the canonical fallback path — same logic on the IPC boundary, in the
service, and in the parity test.

### 7.3 `ReportType::description(locale)`

```rust
impl ReportType {
    pub fn description(&self, locale: Locale) -> Cow<'static, str> {
        use Locale::*;
        match (self, locale) {
            (ReportType::InAlertWindow, En) => Cow::Borrowed("Lots in their alert window"),
            (ReportType::InAlertWindow, Es) => Cow::Borrowed("Lotes en ventana de alerta"),
            (ReportType::Expired, En) => Cow::Borrowed("Expired lots"),
            (ReportType::Expired, Es) => Cow::Borrowed("Lotes vencidos"),
            (ReportType::Next30Days, En) => Cow::Borrowed("Lots expiring in the next 30 days"),
            (ReportType::Next30Days, Es) => Cow::Borrowed("Lotes que vencen en los próximos 30 días"),
            (ReportType::Custom, En) => Cow::Borrowed("Custom filtered report"),
            (ReportType::Custom, Es) => Cow::Borrowed("Reporte personalizado filtrado"),
        }
    }
}
```

`services::reports::build_metadata` calls
`request.kind.description(locale)` and stores the result in
`ReportMetadata.description`. The frontend reads
`ReportMetadata.description` verbatim — no client-side substitution.

### 7.4 Parity test (`#[cfg(test)] mod tests` in `pdf/locale.rs`)

```rust
#[test]
fn pdf_messages_parity_for_en_and_es() {
    let en = pdf_messages(Locale::En);
    let es = pdf_messages(Locale::Es);

    for (en_str, es_str) in [
        (en.document_title.as_ref(), es.document_title.as_ref()),
        (en.header_generated.as_ref(), es.header_generated.as_ref()),
        (en.empty_notice.as_ref(), es.empty_notice.as_ref()),
        // …one entry per field…
    ] {
        assert!(!en_str.is_empty(), "English string is empty");
        assert!(!es_str.is_empty(), "Spanish string is empty for `{en_str}`");
    }
}
```

The test is the single source of truth that every PDF string has a
non-empty counterpart in both locales. Adding a new key without
extending the test fails `cargo test --lib`.

### 7.5 How the renderer uses the table

`pdf/report_pdf.rs::render_report` accepts `locale: Locale` as a new
parameter. The internals compute `let msgs = pdf_messages(locale);`
once and route every visible string through `msgs.*`. Column headers
become `&msgs.columns[i]` instead of `&COLUMNS[i].header`. The
`format_filters` helper becomes:

```rust
fn format_filters(filters: &ReportFilters, msgs: &PdfMessages) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = filters.store_id.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("{}{}", msgs.filter_prefix_store, truncate(s, 16)));
    }
    // …same shape for every prefix…
    parts.join(" · ")
}
```

The `format_days` helper becomes:

```rust
fn format_days(days: i64, msgs: &PdfMessages) -> String {
    if days < 0 {
        msgs.days_ago_template.replace("{n}", &(-days).to_string())
    } else {
        days.to_string()
    }
}
```

`{n}` is the only placeholder; we keep it as a literal `{n}` substring
and replace it, avoiding a dependency on a templating crate.

## 8. IPC changes for `preview_report` and `export_report_pdf`

### 8.1 Rust

```rust
// commands/reports.rs
#[tauri::command]
pub async fn preview_report(
    state: State<'_, AppState>,
    request: ReportRequest,
    locale: String,
) -> Result<ReportData, CommandError> {
    let loc = Locale::parse(&locale);
    let pool = state.pool().await;
    service::preview_report(&pool, request, loc)
        .await
        .map_err(AppError::into)
}

#[tauri::command]
pub async fn export_report_pdf(
    state: State<'_, AppState>,
    request: ReportRequest,
    file_path: String,
    locale: String,
) -> Result<crate::pdf::report_pdf::RenderedReport, CommandError> {
    let loc = Locale::parse(&locale);
    let path = PathBuf::from(&file_path);
    let pool = state.pool().await;
    service::export_report_pdf(&pool, request, path, loc)
        .await
        .map_err(AppError::into)
}
```

`service::preview_report` threads `loc` through to
`build_metadata` (for `description(locale)`) and back to the caller;
the `ReportData` carries the localised description. The PDF path
forwards `loc` into `pdf::report_pdf::render_report`.

### 8.2 Frontend wrapper (`src/lib/reports.ts`)

```ts
export async function previewReport(
  request: ReportRequest,
  locale: SupportedLocale,
): Promise<ReportData> {
  return invoke<ReportData>("preview_report", { request, locale });
}

export async function exportReportPdf(
  request: ReportRequest,
  file_path: string,
  locale: SupportedLocale,
): Promise<PdfExportResult> {
  return invoke<PdfExportResult>("export_report_pdf", {
    request,
    filePath: file_path,
    locale,
  });
}

export async function exportReportPdfWithDialog(
  request: ReportRequest,
  locale: SupportedLocale,
): Promise<PdfExportResult | null> {
  const path = await pickPdfSavePath(`caduxo-${request.kind}`);
  if (!path) return null;
  return exportReportPdf(request, path, locale);
}
```

`ReportsPage.svelte` reads `locale.current` from the rune and passes
it on every call:

```ts
const result = await exportReportPdfWithDialog(request, $locale.current);
```

### 8.3 Why the locale is `&str`, not the enum

Tauri IPC serialises `&str` directly; using a Rust enum would force a
serde tag on the boundary that gains nothing for a two-value type.
`Locale::parse` is the canonical deserialiser, called at the command
boundary so services stay locale-agnostic.

## 9. Notification translation composition (frontend, SKU visible)

`src/lib/notifications.ts` reads the active locale from the rune
**inside the function call**, not at module load:

```ts
import { get } from "svelte/store";
import { locale } from "../i18n/locale.js";
// typesafe-i18n exposes the active LL helper; we read it via a tiny
// adapter that calls into the generated runtime.
import { LL } from "../i18n/i18n.js";

function formatNotificationBody(lot: DueNotificationLot): string {
  const location = lot.location_name
    ? ` · ${lot.store_name} / ${lot.location_name}`
    : ` · ${lot.store_name}`;
  const qty = `${lot.quantity} ${lot.unit}`;
  // typesafe-i18n provides a localised template that keeps the SKU
  // and the placeholders for qty / expiry_date / location.
  return LL.notification.bodyTemplate({
    qty,
    expiry_date: lot.expiry_date,
    location: location.replace(/^ · /, ""),
  });
}

async function showAndRecordNotification(lot: DueNotificationLot): Promise<void> {
  try {
    await sendNotification({
      title: `${LL.notification.titlePrefix()}${lot.sku}`,
      body: formatNotificationBody(lot),
    });
    await markNotificationShown({
      expiry_lot_id: lot.lot_id,
      notification_date: lot.notification_date,
    });
  } catch {
    // unchanged — fire-and-forget on the next tick.
  }
}
```

**Why inside the function.** The rune is reactive; calling
`locale.current` at module load would freeze the value at import time.
Reading the rune (or `LL.notification.titlePrefix()`) inside the
function re-evaluates each fire.

**SKU visibility.** The Spanish template is
`{qty} caduca el {expiry_date} · {location}` and the English template
is `{qty} expires on {expiry_date} · {location}`. The title is
`⚠️ Alerta de caducidad: {sku}` / `⚠️ Expiry alert: {sku}`. The SKU
sits **inside the template** as a concatenation of the prefix and the
SKU value, so it is always present and visibly the same in both
locales.

The backend IPC contract for notifications does **not** change. The
notification pipeline is a frontend composition only.

## 10. `printpdf` Spanish accent coverage risk

`printpdf` 0.7 ships `BuiltinFont::Helvetica` and
`BuiltinFont::HelveticaBold`. Both use **WinAnsiEncoding**, the
standard PDF encoding for the base 14 fonts. WinAnsiEncoding is a
superset of ISO-8859-1 / Windows-1252 and covers every Spanish
character in the supported set:

| Character | Unicode | WinAnsiEncoding byte |
|-----------|---------|----------------------|
| `á` | U+00E1 | 0xE1 |
| `é` | U+00E9 | 0xE9 |
| `í` | U+00ED | 0xED |
| `ó` | U+00F3 | 0xF3 |
| `ú` | U+00FA | 0xFA |
| `ñ` | U+00F1 | 0xF1 |
| `ü` | U+00FC | 0xFC |
| `¿` | U+00BF | 0xBF |
| `¡` | U+00A1 | 0xA1 |

**Risk.** Two failure modes are possible:

1. `printpdf`'s `use_text` accepts a `&str` and re-encodes to the
   font's encoding internally. If the library is treating the input as
   ISO-8859-1 or as raw bytes without an encoding step, the byte
   ordering will still match (every Spanish character's UTF-8 bytes
   encode WinAnsi characters whose top bit set matches the leading
   UTF-8 byte). The visual output is therefore correct under both
   interpretations for our characters.
2. Some PDF readers on older Windows builds map WinAnsi bytes through
   a different code page. Empirically, the base 14 fonts have stable
   glyph coverage for the Latin-1 Supplement across major readers.

**Mitigation.**

- The Rust integration test renders a fixture with every Spanish
  character (`á é í ó ú ñ ü ¿ ¡ Caduxo · Control de caducidades
  Categoría: Dairy · Ubicación: Frigorífico`) into a PDF and asserts
  the file size is greater than a baseline English-only fixture, which
  is a coarse proxy for "the bytes were written". A precise byte-level
  assertion is impractical because `printpdf` compresses streams.
- The verify report for this change includes a manual smoke test that
  opens a Spanish PDF in Preview, Adobe Reader, and Okular and
  confirms the accents render. The dev environment is Linux + Okular;
  the CI environment is headless and uses `pdfinfo` to confirm the
  file is well-formed plus a byte-presence check.
- If a reader breaks, the fallback is a custom TTF. The font
  candidates (Roboto Regular, DejaVu Sans) are out of scope for v1 and
  require a separate change that bundles the TTF in `src-tauri/`.

**Decision for this change.** Accept WinAnsiEncoding coverage. The
spot-check is part of the verify report; if a reader breaks, file a
follow-up change. We do not introduce a custom TTF in this slice.

## 11. App bootstrap sequence

```
main.ts
  ├─ import { initLocale } from "./i18n/locale.js"
  ├─ await initLocale()                    // 1. resolve locale, setLocale()
  └─ mount App.svelte                      // 2. first render uses $LL.*

App.svelte
  ├─ <App /> reads locale.current          // nav buttons use $LL.nav.*
  ├─ <ConfigurationPage /> mounts          // existing optimistic pattern
  ├─ startPeriodicNotificationCheck()      // 3. uses $LL.notification.*
  └─ onDestroy cleanup

ConfigurationPage
  ├─ onMount: getSettings() → language, requireLocation
  ├─ render <select bind:value={locale}>
  ├─ if translationSource.current === "detected": render $LL.detectedHint
  └─ on change: setLocale(next) → updateSettings({ language: next })
```

The init order is critical: `initLocale()` MUST complete before
`App.svelte` is mounted, otherwise the first paint of `$LL.*` would use
the default English runtime and immediately switch. Because
`@tauri-apps/api/os::locale()` is an async IPC call, the first paint
incurs a small delay; the trade-off (correct first paint vs.
flash-of-English) is the right one. The Configuration page's
optimistic update gives an instant visual confirmation when the user
picks a locale, hiding the boot delay.

### 11.1 Detection helper (`src/i18n/detect.ts`)

```ts
import { locale as osLocale } from "@tauri-apps/plugin-os";
import type { SupportedLocale } from "./locale.js";

export async function detectSupportedLocale(): Promise<SupportedLocale> {
  // 1. Production: Tauri OS locale.
  try {
    const tag = await osLocale(); // e.g. "es-MX"
    return mapTag(tag);
  } catch {
    // 2. Dev fallback: navigator.language.
  }
  if (typeof navigator !== "undefined" && navigator.language) {
    return mapTag(navigator.language);
  }
  return "en";
}

function mapTag(tag: string): SupportedLocale {
  const primary = tag.split("-")[0].toLowerCase();
  if (primary === "es") return "es";
  return "en"; // en + everything else → English
}
```

`@tauri-apps/plugin-os` is added to `package.json` and the
`tauri-plugin-os` crate to `src-tauri/Cargo.toml`. The capability
file gains `"os:default"` under `permissions`.

## 12. Review workload forecast and task slicing

The 3000-line session budget is generous, but the change touches every
Svelte component and several backend files. A single PR risks
defeating review focus. The forecast below justifies a **two-PR
chained split**, each well under 1500 lines.

### 12.1 Forecast (net changed lines, approximate)

| Slice | Approx. changed lines |
|-------|-----------------------|
| Backend settings (DTO + repo + service + command + tests) | 180 |
| Backend PDF locale module + tests | 280 |
| Backend report IPC delta + service threading + tests | 150 |
| Backend user_messages module + wrapping in 3 services | 220 |
| Backend Tauri os-plugin registration + capability | 30 |
| Backend subtotal | **860** |
| Frontend i18n trees (en + es) | 400 |
| Frontend i18n runtime (locale.ts, detect.ts, generated) | 290 |
| Frontend App.svelte nav | 30 |
| Frontend ConfigurationPage (selector + hint + re-keying) | 220 |
| Frontend all other components (~18 files × ~20 lines) | 360 |
| Frontend notifications + reports wrapper | 60 |
| Frontend main.ts bootstrap | 20 |
| Frontend subtotal | **1380** |
| Docs (this design, verify report update) | 80 |
| **Total** | **~2320** |

`2320 < 3000`, so a single PR is technically within budget. The
**two-PR chain** is recommended anyway because:

- The backend slice is independently testable with `cargo test --lib`
  and produces no UI churn for human review.
- The frontend slice is reviewable as "is every string routed through
  `$LL.*`" without having to read Rust code.
- A regression in one slice does not block the other.

### 12.2 Chained PR plan

**PR 1 — Backend i18n plumbing (≈860 lines).**

Tasks:

1. `SettingsResponse.language` + `SettingsUpdate.language` +
   `get_language_setting` + `set_language_setting` + IPC validation +
   tests.
2. `pdf/locale.rs` new module + `Locale::parse` + `pdf_messages` +
   parity test.
3. `pdf/report_pdf.rs::render_report(locale: Locale)` thread-through
   for all visible strings; column headers via `PdfMessages.columns`.
4. `ReportType::description(locale)` + `services/reports.rs` threading.
5. `commands/reports.rs` `locale: String` parameter; `Locale::parse`
   at the boundary.
6. `services/user_messages.rs` new module; wrap
   `parse_strict_date`, `validate_quantity`, `validate_alert_days`,
   `validate_filters`, CSV header detection.
7. `commands/notifications.rs`, `commands/reports.rs`,
   `commands/expiry_lots.rs` wrapping of validation errors.
8. `tauri-plugin-os` registration + `os:default` capability.

Verify gate: `cargo test --manifest-path src-tauri/Cargo.toml --lib`
green; `cargo build` green; sample Spanish PDF rendered and
spot-checked for accents.

**PR 2 — Frontend i18n (≈1380 lines).**

Tasks:

1. `npm install typesafe-i18n @tauri-apps/plugin-os`.
2. `src/i18n/{en,es}/index.ts` + generator pipeline (`prebuild`,
   `predev`).
3. `src/i18n/locale.ts` + `src/i18n/detect.ts` + main.ts bootstrap.
4. `src/App.svelte` nav buttons → `$LL.nav.*`.
5. `src/components/ConfigurationPage.svelte` locale selector,
   "Detected" hint, re-keyed toggle copy.
6. `src/components/ReportsPage.svelte` labels + descriptions +
   `locale.current` passed to `previewReport` / `exportReportPdf`.
7. `src/components/DashboardPage.svelte` presets + banner copy.
8. `src/lib/notifications.ts` localised title / body via `LL`.
9. `src/lib/reports.ts` + `src/lib/stores.ts` updated DTOs.
10. Remaining components: `ScanSearchBox`, `StoresPage`,
    `ProductCatalogPage`, `ProductForm`, `CalendarPage`,
    `BackupRestorePage`, `CsvImportPage`, `UnitReviewPage`,
    `LotMovementsPanel`, `UnitReviewBanner`, `inputs/CategoryPicker`,
    and modal helpers — re-keyed copy.

Verify gate: `npm run build` green; `npx svelte-check --threshold
error` clean; manual smoke on Linux + Windows for both locales
including Configuration page selector, detected hint, PDF export
in Spanish (preview UI description + PDF metadata + PDF body), and
OS notification composition in both locales.

### 12.3 Risk-based escalation

If the final forecast after PR 1 lands exceeds **1500 net changed
lines**, the orchestrator MUST pause and ask whether to:

- Split PR 2 into PR 2a (Configuration + App + notifications) and
  PR 2b (other components), or
- Adopt the `single-pr` strategy and accept the larger review
  workload, or
- Adopt `size:exception` and explicitly opt in.

This is the `deliveryStrategy: ask-on-risk` decision round. The
design does not pre-commit to a single PR or a three-PR chain; the
final choice belongs to the orchestrator once the actual diff is
visible.

## 13. Verification commands

The verify gate for this change (after both PRs land) is:

```bash
# Backend
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo build --manifest-path src-tauri/Cargo.toml

# Frontend
npm run build
npx svelte-check --workspace . --threshold error

# Manual smoke (human review, not CI)
# - Configuration: locale selector, detected hint, manual override,
#   restart retains, hint disappears after manual pick, hint
#   reappears if the row is cleared.
# - PDF: preview UI description localised; PDF metadata title and
#   body strings localised; accents (á é í ó ú ñ ü) render correctly
#   in Okular, Preview, and Adobe Reader.
# - Notification: title and body localised; SKU visible in both.
# - Error messages: parse_strict_date surfaces Spanish / English
#   based on active locale; developer-only errors stay English.
```

## 14. Risks and mitigations (design-level)

| Risk | Impact | Mitigation |
|------|--------|------------|
| `typesafe-i18n` Svelte 5 runes integration breaks at upgrade | Build fails | Pin the package version; rely on `prebuild` so the generated `i18n.ts` is always fresh; `svelte-check` catches drift |
| New `language` row clobbers concurrent settings update | Stale `last_selected_store_id` | `SettingsUpdate` remains partial-update (`Option<…>`); no new concurrency primitive |
| `Locale::parse` rejects a valid user tag like `es-419` | Spanish-speaking LATAM users get English | The primary-subtag rule (`es*` → `es`) covers this case; verified in tests |
| `pdf_messages(Locale::Es)` has a typo / missing key | Visible bug in PDF | Parity test fails `cargo test --lib`; CI catches it before merge |
| Two translation trees drift | One language gets a missing copy | `typesafe-i18n` generator is the build-time gate; same pattern on the backend with parity test |
| `printpdf` accent rendering breaks on a reader | Visible garbled text | Spot-check in verify; defer custom TTF to a follow-up change |
| Chained PR 2 is large even after the split | Reviewer fatigue | Re-split per §12.3 if the final diff exceeds 1500 net lines |
| The "Detected" hint is not the persisted row itself | UI vs data drift | Hint is derived from `translationSource.current`, which is set at the same time as the active locale; no separate DB state |
| Notification title concatenation is brittle | Wrong prefix across locales | Title is `${prefix}${sku}` where `prefix` is `LL.notification.titlePrefix()`; SKU is data, never translated |
| `os.locale()` is unavailable in `vite dev` without the Tauri shell | Dev detection fails | Fall back to `navigator.language`; the dev experience matches production closely enough |

## 15. Out of scope reminders (reaffirmed)

The following are explicitly NOT in this change. They are noted here so
the design is unambiguous:

- A third locale (e.g. Portuguese, French) — file a follow-up change.
- RTL support — Svelte CSS layout assumes LTR.
- Locale-aware number / currency formatting — Caduxo does not deal
  in money.
- Locale-aware date format in the PDF — `DD/MM/YYYY` stays.
- Translating user-entered data (product names, descriptions, batch
  codes, notes, store / location names).
- Translating PDF document properties beyond the visible title.
- A first-run locale dialog or in-app banner — the Configuration page
  selector is the only surface.
- Per-store / per-user locale overrides.
- A new Tauri plugin-store for the locale — the persisted row is in
  the existing `app_settings` table.
- Localising the user manual / docs (none ship today).
- A custom TTF font for the PDF — Helvetica + WinAnsiEncoding is
  accepted; a follow-up change bundles a TTF if a reader breaks.

## 16. Open items deferred to `tasks.md`

- Exact list of keys per component (the trees in `src/i18n/en/index.ts`
  enumerate them, but the precise re-keying per file is a
  per-component edit and belongs to `tasks.md`).
- The exact `pdf_messages` static literal strings (the design names
  every key; the values are the spec's Spanish translations).
- Per-component UI regression scenarios for the Configuration page's
  re-keyed toggle copy.
- The verify report's per-locale smoke checklist (manual review
  artefact, not a CI command).