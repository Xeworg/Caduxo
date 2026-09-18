<script lang="ts">
  import { LL } from "../i18n/i18n-svelte.js";
  import { type CsvColumnMapping } from "../lib/csv.js";

  // ── Props ──────────────────────────────────────────────────────────────────

  /** Detected CSV headers. */
  export let headers: string[] = [];
  /** Current mapping (from auto-detect or previous edits). */
  export let initialMapping: CsvColumnMapping = {};
  /** Called when the user confirms the mapping. */
  export let onApply: (mapping: CsvColumnMapping) => void;
  /** Called when the user cancels. */
  export let onCancel: () => void;

  // ── Local state ────────────────────────────────────────────────────────────

  let selectedSku: number | "" = initialMapping.sku ?? "";
  let selectedDescription: number | "" = initialMapping.description ?? "";
  let selectedBarcode: number | "" = initialMapping.barcode ?? "";
  let selectedCategory: number | "" = initialMapping.category ?? "";
  let selectedUnit: number | "" = initialMapping.default_unit ?? "";
  let selectedAlertDays: number | "" = initialMapping.default_alert_days_before ?? "";
  let selectedNotes: number | "" = initialMapping.notes ?? "";

  // ── Derived ─────────────────────────────────────────────────────────────────

  /** True when both required fields are mapped. */
  $: isValid = selectedSku !== "" && selectedDescription !== "";

  /** Maps a header index to the field currently assigned to it (empty string = unassigned). */
  function assignedField(colIdx: number): string {
    if (selectedSku === colIdx) return "sku";
    if (selectedDescription === colIdx) return "description";
    if (selectedBarcode === colIdx) return "barcode";
    if (selectedCategory === colIdx) return "category";
    if (selectedUnit === colIdx) return "default_unit";
    if (selectedAlertDays === colIdx) return "default_alert_days_before";
    if (selectedNotes === colIdx) return "notes";
    return "";
  }

  /**
   * Options for a column dropdown. Each option shows the column index + header name
   * plus a suffix indicating which field is already using that column (if any).
   */
  function colOptions(): Array<{ value: number | ""; label: string }> {
    const opts: Array<{ value: number | ""; label: string }> = [
      { value: "", label: $LL.csvImport.notMapped() },
    ];
    for (let i = 0; i < headers.length; i++) {
      const current = assignedField(i);
      const suffix = current ? ` (→ ${current})` : "";
      opts.push({ value: i, label: `${i + 1}. ${headers[i]}${suffix}` });
    }
    return opts;
  }

  // ── Apply ─────────────────────────────────────────────────────────────────

  function handleApply() {
    if (!isValid) return;
    onApply({
      sku: selectedSku === "" ? null : selectedSku,
      description: selectedDescription === "" ? null : selectedDescription,
      barcode: selectedBarcode === "" ? null : selectedBarcode,
      category: selectedCategory === "" ? null : selectedCategory,
      default_unit: selectedUnit === "" ? null : selectedUnit,
      default_alert_days_before: selectedAlertDays === "" ? null : selectedAlertDays,
      notes: selectedNotes === "" ? null : selectedNotes,
    });
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

<div class="overlay">
  <div class="modal">
    <header class="modal-header">
      <h2>{$LL.csvImport.mapColumns()}</h2>
      <p class="subtitle">
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
              <span class="field-label">SKU</span>
              <span class="req-badge">{$LL.csvImport.required()}</span>
            </td>
            <td>
              <select
                bind:value={selectedSku}
                class={selectedSku === "" ? "sel-error" : ""}
              >
                {#each skuOptions as opt}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            </td>
            <td class="preview">
              {selectedSku !== "" && headers[selectedSku] ? headers[selectedSku] : "—"}
            </td>
          </tr>

          <!-- Description -->
          <tr class={selectedDescription === "" ? "req-row" : ""}>
            <td>
              <span class="field-label">{$LL.csvImport.description()}</span>
              <span class="req-badge">{$LL.csvImport.required()}</span>
            </td>
            <td>
              <select
                bind:value={selectedDescription}
                class={selectedDescription === "" ? "sel-error" : ""}
              >
                {#each descOptions as opt}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            </td>
            <td class="preview">
              {selectedDescription !== "" && headers[selectedDescription]
                ? headers[selectedDescription]
                : "—"}
            </td>
          </tr>

          <!-- UPC/Barcode -->
          <tr>
            <td><span class="field-label">{$LL.csvImport.upcBarcode()}</span></td>
            <td>
              <select bind:value={selectedBarcode}>
                {#each barcodeOptions as opt}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            </td>
            <td class="preview">
              {selectedBarcode !== "" && headers[selectedBarcode]
                ? headers[selectedBarcode]
                : "—"}
            </td>
          </tr>

          <!-- Category -->
          <tr>
            <td><span class="field-label">{$LL.csvImport.categoryName()}</span></td>
            <td>
              <select bind:value={selectedCategory}>
                {#each categoryOptions as opt}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            </td>
            <td class="preview">
              {selectedCategory !== "" && headers[selectedCategory]
                ? headers[selectedCategory]
                : "—"}
            </td>
          </tr>

          <!-- Unit -->
          <tr>
            <td><span class="field-label">{$LL.csvImport.unit()}</span></td>
            <td>
              <select bind:value={selectedUnit}>
                {#each unitOptions as opt}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            </td>
            <td class="preview">
              {selectedUnit !== "" && headers[selectedUnit] ? headers[selectedUnit] : "—"}
            </td>
          </tr>

          <!-- Alert Days -->
          <tr>
            <td><span class="field-label">{$LL.csvImport.alertDays()}</span></td>
            <td>
              <select bind:value={selectedAlertDays}>
                {#each alertOptions as opt}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            </td>
            <td class="preview">
              {selectedAlertDays !== "" && headers[selectedAlertDays]
                ? headers[selectedAlertDays]
                : "—"}
            </td>
          </tr>

          <!-- Notes -->
          <tr>
            <td><span class="field-label">{$LL.csvImport.notes()}</span></td>
            <td>
              <select bind:value={selectedNotes}>
                {#each notesOptions as opt}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            </td>
            <td class="preview">
              {selectedNotes !== "" && headers[selectedNotes] ? headers[selectedNotes] : "—"}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <footer class="modal-footer">
      <button class="btn-cancel" on:click={onCancel}>{$LL.common.cancel()}</button>
      <button
        class="btn-apply"
        disabled={!isValid}
        on:click={handleApply}
      >
        {$LL.csvImport.previewImport()}
      </button>
    </footer>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .modal {
    background: #fff;
    border-radius: 10px;
    width: min(680px, 95vw);
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.15);
  }

  .modal-header {
    padding: 20px 24px 12px;
    border-bottom: 1px solid #e5e7eb;
    flex-shrink: 0;
  }

  .modal-header h2 {
    margin: 0 0 4px;
    font-size: 1.1rem;
    font-weight: 600;
    color: #1e293b;
  }

  .subtitle {
    margin: 0;
    font-size: 0.85rem;
    color: #64748b;
  }

  .table-wrap {
    flex: 1;
    overflow-y: auto;
    padding: 16px 24px;
  }

  .map-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }

  .map-table th {
    text-align: left;
    padding: 6px 8px;
    color: #64748b;
    font-weight: 500;
    font-size: 0.8rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .map-table td {
    padding: 7px 8px;
    border-bottom: 1px solid #f1f5f9;
    vertical-align: middle;
  }

  .req-row td {
    background: #fff8f8;
  }

  .field-label {
    font-weight: 500;
    color: #334155;
    display: block;
  }

  .req-badge {
    font-size: 0.7rem;
    color: #dc2626;
    font-weight: 500;
    display: block;
  }

  select {
    width: 100%;
    padding: 5px 8px;
    border: 1px solid #d1d5db;
    border-radius: 5px;
    font-size: 0.875rem;
    background: #fff;
    color: #1e293b;
  }

  select.sel-error {
    border-color: #dc2626;
    background: #fef2f2;
  }

  .preview {
    color: #94a3b8;
    font-size: 0.8rem;
    font-style: italic;
    white-space: nowrap;
  }

  .modal-footer {
    padding: 14px 24px;
    border-top: 1px solid #e5e7eb;
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    flex-shrink: 0;
  }

  .btn-apply,
  .btn-cancel {
    padding: 7px 18px;
    border-radius: 6px;
    font-size: 0.875rem;
    font-family: inherit;
    cursor: pointer;
    border: none;
    transition: background 0.15s;
  }

  .btn-apply {
    background: #2563eb;
    color: #fff;
  }

  .btn-apply:hover:not(:disabled) {
    background: #1d4ed8;
  }

  .btn-apply:disabled {
    background: #93c5fd;
    cursor: not-allowed;
  }

  .btn-cancel {
    background: #f1f5f9;
    color: #475569;
    border: 1px solid #e2e8f0;
  }

  .btn-cancel:hover {
    background: #e2e8f0;
  }
</style>
