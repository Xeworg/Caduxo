# Delta for Caduxo Expiry Tracker

## ADDED Requirements

### Capability: Internationalisation

#### Requirement: typesafe-i18n Svelte 5 integration

The Svelte 5 frontend MUST render every user-visible string through the
`typesafe-i18n` runtime. The translation function MUST be generated
statically so that referencing a key absent from the active locale tree
fails the TypeScript build rather than rendering silently at runtime.

The active locale MUST be held in a single Svelte 5 reactive rune/store.
Calling the runtime's `setLocale(locale)` MUST re-render every template
that binds to the generated `$LL` helper on the next tick. No component
MUST embed a hard-coded user-visible English or Spanish string outside of
the translation tree.

#### Scenario: missing translation key fails the build

- GIVEN `en.json` defines the key tree under `LL.configuration.language.label`
- AND `es.json` does not define `LL.configuration.language.label`
- WHEN the build runs
- THEN the TypeScript build fails with a missing-key diagnostic that
  points at the offending key path
- AND the missing key is not silently rendered as English at runtime

#### Scenario: locale switch re-renders bound text

- GIVEN the active locale is `en`
- AND a template renders `$LL.configuration.section.lots`
- WHEN the user selects `Español` from the Configuration page selector
- THEN `setLocale("es")` is invoked
- AND the template re-renders with the Spanish translation on the next
  tick without a page reload

#### Scenario: no hard-coded user-visible strings outside the translation tree

- GIVEN the codebase after this slice lands
- WHEN the rendered Svelte components under `src/components/**/*.svelte`
  and `src/App.svelte` are inspected for hard-coded user-visible copy
- THEN every string visible to the user is bound to the translation tree
- AND the existing `Configuración` / `Ubicación inicial obligatoria al
  crear lote` strings are routed through `LL.configuration.*` keys

#### Requirement: supported locales and English fallback

The system MUST support exactly two locales: `en` (English) and `es`
(Spanish). English MUST be the canonical fallback when a translation
key is absent in the active locale, when locale detection fails, when
the persisted preference is empty or unrecognised, or when the
runtime cannot resolve a `&str` locale tag to a supported locale.

#### Scenario: unsupported detection falls back to English

- GIVEN the host OS reports locale tag `fr-FR`
- WHEN the bootstrap resolves the active locale
- THEN the active locale is `en`
- AND no error is surfaced to the user

#### Scenario: persisted empty value falls back to English

- GIVEN the `app_settings` row for `language` exists with value `""`
- WHEN the bootstrap reads the persisted preference
- THEN the active locale is `en`

#### Scenario: unknown `&str` locale on a Rust command falls back to English

- GIVEN the frontend passes `locale = "fr"` to `export_report_pdf`
- WHEN the Rust handler resolves the locale
- THEN the handler treats it as `En`
- AND the PDF renders English strings

#### Requirement: locale detection and bootstrap ordering

At app start, the bootstrap MUST resolve the active locale in this
exact order:

1. The persisted `app_settings.language` value, when present and one of
   `{en, es}`.
2. Otherwise, the OS or WebView locale tag (`@tauri-apps/api/os::locale()`
   in production, `navigator.language` as a development fallback),
   normalised to its lower-cased primary subtag and mapped to the
   closest supported locale (`es*` → `es`, `en*` → `en`, anything else
   → `en`).
3. On any detection failure, English.

The OS locale MUST be re-read on every app start. No `localStorage`
cache of the detected tag is consulted; the persisted `language` row is
the canonical cache, and a `localStorage` mirror is permitted only as
a first-paint accelerator, never as the source of truth.

#### Scenario: persisted value wins over detected tag

- GIVEN the persisted `language` value is `es`
- AND the OS reports `en-US`
- WHEN the bootstrap runs
- THEN the active locale is `es`
- AND the OS tag is not consulted because step 1 succeeded

#### Scenario: closest-supported mapping for es-MX

- GIVEN no persisted `language` value exists
- AND the OS reports `es-MX`
- WHEN the bootstrap resolves the active locale
- THEN the active locale is `es`

#### Scenario: closest-supported mapping for en-GB

- GIVEN no persisted `language` value exists
- AND the OS reports `en-GB`
- WHEN the bootstrap resolves the active locale
- THEN the active locale is `en`

#### Scenario: detection failure falls back to English

- GIVEN the Tauri runtime is unavailable (e.g. `vite dev` without the
  Tauri shell) and `navigator.language` returns `undefined`
- WHEN the bootstrap resolves the active locale
- THEN the active locale is `en`
- AND the application renders normally with English copy

#### Requirement: locale selector with detected hint on Configuration page

