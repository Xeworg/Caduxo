# Proposal — caduxo-i18n-support

## Change metadata

- **Change ID**: `caduxo-i18n-support`
- **Domain**: `caduxo-expiry-tracker` (single-domain project)
- **Artifact store**: `both` (per session preflight; persisted to OpenSpec and
  Engram topic key `sdd/caduxo-i18n-support/proposal`)
- **Review budget**: 3000 changed lines (session override; canonical 800)
- **Delivery strategy**: `auto-chain` (deferred until chaining is selected);
  any phase whose forecast exceeds 3000 lines MUST trigger
  `deliveryStrategy: ask-on-risk` before `tasks.md`.
- **Chain strategy**: `deferred` — no chained split planned at proposal time
- **Strict TDD**: `false` (project default); UI changes rely on manual smoke
- **Execution mode**: `interactive` — this phase completes only `proposal`;
  `spec`, `design`, and `tasks` must wait for explicit user approval

## Problem statement

Caduxo today ships a UI that is mostly English, with two stray Spanish strings
(`Configuración` navigation button and `Ubicación inicial obligatoria al
crear lote` toggle in the existing Configuration page). The remainder of the
user-facing surface is monolingual:

- All Svelte components render English-only navigation tabs, page titles,
  button labels, placeholders, banner copy, modal helpers, and error
  messages.
- The Rust PDF report renderer (`src-tauri/src/pdf/report_pdf.rs`) writes
  English-only title, header, footer, column headers, empty-state notice,
  `"{n} ago"` relative-time label, and filter prefixes (`store=`,
  `location=`, `category=`, `categories (N)`, `urgency=`, `from=`, `to=`).
- `ReportType::description()` in `src-tauri/src/dto/reports.rs` returns
  English strings that flow into both the preview UI and the PDF.
- OS notifications (`src/lib/notifications.ts`) compose English title
  (`⚠️ Expiry alert: ${sku}`) and body (`{qty} expires {date}{location}`)
  with no locale awareness.
- IPC error messages from the backend (e.g.
  `Invalid notification_date format: \`{value}\` (expected YYYY-MM-DD)` from
  `parse_strict_date`) are English-only and surfaced verbatim to the UI.

This is a quality and trust problem for Spanish-speaking small-business
users, not a missing-feature problem: the application's data model and
workflow are correct, but the communication layer speaks a single language
while the user's day-to-day vocabulary is Spanish.

## Outcomes (success criteria)

After this change ships, a small-business user running Caduxo in either
English or Spanish can:

1. **Get the right language on first launch.** When the host OS or browser
   reports `es*` or `en*` as the preferred locale, the app renders in that
   language immediately. Any other tag (or detection failure) falls back to
   English.
2. **Override auto-detection manually.** A locale selector on the
   Configuration page offers `English` and `Español` via a dropdown; the
   chosen value is persisted via the existing settings backend and
   overrides auto-detection on every subsequent launch.
3. **See the detected hint.** When the active locale was inherited from
   OS/WebView detection (no manual choice yet), the selector displays a
   "Detectado: Español" / "Detected: English" hint above the dropdown so
   the user knows where the active value came from.
4. **Restart without re-selecting.** Closing and reopening the app, or
   rebooting the host, retains the manual choice across runs.
5. **Read every visible UI string in the chosen language.** Navigation
   tabs, page titles, button labels, placeholders, banner copy, modal
   helpers, success and error messages, and the existing
   `Configuración` / `Ubicación inicial obligatoria al crear lote` strings
   are all locale-aware. No English residue remains in the supported set.
6. **Get the PDF report in the chosen language.** Title, header, footer,
   column headers (`SKU`, `Description`, `Store / Location`, `Qty`,
   `Expiry`, `Days`, `Alert`, `Batch`), empty-state notice, `"{n} ago"`
   label, filter prefixes (`tienda=`, `ubicación=`, `categoría=`,
   `categorías (N)`, `urgencia=`, `desde=`, `hasta=`), and
   `ReportMetadata.description` render in Spanish or English end-to-end.
