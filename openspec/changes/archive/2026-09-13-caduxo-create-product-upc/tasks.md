# Tasks: Capture UPC/Barcode While Creating a Product

Change: `caduxo-create-product-upc`
Source of truth: `openspec/changes/caduxo-create-product-upc/proposal.md`, `specs/caduxo-expiry-tracker/spec.md`, `design.md`.

This change is local and additive: a new create-form Barcodes subsection, a typed barcode-attach wrapper, and a dashboard wiring fix. Backend, schema, and command surface are untouched. Strict TDD is disabled in `openspec/config.yaml` and the project has no frontend test harness, so verification is via existing Rust regression tests plus manual smoke checks.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~140 additions/deletions across 3 files (design.md line-budget estimate: `+35/-20` `lib/products.ts`, `+60/-15` `ProductForm.svelte`, `+3/-5` `DashboardPage.svelte`). No backend changes. |
| 400-line budget risk | Low (≈35% of the 400-line canonical threshold; no migrations, no generated artifacts, no cross-cutting concerns). |
| Chained PRs recommended | No (single frontend slice; one logical change; small diff keeps review focused). |
| Suggested split | Single PR. |
| Delivery strategy | ask-on-risk (parent should confirm single-PR plan with the user before `sdd-apply`; ask-on-risk requires a delivery decision even when risk is Low). |
| Chain strategy | pending (user has not yet approved single-pr vs alternative). |

```text
Decision needed before apply: Yes
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low
```

## Scope summary

- Frontend-only; backend surface is unchanged.
- Existing Rust tests already cover the classification the new wrapper relies on:
  - `src-tauri/src/services/products.rs::tests::barcode_uniqueness_across_products` proves cross-product `DuplicateField(barcode)`.
  - `src-tauri/src/services/products.rs::tests::add_secondary_barcode_to_same_product_is_allowed` proves same-product distinct-value adds succeed.
- No new automated tests are introduced (no frontend harness). Verification of the wrapper is via these existing backend tests plus manual smoke checks below.

## Implementation work

### 1. Typed barcode-attach wrapper (`src/lib/products.ts`)

- [x] Add exported type `BarcodeAttachOnCreateResult` mirroring the design's discriminated union: `{ ok: true; barcode: ProductBarcodeResponse } | { ok: false; kind: "duplicate_other"; message: string } | { ok: false; kind: "duplicate_same"; message: string } | { ok: false; kind: "other"; message: string }`. <!-- sdd-owner: implementation -->
- [x] Add exported `addProductBarcodeOnCreate(input: ProductBarcodeCreate): Promise<BarcodeAttachOnCreateResult>` that (a) trims `input.barcode`, (b) calls `addProductBarcode` with the trimmed value, (c) on success returns `{ ok: true, barcode }`, (d) on `CommandError` matching `kind === "duplicate_field"` and `detail.field === "barcode"`, calls `listProductBarcodes(input.product_id)` to classify same-vs-other, returning `duplicate_same` if `b.barcode === trimmed value` else `duplicate_other`, (e) on any other thrown error returns `{ ok: false, kind: "other", message: String(e) }`. The outer function must never throw. <!-- sdd-owner: implementation -->
- [x] Add private `isDuplicateFieldError(e: unknown): boolean` helper that matches the structured `CommandError` wire shape (`kind: "duplicate_field"`, `detail.field === "barcode"`) AND the `DomainError::DuplicateField` `Display` string `"uniqueness violation: barcode"` (defined in `src-tauri/src/error.rs:15`) as a defensive fallback. Do not export it. <!-- sdd-owner: implementation -->
- [x] Remove the existing `addProductBarcodeIfNew` function (lines 207–216 of `src/lib/products.ts`) and its import from `ProductForm.svelte`. <!-- sdd-owner: implementation -->

Acceptance evidence:

- `grep -n "addProductBarcodeIfNew" src/` returns no matches in `src/`.
- `grep -n "BarcodeAttachOnCreateResult\|addProductBarcodeOnCreate" src/lib/products.ts` shows the new exports.
- TypeScript compiles cleanly under the project's existing build (`npm run check` / equivalent).

### 2. Create-form Barcodes subsection + submit flow (`src/components/ProductForm.svelte`)

