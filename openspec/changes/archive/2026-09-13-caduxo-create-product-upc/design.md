# Design: Capture UPC/Barcode While Creating a Product

## Executive summary

Today, attaching a UPC/barcode to a product requires the product to exist first: the create-product form has no barcode field, and the only place to add a barcode is the post-create detail page. This change adds a lightweight **Barcodes** subsection to the create form (rendered only in `mode === "create"`), an `addProductBarcodeOnCreate` wrapper that returns a discriminated result instead of throwing, and a wiring change in `DashboardPage.svelte` so that the scan quick-create path seeds the new UPC field instead of silently double-attaching the value as both SKU and barcode. The backend is untouched: the flow reuses the existing `create_product` and `add_product_barcode` commands, and barcode attach failures are surfaced as non-blocking inline notices while the new product itself is always saved and `onSaved` is always invoked.

## Resolved open decisions

The proposal surfaced three open decisions. Each is resolved here against the spec as the source of truth.

### Decision 1 — "Set as primary" checkbox in the create-form subsection

**Resolution: render the checkbox, unchecked by default. Pass the user's choice verbatim to `add_product_barcode`.**

The spec is explicit ("an optional 'Set as primary' checkbox (unchecked by default)") and the architectural decision in the proposal's "Confirmed product decisions" lists this UI as acceptable as long as it stays simple. The backend (`db/repositories/products.rs::insert_barcode`) honors `is_primary` exactly as passed — it does **not** auto-promote the first barcode on a brand-new product — so the checkbox value is the source of truth.

UX edge case to flag in the change description (but **not** a reason to deviate from the spec): a freshly-created product can end up with one non-primary barcode when the user leaves the box unchecked. The user can flip it on the detail page, which is the canonical place for post-create barcode management. This mirrors the detail-page behavior (which also defaults the box to unchecked for new entries), preserving one mental model across the two screens.

### Decision 2 — Dashboard quick-create prop name and transition shape

**Resolution: introduce `prefillUpc`, do not keep `prefillBarcode` in any form, and remove the silent `addProductBarcodeIfNew` helper in the same slice.**

Three reasons drive this:

1. **Semantics change.** The current `prefillBarcode` triggers a silent post-save attach; the new flow needs to seed a visible input field. Repurposing the same prop name for a different effect is a footgun for any external caller (and there is one today, in `DashboardPage.svelte`).
2. **The spec mandates removal of the silent helper** ("the silent helper MUST be removed once no production caller remains") and that the create form MUST NOT invoke it after migration. Keeping `prefillBarcode` even briefly reintroduces a path that the spec has retired.
3. **There is no external caller beyond `DashboardPage.svelte`.** `grep prefillBarcode` returns only the proposal/spec text, the form, and the dashboard. A no-transition-window cutover is safe and matches the "minimal additive change, smallest diff" guidance.

The dashboard wiring therefore becomes: `prefillUpc={quickCreateScannedValue}` (and the unused `quickCreateBarcodeValue` local is deleted). The `prefillSku` prop is also removed from `ProductForm` since UPC and SKU now travel through different props and the form has an explicit SKU field the user types into.

### Decision 3 — Typed wrapper classification of duplicate errors

**Resolution: classify by re-querying the affected product's own barcode list when the backend reports `DuplicateField`.**

The backend (`services/products.rs::add_barcode` → `barcode_unique_error`) translates SQLite `UNIQUE constraint failed: product_barcodes.barcode` into `CommandError::DuplicateField { field: "barcode", value }`. It does **not** identify the owning product. The wrapper therefore classifies as follows:

| Backend outcome | Wrapper result |
|----------------|---------------|
| `add_product_barcode` returns a `ProductBarcodeResponse` | `{ ok: true, barcode }` |
| Throws/rejects with `DuplicateField { field: "barcode", value }` AND `listProductBarcodes(saved_product_id)` already contains a row with `barcode === trimmed value` | `{ ok: false, kind: "duplicate_same", message }` |
| Throws/rejects with `DuplicateField { field: "barcode", value }` AND the same-product list does **not** contain that value | `{ ok: false, kind: "duplicate_other", message }` |
| Any other thrown error | `{ ok: false, kind: "other", message }` (uses `String(e)` from the existing `catch (e: unknown)` pattern) |
| Wrapper never throws | (enforced by an inner `try/catch` around the classification lookup) |

The classify-by-list approach is the right trade-off because:

- It uses an existing, fast, well-tested read endpoint (`listProductBarcodes`) and avoids introducing a new backend command or richer error shape.
- On a freshly created product, the same-product list is either empty (the common path) or, in the defensive retry case, contains the conflicting value. There is no third meaningful state.
- It is consistent with how `ProductDetailPage.svelte` already lists barcodes to render the section.
- A race (another process attaching or detaching a barcode between our `add_product_barcode` and the classify lookup) collapses safely to `duplicate_other` or `duplicate_same` based on whatever the list shows at that moment — both are non-blocking and the user sees an appropriate notice.

