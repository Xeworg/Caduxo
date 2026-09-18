# Explore — caduxo-i18n-support

## Scope of this exploration

Caduxo today is shipped mostly in English and a small amount of Spanish (the
`Configuración` nav tab and the `Ubicación inicial obligatoria al crear lote`
toggle in `ConfigurationPage`). There is no i18n library in
`package.json`, no `i18n` / `l10n` directory, and no locale-aware backend
text. Hardcoded English appears in:

- The Svelte 5 frontend (navigation, dashboard, reports UI, modal
  placeholders, success/error messages).
- The Rust backend PDF report renderer (`printpdf`) — title, headers, footers,
  column labels, empty notice, "X ago" formatter.
- The Rust backend report DTO (`ReportType::description()`) — the metadata
  description is built in Rust and surfaced both in the preview UI and the
  PDF.
- The OS notification pipeline (`lib/notifications.ts`) — title/body
  formatting for `sendNotification`.
- Error messages returned over IPC from the backend (e.g.
  `Invalid notification_date format: \`X\` (expected YYYY-MM-DD)`).

This change adds **English + Spanish** support, surfaces the user's choice
through a manual selector in the Configuration page, persists it through the
existing settings backend, and translates the visible UI, the OS notification
copy, and the PDF report. The "after screens/texts stabilized" trigger in the
user request is satisfied today: the wording of the major surfaces
(dashboard, reports, configuration, navigation, banners) is stable and the user
explicitly approved starting the i18n work in this change.

The change does **not** add RTL support, currency/number/date localisation
beyond what is needed for the report's existing `DD/MM/YYYY` date format, or
any new locale beyond English/Spanish. Cataloguing, store/owner/product
descriptions, and free-text notes are user data and are not translated.

This document is **substrate for the proposal phase**. It does not propose the
schema, the IPC contract, or the UI layout — those belong to the proposal. It
captures the current string inventory, the supported options for each
implementation axis, the user-confirmed decisions, the non-goals, the risks,
and the open decisions that must be settled before the proposal is written.

---

## 1. Repository state

Working tree is clean except for the untracked `.codegraph/` directory.
Branch is `feat/i18n-support`. The previous ODD tracking file
(`odd/tasks/...`) was removed at user request before SDD was started, so no
pre-SDD artifact is being carried forward. No active OpenSpec changes are open
under `openspec/changes/`; the most recent archived change
(`2026-09-16-caduxo-lot-movement-ledger`) settled the lot movement ledger
with a complete archive folder.

### Project config (`openspec/config.yaml`)

| Knob                          | Value     |
|-------------------------------|-----------|
| `project`                     | `Caduxo`  |
| `artifactStore` (project)     | `both`    |
| `sdd.executionMode`           | `interactive` |
| `sdd.reviewBudgetChangedLines`| `800`     |
| `sdd.chainedPrStrategy`       | `auto-forecast` |
| `sdd.strictTdd`               | `false`   |
| `product.priorityPlatforms`   | `windows`, `linux` |
| `product.nonGoals`            | POS, billing, payments, accounting, full inventory |
| `product.summary`             | Local-first desktop application for product expiry tracking only. |

Session preflight overrides (current change):

- `executionMode`: `interactive`. This phase completes only `explore`; the
  proposal phase must wait for explicit user approval.
- `artifactStore`: `both`. Persist the exploration to both
  `openspec/changes/caduxo-i18n-support/` and the Engram topic key
  `sdd/caduxo-i18n-support/explore`.
- `deliveryStrategy`: `auto-chain` (no chain decision needed yet — only the
  `explore` phase is in scope).
- `reviewBudgetChangedLines`: `3000` (session override; canonical is 800). Any
  forecast above 3000 changed lines MUST trigger the
  `deliveryStrategy: ask-on-risk` decision round before `tasks.md`.
- `strictTdd`: `false`. Behavioural tests in Rust are still useful where they
  exist today; no new test harness will be added for the frontend.

### Frontend / backend split

- Frontend: Svelte 5 + Vite + TypeScript, mounted via Tauri 2
  (`src/main.ts`, `src/App.svelte`). No i18n library is installed; the
  candidate `typesafe-i18n` is the user's selected library preference.
- Backend: Rust + Tauri in `src-tauri/` (SQLite, command/handler layer,
  services + repositories). `printpdf` 0.7 powers the report PDF. There is no
  `i18n` Rust crate; the only user-facing backend text is in
  `src-tauri/src/pdf/report_pdf.rs`, `src-tauri/src/dto/reports.rs`, and a few
  `DomainError::Validation` strings.