7. **Get OS notifications in the chosen language** with the SKU kept
   visible in both locales so the lot can still be identified operationally.
8. **Trust the type system.** The Svelte build fails when a translation
   key is added to `en.json` but not to `es.json` (and vice versa), so the
   two locales stay in lockstep.

## Scope (in scope)

- Two locales: `en` (English) and `es` (Spanish).
- A `typesafe-i18n` integration for the Svelte 5 frontend, with a reactive
  `locale` rune/store that calls `setLocale()` and re-renders templates.
- Closest-supported mapping of detected locales (e.g. `es-MX` → `es`,
  `en-GB` → `en`); unsupported tags fall back to English.
- OS / WebView locale detection at app start, with the persisted manual
  choice taking precedence after the first run.
- Persistence of the locale preference in the existing `app_settings`
  key-value table via a new `language` field on `SettingsResponse` and
  `SettingsUpdate` (default `"en"`, missing row treated as `"en"`).
- A `<select>`-based locale selector rendered inside the existing
  Configuration page, using the optimistic-update pattern already used by
  the toggle.
- A "Detectado: Español" / "Detected: English" hint rendered above the
  selector when the active value was inherited from auto-detection and
  has not been manually overridden.
- Locale-aware translation of every visible UI string in
  `src/App.svelte`, `src/components/**/*.svelte`, and the existing
  `Configuración` / `Ubicación inicial obligatoria al crear lote` copy.
- Locale-aware translation of OS notification title/body via
  `typesafe-i18n` (`formatNotificationBody`, `sendNotification`). The SKU
  remains visible in the notification body across both locales.
- Locale-aware translation of the PDF report: title, header, footer,
  column headers, empty notice, `"{n} ago"`, filter prefixes, and
  `ReportType::description()`. The locale tag is passed as a new
  parameter on `preview_report` and `export_report_pdf`.
- Locale-aware translation of user-visible backend error strings surfaced
  via IPC, scoped to the `parse_strict_date` validation messages and the
  handful of `DomainError::Validation` strings that the UI displays
  verbatim. Developer-only errors stay in English.
- A Rust-side `pdf/locale.rs` module exposing `fn pdf_messages(locale) ->
  PdfMessages`, with a unit test that asserts the same set of keys is
  populated for both `en` and `es`.

## Non-goals (reaffirmed and tightened)

- **Locales other than `en` and `es`.** Out of scope for v1. The
  `typesafe-i18n` file-per-locale layout and the locale-tagged PDF path
  both scale to additional languages without architectural rework, but no
  third locale is shipped.
- **RTL support.** Hebrew, Arabic, and other RTL scripts remain out of
  scope; the Svelte CSS layout assumes LTR.
- **Locale-aware number / currency formatting.** Caduxo does not deal in
  money (per the project `nonGoals` in `openspec/config.yaml`). Numbers
  in the report stay unformatted.
- **Locale-aware date format in the PDF.** The current `DD/MM/YYYY`
  format is preserved. Switching to ISO `YYYY-MM-DD` is a separate
  decision and is not part of this change.
- **Translation of user data.** Product names, descriptions, barcodes,
  batch codes, notes, owner names, and store/location names are user
  data and are NOT translated.
- **Translation of PDF document properties** beyond the visible title
  (author, subject, keywords stay English).
- **A first-run language picker dialog or in-app banner.** v1 surfaces
  the locale solely via the Configuration page selector.
- **Per-store / per-user locale overrides.** One locale per app
  installation.
- **Localising the user manual or docs.** No user manual ships today.
- **A new Tauri plugin-store for the locale.** The locale persists
  alongside the other settings in the existing SQLite `app_settings`
  table.
- **Currency, POS, billing, accounting, full inventory** — all remain
  project non-goals per `openspec/config.yaml`.

## Confirmed product decisions (locked)

Recapping the user-confirmed scope and the proposal question round
answers; these are committed by the proposal and are not open for
re-litigation in later phases unless the user revisits them.

