# PDF locale formatting

## Goal
Make report PDF locale-sensitive formatting consistent for English and Spanish without adding a new locale.

## Scope
- Fix singular/plural relative expired-day labels such as Spanish `hace 1 día`.
- Make PDF date formatting locale-aware for supported locales.
- Make PDF quantity formatting locale-aware for supported locales.
- Move the store/location separator into locale messages.
- Make multi-category filter labels singular/plural through locale messages.

## Non-goals
- Do not localize backend validation/user messages in this slice.
- Do not change frontend formatters in this slice.
- Do not add a third locale.
- Do not redesign PDF layout or columns.

## Tasks
- [x] Task 1: Add locale message fields/helpers for PDF pluralization, date, quantity, and store/location separator.
- [x] Task 2: Route report PDF formatting through those locale helpers.
- [x] Task 3: Run focused Rust checks/tests and record results.
- [x] Task 4: Commit the verified slice after explicit user request.

## Evidence

### Task 1
- `src-tauri/src/pdf/locale.rs` already had the parent-prepared field splits
  applied before delegation landed: `filter_prefix_categories_one/other`
  (replacing the old `filter_prefix_categories_n`), `store_location_separator`,
  and `days_ago_one/other` (replacing `days_ago_template`).
- Both locales populate the new fields and the locale.rs parity / Spanish /
  English-baseline tests assert them.

### Task 2
- `src-tauri/src/pdf/report_pdf.rs` now routes every locale-sensitive helper
  through `PdfMessages`:
  - `format_days` selects `days_ago_one` for `-1` and `days_ago_other`
    otherwise, fixing the `hace 1 días` regression.
  - `format_filters` uses `filter_prefix_categories_other` for multi-category
    count labels; the singular key=value form
    (`filter_prefix_category`) is preserved for the single-category case to
    keep the existing `category=<id>` snapshot behaviour.
  - `format_store_location` joins store and location through
    `PdfMessages.store_location_separator` (no more hardcoded `" / "`).
  - `format_iso_date_for_locale` (renamed from `format_date_dd_mm_yyyy`)
    emits `MM/DD/YYYY` for English and `DD/MM/YYYY` for Spanish.
  - `format_qty` strips `.0` for integer-valued quantities and swaps the
    decimal separator to `,` for Spanish while leaving raw precision alone.
  - `humanize_generated_at` threads `Locale` through and renders the date
    portion of the header timestamp with the same locale rule, so the
    `Generated:` line in Spanish PDFs shows e.g. `15/01/2025 10:30:00`.
  - `draw_header` and `draw_table_row` accept `Locale` and forward it to the
    helpers; `render_report` passes its `locale` argument through unchanged.

### Task 3
- `cargo check --lib`: clean (no warnings, no errors).
- `cargo clippy --lib --no-deps --tests`: touched `src/pdf/*` modules clean.
  The command still reports pre-existing out-of-scope diagnostics outside the
  PDF slice, including a hard `clippy::approx_constant` error in
  `src/domain/lot_movements.rs` plus warnings in service modules.
- `cargo test --lib pdf::`: **41 passed; 0 failed.** Includes:
  - New: `format_qty_strips_trailing_zero_for_integer_values`,
    `format_qty_uses_dot_decimal_separator_in_english`,
    `format_qty_uses_comma_decimal_separator_in_spanish`,
    `format_iso_date_for_locale_uses_mm_dd_yyyy_in_english`,
    `format_iso_date_for_locale_uses_dd_mm_yyyy_in_spanish`,
    `format_iso_date_for_locale_passes_through_malformed_input`,
    `format_days_uses_english_singular_template_for_one_expired_day`,
    `format_days_uses_english_plural_template_for_many_expired_days`,
    `format_days_uses_spanish_singular_template_for_one_expired_day`,
    `format_days_uses_spanish_plural_template_for_many_expired_days`,
    `format_days_renders_non_negative_as_bare_number`,
    `format_store_location_uses_locale_separator`,
    `format_store_location_drops_empty_location_name`,
    `humanize_generated_at_strips_timezone_and_subseconds_in_english`,
    `humanize_generated_at_uses_spanish_date_order`.
  - Updated: `format_filters_uses_locale_prefixes_in_english`,
    `format_filters_uses_locale_prefixes_in_spanish` (still assert the
    count-form output, now sourced from `filter_prefix_categories_other`).
- `cargo test --lib` (full library suite): **455 passed; 0 failed.**
- `cargo test --doc`: 1 pre-existing doctest failure
  (`pdf::locale::Locale::parse` references `caduxo_lib::pdf::locale` but
  `lib.rs` declares `mod pdf` privately). Not introduced by this slice and
  outside the allowed edit surfaces; reported as a risk below.

### Task 4
- User explicitly agreed to commit after the verified slice.
- Commit: `84d51f5 fix: localize pdf formatting`.