### Canonical spec surface

`openspec/specs/caduxo-expiry-tracker/spec.md` is the only domain. Capabilities
this change will most likely touch (to be confirmed by the proposal):

| Capability              | Why this change cares |
|-------------------------|-----------------------|
| `Application settings`  | Adds the persisted `language` preference, alongside `last_selected_store_id` and `require_initial_location_on_lot_create`. |
| `Reports`               | PDF header/footer/column text and `ReportMetadata.description` must be locale-aware. |
| `Local notifications`   | OS notification title/body must be locale-aware. |
| `Dashboard`, `Stores & locations`, `Products`, `Calendar`, `Backup & restore`, `CSV import`, `Expiry lots`, `Lot movements`, `Unit definitions`, `Categories` | Navigation tabs, page titles, button labels, placeholders, and modal helpers all become locale-aware. |
| `Engineering safety`    | PDF rendering and notification flow must not regress logging / PII rules (do not log translated strings into tracing). |

No current requirement covers i18n. The proposal will likely add a new
capability — **`## Capability: Internationalisation`** — and add or modify
requirements under `Application settings` and `Reports`.

---

## 2. User-confirmed scope

These decisions are explicit user input for this change and are not up for
re-decision in the proposal phase unless the user revisits them.

| Area                    | Decision |
|-------------------------|----------|
| Trigger                 | Start after screens/texts stabilized (user-confirmed now; the major screens are stable). |
| Supported locales       | English (`en`) and Spanish (`es`). |
| Fallback                | English when a key is missing or detection fails. |
| Auto-detect             | Detect the user's OS/browser locale where possible. |
| Persist manual choice   | Yes — the manual choice must override auto-detection on next launch. |
| Manual selector         | Add a locale selector in the Configuration page. |
| Scope of translation    | Visible UI + backend/Tauri user-facing messages. |
| Frontend library        | `typesafe-i18n` (user-selected). |
| PDF report translation  | In scope for this change. |
| Notification copy       | In scope (already implied by "backend/Tauri user-facing messages"). |
| Non-Spanish/English     | Out of scope for v1 — no third locale. |

---

## 3. Current string inventory (where translation must land)

This is the surface that the proposal/spec must cover. Counts are approximate
because some files mix literal text and `$t`/placeholder text already.

#### Frontend (`src/`)

- `src/App.svelte` — 8 navigation button labels + `aria-label="Main
  navigation"` (the `Configuración` button is the only existing Spanish
  string).
- `src/components/ConfigurationPage.svelte` — page title (`Configuración`),
  section title (`Lotes`), setting label + description
  (`Ubicación inicial obligatoria al crear lote`, helper text),
  `Cargando…`, `Guardando…`, `No se pudieron cargar los ajustes`,
  `Error al guardar el ajuste`, `aria-label`. **This is the home of the new
  language selector.**
- `src/components/DashboardPage.svelte` — `PRESET_LABELS` (`All`, `Expired`,
  `Today`, `Alert window`, `Next 7 days`, `Next 30 days`), error message
  copy, banner messages.
- `src/components/ReportsPage.svelte` — `REPORT_TYPES` (`In alert window`,
  `Expired`, `Next 30 days`, `Custom`) + per-type descriptions, urgency
  options (`All urgencies`, `Expired`, `Today`, `Alert window`, `Next 30
  days`, `Future`), the "Exported X rows across Y pages" success message,
  error messages from `humanizeError`.
- `src/components/ScanSearchBox.svelte` — default placeholder
  (`Scan barcode or type SKU…`), error messages.
- `src/components/ProductForm.svelte`, `ProductCatalogPage.svelte`,
  `StoresPage.svelte`, `CalendarPage.svelte`, `BackupRestorePage.svelte`,
  `CsvImportPage.svelte`, `UnitReviewPage.svelte`, `LotMovementsPanel.svelte`
  — page titles, button labels, modal helpers, placeholders.
- `src/components/inputs/*.svelte` — `CategoryPicker.svelte` has
  `Search categories…`; other inputs have similar placeholders.
- `src/components/UnitReviewBanner.svelte` — banner copy with plural forms
  (`product has` / `products have`, `Review`, `Dismiss`).