| # | Decision | Source |
|---|----------|--------|
| 1 | Trigger: start now that screens/texts are stable | User scope |
| 2 | Supported locales: English (`en`) + Spanish (`es`) | User scope |
| 3 | English fallback when a key is missing or detection fails | User scope |
| 4 | Auto-detect from OS/browser locale where possible | User scope |
| 5 | Persist the manual choice so it overrides auto-detection | User scope |
| 6 | Manual selector lives on the existing Configuration page | User scope |
| 7 | Scope of translation: visible UI + backend/Tauri user-facing messages | User scope |
| 8 | Frontend library: `typesafe-i18n` | User scope |
| 9 | PDF report translation is IN scope for v1 | User scope |
| 10 | OS notification translation is IN scope | User scope (implied by 7) |
| 11 | Selector widget: **Dropdown** (`<select>`) | Proposal question round |
| 12 | Show "Detectado: Español" / "Detected: English" hint above the selector when the active value came from auto-detection and no manual choice is on file | Proposal question round |
| 13 | PDF filter keys translate into Spanish (`tienda=`, `ubicación=`, `categoría=`, `categorías (N)`, `urgencia=`, `desde=`, `hasta=`) | Proposal question round |
| 14 | OS notification body keeps the SKU visible in both locales | Proposal question round |
| 15 | Closest-supported mapping for detected tags (`es-MX` → `es`, `en-GB` → `en`); unsupported tags fall back to English | Explore recommendation, adopted |
| 16 | OS locale is re-read on every app start; no `localStorage` cache of the detected tag (the persisted `language` row is the canonical cache) | Explore recommendation, adopted |
| 17 | Backend `DomainError::Validation` strings surfaced to the UI are translated; developer-only errors stay English | Explore recommendation, adopted |

## Proposed capabilities

The proposal adds **one new capability** and modifies **two existing
capabilities** in `openspec/specs/caduxo-expiry-tracker/spec.md`. The
spec phase will translate the proposed requirements below into the
canonical `#### Scenario:` shape.

### New capability: *Internationalisation*

- The Svelte 5 frontend renders all user-visible strings through a
  `typesafe-i18n` reactive binding (`$LL.foo.bar()` or the equivalent
  `$t` template form). The translation function is typed; missing keys
  fail the TypeScript build.
- The app supports two locales, `en` and `es`. English is the fallback
  when a key is absent in any other locale.
- The active locale is held in a Svelte 5 reactive store (`locale` rune)
  that calls `typesafe-i18n`'s `setLocale()` on change.
- At app start, the bootstrap reads the host locale from
  `@tauri-apps/api/os::locale()` (production) or `navigator.language`
  (development fallback), normalises the tag to its lower-cased primary
  subtag, and maps to the closest supported locale (`es*` → `es`,
  `en*` → `en`, anything else → `en`).
- The persisted `app_settings.language` value is consulted on startup and
  takes precedence over detection after the first run.
- The Configuration page renders a locale selector (dropdown) listing
  `English` and `Español`, with a "Detectado: ..." / "Detected: ..."
  hint above it when the active value came from auto-detection and no
  manual choice is persisted yet.
- Selecting a locale triggers an optimistic update via the existing
  `updateSettings({ language })` IPC command, with rollback on failure
  (mirroring the toggle pattern on the same page).
- `typesafe-i18n` keys for Spanish mirror the English key tree exactly.
  Any key added under `en.json` without a matching `es.json` entry is a
  build error.

### Modified capability: *Application settings* (implicit — no formal capability today)

- `SettingsResponse` gains a new field `language: String` (default `"en"`).
- `SettingsUpdate` accepts an optional `language: Option<String>`; a
  partial update of only `language` works without touching
  `last_selected_store_id` or `require_initial_location_on_lot_create`.
- The `app_settings` key-value repository gains helper functions
  `get_language_setting() -> Option<String>` and
  `set_language_setting(value: String)`. Pre-existing rows missing the
  `language` key behave as `Some("en")`.
- The Configuration page fetches the locale via the existing
  `getSettings()` IPC call (no new command) and persists changes via
  the existing `updateSettings()` IPC call.

