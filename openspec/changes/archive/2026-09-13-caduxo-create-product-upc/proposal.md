# Proposal: Capture UPC/Barcode While Creating a Product

## Summary

Allow the user to attach a UPC/barcode to a product during the create-product flow, not only after the product already exists on the product detail page. The create form grows a lightweight barcode section that mirrors the existing detail-page barcode UX (value, optional type, optional primary flag), and a scanner quick-create path that today pre-fills the SKU can pre-fill that same UPC field instead. Backend, schema, and command surface remain untouched: the create flow reuses the existing `createProduct` and `add_product_barcode` commands in sequence.

## Problem

Today, a barcode can only be added after a product exists:

- `ProductForm.svelte` create mode has no UPC/barcode field. Required fields are SKU, Description, Default unit, Alert days, Notes. Category is optional.
- `ProductDetailPage.svelte` is the only place a user can add a UPC/barcode via the "Barcodes" section. The section exposes value, optional type (with EAN13/EAN8/UPC/CODE128/CODE39/QR suggestions), and a "Set as primary" checkbox.
- `DashboardPage.svelte` scan quick-create already drives `ProductForm` with `prefillSku` and `prefillBarcode`. The `prefillBarcode` path calls `addProductBarcodeIfNew` after the product is saved and silently swallows any error (including the case where the scanned value is already attached to a different product).

This creates two frictions:

1. Manual path: a user creating a product from the catalog UI has to first create the product, then open it on the detail page just to add the UPC they already had in hand. Two screens for a single mental operation.
2. Scanner path: a scanned-but-unmatched value becomes the SKU of the new product, and the same value is also attached as a barcode post-save. If that scanned value already belongs to another product, the user is told nothing — the barcode is silently dropped while the SKU is still saved, which is confusing and offers no recovery path.

## Goals

- Let a user enter a UPC/barcode in the create-product form in one screen.
- Reuse the existing detail-page barcode field shape (value, optional type, optional primary) so users do not need to learn a second UI.
- Let the scanner quick-create path populate the same UPC field instead of only the SKU field when the scanned value matches a UPC shape.
- When the new product is saved but the barcode attach fails because the UPC already belongs to another product, create the product successfully and surface an inline, non-blocking notice naming the other product (or, when the backend does not identify it, that the value is already in use).
- Keep SKU the canonical product code; UPC remains optional and may be omitted.

## Non-goals

- No database schema changes. `products` and `product_barcodes` tables are unchanged.
- No new backend command. The proposal reuses the existing `create_product` and `add_product_barcode` commands.
- No atomic "create product with barcode" DTO. Two-step flow is intentional: a product without UPC must remain easy and valid.
- No product-identity or barcode-identity refactor (e.g., no SKU↔UPC unification, no composite keys).
- No CSV import changes. Import continues to call `add_product_barcode` post-create as it does today.
- No reports, dashboard, scanner, or notification changes beyond pre-filling the unified UPC field in the create form.
- No multi-store, multi-location, or category behavior changes.
- No backend/frontend test-harness expansion. Tests accompany the implementation slice as the project standard requires, but no new tooling.

## Proposed solution

### UX shape of the create form

Add a compact "Barcodes" subsection to the create form, rendered only in `mode === "create"`, mirroring the detail-page section but trimmed to the single attach-on-create case:

- One primary input labeled "Barcode value" with placeholder `e.g. 7501234567890`. Empty input means "no barcode" and is the default.
- One optional "Type" input with the same `<datalist>` suggestions used on the detail page: `EAN13`, `EAN8`, `UPC`, `CODE128`, `CODE39`, `QR`.
- One checkbox "Set as primary" (unchecked by default; if the backend currently auto-promotes the first barcode to primary for a new product, this checkbox is a no-op and should be hidden or relabeled — see Open decision 1).
- Inline error/notice slot rendered under the subsection when the post-save attach step produces a problem.

The subsection uses the same visual treatment as the detail-page Barcodes section (header, two-column grid, checkbox row, helper text) so users moving between create and detail pages see one shape. It is not interactive in the sense of a list — there is only one row because the goal is "attach while creating", not "manage the barcode list". Users who need multiple barcodes add the rest from the detail page after creation, exactly as they do today.

### Submit flow

The submit handler continues to call `createProduct` first. After a successful product save it conditionally calls a new wrapper (see `addProductBarcodeOnCreate` below) when the UPC field is non-empty:

1. `createProduct(payload)` — unchanged.
2. If `upcValue` (trimmed) is non-empty:
   - Call `addProductBarcodeOnCreate({ product_id: saved.id, barcode: trimmed, barcode_type: trimmedType || null, is_primary: <user choice> })`.
   - On success: no UI noise.
   - On duplicate-belonging-to-another-product: render an inline notice above the form actions stating that the barcode was not attached because it already belongs to another product, and stay on the form so the user can decide whether to leave it empty, edit it, or open the other product. The product is **not** rolled back.
   - On any other backend error (including duplicate within the same product, validation failure, or command error): render the same inline error slot with the backend message, but keep the product saved.