- `src/lib/notifications.ts` — `formatNotificationBody` produces
  `${qty} expires ${expiry_date}${location}`; `sendNotification` title is
  `⚠️ Expiry alert: ${lot.sku}`. **Backend messages (alert windows, errors)
  also pass through the title/body that the OS shows.**

#### Backend (`src-tauri/src/`)

- `src-tauri/src/pdf/report_pdf.rs` — PDF document title (`"Caduxo Report"`),
  header (`"Generated: {}  ·  Rows: {}"`, `"Filters: {}"`), footer
  (`"Page {} of {}"`, `"Caduxo · Expiry Tracker"`), empty notice
  (`"No rows match the current report filters."`), all column headers
  (`SKU`, `Description`, `Store / Location`, `Qty`, `Expiry`, `Days`,
  `Alert`, `Batch`), and `"{} ago"` for negative day counts. The PDF
  metadata `name` (`"Caduxo Report"`) is also user-visible in PDF reader
  title bars.
- `src-tauri/src/dto/reports.rs` — `ReportType::description()` returns
  English (`"Lots in their alert window"`, `"Expired lots"`,
  `"Lots expiring in the next 30 days"`, `"Custom filtered report"`). This
  string flows into both `ReportMetadata.description` (used by the preview
  UI and the PDF) and the frontend `lib/reports.ts` DTO.
- `src-tauri/src/pdf/report_pdf.rs::format_filters` — English-style filter
  prefixes (`store=`, `location=`, `category=`, `categories (N)`,
  `urgency=`, `from=`, `to=`).
- `src-tauri/src/services/notifications.rs::parse_strict_date` —
  `"Invalid {label} format: \`{value}\` (expected YYYY-MM-DD)"` (English).
- Other `DomainError::Validation` / `Internal` messages — currently English;
  the proposal will decide whether to translate them or keep them
  developer-facing only.

The user's selected scope **does not** require translating product/owner
catalog content (SKUs, descriptions, barcodes, notes). Those are user data.

---

## 4. Implementation axes and the available options

For each axis below, the proposal will pick one path; this section captures
the options and the recommendation so the proposal can stay focused.

### 4.1 Frontend i18n runtime

**Candidate library:** `typesafe-i18n` (user-selected).

- Generates a strongly-typed translation function from `en.json` (and other
  locales) so missing keys break the TypeScript build.
- Supports a Svelte adapter (`typesafe-i18n` provides `addBaseTranslations`,
  `setLocale`, `LL` for `$LL.foo.bar()` in templates).
- Bundle size: the generated `i18n.ts` only includes the active locale's
  tree.
- Works without a global store; `setLocale()` triggers reactivity via the
  adapter.
- Mature, zero-runtime-dependency, supports ICU plural rules.

**Other options (not selected):**

- `svelte-i18n` — simpler but less type-safe; a proposal with this choice
  would need to justify why typesafety was rejected.
- `@tannin/svelte-i18n` / `i18next` — heavier; less idiomatic for Svelte 5
  runes.
- Hand-rolled map — fragile; the user's choice already settled this.

**Recommendation:** adopt `typesafe-i18n` as the user requested. Add a
`src/i18n/` directory with `en/index.ts` and `es/index.ts`, the generated
`i18n.ts` / `i18n-util.ts`, and a Svelte 5 reactive `locale` store that calls
`setLocale()` and persists the choice via `updateSettings()`.

### 4.2 Locale detection

Tauri 2 exposes the host OS locale through `@tauri-apps/api/os` (`locale()`
returns the OS-preferred language tag). The WebView also exposes
`navigator.language`. Both are reasonable signals.

**Options:**

- (a) Tauri `os.locale()` — most accurate on desktop (matches the actual OS
  preference) and survives a missing/broken WebView locale. Recommended.
- (b) `navigator.language` — works even in `vite dev` without Tauri.
- (c) Both, with Tauri taking precedence on desktop.

**Recommendation:** option (c). In the production app, prefer
`os.locale()` and fall back to `navigator.language` (useful during
`vite dev` without the Tauri runtime). Normalise the tag (split on `-`,
take the lower-cased primary subtag) and map to the supported set.

### 4.3 Persistence

The `app_settings` key-value table already exists
(`src-tauri/src/db/repositories/settings.rs`). The current DTO is:

```rust
pub struct SettingsResponse {
    pub last_selected_store_id: Option<String>,
    pub require_initial_location_on_lot_create: bool,
}
```

