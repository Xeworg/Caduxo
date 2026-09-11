# Proposal: Caduxo Expiry Tracker

## Summary

Build Caduxo as a local-first Windows/Linux desktop application for product expiry tracking. Caduxo helps users register products, manage expiry lots, receive local alerts, and produce simple printable/PDF reports without adding POS, billing, accounting, or full inventory management features.

## Problem

Small businesses often track expiry dates manually or inside tools that are not designed for expiry workflows. This creates operational risk: expired products stay on shelves, soon-to-expire lots are missed, and staff lack a simple daily view of what needs action.

## Goals

- Register products quickly by SKU or UPC/barcode scanner input.
- Track one or more expiry lots per product.
- Support one or multiple stores/locales.
- Keep internal locations optional.
- Alert users locally once per day during each lot's alert window.
- Keep expired items highly visible on the dashboard after notifications stop.
- Generate simple operational reports, including PDF export.
- Run offline with local SQLite storage.
- Avoid administrator permissions for normal execution.

## Non-goals

- POS/cash register behavior.
- Sales, billing, payments, invoices, or accounting.
- Full inventory valuation or purchasing.
- Cloud sync.
- Multi-user permissions.
- Mobile app.
- WhatsApp, Telegram, email, or remote notifications in v1.
- Background notifications after the app is fully closed.
- Advanced scanner device integration.

## Proposed solution

Use Tauri v2 + Rust + SQLite.

Caduxo will store all data locally and expose a desktop UI optimized for fast daily operation:

1. User creates the first store/local on first run.
2. User creates products manually or imports them from CSV.
3. Products have mandatory unique SKUs and optional multiple UPC/barcodes.
4. User creates expiry lots linked to a product.
5. When multiple stores exist, lot creation asks which store/local owns the lot.
6. User may optionally assign an internal location.
7. Product default alert days are confirmed during product creation, seeded by software default of 30 days.
8. Lot alert days are pre-filled from product default but editable per lot.
9. Dashboard and reports show urgent, expired, and upcoming lots.
10. PDF reports are generated directly in Rust using a structured PDF library such as `printpdf`.

## User experience

### First run

- App asks the user to create the first store/local.
- App explains that more stores and optional internal locations can be added later.

### Product creation

Required:

- SKU.
- Description.
- Product default alert days before expiry.

Optional:

- One or more UPC/barcodes.
- Category from editable category list.
- Default unit.
- Notes.

### Expiry lot creation

Required:

- Product.
- Quantity.
- Unit.
- Expiry date.
- Alert days before expiry.

Conditionally required:

- Store/local only when multiple stores exist.

Optional:

- Internal location.
- Batch/lot code.
- Notes.

### Dashboard

The dashboard is the primary work surface. It must show:

- Expired lots in a highly visible section.
- Lots expiring today.
- Lots currently inside their configured alert window.
- Next 30 days.
- Store/local filter.
- Scan/search field for SKU/UPC.

### Alerts

Caduxo calculates alerts locally.

```text
alert_start_date = expiry_date - alert_days_before
```

For each active lot:

- Notify once per day from `alert_start_date` through `expiry_date`.
- Do not notify more than once for the same lot on the same day.
- Stop OS notifications after expiry date.
- Keep expired lots highly visible in the dashboard until resolved/archived.

### Reports

MVP reports:

1. In alert window.
2. Expired.
3. Next 30 days.
4. Custom filtered report.

Outputs:

- On-screen report preview.
- PDF export.
- CSV export.

PDF generation should prioritize predictable tables and print readability over complex layout.

## Data model direction

Use SQLite with these core tables:

- `stores`
- `store_locations`
- `categories`
- `products`
- `product_barcodes`
- `expiry_lots`
- `notification_log`
- `app_settings`

The operational center is `expiry_lots`. Products represent catalog identity; expiry lots represent quantities and expiry dates.

## CSV import

Support simple product catalog import with column mapping.

Canonical fields:

```csv
upc,sku,description
```

Rules:

- `sku` required.
- `description` required.
- `upc` optional.
- User can map alternate column names.
- Import preview should show detected rows and duplicate warnings.

## Acceptance criteria

- User can create first store/local during first run.
- User can create/edit/archive products.
- SKU is required and unique.
- Product can have multiple barcodes.
- User can create/edit/archive expiry lots.
- User can partially resolve lot quantities.
- Dashboard highlights expired lots strongly.
- Dashboard shows alert-window and next-30-days lots.
- Local OS notification fires once per day per active lot during alert window.
- Notification log prevents duplicate same-day lot notifications.
- User can import product catalog from CSV with column mapping.
- User can generate PDF reports.
- User can export report data to CSV.
- App works offline with local SQLite persistence.

## Risks

| Risk | Mitigation |
|------|------------|
| Tauri dependency behavior varies across Windows/Linux | Validate packaging early with a small spike |
| PDF generation with `printpdf` may require manual table layout | Keep report layouts simple and fixed-width initially |
| Notifications depend on OS permission/app being open | Dashboard remains the authoritative alert surface |
| CSV imports can create bad data | Use preview, duplicate checks, and explicit mapping |
| Multi-store logic can complicate single-store users | Make store implicit when only one exists |

## Open decisions before design

1. Exact frontend framework: Svelte, Solid, React, or another option.
2. Exact Rust DB approach: `rusqlite` vs `sqlx`.
3. PDF layout details: page size, orientation, max columns, pagination.
4. Whether categories require a dedicated table in MVP or can start as controlled values.
5. Backup/restore UX: copy DB file, export bundle, or both.