3. `onSaved(saved)` is always called once the product exists, so navigation away from the create form is not blocked by barcode attach outcome.

The existing `prefillBarcode` prop on `ProductForm` becomes redundant in the new flow because the scanner pre-fill seeds the UPC field directly. The proposal keeps the prop for now to avoid breaking the Dashboard quick-create path during the transition — see Open decision 2.

### Backend wrapper

Add a thin wrapper `addProductBarcodeOnCreate` in `src/lib/products.ts` next to `addProductBarcodeIfNew`. The wrapper differs from the existing `addProductBarcodeIfNew` in one important way:

- It returns a small discriminated result instead of swallowing errors:
  - `{ ok: true }` — barcode attached.
  - `{ ok: false, kind: "duplicate_other" }` — barcode already attached to a different product (current backend uniqueness rejection). The form uses this to render the "already on another product" notice.
  - `{ ok: false, kind: "duplicate_same" }` — same barcode already attached to this product (defensive — should not happen on a fresh product, but possible if the user retries).
  - `{ ok: false, kind: "other", message }` — any other failure; the form renders the message.
- It must **not** throw. The create flow must never crash on barcode failure.

The existing `addProductBarcodeIfNew` stays in place because the Dashboard quick-create path still uses it, and its silent behavior is acceptable there for the transition (see Open decision 2).

### Scanner integration

The dashboard's scan/search field currently matches a barcode first and a SKU second, then opens quick-create with `prefillSku` and `prefillBarcode`. In the new flow:

- If the scanned value matches an existing product by barcode, behavior is unchanged: open the matched product.
- If the scanned value does not match anything, the quick-create modal opens with the scanned value seeded into the UPC field of the create form instead of the SKU field. The user still must type a SKU themselves — UPC and SKU are not unified.

This requires the dashboard's quick-create wiring to set a new `prefillUpc` prop (or equivalent) on `ProductForm` instead of/in addition to `prefillBarcode`. See Open decision 2.

### Backend surface

No changes to:

- `commands::products::create_product`
- `commands::products::add_product_barcode`
- `service::add_barcode`
- `product_barcodes` schema or uniqueness constraints
- `ProductBarcodeCreate` / `ProductBarcodeResponse` DTOs

The existing uniqueness rejection path that `add_product_barcode` already returns is sufficient; the frontend wrapper classifies the error and the form renders it.

## User experience

### Create product (catalog UI)

1. User opens the create product form.
2. Required fields: SKU, Description, Alert days.
3. New "Barcodes" subsection visible below the description row, above the unit/alert grid.
4. User types a UPC. Optionally picks a type from the datalist. Optionally checks "Set as primary".
5. User submits. Product is created.
6. If the barcode attaches, the success path is unchanged from today.
7. If the barcode is already on another product, the user sees an inline notice: "Barcode `<value>` already belongs to another product and was not attached." The product is still created. The user can either close the form, edit the UPC, or use the detail page later.

### Create product (scanner quick-create)

1. User scans a value that matches no product.
2. Quick-create modal opens. The UPC field is pre-filled with the scanned value.
3. User types SKU and description (and any other required fields), then submits.
4. Same outcomes as the catalog UI create path.

### Create product with no barcode

1. User leaves the UPC field empty.
2. Submit creates the product. No barcode call is made. No error.

### Edit product

Unchanged. The Barcodes subsection is not added in `mode === "edit"`. Barcode management continues to happen on the detail page.

## Affected areas

| Area | Change |
|------|--------|
| `src/components/ProductForm.svelte` | Add the Barcodes subsection in create mode only. Add a typed barcode-attach result state. Wire `addProductBarcodeOnCreate` into the submit flow. |
| `src/lib/products.ts` | Add `addProductBarcodeOnCreate` returning a discriminated result. Keep `addProductBarcodeIfNew` for the dashboard transition. |
| `src/components/DashboardPage.svelte` | Switch quick-create from `prefillBarcode` to a UPC-field pre-fill (new prop, e.g. `prefillUpc`) and stop double-attaching via the silent path. |
| `openspec/specs/caduxo-expiry-tracker/spec.md` | No spec change required; existing requirements ("multiple barcodes per product", "barcode uniqueness") already cover the desired behavior. The post-MVP backlog item 2 ("extend SKU/product code handling to support additional codes on product/SKU registration") will be checked off by this change. |

## Behavior rules and edge cases