### Modified capability: *Reports*

- `preview_report` and `export_report_pdf` accept a new `locale: &str`
  parameter (BCP-47 primary subtag or full tag).
- The PDF renders title, header, footer, column headers, empty notice,
  `"{n} ago"`, and filter prefixes using the new `pdf/locale.rs`
  translation table. Column widths and pagination are unchanged.
- `ReportType::description()` becomes a function of locale; the function
  signature changes from `fn description(&self) -> &'static str` to
  `fn description(&self, locale: Locale) -> Cow<'static, str>` (or an
  equivalent newtype result).
- The preview UI receives `ReportMetadata.description` already localised;
  the frontend does not need to substitute it locally.

### Modified capability: *Local notifications*

- `src/lib/notifications.ts::formatNotificationBody` and the
  `sendNotification` title compose their strings through
  `typesafe-i18n`'s active locale.
- The SKU remains visible in the body across both locales because it is
  operational data, not copy. The surrounding template is localised.
- The backend IPC contract for notifications is unchanged; locale
  selection happens entirely on the frontend.

## UX at high level

### Locale selector (on the existing Configuration page)

The Configuration page already hosts `Ubicación inicial obligatoria al
crear lote` (a toggle). This change adds a new section above it:

- A short section title (locale-aware) such as **Idioma** / **Language**.
- A "Detected" hint paragraph when the active value was inherited from
  auto-detection and no manual override exists:
  - `Detectado: Español` (when Spanish was auto-detected)
  - `Detected: English` (when English was auto-detected, including the
    default fallback case)
- A `<select>` dropdown listing the supported locales by their own
  names (`English`, `Español`).
- The currently active value is the selected option. Changing the value
  fires the optimistic update immediately.
- A small inline error label uses the existing "no se pudo guardar /
  could not save" pattern on failure, with the error message translated.

The rest of the Configuration page remains as today; the new section is
added without changing the existing toggle's placement or copy.

### PDF report (in Spanish)

When the active locale is `es`, the PDF renders:

- Title bar: `Reporte Caduxo` / `Caduxo Report` in the document title and
  PDF metadata `Title`.
- Header line: `Generado: {date}  ·  Filas: {n}` /
  `Generated: {date}  ·  Rows: {n}`.
- Filter line: `Filtros: tienda=Bodega · ubicación=Exhibición · categorías
  (3) · urgencia=Vence · desde=2026-10-01 · hasta=2026-10-31` /
  `Filters: store=Bodega · location=Exhibición · categories (3) ·
  urgency=Expiring · from=2026-10-01 · to=2026-10-31`.
- Column headers: `SKU`, `Descripción`, `Tienda / Ubicación`, `Cantidad`,
  `Caducidad`, `Días`, `Alerta`, `Lote`.
- Empty notice: `Ninguna fila coincide con los filtros actuales.`
- Footer: `Página {n} de {m}` / `Page {n} of {m}`.
- Brand line: `Caduxo · Control de caducidades` / `Caduxo · Expiry Tracker`.
- Relative-time formatter: `hace {n} días` / `{n} days ago`.
- `ReportMetadata.description` (used by the preview UI and the PDF
  caption): `Lotes en ventana de alerta` / `Lots in alert window`, etc.

Column widths, the empty-state layout, and pagination are unchanged from
today.

### OS notification (in Spanish)

When the active locale is `es`:

- Title: `⚠️ Alerta de caducidad: {sku}` (the `⚠️` glyph is preserved;
  the SKU is unchanged because it is operational data).
- Body: `{qty} caduca el {expiry_date}{location}` /
  `{qty} expires on {expiry_date}{location}`.

The `{qty}`, `{expiry_date}`, and `{location}` placeholders are
substituted with raw data exactly as today.

## Architecture sketch

### Frontend (`src/`)

- `src/i18n/en/index.ts` — generated `typesafe-i18n` translations for
  English, derived from the current English strings in `src/App.svelte`,
  `src/components/**/*.svelte`, `src/lib/notifications.ts`, and the
  Configuration page's mixed-language content.
