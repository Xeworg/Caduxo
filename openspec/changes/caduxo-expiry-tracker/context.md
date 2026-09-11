# Context: Caduxo Expiry Tracker

Caduxo is a local-first Windows/Linux desktop application focused only on product expiry tracking.

The current product discovery artifact is `docs/prd.md`.

## Confirmed constraints

- No POS, billing, payment, accounting, or full inventory management scope.
- Tauri v2 + Rust + SQLite is the current preferred stack.
- PDF reports should be generated directly in Rust with a library such as `printpdf`.
- Normal execution must not require administrator permissions.
- CSV import supports simple `upc`, `sku`, `description` data and manual column mapping.
- SKU is mandatory and unique.
- Products may have multiple UPC/barcodes.
- Stores/locales are user-created; first run asks the user to create the first store.
- Internal store locations are optional and user-created.
- Alerts are local OS notifications for the current user while the app is running.
- Alert behavior: once per day per active lot from `expiry_date - alert_days_before` through `expiry_date`.
