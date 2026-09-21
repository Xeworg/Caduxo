<!--
  ColumnMapper.svelte — CSV column-to-field mapper (PR 13 cleanup pass of
  caduxo-daisyui-redesign).

  Migration to shared primitives:
    - Modal.svelte — replaces the legacy `.overlay / .mapper-box` shell so
      focus trap, Escape handling, backdrop click, and DaisyUI's dialog
      state machine come from the shared primitive. The mapper opens via
      a bound `open` prop (two-way).
    - Listbox.svelte — replaces every native `<select>` (sku, description,
      barcode, category, unit, alert_days_before, notes). Native selects on
      WebKit / Chromium leak OS-styled chrome into the option list; the
      Listbox primitive renders a themed popover that inherits DaisyUI
      tokens for both `caduxo-light` and `dark`.
    - Button.svelte — replaces the legacy `.btn-apply / .btn-cancel`
      hand-rolled buttons.

  All visible copy stays on the `$LL.csvImport.*` keys (no new i18n keys).
-->
<script lang="ts">
  import { LL } from "../i18n/i18n-svelte.js";
  import { type CsvColumnMapping } from "../lib/csv.js";
  import Modal from "./ui/Modal.svelte";
  import Listbox from "./ui/Listbox.svelte";
  import Button from "./ui/Button.svelte";

  // ── Props ──────────────────────────────────────────────────────────────────

  /** Detected CSV headers. */
  export let headers: string[] = [];
  /** Current mapping (from auto-detect or previous edits). */
  export let initialMapping: CsvColumnMapping = {};
  /** Called when the user confirms the mapping. */
  export let onApply: (mapping: CsvColumnMapping) => void;
  /** Called when the user cancels. */
  export let onCancel: () => void;
  /** Controls modal open state. Two-way bindable for parent-driven close. */
  export let open: boolean = true;

  // ── Local state ────────────────────────────────────────────────────────────

  // Listbox contract: `value: string`. We keep strings here and convert
  // to number in `handleApply` (the backend contract is `number | null`).
  let selectedSku: string = initialMapping.sku != null ? String(initialMapping.sku) : "";
  let selectedDescription: string = initialMapping.description != null ? String(initialMapping.description) : "";
  let selectedBarcode: string = initialMapping.barcode != null ? String(initialMapping.barcode) : "";
  let selectedCategory: string = initialMapping.category != null ? String(initialMapping.category) : "";
  let selectedUnit: string = initialMapping.default_unit != null ? String(initialMapping.default_unit) : "";
  let selectedAlertDays: string = initialMapping.default_alert_days_before != null ? String(initialMapping.default_alert_days_before) : "";
  let selectedNotes: string = initialMapping.notes != null ? String(initialMapping.notes) : "";

  // ── Derived ─────────────────────────────────────────────────────────────────

  /** True when both required fields are mapped. */
  $: isValid = selectedSku !== "" && selectedDescription !== "";

  /** Maps a header index to the field currently assigned to it (empty string = unassigned). */
  function assignedField(colIdx: number): string {
    const idx = String(colIdx);
    if (selectedSku === idx) return "sku";
    if (selectedDescription === idx) return "description";
    if (selectedBarcode === idx) return "barcode";
    if (selectedCategory === idx) return "category";
    if (selectedUnit === idx) return "default_unit";
    if (selectedAlertDays === idx) return "default_alert_days_before";
    if (selectedNotes === idx) return "notes";
    return "";
  }

  /**
   * Options for a column dropdown. Each option shows the column index + header name
   * plus a suffix indicating which field is already using that column (if any).
   */
  function colOptions(): Array<{ value: string; label: string; disabled?: boolean }> {
    const opts: Array<{ value: string; label: string; disabled?: boolean }> = [
      { value: "", label: $LL.csvImport.notMapped() },
    ];
    for (let i = 0; i < headers.length; i++) {
      const current = assignedField(i);
      const suffix = current ? ` (→ ${current})` : "";
      opts.push({ value: String(i), label: `${i + 1}. ${headers[i]}${suffix}` });
    }
    return opts;
  }

  // ── Apply ─────────────────────────────────────────────────────────────────

  function handleApply() {
    if (!isValid) return;
    const numOrNull = (s: string): number | null => (s === "" ? null : Number(s));
    onApply({
      sku: numOrNull(selectedSku),
      description: numOrNull(selectedDescription),
      barcode: numOrNull(selectedBarcode),
      category: numOrNull(selectedCategory),
      default_unit: numOrNull(selectedUnit),
      default_alert_days_before: numOrNull(selectedAlertDays),
      notes: numOrNull(selectedNotes),
    });
  }

  function handleClose() {
    onCancel();
  }

  // Build options once per render to avoid re-creating on each access.
  $: skuOptions = colOptions();
  $: descOptions = colOptions();
  $: barcodeOptions = colOptions();
  $: categoryOptions = colOptions();
  $: unitOptions = colOptions();
  $: alertOptions = colOptions();
  $: notesOptions = colOptions();