**Options:**

- (a) Add a third field `language: String` to `SettingsResponse` /
  `SettingsUpdate` and persist it via the existing key-value repository.
  Cleanest fit with the current model. Default = `"en"`. Existing rows
  migrate by treating `None` as `"en"`.
- (b) Persist via a dedicated Tauri store (`@tauri-apps/plugin-store`)
  outside SQLite. Adds a new dependency and bypasses the existing settings
  table for one field. Rejected — current settings already live in SQLite
  and the user explicitly asked to persist "in Settings backend".

**Recommendation:** option (a). The `SettingsUpdate` already accepts
`Option<…>` fields, so partial updates (only `language`) work without
changes to other IPC commands. A new repository helper
`get_language_setting` / `set_language_setting` keeps the pattern consistent
with `get_last_selected_store_id`.

### 4.4 Frontend locale bootstrap

On app start, the Configuration page and other surfaces need to know the
locale before the first render of any text. Two strategies:

- (a) Synchronous `localStorage` cache + async backend hydration. The
  backend `/settings/language` call returns the persisted value; the frontend
  caches it in a small Svelte store so the first paint already uses the
  right locale.
- (b) Wait for `getSettings()` before the first paint of text. Simpler but
  produces a flash of fallback content.

**Recommendation:** option (a). Boot order: read OS/WebView locale → match
against supported set → check `localStorage` cache → kick off
`getSettings()` async → persist to DB if missing. This gives an
instant-correct first paint and a guaranteed-correct second paint.

### 4.5 PDF report translation

The PDF renderer is in Rust. Two viable strategies:

- (a) **Backend-side translation.** The frontend passes the desired locale
  with `export_report_pdf(request, file_path, locale)`. The Rust side holds
  a small translation table (no ICU dependency) for the PDF-only strings
  (header, footer, empty notice, column headers, "X ago", filter prefixes)
  and the `ReportType::description()` function becomes a function of locale.
- (b) **Frontend pre-translates metadata.** The preview UI already gets
  `ReportMetadata.description` in English; the proposal could have the
  frontend substitute the localised string before rendering. This works for
  the preview but **not** for the PDF, because `render_report` writes the
  PDF directly in Rust. The PDF must be rendered in the chosen locale in
  Rust.

**Recommendation:** option (a). Pass `locale` (or a `&str` BCP-47 tag) into
`preview_report` and `export_report_pdf`. Add a small `pdf/locale.rs`
module with a `fn pdf_messages(locale: Locale) -> PdfMessages` returning the
strings. Run a Rust unit test for both locales.

### 4.6 Notification translation

`formatNotificationBody` and the title are in `src/lib/notifications.ts`.
**Recommendation:** translate on the frontend with `typesafe-i18n`. The
backend stays locale-agnostic — `lib/notifications.ts` reads the active
locale from the Svelte store before composing title/body. This keeps the
backend DTOs free of locale concerns.

### 4.7 Date/number formatting

The report uses `DD/MM/YYYY` today and a `qty unit` shape. Two options:

- (a) Translate the `Expiry` column header to `Fecha de caducidad` /
    `Expiry` and leave the `DD/MM/YYYY` format unchanged. The "DD/MM/YYYY"
    format is more readable in Spanish than the US-style `MM/DD/YYYY`.
- (b) Switch to ISO `YYYY-MM-DD` everywhere for the report. Loses visual
    brevity in English.

**Recommendation:** keep the current `DD/MM/YYYY` format. The
`format_date_dd_mm_yyyy` helper in `report_pdf.rs` is independent of locale;
this is a deliberate non-decision. Numbers (qty) stay unformatted; the
unit string is data, not translation.

---

## 5. Product problem in plain language

Small business owners running Caduxo in Spanish-speaking regions currently
get a UI that is mostly English, with a handful of stray Spanish strings
(`Configuración`, `Ubicación inicial obligatoria al crear lote`). They also
get English-only OS notifications, an English-only PDF report (header,
footer, column labels), and English-only error messages. This is a quality
and trust problem, not a feature gap — the application's logic is correct,
but its communication layer is monolingual.

Adding English/Spanish support solves this without expanding the product's
scope:

- The user keeps the local-first, single-operator workflow.
- The data model stays the same (catalog, lots, stores, locations,
  movements, settings) — no new entities.
- The PDF report layout stays the same; only the text strings change.
- Notifications stay local OS notifications; only their copy changes.
- Settings get one new persisted field (`language`), with the same
  optimistic-update UI pattern the Configuration page already uses.