- [x] Replace the `prefillSku` and `prefillBarcode` props with a single `prefillUpc: string | undefined = undefined` prop. Update the prop doc comment to describe UPC-field seeding on create mount. <!-- sdd-owner: implementation -->
- [x] Add local state: `let upcValue = "";`, `let upcType = "";`, `let upcIsPrimary = false;`, `let barcodeNotice = "";`. <!-- sdd-owner: implementation -->
- [x] Extend the existing create-mount `$:` reactive block: when `mode === "create"` and not yet initialized, seed `upcValue` from `prefillUpc` (only if defined) alongside the existing `suggestedProductAlertDays()` fetch. Do not seed SKU. <!-- sdd-owner: implementation -->
- [x] Add the Barcodes subsection markup inside an `{#if mode === "create"}` block, placed below the description row and above the `grid-2` for default unit / alert days. Mirror `src/components/ProductDetailPage.svelte` Barcodes shape: section header `<h3>Barcodes</h3>`, `grid-2` with "Barcode value" (`placeholder="e.g. 7501234567890"`, `autocomplete="off"`) and "Type (optional)" with `<datalist id="barcode-types">` offering `EAN13`, `EAN8`, `UPC`, `CODE128`, `CODE39`, `QR`, then a `checkbox-label` row with "Set as primary", then a conditional non-blocking `barcodeNotice` element styled as `.alert-info`. <!-- sdd-owner: implementation -->
- [x] Add a `.alert-info` CSS class (yellow/neutral palette) and ensure the notice uses `role="status"`, not `role="alert"`. <!-- sdd-owner: implementation -->
- [x] Rewrite the create-mode branch of `submit()`: after `saved = await createProduct(payload)`, when `upcValue.trim() !== ""`, call `addProductBarcodeOnCreate({ product_id: saved.id, barcode: trimmed, barcode_type: upcType.trim() || null, is_primary: upcIsPrimary })`; on `result.ok === false`, set `barcodeNotice` per the design's wording for each `kind` (`duplicate_other`: `Barcode "<value>" already belongs to another product and was not attached.`; `duplicate_same`: `result.message || Barcode "<value>" is already attached to this product.`; `other`: `result.message`). Always call `onSaved(saved)` once. <!-- sdd-owner: implementation -->
- [x] Remove the legacy silent post-save block that called `addProductBarcodeIfNew` inside the create branch (lines 153–161 of `ProductForm.svelte`). <!-- sdd-owner: implementation -->
- [x] Remove `prefillSku` initialization (`sku = prefillSku;` line 79) and the surrounding comment; SKU is now user-typed in every entry path. <!-- sdd-owner: implementation -->

Acceptance evidence:

- `grep -n "prefillSku\|prefillBarcode\|addProductBarcodeIfNew" src/components/ProductForm.svelte` returns no matches.
- `grep -n "prefillUpc\|upcValue\|barcodeNotice\|addProductBarcodeOnCreate" src/components/ProductForm.svelte` shows the new wiring.
- Visual check (manual smoke): creating a product with the UPC field empty still saves and shows no notice.
- Visual check (manual smoke): creating a product with a UPC that already belongs to another product still saves the product and shows the inline notice.

### 3. Dashboard quick-create wiring (`src/components/DashboardPage.svelte`)

- [x] Remove the local `let quickCreateBarcodeValue = "";` declaration and the line that assigns it inside `handleScanNotFound`. <!-- sdd-owner: implementation -->
- [x] Update the `<ProductForm>` invocation in the quick-create modal to pass only `prefillUpc={quickCreateScannedValue}` (drop `prefillSku` and `prefillBarcode`). <!-- sdd-owner: implementation -->
- [x] Update the modal hint copy from "the scanned value has been pre-filled" to a specific message that names the UPC field and reminds the user to type a SKU (e.g. "the scanned value has been pre-filled into the Barcode field. Type a SKU and submit."). <!-- sdd-owner: implementation -->
- [x] Confirm `handleScanFound` is unchanged: matched scans still open the product directly via `handleScanFound` → `viewProduct` / `openResolve`. <!-- sdd-owner: implementation -->

Acceptance evidence:

- `grep -n "quickCreateBarcodeValue\|prefillSku\|prefillBarcode\|prefillUpc" src/components/DashboardPage.svelte` shows `prefillUpc` only (no other legacy prop names).
- Manual smoke: scanning a value that matches an existing product still opens that product (no quick-create modal).
- Manual smoke: scanning a value that matches no product opens quick-create with the Barcode field pre-filled and the SKU field empty.

### 4. Backend surface (no code change — verification only)

- [x] Re-run `cargo test --manifest-path src-tauri/Cargo.toml services::products::tests::barcode_uniqueness_across_products services::products::tests::add_secondary_barcode_to_same_product_is_allowed services::products::tests::add_barcode_to_missing_product_returns_not_found` and confirm all three pass. These tests cover the three paths the new wrapper classifies. <!-- sdd-owner: implementation -->

### 5. Manual smoke checks

These run against the production-built app (dev mode acceptable) after tasks 1–4 are applied. Each must be checked before archive.

