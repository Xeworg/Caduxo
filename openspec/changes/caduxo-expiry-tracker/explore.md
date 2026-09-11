# Explore: Caduxo Expiry Tracker

## Executive summary

Caduxo is a local-first Windows/Linux desktop application for managing product expiry dates. It should remain narrowly focused on expiry tracking and avoid becoming a POS, billing, accounting, or full inventory system.

The product direction is strong enough to proceed to proposal, with a few decisions still open around report generation details, PDF layout limits, and first-run/default data behavior.

## Confirmed product decisions

| Area | Decision |
|------|----------|
| Core purpose | Track product expiry dates only |
| Platforms | Windows and Linux first |
| App stack | Tauri v2 + Rust + SQLite |
| Data model | Product catalog + barcodes + expiry lots + stores + optional locations |
| SKU | Mandatory and unique |
| UPC/barcodes | Optional, multiple per product |
| Scanner support | Keyboard-wedge mode: scanner types code into focused input |
| Stores/locales | User-created; required on lots only when multiple stores exist |
| Internal locations | Optional, user-created inside stores |
| Alerts | Local OS notifications while app is running; expired items stay prominent in dashboard after notifications stop |
| Alert cadence | Once per day per active lot during its alert window |
| Default alert days | Software suggests 30 days when creating a product |
| Product alert default | User confirms/defines default per product |
| Lot alert value | Pre-filled from product, editable per lot |
| CSV import | Simple catalog import with column mapping |
| Reports | Simple operational reports with PDF export required |
| PDF strategy | Structured PDF generated from Rust, likely with `printpdf` |

## Main user jobs

1. Register products quickly using SKU or UPC.
2. Import a basic product catalog from CSV.
3. Register expiry lots for products.
4. Receive local daily alerts during each lot's alert window.
5. See urgent expiry work immediately on the dashboard.
6. Print/export simple expiry reports as PDF.
7. Work with one or multiple stores/locales.

## Recommended MVP boundary

### In scope

- First-run store setup.
- Product CRUD.
- Multiple barcode values per product.
- CSV product import with column mapping.
- Expiry lot CRUD.
- Optional internal locations.
- Dashboard with urgent lots.
- Local daily OS notifications while app is running.
- Simple reports:
  - In alert window.
  - Expired.
  - Next 30 days.
  - Custom filtered report.
- PDF export using Rust-side generation.
- CSV export.
- Manual backup/restore path.

### Out of scope

- POS features.
- Billing, payments, invoicing, accounting.
- Purchase orders or stock valuation.
- Multi-user roles and permissions.
- Cloud sync.
- Mobile app.
- WhatsApp, Telegram, email, or remote notifications.
- Background notifications after the app is fully closed.
- Advanced scanner driver integration.
- Complex analytics/BI reporting.

## Key risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tauri WebView dependency differs by OS | Packaging may be less “single binary” than expected | Validate Windows/Linux packaging early |
| PDF generation layout complexity | `printpdf` requires manual layout | Keep reports table-based and visually simple |
| Local notifications may need permissions | User may miss alerts if permission denied or app closed | Always show dashboard alerts; document notification behavior |
| Multiple stores can complicate entry | Slower lot creation | Remember last selected store; ask for first store only once |
| Product/barcode duplicates | Data confusion | Enforce unique SKU and unique barcode; provide review path on import |
| CSV formats vary | Import friction | Provide column mapping and preview before import |

## Product assumptions

- Users are comfortable scanning or typing SKU/UPC.
- Most barcode scanners used by local shops behave like keyboards.
- The user accepts that notifications work while the app is running, not as a background service after close.
- PDF reports are operational lists, not branded or highly designed documents.
- A local SQLite file is acceptable for the first product version.
- Categories should be a user-editable list.
- Partial lot resolution is required.

## Open questions for proposal

1. Should report PDFs include business name/logo later, or only text/table for MVP?
2. Should each store/local have independent alert settings, or should alert rules stay product/lot-based only?

## Recommendation

Proceed to `proposal`. Before writing the proposal, run a short product question round focused on the remaining business rules: single-store behavior, category handling, partial lot resolution, and expired-notification behavior.