**Fallback classification rule**: if the same-product list query itself fails (network/DB), the wrapper returns `{ ok: false, kind: "duplicate_other", message }` — the conservative choice. The notice will read "already belongs to another product" which is acceptable even in the rare race case.

## Architecture decision

The change is local and additive to the existing layered architecture. No new modules, no new commands, no schema change. The frontend gains one new wrapper function and one new UI subsection; the dashboard loses a redundant prop and gains a clearer one. The product form's submit flow is restructured to keep `createProduct` first and treat the barcode attach as a non-blocking post-step, mirroring how the existing dashboard scan path *intended* to work but did not.

```text
ProductForm.svelte (create mode)
├── user enters SKU, description, ... UPC value, type, is_primary
├── on submit:
│   ├── createProduct(payload) → ProductResponse | throw (caller-visible error, product NOT created)
│   └── if upcValue.trim() !== "":
│       └── addProductBarcodeOnCreate({ product_id: saved.id, barcode, barcode_type, is_primary })
│           └── on { ok: true }                → no UI noise
│           └── on { ok: false, kind: "duplicate_other" }  → inline notice: "already belongs to another product"
│           └── on { ok: false, kind: "duplicate_same" }   → inline notice: backend message (defensive retry)
│           └── on { ok: false, kind: "other", message }    → inline notice: message
│   └── onSaved(saved) is always invoked once the product exists
```

```text
DashboardPage.svelte
├── ScanSearchBox → handleScanNotFound(scannedValue)
├── sets quickCreateScannedValue (the modal's pre-fill source)
└── opens <ProductForm mode="create" prefillUpc={quickCreateScannedValue} />
    └── the form's UPC field is pre-filled, SKU is empty, the user types the SKU
```

## Component design

### `src/lib/products.ts` — new `addProductBarcodeOnCreate`

Add next to `addProductBarcodeIfNew`, with the latter deleted in the same slice.

```ts
/** Discriminated result of attempting to attach a barcode during product creation. */
export type BarcodeAttachOnCreateResult =
  | { ok: true; barcode: ProductBarcodeResponse }
  | { ok: false; kind: "duplicate_other"; message: string }
  | { ok: false; kind: "duplicate_same";  message: string }
  | { ok: false; kind: "other";            message: string };

/**
 * Attaches a barcode to a freshly created product. Never throws.
 *
 * On `DuplicateField`, distinguishes same-product vs other-product by listing the
 * product's current barcodes and checking membership of the trimmed value.
 */
export async function addProductBarcodeOnCreate(
  input: ProductBarcodeCreate,
): Promise<BarcodeAttachOnCreateResult> {
  const value = input.barcode.trim();
  try {
    const created = await addProductBarcode({ ...input, barcode: value });
    return { ok: true, barcode: created };
  } catch (e) {
    const message = String(e);
    const looksLikeDuplicate = isDuplicateFieldError(e);
    if (looksLikeDuplicate) {
      try {
        const existing = await listProductBarcodes(input.product_id);
        if (existing.some((b) => b.barcode === value)) {
          return { ok: false, kind: "duplicate_same", message };
        }
      } catch {
        // Fall through to duplicate_other — the conservative choice.
      }
      return { ok: false, kind: "duplicate_other", message };
    }
    return { ok: false, kind: "other", message };
  }
}
```

`isDuplicateFieldError(e)` is a small private helper that inspects the thrown error. Tauri 2.x invokes surface `CommandError` instances with a `kind` discriminator; the helper matches `kind === "duplicate_field"` and `detail.field === "barcode"`. As a defensive backup, it also matches the `String(e)` representation `"uniqueness violation: barcode"` (the `DomainError::DuplicateField` `Display` impl from `src-tauri/src/error.rs:15`), because that string is what some IPC paths actually deliver to the frontend.

### `src/components/ProductForm.svelte` — create-mode Barcodes subsection

New state:

```ts
let upcValue = "";
let upcType = "";
let upcIsPrimary = false;
let barcodeNotice = ""; // empty = no notice
```

New UI block, rendered only inside `{#if mode === "create"}` and placed below the description row, above the `grid-2` for default-unit / alert-days (to mirror the detail-page Barcodes visual order):