- [x] Catalog UI, empty UPC: open the create form, leave the UPC field empty, submit with valid SKU + description. Product is created, no barcode-related notice is rendered, navigation away from the form proceeds. Evidence: user manual acceptance — "ha funcionado he probado y funciona". <!-- sdd-owner: implementation -->
- [x] Catalog UI, new UPC: open the create form, enter a fresh UPC, leave "Set as primary" unchecked, submit. Product is created, no inline notice is rendered, the detail page shows the barcode. Evidence: user manual acceptance — "ha funcionado he probado y funciona". <!-- sdd-owner: implementation -->
- [x] Catalog UI, duplicate-other UPC: pre-stage a barcode on product A, open the create form, type A's barcode value, submit. Product is created (not rolled back), inline notice reads `Barcode "<value>" already belongs to another product and was not attached.`, navigation away still proceeds. Evidence: user manual acceptance — "ha funcionado he probado y funciona". <!-- sdd-owner: implementation -->
- [x] Catalog UI, "Set as primary" checked: create a product with one UPC and the box ticked; on the detail page, the new barcode shows the `Primary` badge. Evidence: user manual acceptance — "ha funcionado he probado y funciona". <!-- sdd-owner: implementation -->
- [x] Catalog UI, "Set as primary" unchecked: create a product with one UPC and the box unticked; on the detail page, the new barcode is listed without a `Primary` badge. Evidence: user manual acceptance — "ha funcionado he probado y funciona". <!-- sdd-owner: implementation -->
- [x] Dashboard, matched scan: scan a value that matches an existing product by barcode; the matched product opens, no quick-create modal appears. Evidence: user manual acceptance — "ha funcionado he probado y funciona". <!-- sdd-owner: implementation -->
- [x] Dashboard, unmatched scan: scan a value that matches no product; the quick-create modal opens, the UPC field is pre-filled, the SKU field is empty, and the hint copy names the Barcode field. Evidence: user manual acceptance — "ha funcionado he probado y funciona". <!-- sdd-owner: implementation -->
- [x] Dashboard, scan that collides on attach: in quick-create, submit with the scanned UPC and a typed SKU; if the scanned UPC already belongs to another product, the product is saved and the inline notice is shown without blocking navigation to lot entry. Evidence: user manual acceptance — "ha funcionado he probado y funciona". <!-- sdd-owner: implementation -->
- [x] Edit mode unchanged: open the edit form for an existing product; no Barcodes subsection is rendered (the detail-page Barcodes section is the management surface). Evidence: user manual acceptance — "ha funcionado he probado y funciona". <!-- sdd-owner: implementation -->

### 6. Documentation note for archive

- [x] When `openspec/changes/caduxo-create-product-upc/status.md` is created during the apply phase, include a note that the post-MVP backlog item 2 ("extend SKU/product code handling to support additional codes on product/SKU registration" — recorded in `openspec/changes/archive/2026-09-13-caduxo-expiry-tracker/tasks.md`) is satisfied by this change and should be marked done in the archived backlog. Evidence: recorded in `apply-progress.md`. <!-- sdd-owner: implementation -->

## Lifecycle gates (parent-owned)

- [x] Run bounded review of the diff against `proposal.md`, `spec.md`, `design.md`. Confirm: (a) Barcodes subsection renders only in create mode, (b) `addProductBarcodeOnCreate` never throws, (c) `onSaved` always fires once the product exists, (d) `prefillBarcode`/`prefillSku` are gone, (e) `addProductBarcodeIfNew` is gone, (f) backend surface is untouched. Evidence: parent grep/readback confirmed create-only render, `onSaved(saved)`, no legacy names under `src/`, and diff limited to frontend files plus OpenSpec artifacts. <!-- sdd-owner: parent -->
- [x] Run `sdd-apply` to materialize the changes on `feature/create-product-upc` and capture `apply-progress.md` evidence. Evidence: `openspec/changes/caduxo-create-product-upc/apply-progress.md`; native apply objective settled after user-accepted `size:exception`. <!-- sdd-owner: parent -->
- Next lifecycle step: run `sdd-verify` after apply is green; confirm spec scenarios (`UPC attach during product creation`, `non-blocking barcode attach on create`, `typed barcode attach wrapper for create flow`, `scanner quick-create seeds UPC field`, `silent barcode helper retired once nothing calls it`) are all satisfied by the diff + manual smoke evidence. This is not an implementation task checkbox.
- Final lifecycle step: run `sdd-archive` once verify passes; ensure the archive-report records the post-MVP backlog item-2 closeout and that the canonical requirements `multiple barcodes per product` and `barcode uniqueness` remain untouched. This is not an implementation task checkbox.
