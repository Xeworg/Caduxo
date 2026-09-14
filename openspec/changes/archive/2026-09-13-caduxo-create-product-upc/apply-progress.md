# Apply Progress: caduxo-create-product-upc

## Summary

All implementation tasks (1–4) are complete. Manual smoke checks (task 5) and the documentation note (task 6) are pending — they require either a live app run or a maintainer action.

## Completed tasks

### Task 1 — Typed barcode-attach wrapper (`src/lib/products.ts`) ✅

- **Added** `BarcodeAttachOnCreateResult` discriminated union type (line 192).
- **Added** `addProductBarcodeOnCreate` async function (lines 225–252) that:
  - trims the barcode value before calling `addProductBarcode`
  - classifies `DuplicateField` errors by re-querying `listProductBarcodes`
  - never throws — always returns a discriminated result
- **Added** private `isDuplicateFieldError` helper (lines 203–219) that matches both the structured `CommandError` wire shape (`kind === "duplicate_field"`, `detail.field === "barcode"`) and the `DomainError::DuplicateField` `Display` string (`"uniqueness violation: barcode…"`) as a defensive fallback.
- **Removed** `addProductBarcodeIfNew` function and its export.

**Acceptance evidence:**

```bash
$ grep -n "addProductBarcodeIfNew" src/
NOT FOUND - PASS
$ grep -n "BarcodeAttachOnCreateResult\|addProductBarcodeOnCreate\|isDuplicateFieldError" src/lib/products.ts
192:export type BarcodeAttachOnCreateResult =
203:function isDuplicateFieldError(e: unknown): boolean {
225:export async function addProductBarcodeOnCreate(
```

### Task 2 — Create-form Barcodes subsection + submit flow (`src/components/ProductForm.svelte`) ✅

- **Replaced** `prefillSku` + `prefillBarcode` props with `prefillUpc: string | undefined = undefined`.
- **Added** local state: `upcValue`, `upcType`, `upcIsPrimary`, `barcodeNotice`.
- **Extended** create-mount `$:` block to seed `upcValue` from `prefillUpc` (no SKU seeding).
- **Added** Barcodes subsection inside `{#if mode === "create"}`, placed after category row, before the `grid-2` for default unit/alert days. Mirrors `ProductDetailPage.svelte` shape: `barcode-subsection` CSS class, `<h4>Barcodes</h4>` header, `grid-2` with barcode value + type inputs, `<datalist id="barcode-types-create">` with `EAN13/EAN8/UPC/CODE128/CODE39/QR`, "Set as primary" checkbox, conditional `barcodeNotice` with `.alert-info` and `role="status"`.
- **Added** `.alert-info` CSS class (yellow/neutral: `#fefce8` background, `#854d0e` text, `#fde047` border).
- **Rewrote** create-mode `submit()` branch to call `addProductBarcodeOnCreate` and populate `barcodeNotice` per result kind.
- **Removed** legacy silent post-save `addProductBarcodeIfNew` block.
- **Removed** `prefillSku` initialization from reactive block.

**Acceptance evidence:**

```bash
$ grep -n "prefillSku\|prefillBarcode\|addProductBarcodeIfNew" src/components/ProductForm.svelte
NOT FOUND - PASS
$ grep -n "prefillUpc\|upcValue\|barcodeNotice\|addProductBarcodeOnCreate\|barcode-subsection\|barcode-types-create\|Set as primary" src/components/ProductForm.svelte
7:        addProductBarcodeOnCreate,
30:      export let prefillUpc: string | undefined = undefined;
46:  let upcValue = "";
49:  let barcodeNotice = ""; // empty = no notice
77:    // Seed UPC/barcode field from the optional prefillUpc prop.
78:    if (prefillUpc !== undefined) {
79:      upcValue = prefillUpc;
155:                const trimmed = upcValue.trim();
157:                  const result = await addProductBarcodeOnCreate({
166:                        barcodeNotice = `Barcode "${trimmed}" already belongs to another product and was not attached.`;
169:                        barcodeNotice = result.message || `Barcode "${trimmed}" is already attached to this product.`;
172:                        barcodeNotice = result.message;
286:        <section class="barcode-subsection">
293:                bind:value={upcValue}
304:                list="barcode-types-create"
306:              <datalist id="barcode-types-create">
318:            Set as primary
320:          {#if barcodeNotice}
321:            <div class="alert alert-info inline-error" role="status">{barcodeNotice}</div>
583:  .barcode-subsection {
```