</script>

<Modal
  bind:open
  size="wide"
  showClose
  closeLabel={$LL.lotMovements.modal.close()}
  titleId="column-mapper-title"
  aria-label={$LL.csvImport.mapColumns()}
  oncancel={handleClose}
  onclose={handleClose}
>
  {#snippet children()}
    <header class="mapper-header">
      <h2 id="column-mapper-title">{$LL.csvImport.mapColumns()}</h2>
      <p class="mapper-subtitle">
        {$LL.csvImport.mapColumnsSubtitle()}
      </p>
    </header>

    <div class="table-wrap">
      <table class="map-table">
        <thead>
          <tr>
            <th>{$LL.csvImport.field()}</th>
            <th>{$LL.csvImport.column()}</th>
            <th>{$LL.csvImport.preview()}</th>
          </tr>
        </thead>
        <tbody>
          <!-- SKU -->
          <tr class={selectedSku === "" ? "req-row" : ""}>
            <td>
              <span class="field-label">{$LL.csvImport.sku()}</span>
              <span class="req-badge">{$LL.csvImport.required()}</span>
            </td>
            <td>
              <Listbox
                bind:value={selectedSku}
                options={skuOptions}
                size="sm"
                aria-label={$LL.csvImport.sku()}
                aria-describedby="map-col-sku-hint"
                invalid={selectedSku === ""}
              />
            </td>
            <td class="preview" id="map-col-sku-hint">
              {selectedSku !== "" && headers[Number(selectedSku)] ? headers[Number(selectedSku)] : "—"}
            </td>
          </tr>

          <!-- Description -->
          <tr class={selectedDescription === "" ? "req-row" : ""}>
            <td>
              <span class="field-label">{$LL.csvImport.description()}</span>
              <span class="req-badge">{$LL.csvImport.required()}</span>
            </td>
            <td>
              <Listbox
                bind:value={selectedDescription}
                options={descOptions}
                size="sm"
                aria-label={$LL.csvImport.description()}
                aria-describedby="map-col-desc-hint"
                invalid={selectedDescription === ""}
              />
            </td>
            <td class="preview" id="map-col-desc-hint">
              {selectedDescription !== "" && headers[Number(selectedDescription)]
                ? headers[Number(selectedDescription)]
                : "—"}
            </td>
          </tr>

          <!-- UPC/Barcode -->
          <tr>
            <td><span class="field-label">{$LL.csvImport.upcBarcode()}</span></td>
            <td>
              <Listbox
                bind:value={selectedBarcode}
                options={barcodeOptions}
                size="sm"
                aria-label={$LL.csvImport.upcBarcode()}
                aria-describedby="map-col-bc-hint"
              />
            </td>
            <td class="preview" id="map-col-bc-hint">
              {selectedBarcode !== "" && headers[Number(selectedBarcode)]
                ? headers[Number(selectedBarcode)]
                : "—"}
            </td>
          </tr>

          <!-- Category -->
          <tr>
            <td><span class="field-label">{$LL.csvImport.categoryName()}</span></td>
            <td>
              <Listbox
                bind:value={selectedCategory}
                options={categoryOptions}
                size="sm"
                aria-label={$LL.csvImport.categoryName()}
                aria-describedby="map-col-cat-hint"
              />
            </td>
            <td class="preview" id="map-col-cat-hint">
              {selectedCategory !== "" && headers[Number(selectedCategory)]
                ? headers[Number(selectedCategory)]
                : "—"}
            </td>
          </tr>

          <!-- Unit -->
          <tr>
            <td><span class="field-label">{$LL.csvImport.unit()}</span></td>
            <td>
              <Listbox
                bind:value={selectedUnit}
                options={unitOptions}
                size="sm"
                aria-label={$LL.csvImport.unit()}
                aria-describedby="map-col-unit-hint"
              />
            </td>
            <td class="preview" id="map-col-unit-hint">
              {selectedUnit !== "" && headers[Number(selectedUnit)] ? headers[Number(selectedUnit)] : "—"}
            </td>
          </tr>

          <!-- Alert Days -->
          <tr>
            <td><span class="field-label">{$LL.csvImport.alertDays()}</span></td>
            <td>
              <Listbox
                bind:value={selectedAlertDays}
                options={alertOptions}
                size="sm"
                aria-label={$LL.csvImport.alertDays()}
                aria-describedby="map-col-alert-hint"
              />
            </td>
            <td class="preview" id="map-col-alert-hint">
              {selectedAlertDays !== "" && headers[Number(selectedAlertDays)]
                ? headers[Number(selectedAlertDays)]
                : "—"}
            </td>
          </tr>

          <!-- Notes -->
          <tr>
            <td><span class="field-label">{$LL.csvImport.notes()}</span></td>
            <td>
              <Listbox
                bind:value={selectedNotes}
                options={notesOptions}
                size="sm"
                aria-label={$LL.csvImport.notes()}
                aria-describedby="map-col-notes-hint"
              />
            </td>
            <td class="preview" id="map-col-notes-hint">
              {selectedNotes !== "" && headers[Number(selectedNotes)] ? headers[Number(selectedNotes)] : "—"}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  {/snippet}

  {#snippet footer()}
    <Button variant="ghost" onclick={handleClose}>
      {$LL.common.cancel()}
    </Button>
    <Button
      variant="primary"
      onclick={handleApply}
      disabled={!isValid}
    >
      {$LL.csvImport.previewImport()}
    </Button>
  {/snippet}
</Modal>

<style>
  /* ── Header ────────────────────────────────────────────────────────────── */
  .mapper-header {
    padding-bottom: 12px;
    border-bottom: 1px solid var(--color-base-200);
    margin-bottom: 16px;
  }

  .mapper-header h2 {
    margin: 0 0 4px;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--color-base-content);
  }

  .mapper-subtitle {
    margin: 0;
    font-size: 0.85rem;
    color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
  }

  /* ── Table ─────────────────────────────────────────────────────────────── */
  .table-wrap {
    max-height: 60vh;
    overflow-y: auto;
  }

  .map-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }

  .map-table th {
    text-align: left;
    padding: 6px 8px;
    color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
    font-weight: 500;
    font-size: 0.8rem;
    border-bottom: 1px solid var(--color-base-200);
  }

  .map-table td {
    padding: 7px 8px;
    border-bottom: 1px solid var(--color-base-200);
    vertical-align: middle;
  }

  .req-row td {
    background: color-mix(in oklch, var(--color-error) 6%, transparent);
  }

  .field-label {
    font-weight: 500;
    color: var(--color-base-content);
    display: block;
  }

  .req-badge {
    font-size: 0.7rem;
    color: var(--color-error);
    font-weight: 500;
    display: block;
  }

  .preview {
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    font-size: 0.8rem;
    font-style: italic;
    white-space: nowrap;
  }
</style>