```svelte
{#if mode === "create"}
  <section class="section barcode-subsection">
    <div class="section-header"><h3>Barcodes</h3></div>
    <div class="grid-2">
      <label>
        Barcode value
        <input type="text" bind:value={upcValue}
               placeholder="e.g. 7501234567890" autocomplete="off" />
      </label>
      <label>
        Type (optional)
        <input type="text" bind:value={upcType}
               placeholder="e.g. EAN13, UPC" list="barcode-types-create" />
        <datalist id="barcode-types-create">
          <option value="EAN13"></option>
          <option value="EAN8"></option>
          <option value="UPC"></option>
          <option value="CODE128"></option>
          <option value="CODE39"></option>
          <option value="QR"></option>
        </datalist>
      </label>
    </div>
    <label class="checkbox-label">
      <input type="checkbox" bind:checked={upcIsPrimary} />
      Set as primary
    </label>
    {#if barcodeNotice}
      <div class="alert alert-info inline-error" role="status">{barcodeNotice}</div>
    {/if}
  </section>
{/if}
```

Notice styling: a non-error, non-blocking "info" tone (yellow/neutral palette). The existing `.alert-error` is reused only for product-creation failures. A new `.alert-info` CSS class is added.

Pre-fill init: a single `$:` reactive block that fires once on `mode === "create"` mount. It (a) fetches the suggested alert days via `suggestedProductAlertDays()` (already wired), (b) seeds `upcValue` from `prefillUpc` if provided. No SKU pre-fill.

New submit branch:

```ts
} else {
  saved = await createProduct(payload);
  const trimmed = upcValue.trim();
  if (trimmed !== "") {
    const result = await addProductBarcodeOnCreate({
      product_id: saved.id,
      barcode: trimmed,
      barcode_type: upcType.trim() || null,
      is_primary: upcIsPrimary,
    });
    if (!result.ok) {
      switch (result.kind) {
        case "duplicate_other":
          barcodeNotice = `Barcode "${trimmed}" already belongs to another product and was not attached.`;
          break;
        case "duplicate_same":
          barcodeNotice = result.message || `Barcode "${trimmed}" is already attached to this product.`;
          break;
        case "other":
          barcodeNotice = result.message;
          break;
      }
    }
  }
}
onSaved(saved); // unconditional
```

If `createProduct` throws, `errorMsg` is set and `barcodeNotice` is never touched (we never get to the attach step). This preserves the existing caller-visible failure shape for product creation.

### `src/components/DashboardPage.svelte` — quick-create wiring

- Remove the local `quickCreateBarcodeValue` declaration and the line that sets it inside `handleScanNotFound` (now redundant — the scanned value is the pre-fill).
- Update the `<ProductForm>` invocation to use `prefillUpc={quickCreateScannedValue}` instead of `prefillSku`/`prefillBarcode`.
- Update the hint text from "the scanned value has been pre-filled" (which today is ambiguous between SKU and barcode) to be specific: "the scanned value has been pre-filled into the Barcode field. Type a SKU and submit."

### `src/components/ProductDetailPage.svelte`

No change. The Barcodes section on the detail page is the canonical management surface and is already complete.

## Data flow

```text
Catalog UI (manual create):
  user → ProductForm (mode=create)
       → createProduct(payload)
       → saved (id)
       → if upcValue !== "":
            addProductBarcodeOnCreate(input)
              ├─ ok true                 → no UI noise
              ├─ duplicate_other         → inline notice, saved.id intact
              ├─ duplicate_same          → inline notice, saved.id intact
              └─ other                   → inline notice (message), saved.id intact
       → onSaved(saved)

Dashboard (scan quick-create):
  ScanSearchBox → findProductByScan
    ├─ Found       → handleScanFound → open product (unchanged)
    └─ NotFound    → handleScanNotFound(scannedValue)
                  → showQuickCreate = true
                  → <ProductForm prefillUpc=scannedValue />
                  → submit flow as above
```

## Backend surface

No changes to:

- `commands::products::create_product` / `add_product_barcode`
- `services::products::create_product` / `add_barcode`
- `db/repositories/products.rs::insert_barcode`
- `product_barcodes` schema or UNIQUE constraint
- `ProductBarcodeCreate` / `ProductBarcodeResponse` DTOs
- `CommandError::DuplicateField` wire shape

The wrapper relies on `list_product_barcodes` (already exported from `lib/products.ts`) to do the same-product vs other-product classification.

## File changes

| File | Change | Approx. lines |
|------|--------|--------------|
| `src/lib/products.ts` | Add `BarcodeAttachOnCreateResult` type and `addProductBarcodeOnCreate`; remove `addProductBarcodeIfNew`; add private `isDuplicateFieldError` helper | +35 / -20 |
| `src/components/ProductForm.svelte` | Add `prefillUpc` prop (replaces `prefillSku`/`prefillBarcode`); add Barcodes subsection markup + CSS; add attach step in submit; remove silent post-save block | +60 / -15 |
| `src/components/DashboardPage.svelte` | Remove `quickCreateBarcodeValue`; rename prop wiring to `prefillUpc`; tighten hint copy | +3 / -5 |
| `src/components/ProductDetailPage.svelte` | None | 0 |
| Backend (`src-tauri/...`) | None | 0 |