- Empty UPC field → no `add_product_barcode` call, no UI noise.
- UPC contains whitespace → trim before attach (consistent with detail page).
- UPC attached to another product → product created, inline notice, no rollback.
- UPC already attached to this product → backend uniqueness rejection; product created; inline notice with backend message.
- Backend command error (network, DB, schema) → product created; inline error message; no rollback.
- "Set as primary" unchecked on a brand-new product where the backend auto-promotes the first barcode → checkbox is hidden or relabeled (see Open decision 1).
- "Set as primary" checked but the attach fails → checkbox state is reset on form close; product is created without a primary marker.
- Multiple barcodes on the same new product → not supported via create form. User adds the rest from the detail page after creation.
- Scanner quick-create that resolves to an existing product → unchanged behavior (open the product).
- Scanner quick-create whose value the user later edits to a different UPC → behaves the same as the manual-create path with a non-empty UPC field.

## Risks

| Risk | Mitigation |
|------|------------|
| Backend returns "duplicate barcode" but does not tell us which product owns it, so the user sees only a generic message. | The notice wording is acceptable as "already belongs to another product". A follow-up could enrich the error to include the owning product's description/sku, but is out of scope here. |
| Create flow now does an extra backend call after `create_product`. A failure here could be confused with a product-creation failure. | The UI separates the create outcome (navigation/callback) from the attach outcome (inline notice); `onSaved` is always invoked once the product exists. |
| `addProductBarcodeIfNew` already swallows errors silently in the dashboard quick-create path. Mixing the new typed wrapper with the silent one risks inconsistent behavior across the two entry points. | The dashboard path is migrated to the new wrapper as part of this change so both paths use the typed result. The silent helper is removed once nothing calls it. |
| Scanner pre-fill currently seeds both SKU and barcode with the same value. Migrating to UPC-only pre-fill changes a behavior the user may have grown used to. | Confirmed product decision: scanner quick-create seeds only the UPC field. The user types the SKU. |
| Adding a new subsection to the create form increases visual complexity for users who never use barcodes. | The subsection is collapsed-by-default visual treatment is not required; the section is short and uses the same look the user already sees on the detail page. Empty UPC is the default and the most common path. |

## Rollback

The change is additive and local:

- Removing the Barcodes subsection from `ProductForm.svelte` returns the create form to its current shape.
- Removing `addProductBarcodeOnCreate` from `src/lib/products.ts` and reverting `DashboardPage.svelte` to use `prefillBarcode` reverts the scanner path to its current behavior.
- No database migration is introduced, so no schema rollback is required.

## Success criteria

- A user creating a product from the catalog UI can attach a UPC without leaving the create form.
- A user creating a product from the scanner quick-create path can submit with the scanned value as the UPC and a typed SKU.
- The detail-page Barcodes section is unchanged and continues to be the place to manage multiple barcodes per product.
- When the UPC entered at create time already belongs to another product, the new product is still created and the user sees an inline notice naming the situation.
- When the UPC field is left empty, no `add_product_barcode` call is made and no error appears.
- Edit mode of the product form is unchanged.
- The post-MVP backlog item "extend SKU/product code handling to support additional codes on product/SKU registration" is satisfied by this change and can be moved to done.

## Open decisions before design

1. Should the "Set as primary" checkbox appear in the create-form barcode subsection? Two reasonable answers:
   - **Yes**, to mirror the detail page exactly; pass `is_primary: <checkbox>` through to `add_product_barcode`. Simple, consistent.
   - **No**, because a brand-new product has no other barcodes yet, so the first attach is implicitly primary. Hide the checkbox and always pass `is_primary: true` (matching the current `prefillBarcode` path).
   The design phase should confirm with the implementer which is lighter and consistent with the current backend auto-promotion behavior on a fresh product.
2. How exactly to migrate the dashboard quick-create path:
   - Option A: keep the `prefillBarcode` prop and have the create form internally route it into the UPC field; remove the post-save silent attach.
   - Option B: introduce a new `prefillUpc` prop, leave `prefillBarcode` in place during the transition, and remove it once `DashboardPage` migrates.
   The design phase picks one. Both avoid behavior drift.
3. The post-save attach error is currently surfaced from the typed wrapper as `kind: "duplicate_other"`. If the backend later enriches the error to identify the owning product, the notice copy should be updated. For this proposal the notice copy is intentionally generic.

## Confirmed product decisions (from question round)

- Backend behavior: use the existing two-step flow (`create_product` then `add_product_barcode`), not an atomic DTO redesign. Reason: a SKU may not have a UPC and omitting it must be easy.
- Create form fields: keep the create-form barcode subsection lightweight and consistent with the detail-page Barcodes section. The user is okay with "Set as primary" if it stays simple.
- Duplicate UPC behavior: create the product first; then in the UPC attach step, inform the user (inline) if the UPC already belongs to another product. Do **not** block product creation on barcode attach failure.
- Scanner integration: a single UPC/barcode field on the create form. Scan pre-fills that field. The SKU field is no longer auto-filled from the scanned value.