The change also sets the project up for additional locales later: the
`typesafe-i18n` file-per-locale layout and the locale-tagged PDF path both
scale to additional languages without architectural rework.

---

## 6. User-facing workflows (target, not proposal)

These are the workflows the proposal/spec must support; each is a thin
description of behaviour, not a UI design.

1. **First-run language pick.** On first launch, the app detects an
   OS/browser locale, normalises it, and matches it against `{en, es}`.
   If unsupported, defaults to `en`. The Configuration page shows the
   detected value in the manual picker.
2. **Manual change.** The user opens Configuration, picks `English` or
   `Español`, and the UI re-renders immediately. The change is persisted
   via `updateSettings({ language })` (optimistic UI with rollback on
   failure, matching the existing toggle pattern).
3. **Restart retains choice.** On the next launch, the bootstrap reads the
   persisted language first and uses it before any backend call resolves.
4. **PDF in chosen language.** When the user generates a PDF, the PDF
   header, footer, column labels, empty notice, filter prefixes, and
   `ReportMetadata.description` all render in the chosen language.
5. **Notifications in chosen language.** The OS notification title/body
   are composed in the chosen language at the moment the periodic check
   runs.

---

## 7. Recommended MVP boundary

### In scope

- Two locales: `en`, `es`. English fallback for unsupported tags.
- `typesafe-i18n` Svelte 5 integration with `setLocale` and a reactive store.
- OS/WebView locale detection at startup, with persisted preference taking
  precedence after the first run.
- Manual locale selector in the Configuration page with optimistic update
  (re-using the existing `getSettings` / `updateSettings` pattern).
- Persisted `language` field in `SettingsResponse` / `SettingsUpdate` and a
  matching `app_settings` row (no separate migration beyond treating
  missing value as `"en"`).
- Locale-aware PDF report: title, header, footer, column headers, empty
  notice, "X ago", filter prefixes, and `ReportType::description()`.
- Locale-aware frontend notification copy (formatted in TS).
- Locale-aware visible UI across all Svelte components in `src/components/`
  and `src/App.svelte`, including the mixed-language `Configuración`
  toggle.
- Locale-aware error messages returned by `parse_strict_date` and a small
  handful of user-visible `DomainError::Validation` strings surfaced in the
  UI (others stay developer-only English).

### Out of scope

- Locales other than `en` and `es`.
- RTL support (Hebrew, Arabic).
- Locale-aware number/currency formatting — Caduxo does not deal in money
  (per product non-goals).
- Translation of user-entered data (product names, descriptions, batch
  codes, notes).
- Translation of PDF document properties beyond the visible title.
- Switching `DD/MM/YYYY` to a locale-aware date format in this change.
- A language picker outside the Configuration page (no in-app banner /
  first-run dialog for v1).
- Per-store / per-user locale overrides.
- Localising the user manual / docs (none shipped today).

---