- `src/i18n/es/index.ts` — the same key tree, translated to Spanish.
- `src/i18n/i18n-util.ts`, `src/i18n/i18n.ts` — generated runtime and
  type definitions emitted by `typesafe-i18n`'s generator.
- `src/i18n/locale.ts` — a small Svelte 5 module exposing:
  - `initLocale()` — reads OS/WebView locale, consults persisted
    preference, sets the active locale, hydrates `localStorage` cache.
  - `setLocale(locale: SupportedLocale)` — calls `typesafe-i18n`'s
    `setLocale()`, persists via `updateSettings({ language })` if
    available, updates the reactive rune.
  - A reactive `$locale` rune for templates that need to branch on
    the active locale (rare).
- `src/i18n/detect.ts` — pure helper that maps a `navigator.language`
  or `@tauri-apps/api/os::locale()` tag to the closest supported locale.
- `src/components/ConfigurationPage.svelte` — gains the new locale
  selector section, the "Detectado" hint, and uses `$LL.configuration.*`
  for the existing section / toggle copy.

### Backend (`src-tauri/src/`)

- `src-tauri/src/pdf/locale.rs` (new module) — exposes
  `fn pdf_messages(locale: Locale) -> PdfMessages` returning the PDF-only
  strings (header, footer, empty notice, column headers, `"{} ago"`,
  filter prefixes, title, brand line). A small `enum Locale { En, Es }`
  wraps parsing of the `&str` parameter.
- `src-tauri/src/pdf/report_pdf.rs` — accepts `locale: Locale`, looks up
  strings via `pdf_messages(locale)`. No ICU dependency.
- `src-tauri/src/dto/reports.rs` — `ReportType::description(locale:
  Locale) -> Cow<'static, str>` returns the locale-aware description.
- `src-tauri/src/db/repositories/settings.rs` — gains
  `get_language_setting() -> Option<String>` and
  `set_language_setting(value: String)`. `SettingsResponse` /
  `SettingsUpdate` gain the `language` field.
- `src-tauri/src/services/notifications.rs` — the user-visible
  `parse_strict_date` validation message and a small handful of
  `DomainError::Validation` strings surfaced to the UI are routed
  through a tiny `errors::user_message(kind, locale)` lookup. Developer
  errors stay English.

### Rust unit test

`src-tauri/src/pdf/locale.rs` carries a `#[cfg(test)] mod tests` that
asserts both `En` and `Es` return the same set of keys populated
(non-empty strings) for a fixed list of `PdfMessageKey` variants. This is
the single source of truth that the two locales stay in sync for the
PDF surface.

## Data migration / business policy

### `app_settings` migration

- The `app_settings` key-value table already exists. The new `language`
  row is written the first time the user picks a locale, or by the
  bootstrap if detection picks a supported non-English tag.
- Older databases missing the row behave as if `language = "en"`. No
  schema migration is required.
- The default for `language` is `"en"` everywhere the field is read;
  `None` (legacy) and `""` (defensive) both map to `"en"`.

### Locale detection policy

- Re-read on every app start. The persisted `language` row is the
  canonical cache; `localStorage` mirrors it for first-paint speed only.
- Detection is best-effort: any failure (Tauri API unavailable during
  `vite dev`, tag absent, browser privacy mode) falls back to English.
- Closest-supported mapping: `es*` → `es`, `en*` → `en`, anything else
  → `en`. No fuzzy distance matching; only the primary subtag is
  consulted.

### Translation key policy

- Every key in `en.json` has a counterpart in `es.json` and vice versa.
  `typesafe-i18n` enforces this at build time.
- Keys are namespaced by surface (e.g. `nav.dashboard`,
  `configuration.language.label`, `notification.title`,
  `pdf.column.sku`) so future locales do not collide.

## Risks and mitigations

