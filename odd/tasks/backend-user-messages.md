# Backend user-message localization

## Goal
Wire the existing backend user-message catalog into user-visible validation errors so Spanish UI flows no longer receive English or hardcoded Spanish backend validation strings.

## Scope
- Use `localize_validation` for report validation errors that already receive `Locale`.
- Add a typed/localizable backend message for the expiry-lot location-required validation.
- Wire locale through expiry-lot and CSV Tauri command wrappers so known `DomainError::Validation` strings can be localized before crossing IPC.
- Align CSV missing-column messages with the existing `UserMessage` catalog if needed.
- Update frontend wrappers/call sites only as needed to pass the current locale to changed commands.

## Non-goals
- Do not localize `BusinessRule`, `NotFound`, or infrastructure errors in this slice.
- Do not redesign `CommandError` or the IPC error shape.
- Do not localize persisted sentinel data such as `Sin ubicacion`; it remains a stable database value.
- Do not convert per-row CSV import `CsvImportRowOutcome::Invalid { reason }` to typed localized variants in this slice.
- Do not add a third locale.

## Tasks
- [x] Task 1: Extend/align backend user-message catalog and service validation strings.
- [x] Task 2: Wire locale/localize_validation through report, expiry-lot, and CSV command paths.
- [x] Task 3: Update frontend command wrappers/call sites for changed command signatures.
- [x] Task 4: Run focused Rust/frontend checks and independent verification.
- [x] Task 5: Commit the verified slice after explicit user request.

## Evidence

### Task 1
- `services/user_messages.rs`: added `LocationRequired` variant (English canonical `"Please select a location"`, Spanish `"Selecciona una ubicación"`), wired through `user_message(...)` and `parse_user_message_kind(...)`. Register the module in `services/mod.rs`.
- `services/expiry_lots.rs`: location-required validation now emits `user_message(UserMessage::LocationRequired, Locale::En)` instead of the hardcoded Spanish string. Expiry-date validation now emits the canonical `InvalidDateFormat { label: "expiry date", value }` text in both strict-shape and parse-failure branches.
- `services/csv_io.rs`: `preview_product_csv` and `import_product_csv` now emit `user_message(UserMessage::SkuColumnNotDetected, Locale::En)` and `user_message(UserMessage::DescriptionColumnNotDetected, Locale::En)` for the missing-column error path, matching the UserMessage catalog exactly.
- `services/reports.rs`: `optional_date` and the inverted-range branch in `validate_filters` now emit the canonical catalog text (`InvalidDateFormat { label, value }` and `InvertedDateRange { from, to }`) in English.
- The persisted `Sin ubicacion` sentinel is unchanged; the canonical English path preserves it as a stable database value while letting the IPC boundary translate the user-visible validation message.
- Pre-existing test `localize_validation_ignores_internal_error` referenced `DomainError::Internal`, a variant that was removed from the error model. Updated the test (inside the allowed `user_messages.rs` surface) to exercise a non-Validation variant (`NotFound`) and renamed it to `localize_validation_ignores_non_validation_error`. Behavioural contract (non-Validation errors pass through unchanged) is preserved.
- Service-layer assertion `message == "Selecciona una ubicación"` in `services::expiry_lots::create_lot_requires_location_when_setting_is_on` updated to `message == "Please select a location"` — the service now emits canonical English.

### Task 2
- `services/reports.rs`: `preview_report` now wraps `validate_filters` with `localize_validation(AppError::Domain(e), locale)` at the entry point. `export_report_pdf` reuses `preview_report` and inherits the localised flow.
- `commands/expiry_lots.rs`: `create_expiry_lot` accepts a `locale: String` argument, parses it via `Locale::parse`, and passes the result through `localize_validation` before `AppError::into`. Only this command gained a locale parameter — the other expiry-lot commands surface `BusinessRule`/non-catalog `Validation` strings that are explicitly out of scope.
- `commands/csv_io.rs`: `preview_product_csv` and `import_product_csv` accept a `locale: String` argument, parse it via `Locale::parse`, and route errors through `localize_validation` before `AppError::into`. `read_csv_text`, `export_products_csv`, and `export_report_csv` do not emit catalog messages and stay signature-compatible.

### Task 3
- `lib/expiry_lots.ts`: `createExpiryLot(input, locale?)` accepts an optional `SupportedLocale` parameter (defaulting to the active rune state via `resolveLocale`) and forwards the resolved value as `locale` in the `invoke` payload. Other wrappers in the file are unchanged.
- `lib/csv.ts`: `previewProductCsv(input, locale?)`, `importProductCsv(input, locale?)`, and `importProductCsvPreview(locale?)` accept an optional `SupportedLocale` parameter that defaults to the active rune state. They forward the resolved value as `locale` in every `invoke` payload. `readCsvText`, `exportProductsCsv`, `pickCsvFile`, and the `pickCsvSavePath` flow helpers stay signature-compatible.
- The `resolveLocale` helper in both files reads `activeLocale.current` and falls back to `DEFAULT_LOCALE` while the rune is uninitialised, so existing call sites in `components/CsvImportPage.svelte` and `components/LotForm.svelte` keep compiling without an explicit locale argument. Locale flows to the backend on every backend call.

### Task 4
- `cargo test --lib services::user_messages` — 21/21 pass (includes new `location_required_en`, `location_required_es`, `parse_location_required_roundtrips`, `localize_validation_translates_location_required`).
- `cargo test --lib services::reports` — 26/26 pass. `validate_filters_rejects_inverted_range` and `validate_filters_rejects_malformed_date` still match the `Validation` shape; the canonical text still contains both endpoint labels for the inverted check.
- `cargo test --lib services::expiry_lots` — 33/33 pass. `create_lot_requires_location_when_setting_is_on` updated to assert the new English canonical; `create_lot_rejects_invalid_expiry_date` and quantity guards pass against the canonical `InvalidDateFormat` text.
- `cargo test --lib services::csv_io` — 29/29 pass. `preview_rejects_when_required_columns_missing`, `import_requires_sku_and_description_columns`, and the rest still classify their errors as `DomainError::Validation`.
- Worker observed `cargo test --lib` — 476/476 pass, but independent verification observed 475/476 with unrelated `services::categories::tests::search_prefix_match_for_partial_query` failing only in the full parallel suite. The same test passed individually and at HEAD without this slice; focused slice suites pass.
- `cargo build --bin caduxo` — clean compile of the Tauri entry point with the updated `#[tauri::command]` signatures.
- `npx svelte-check --no-tsconfig` — 0 errors, 0 warnings. The frontend wrappers' optional locale parameter preserves backwards compatibility with the existing component call sites.
- Independent `gentle-ai-verify` PASS: confirmed catalog/localization flow, TypeScript wrapper safety, focused checks, and that `services/mod.rs` one-line module export is necessary and safe despite being added outside the original worker surface list.

### Task 5
- User explicitly agreed to commit after the verified slice.
- Commit: `70cef2b feat: localize backend validation messages`.