## 8. Key risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Many small strings across many Svelte files lead to a large diff | Could exceed the 3000-line session budget and require a chained PR | Group work into a single translation pass per surface; use typesafe-i18n generation so missing keys fail the build rather than landing as English fallbacks |
| Locale detection is unreliable on Windows in some Tauri builds | First-run language may not match user expectations | Detection is the default; the user can always pick manually in Configuration |
| `typesafe-i18n` generated types may clash with Svelte 5 runes | Build / type errors | Generate types into a dedicated `src/i18n/` dir, re-export a typed `LL` helper, document the import path in the spec |
| PDF strings need a Rust-side translation table that has to stay in sync with the frontend `typesafe-i18n` keys | Two sources of truth, drift | Keep the Rust table scoped to PDF/header/footer/column/empty/filter keys only; document the keys; add a Rust unit test that asserts both locales produce the same set of keys |
| `ReportMetadata.description` is now locale-dependent; the frontend preview UI relies on it | Frontend must rebuild the description on locale change if it caches it | Keep the description as a string in the DTO; the frontend can re-render with the localised label via a small lookup table without changing the DTO shape |
| Persisting `language` requires a new `SettingsResponse` field — small breaking change for any external IPC consumer | None today (single frontend) | Add as `Option<String>` / default `"en"` so missing values are tolerated; document in the proposal |
| Tauri `os.locale()` is unavailable during `vite dev` without the Tauri runtime | Dev-time locale detection fails | Fall back to `navigator.language`; the dev experience matches the production runtime closely enough |
| Notification title/body contain the SKU, which is user data and stays untranslated — half-translated copy looks wrong | Cosmetic | Keep SKU untranslated (it's a code); the surrounding template is translated |
| Changing the Configuration page wording could regress the existing "Ubicación inicial obligatoria" toggle UI | UX regression on a stable page | Re-test the toggle after re-keying; preserve the existing copy semantics |
| Mixed-language configuration page (`Configuración` is already Spanish, other strings English) becomes inconsistent after translation | Visible regression | Pick a single canonical locale for v1 of the Configuration page; the selector applies to the whole page including the toggle |
| `app_settings` migration for the new `language` row | Older databases don't have the row | Treat missing `language` value as `"en"`; no schema migration is required because the table already exists |
| `printpdf` 0.7 built-in font (Helvetica) has limited Latin Extended coverage; Spanish accents/ñ may not render | Visible garbled text | Spanish uses Latin-1 Supplement (á, é, í, ó, ú, ñ, ü) which Helvetica 0.7 ships with; spot-check in the proposal with a sample PDF. If coverage is incomplete, fall back to a custom TTF (out of scope unless trivial) |

---

## 9. Product assumptions

- The user wants English/Spanish only for v1; other locales are explicitly
  out of scope.
- "After screens/texts stabilized" means today's wording is the source of
  truth — translations are derived from the current English strings (and
  from the current Spanish strings that are already mixed in, e.g.
  `Configuración`).
- The Configuration page's optimistic-update pattern is the right pattern
  to copy for the language selector.
- The settings backend (`app_settings` key-value table) is the right place
  to persist the preference; a separate plugin-store would be inconsistent.
- The PDF renderer stays in Rust; locale passes through the IPC contract.
- Caduxo's PDF is an operational document, not a branded artifact; column
  widths and pagination stay fixed.
- The user accepts that the rest of the app's UI text (English today) is
  already the canonical source for `en` translations.

---

## 10. Open questions for proposal

1. Should the language selector be a dropdown (`<select>`) or a radio
   group? The Configuration page currently uses a toggle; consistency may
   favor a radio group, but a dropdown scales to more locales later.
2. Should `os.locale()` map to the closest supported locale (e.g. `es-MX` →
   `es`) or only match the exact tag? The product assumption is "closest
   supported" — confirm.
3. Should `os.locale()` be called on every app start (cheap IPC roundtrip)
   or cached in `localStorage`? The proposal should pick one.
4. Should the language selector show "Detected: Español" hint when the
   current value comes from auto-detection, or is the active value
   sufficient? Cosmetic.
5. For the PDF, should `report_type` filter labels
   (`store=`/`location=`/`category=`) be translated (`tienda=`/`ubicación=`)
   or stay stable English keys for parsing friendliness? Stable English
   keys is the safer default.
6. Should the OS notification body include the SKU unconditionally, or
   should the SKU be omitted in the user's selected locale to keep the
   body fully localised? Trade-off between operational usefulness (SKU
   identifies the lot) and copy consistency.

These are the only product-level open questions for the proposal phase.
Implementation-level questions (e.g. Svelte 5 reactive binding of
`setLocale`, `typesafe-i18n` plugin pipeline, Rust unit test placement)
belong in the design phase.

---

## 11. Recommendation

Proceed to `proposal`. The user-selected scope is unambiguous, the current
string inventory is bounded, and `typesafe-i18n` plus the existing
`app_settings` table cover the persistence and runtime sides cleanly. The
PDF report translation is the only non-trivial engineering axis (Rust-side
locale table + new `locale` parameter on the IPC commands), and even that
is a small, localised change.

Before writing the proposal, the proposal-phase agent should:

1. Run a short product question round focused on the open questions in
   §10 (selector widget, locale-mapping rule, PDF filter key localisation,
   notification SKU inclusion).
2. Confirm the `typesafe-i18n` Svelte 5 wiring shape (reactive `$locale`
   store, `$LL` helper usage in templates) so the spec can name a single
   binding pattern.
3. Confirm the IPC contract delta: `language` field on
   `SettingsResponse` / `SettingsUpdate` and a new `locale: &str` parameter
   on `preview_report` and `export_report_pdf`.

Phase gate: complete only the `explore` phase this turn. Do not begin the
`proposal` phase unless the user explicitly approves it.