1. **Many small strings across many Svelte files lead to a large diff.**
   The review budget is 3000 lines (session override; canonical 800).
   Mitigation: the spec phase commits to a single translation pass per
   surface, typesafe-i18n's generation step is the gate, and the
   `deliveryStrategy: ask-on-risk` decision round pauses before
   `tasks.md` if the forecast exceeds the budget.

2. **Locale detection is unreliable on Windows in some Tauri builds.**
   First-run language may not match user expectations on hosts with
   non-standard locale tags. Mitigation: detection is the default, the
   user can always override via the Configuration page, and the
   "Detectado: ..." hint surfaces what was detected.

3. **`typesafe-i18n` generated types may clash with Svelte 5 runes.**
   Build / type errors during integration. Mitigation: generate types
   into a dedicated `src/i18n/` directory, re-export a typed `LL`
   helper, document the import path in the spec, and gate the
   `npm run build` step in the verify report.

4. **PDF strings need a Rust-side translation table that has to stay in
   sync with the frontend keys.** Two sources of truth, drift risk.
   Mitigation: keep the Rust table scoped to PDF-only keys (header,
   footer, column, empty, filter, title, brand, `"{n} ago"`,
   `ReportType::description`); document the keys; add the
   `pdf/locale.rs` unit test that asserts both locales populate the same
   set of keys.

5. **`ReportMetadata.description` is now locale-dependent; the frontend
   preview UI relies on it.** The frontend must rebuild the description
   on locale change if it caches it. Mitigation: keep the description as
   a `String` in the DTO (already returned that way), and have the
   preview UI bind directly to the DTO without client-side caching.

6. **Persisting `language` is a small breaking change for any external
   IPC consumer.** None today (single frontend). Mitigation: add the
   field as a defaulted `String` so missing values are tolerated; the
   `language` field is returned on every `getSettings` call but is
   always populated; document the addition in the proposal and spec.

7. **Tauri `os.locale()` is unavailable during `vite dev` without the
   Tauri runtime.** Dev-time locale detection fails. Mitigation: fall
   back to `navigator.language`; the dev experience matches production
   closely enough for translation work.

8. **Notification title/body contain the SKU, which is user data and
   stays untranslated.** Half-translated copy looks wrong cosmetically.
   Mitigation: keep the SKU untranslated (it is an operational code);
   the surrounding template is fully localised; the notification looks
   consistent within its template and the user understands the SKU is
   data.

9. **Changing the Configuration page wording could regress the existing
   `Ubicación inicial obligatoria al crear lote` toggle UI.** UX
   regression on a stable page. Mitigation: re-test the toggle after
   re-keying; preserve the existing copy semantics through the new
   `configuration.locationRequired.*` keys.

10. **Mixed-language Configuration page becomes inconsistent after
    translation.** Today the page title and toggle are Spanish and the
    surrounding strings are English. Mitigation: pick a single canonical
    locale for v1 of the Configuration page (whichever the user has
    selected); the selector applies to the whole page including the
    toggle, so there is no longer a mixed-language default.

11. **`printpdf` 0.7 built-in font (Helvetica) has limited Latin Extended
    coverage; Spanish accents / ñ may not render.** Visible garbled
    text. Mitigation: Spanish uses Latin-1 Supplement (á, é, í, ó, ú, ñ,
    ü) which Helvetica 0.7 ships with; spot-check in the design phase
    with a sample PDF containing every accented character. If coverage
    is incomplete, fall back to a custom TTF (out of scope unless
    trivial).

12. **Concurrent settings updates could clobber an `language` choice.**
    Two windows / fast retries racing on `updateSettings`. Mitigation:
    the existing `SettingsUpdate` semantics are partial-update safe
    (`Option<...>` fields), and the optimistic-update UI already
    retries on failure; the spec does not introduce a new
    concurrency primitive.

13. **PDF filter key parsing friendliness.** Today the filter line uses
    stable English keys (`store=`, `location=`, `category=`, `urgency=`,
    `from=`, `to=`); after translation, a screen-scraping external
    consumer that parsed the English keys will break. Mitigation: this
    is acceptable — the PDF is a user-facing operational document, not
    a machine-readable export; CSV is the contract for machine
    consumption; document the change in the verify report.