The Configuration page MUST render a locale selector inside its existing
layout, above the `Ubicación inicial obligatoria al crear lote` toggle.
The selector MUST be a native `<select>` element listing `English` and
`Español` by their own names. Above the selector, a hint paragraph MUST
render `Detectado: Español` or `Detected: English` when the active
locale was inherited from OS/WebView detection and the user has not yet
made a manual choice. The hint MUST disappear once a manual choice is
persisted. The selector MUST optimistically update the UI on change and
call the existing `updateSettings({ language })` IPC command; on IPC
failure, the selector MUST roll back to the prior locale and surface a
translated "could not save" inline error in the same pattern used by the
existing toggle.

#### Scenario: detected hint renders above the selector on first run

- GIVEN no persisted `language` value exists
- AND the OS reports `es-MX`
- WHEN the Configuration page renders
- THEN the selector's active option is `Español`
- AND the hint reads `Detectado: Español`
- AND the hint is positioned above the selector

#### Scenario: detected hint is in English when detection picked English

- GIVEN no persisted `language` value exists
- AND the OS reports `en-GB`
- WHEN the Configuration page renders
- THEN the hint reads `Detected: English`

#### Scenario: hint disappears after manual selection

- GIVEN the Configuration page renders with the detected hint visible
- WHEN the user selects `English` from the dropdown
- THEN `updateSettings({ language: "en" })` is invoked
- AND on success the hint is no longer rendered
- AND the persisted `language` value is `en`
- AND the selector's active option is `English`

#### Scenario: selector update rolls back on IPC failure

- GIVEN the active locale is `es`
- AND the persisted `language` value is `es`
- WHEN the user selects `English`
- AND `updateSettings({ language: "en" })` returns an error
- THEN the selector returns to `Español`
- AND the persisted `language` value remains `es`
- AND a translated "could not save" message is rendered in the inline
  error slot in Spanish

#### Scenario: hint reappears if persisted value is cleared

- GIVEN a previous session persisted `language = "es"`
- AND the user has cleared the row manually (or the row was removed)
- WHEN the bootstrap runs again on the next launch
- THEN detection is consulted
- AND the Configuration page renders the detected hint once more

#### Requirement: persisted language setting

`SettingsResponse` MUST include a `language: String` field, populated
from `app_settings.language` and defaulting to `"en"` when the row is
absent, empty, or unrecognised. `SettingsUpdate` MUST accept an optional
`language: Option<String>` field whose presence causes only that key to
be updated; partial updates MUST NOT touch `last_selected_store_id` or
`require_initial_location_on_lot_create`. The settings repository MUST
expose `get_language_setting() -> Option<String>` and
`set_language_setting(value: String)` helpers. Values outside `{en, es}`
MUST be rejected by `set_language_setting` and rejected by the IPC
handler; the existing rows are not rewritten by the rejection.

#### Scenario: missing row behaves as English

- GIVEN no `language` row exists in `app_settings`
- WHEN `getSettings` is invoked
- THEN the response carries `language = "en"`
- AND the field is non-optional on the response

#### Scenario: partial update of only language

- GIVEN the existing settings are
  `{ last_selected_store_id: Some("store-1"), require_initial_location_on_lot_create: true, language: "en" }`
- WHEN the frontend calls `updateSettings({ language: Some("es") })`
- THEN `app_settings.language` is set to `"es"`
- AND `last_selected_store_id` remains `Some("store-1")`
- AND `require_initial_location_on_lot_create` remains `true`

#### Scenario: invalid value is rejected

- GIVEN the IPC handler receives
  `updateSettings({ language: Some("fr") })`
- WHEN the handler validates the payload
- THEN the update is rejected with a validation error
- AND the persisted `language` value is unchanged

#### Requirement: translation key parity between locales

Every key defined under `en.json` MUST have a non-empty counterpart
under `es.json` and vice versa. The `typesafe-i18n` generator MUST
enforce this constraint at build time so missing or empty translations
surface as build errors rather than runtime English fallbacks. On the
Rust side, the PDF message table MUST assert the same invariant for
both `En` and `Es`: every documented key MUST be populated in both
locales.

#### Scenario: missing Spanish counterpart fails the frontend build

- GIVEN `en.json` defines `LL.configuration.language.detectedHint`
- AND `es.json` does not define that key
- WHEN the generator runs as part of `npm run build`
- THEN the TypeScript build fails
- AND the missing key is reported by its `LL.*` path

#### Scenario: PDF message table populates the same keys for both locales

- GIVEN the Rust PDF message table enumerates the documented key set
- WHEN a parity test asserts the table for both `En` and `Es`
- THEN every key returns a non-empty string in both locales
- AND the parity test fails if any key is empty in either locale

#### Requirement: locale-aware user-visible backend error strings

The backend MUST surface user-visible validation strings in the active
locale when those strings are returned over IPC and displayed verbatim
by the UI. The contract covers, at minimum:

- The `parse_strict_date` validation message:
  - English: `Invalid {label} format: \`{value}\` (expected YYYY-MM-DD)`
  - Spanish: `Formato de {label} no válido: \`{value}\` (se esperaba YYYY-MM-DD)`
- The `DomainError::Validation` strings the UI currently displays
  verbatim to users.

The active locale MUST be passed through the IPC payload when an
operation creates a user-visible validation string. Developer-only
`DomainError::Internal` strings and tracing logs MUST remain in English.

#### Scenario: validation message is rendered in Spanish

- GIVEN the active locale is `es`
- WHEN the user submits a lot with an invalid `notification_date`
- THEN the backend returns a localised validation string that begins
  with `Formato de`
- AND the form surfaces that string verbatim in the inline error slot

#### Scenario: validation message is rendered in English

- GIVEN the active locale is `en`
- WHEN the user submits a lot with an invalid `notification_date`
- THEN the backend returns the original English validation string
- AND the form surfaces it verbatim

#### Scenario: developer error remains English

- GIVEN any backend code emits a `DomainError::Internal`
- WHEN the error is logged or returned over IPC
- THEN the string is in English regardless of the active locale
- AND no translation table is consulted for the string

### Capability: Reports

#### Requirement: locale-aware PDF report strings

The PDF report renderer MUST render the document title, header line,
filter-line prefix, footer line, brand line, column headers,
empty-state notice, `"{n} ago"` relative-time label, and filter
prefixes (`store=`, `location=`, `category=`, `categories (N)`,
`urgency=`, `from=`, `to=`) using the locale passed on the IPC
command. Column widths, row layout, and pagination MUST remain
unchanged from today.

The Spanish equivalents MUST be:

- Document title: `Reporte Caduxo`
- Header line label: `Generado: {date}  ·  Filas: {n}`
- Filter line prefix: `Filtros:`
- Filter prefixes: `tienda=`, `ubicación=`, `categoría=`,
  `categorías (N)`, `urgencia=`, `desde=`, `hasta=`
- Column headers: `SKU`, `Descripción`, `Tienda / Ubicación`,
  `Cantidad`, `Caducidad`, `Días`, `Alerta`, `Lote`
- Empty-state notice: `Ninguna fila coincide con los filtros actuales.`
- Relative-time formatter: `hace {n} días`
- Footer: `Página {n} de {m}`
- Brand line: `Caduxo · Control de caducidades`

#### Scenario: Spanish PDF renders translated filter prefixes

- GIVEN the active locale is `es`
- WHEN the user exports a PDF report with
  `store = "Bodega"` and `category = ["Dairy"]`
- THEN the filter line reads
  `Filtros: tienda=Bodega · categoría=Dairy`
- AND no English filter prefixes (`store=`, `category=`) appear in the
  document

#### Scenario: English PDF is unchanged

- GIVEN the active locale is `en`
- WHEN the user exports a PDF report
- THEN the filter line reads
  `Filters: store=Bodega · category=Dairy`
- AND the column headers match the pre-translation English text

#### Scenario: empty-state notice is translated

- GIVEN the active locale is `es`
- AND the report rows are empty
- WHEN the PDF renders
- THEN the empty-state notice reads
  `Ninguna fila coincide con los filtros actuales.`

#### Scenario: relative-time label is translated

- GIVEN the active locale is `es`
- AND a row's `days_remaining` is `-3`
- WHEN the PDF renders that row's `Days` cell
- THEN the cell reads `hace 3 días`

#### Scenario: column widths and pagination are unchanged

- GIVEN any PDF export, before or after this slice lands
- WHEN the rendered PDF is compared row-for-row with a pre-translation
  baseline
- THEN column widths, row spacing, font choice, and page breaks are
  identical
- AND only the visible text differs

#### Requirement: locale-aware report type descriptions

`ReportType::description()` MUST return a localised string drawn from
the active locale. The four canonical mappings are:

- `Lots in alert window` ↔ `Lotes en ventana de alerta`
- `Expired lots` ↔ `Lotes vencidos`
- `Lots expiring in the next 30 days` ↔ `Lotes que vencen en los
  próximos 30 días`
- `Custom filtered report` ↔ `Reporte personalizado filtrado`

The preview UI and the PDF MUST both receive the localised description
already resolved by the backend; the frontend MUST NOT translate the
description locally.

#### Scenario: preview description is in the active locale

- GIVEN the active locale is `es`
- WHEN the user opens the Reports page and selects the `Expired`
  report type
- THEN `ReportMetadata.description` reads `Lotes vencidos`

#### Scenario: PDF metadata description is in the active locale

- GIVEN the active locale is `es`
- WHEN the user exports the `Expired` report to PDF
- THEN the report caption rendered in the PDF body reads
  `Lotes vencidos`