Total: ~100 lines of frontend churn, all additive or one-for-one renames. Backend untouched, so no migration and no Rust test churn.

## Tests

**Backend** — none required. The existing test `services::products::tests::barcode_uniqueness_across_products` already proves the backend returns `DuplicateField(barcode)` for cross-product collisions, and `add_secondary_barcode_to_same_product_is_allowed` proves same-product, distinct-value adds succeed. No backend surface changes here.

**Frontend** — there is no existing JS test harness in the project (`grep "*.test.ts"` / `*.spec.ts` returns nothing in `src/`). Per the proposal's non-goal ("no backend/frontend test-harness expansion"), we do not introduce Vitest in this slice. The wrapper's classification logic is exercised through manual smoke tests at apply time (test plan documented in `tasks.md`):

1. Manual: catalog UI create with empty UPC → product saved, no notice.
2. Manual: catalog UI create with new UPC, `is_primary` ticked → product saved, barcode attached as primary.
3. Manual: catalog UI create with UPC already on another product → product saved, inline notice.
4. Manual: dashboard scan unmatched value → quick-create opens with UPC pre-filled, SKU empty.
5. Manual: dashboard scan value that collides with another product's barcode → product saved, inline notice, SKU is whatever the user typed.

Two automated coverage points exist without new harness:

- The wrapper's same-product check reuses `listProductBarcodes`, which is the same Tauri command the detail page already calls; if that command regresses, the detail page regresses too.
- The wrapper never throws (the spec mandates this); a future Rust-side or integration-level test can assert this if the project later adopts a frontend test harness.

## Rollout

The change is additive and local:

- Removing the Barcodes subsection from `ProductForm.svelte` returns the create form to its current shape (no field).
- Reverting `DashboardPage.svelte` to not pass `prefillUpc` returns quick-create to "no UPC pre-fill, user types UPC themselves if they want one."
- No backend changes, so no migration. No DTO changes, so no contract risk.

The rollout order in `tasks.md` is:

1. `src/lib/products.ts` — add wrapper, remove silent helper, add `BarcodeAttachOnCreateResult`.
2. `src/components/ProductForm.svelte` — add subsection + attach step.
3. `src/components/DashboardPage.svelte` — switch wiring to `prefillUpc`.
4. Manual smoke checks (test plan above).
5. Archive the change once green.

## Risks

| Risk | Mitigation |
|------|-----------|
| Wrapper misclassifies `duplicate_same` as `duplicate_other` (e.g. due to a race where the same product has the barcode between the insert attempt and the list query) | Conservative: list-based check is best-effort. Both outcomes render a non-blocking inline notice and the product is saved either way; the user can resolve on the detail page. |
| `CommandError` wire format changes between Tauri minor versions and breaks `isDuplicateFieldError`'s structured-field path | Fallback regex matches the `Display` string `"uniqueness violation: barcode"`, which is stable as long as `DomainError::DuplicateField`'s `#[error("…")]` annotation in `src-tauri/src/error.rs` is unchanged. |
| Removing `prefillBarcode` and `prefillSku` breaks any out-of-tree caller (none today, but conceivable) | `grep` confirms only `ProductForm.svelte` and `DashboardPage.svelte` reference these props; the change is self-contained. |
| User leaves "Set as primary" unchecked on a fresh product with one barcode → product has no primary barcode → search results show `primary_barcode: null` until the user toggles it on the detail page | Mirrors the detail-page default. Documented in the change description; not a blocker. |
| Backend later enriches `DuplicateField` with an owning-product id (e.g. `field: "barcode", value: "...", owner_product_id: "..."`) | Forward-compatible: the wrapper's classify-by-list path still works. A future change can short-circuit by reading the new field and skipping the list query. |
| The dashboard hint copy says "the scanned value has been pre-filled into the Barcode field. Type a SKU and submit" but a user types a SKU that collides with an existing SKU | `createProduct` returns `DuplicateField { field: "sku" }` which is surfaced through the existing `errorMsg` block at the top of the form — unchanged behavior. |

## Post-MVP backlog

The proposal notes that the post-MVP backlog item 2 ("extend SKU/product code handling to support additional codes on product/SKU registration") is satisfied by this change and can be moved to done. No new post-MVP items are introduced.

## Design decisions deferred to apply time

- Exact wording of the "already belongs to another product" notice (the spec mandates generic copy; can be tightened later).
- Whether to render the same `<datalist id="barcode-types-create">` id as the detail page or use a different id (the id is scoped to the form, so duplication is harmless — choose the detail-page id for symmetry).
- Whether the wrapper's `isDuplicateFieldError` helper should be exported (default: not exported; private to `lib/products.ts`).