## Rollout and phasing

The phases are illustrative; `tasks.md` will forecast each one against
the 3000-line session budget before locking.

- **Phase 0 — Confirm proposal.** Lock this proposal with the user.
  No code. Resolves the proposal question round.
- **Phase 1 — Backend settings + locale plumbing.** `language` field on
  `SettingsResponse` / `SettingsUpdate`; repository helpers; bootstrap
  hooks; no UI yet. Verify: `cargo test --lib` green.
- **Phase 2 — Frontend `typesafe-i18n` integration.** Install the
  package, generate `src/i18n/`, wire the Svelte 5 reactive store,
  translate `src/App.svelte` and one representative page to prove the
  binding pattern. Verify: `npx svelte-check` green, `npm run build`
  green.
- **Phase 3 — Full UI translation pass.** Translate every visible string
  across `src/components/**/*.svelte`; add the Configuration page
  dropdown selector and the "Detectado" hint. Verify: manual smoke on
  Linux + Windows.
- **Phase 4 — PDF + notifications translation.** Backend `pdf/locale.rs`
  + new `locale` parameter on `preview_report` and `export_report_pdf`;
  Rust unit test; `ReportType::description` locale-aware; notification
  copy localised. Verify: `cargo test --lib` green + sample PDF
  inspection in both locales.
- **Phase 5 — Polish.** Spot-check accented characters in the PDF; verify
  no English residue remains in any supported-locale UI surface; backup
  / restore round-trip in both locales.

The canonical verify gate (subject to the design phase confirming
specific commands) is:

- `cargo test --manifest-path src-tauri/Cargo.toml --lib`
- `npx svelte-check --workspace . --threshold error`
- `npm run build`
- Manual smoke on Linux + Windows covering: locale selector on
  Configuration, first-run auto-detection, manual override, restart
  retains choice, full UI walkthrough in `es` and `en`, PDF generation
  in both locales, OS notification body in both locales, error message
  in both locales.

## Open questions (proposal question round) — RESOLVED

All items below were settled during the orchestrator-confirmed decision
round that preceded this proposal. They are recorded here for
traceability; each one is now a locked decision that the spec, design,
and tasks phases implement as-is.

These are product / UX level only — no harness mechanics (test
commands, PR shape, line budgets) are raised here.

1. **Selector widget.** **RESOLVED — Dropdown.** The Configuration page
   renders a `<select>` element listing `English` and `Español`. The
   dropdown scales to additional locales without UI rework; a radio
   group would have been more compact for two options but less
   extensible.

2. **Detected system language hint.** **RESOLVED — Show.** When the
   active value was inherited from auto-detection and no manual
   override is on file, the Configuration page renders a "Detectado:
   Español" / "Detected: English" hint above the dropdown. The hint
   disappears once the user makes a manual selection.

3. **PDF filter key localisation.** **RESOLVED — Translate.** The PDF
   filter prefixes (`store=`, `location=`, `category=`, `categories
   (N)`, `urgency=`, `from=`, `to=`) translate to Spanish
   (`tienda=`, `ubicación=`, `categoría=`, `categorías (N)`,
   `urgencia=`, `desde=`, `hasta=`) in the rendered PDF. The PDF is a
   user-facing operational document; CSV export remains the
   machine-readable contract and is unchanged.

4. **Notification SKU visibility.** **RESOLVED — Always visible.** The
   OS notification body includes the SKU in both locales because the
   SKU is operational data that identifies the lot. The surrounding
   template (`{qty} caduca el {date}{location}` /
   `{qty} expires on {date}{location}`) is fully localised.

Once these were answered, the proposal moved forward. The spec phase
will record these decisions as locked delta requirements under
`openspec/changes/caduxo-i18n-support/specs/caduxo-expiry-tracker/spec.md`
(per OpenSpec conventions), the design phase will settle the Svelte 5
binding shape and the `typesafe-i18n` generator pipeline, and the
tasks phase will forecast each implementation slice against the
3000-line review budget before any code is written.