- AND the preview description and the PDF caption match

### Capability: Local notifications

#### Requirement: locale-aware notification title and body

The notification title and body MUST be composed through the active
locale using the translation tree. The SKU MUST remain visible in the
notification body in both locales because the SKU is operational data
that identifies the lot. The Spanish equivalents MUST be:

- Title: `⚠️ Alerta de caducidad: {sku}` (the `⚠️` glyph is preserved)
- Body: `{qty} caduca el {expiry_date} · {location}` (with the same
  placeholder substitution semantics as the English version)

The backend IPC contract for notifications MUST remain unchanged;
locale selection happens entirely on the frontend through the active
`locale` rune.

#### Scenario: Spanish notification keeps the SKU visible

- GIVEN the active locale is `es`
- WHEN a daily alert fires for lot L with
  `sku = "ABC-001"`, `qty = 4`,
  `expiry_date = "2026-10-15"`, and `location = "Bodega"`
- THEN the notification title contains `ABC-001`
- AND the notification body reads the Spanish template
  `4 caduca el 2026-10-15 · Bodega`

#### Scenario: English notification is unchanged

- GIVEN the active locale is `en`
- WHEN a daily alert fires
- THEN the title reads `⚠️ Expiry alert: ABC-001`
- AND the body reads `4 expires on 2026-10-15 · Bodega`

#### Scenario: locale switch between fires re-composes the title

- GIVEN a lot has already fired an alert with locale `en`
- AND the user switches the active locale to `es`
- WHEN the next daily alert for the same lot fires
- THEN the title uses the Spanish template
- AND the previously shown English notification is not retroactively
  re-rendered

## MODIFIED Requirements

### Capability: Reports

#### Requirement: PDF export

`preview_report` and `export_report_pdf` MUST accept a new `locale:
&str` parameter carrying the active locale tag. The frontend MUST pass
the active locale from the Svelte `locale` rune on every call. The
Rust handler MUST resolve the `&str` to one of the supported locales
(`En`, `Es`), falling back to `En` for any unrecognised tag, and MUST
route every PDF string through the locale-aware PDF message table.

(Previously: the requirement stated only that reports export to PDF
using Rust-side structured generation. It did not pin a locale
contract on the IPC command, did not name a backend translation
table, and did not specify the empty-state, relative-time, or filter
prefix strings.)

#### Scenario: export_report_pdf receives the active locale

- GIVEN the active locale is `es`
- WHEN the user clicks `Exportar PDF` on the Reports page
- THEN the IPC command `export_report_pdf` is invoked with
  `locale = "es"`
- AND the resulting PDF renders the Spanish strings defined under
  the locale-aware PDF report strings requirement

#### Scenario: preview_report receives the active locale

- GIVEN the active locale is `es`
- WHEN the user opens the report preview
- THEN the IPC command `preview_report` is invoked with
  `locale = "es"`
- AND `ReportMetadata.description` returned to the frontend is the
  Spanish string

#### Scenario: unknown locale falls back to English

- GIVEN the frontend passes `locale = "fr"` to `export_report_pdf`
- WHEN the Rust handler resolves the locale
- THEN the handler treats it as `En`
- AND the PDF renders English strings
- AND no error is returned to the caller

### Capability: Local notifications

#### Requirement: daily alert window notification

Caduxo MUST generate local OS notifications once per day for each
active lot from `expiry_date - alert_days_before` through
`expiry_date`. The notification title and body MUST be composed
through the active locale using the translation tree; the SKU MUST
remain visible in the body in both locales because the SKU is
operational data that identifies the lot. Locale selection MUST
happen on the frontend through the active `locale` rune, and the
backend IPC contract for notifications MUST remain unchanged.

(Previously: the requirement stated only that daily notifications are
generated for active lots in the alert window. It did not pin a
locale-aware title/body template, did not name the SKU visibility
invariant, and did not constrain the backend IPC contract.)

#### Scenario: daily alert composition honours the active locale

- GIVEN a lot has `sku = "ABC-001"`, `qty = 4`,
  `expiry_date = "2026-10-15"`, and `location = "Bodega"`
- WHEN a daily alert fires for the lot while the active locale is `es`
- THEN the notification title is composed from the Spanish template
  `⚠️ Alerta de caducidad: ABC-001`
- AND the body is composed from the Spanish template
  `4 caduca el 2026-10-15 · Bodega`
- AND the body includes the SKU exactly as it appears in the lot

#### Scenario: backend notification IPC contract is unchanged

- GIVEN the daily alert check runs in `notifications.ts`
- WHEN it composes the title and body
- THEN it calls the existing notification IPC command with the same
  payload shape as before this slice
- AND no new locale parameter is added to the backend payload