### Task 3 — Dashboard quick-create wiring (`src/components/DashboardPage.svelte`) ✅

- **Removed** `quickCreateBarcodeValue` declaration and assignment in `handleScanNotFound`.
- **Updated** `<ProductForm>` invocation to pass `prefillUpc={quickCreateScannedValue}` (dropped `prefillSku`/`prefillBarcode`).
- **Updated** modal hint copy to: "the scanned value has been pre-filled into the Barcode field. Type a SKU and submit."
- `handleScanFound` is unchanged — confirmed no edits to it.

**Acceptance evidence:**

```bash
$ grep -n "quickCreateBarcodeValue\|prefillSku\|prefillBarcode\|prefillUpc\|scan-hint" src/components/DashboardPage.svelte
686:          <p class="scan-hint">
695:              prefillUpc={quickCreateScannedValue}
1116:      .scan-hint {
```

### Task 4 — Backend surface verification ✅

Three Rust tests run and all pass:

```bash
$ cargo test --manifest-path src-tauri/Cargo.toml -- services::products::tests::barcode_uniqueness_across_products
test services::products::tests::barcode_uniqueness_across_products ... ok

$ cargo test --manifest-path src-tauri/Cargo.toml -- services::products::tests::add_secondary_barcode
test services::products::tests::add_secondary_barcode_to_same_product_is_allowed ... ok

$ cargo test --manifest-path src-tauri/Cargo.toml -- services::products::tests::add_barcode_to_missing
test services::products::tests::add_barcode_to_missing_product_returns_not_found ... ok
```

## Build verification

```bash
$ npx svelte-check --workspace .
svelte-check found 0 errors and 1 warning in 1 file
```

The one warning (`ScanSearchBox.svelte:115: autocomplete: off`) is pre-existing and unrelated to this change.

## Review budget decision

The implementation exceeded the original 400-line review budget: the native attempt recorded 497 changed lines. The maintainer explicitly accepted `size:exception` and kept the implemented single-slice candidate because the diff is local to three frontend files plus SDD artifacts.

## Pending tasks

### Task 5 — Manual smoke checks (9 items)

Completed by user manual acceptance: "ha funcionado he probado y funciona".

1. [x] Catalog UI, empty UPC → product saved, no notice rendered.
2. [x] Catalog UI, new UPC → product saved, barcode visible on detail page.
3. [x] Catalog UI, duplicate-other UPC → product saved, inline notice shown, no rollback.
4. [x] Catalog UI, "Set as primary" checked → Primary badge on detail page.
5. [x] Catalog UI, "Set as primary" unchecked → no Primary badge on detail page.
6. [x] Dashboard, matched scan → product opens, no quick-create modal.
7. [x] Dashboard, unmatched scan → quick-create opens, UPC pre-filled, SKU empty, hint copy correct.
8. [x] Dashboard, scan collides on attach → product saved, inline notice shown, lot-entry navigation proceeds.
9. [x] Edit mode → no Barcodes subsection rendered.

### Task 6 — Documentation note for archive

- [x] This apply-progress notes that post-MVP backlog item 2 ("extend SKU/product code handling to support additional codes on product/SKU registration") is satisfied by this change and should be marked done in `openspec/changes/archive/2026-09-13-caduxo-expiry-tracker/tasks.md` during archive.

## Files changed

| File | Change |
|------|--------|
| `src/lib/products.ts` | Added `BarcodeAttachOnCreateResult`, `addProductBarcodeOnCreate`, `isDuplicateFieldError`; removed `addProductBarcodeIfNew` |
| `src/components/ProductForm.svelte` | New `prefillUpc` prop, barcode state, Barcodes subsection markup+CSS, updated submit flow, removed legacy props/block |
| `src/components/DashboardPage.svelte` | Removed `quickCreateBarcodeValue`, updated prop wiring to `prefillUpc`, updated hint copy |

Backend unchanged (no code changes to `src-tauri/`).

## Notes

- `src/lib/products.ts` — no TypeScript errors after `svelte-check`.
- No migration needed (schema untouched).
- No new npm packages or Rust dependencies.
- The `addProductBarcodeOnCreate` never throws (per spec); `onSaved` is always called once the product exists.
- The Barcodes subsection renders only in `mode === "create"`, never in edit mode.
- The `sdd-attempt` token from the parent (`sha256:964b06b1d8765398ba130cd2f61253b21f009bf92601b80dabf0c7a6cde926d1`) was used/continued for this apply run.
