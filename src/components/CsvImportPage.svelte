<script lang="ts">
  import {
    type CsvColumnMapping,
    type CsvPreviewResponse,
    type CsvPreviewRowStatus,
    type CsvImportResult,
    type CsvImportRowOutcome,
    type ConflictStrategy,
    previewProductCsv,
    importProductCsv,
    readCsvText,
    pickCsvFile,
  } from "../lib/csv.js";
  import ColumnMapper from "./ColumnMapper.svelte";
  import { LL } from "../i18n/i18n-svelte.js";

  // ── Stage machine ────────────────────────────────────────────────────────

  type Stage =
    | { name: "select" }
    | { name: "mapping"; headers: string[]; mapping: CsvColumnMapping; filePath: string }
    | { name: "preview"; preview: CsvPreviewResponse; mapping: CsvColumnMapping; filePath: string }
    | { name: "result"; result: CsvImportResult; strategy: ConflictStrategy };

  let stage: Stage = { name: "select" };
  let errorMsg = "";
  let selectedStrategy: ConflictStrategy = "skip";
  let importingCommit = false;

  // ── File selection ─────────────────────────────────────────────────────

  async function selectAndPreview() {
    errorMsg = "";
    const path = await pickCsvFile();
    if (!path) return;

    try {
      const content = await readCsvText(path);
      const preview = await previewProductCsv({ content });

      stage = {
        name: "mapping",
        headers: preview.headers,
        mapping: preview.mapping_used,
        filePath: path,
      };
    } catch (err) {
      errorMsg = String(err);
    }
  }

  // ── Column mapping → preview ─────────────────────────────────────────────

  async function handleMappingConfirm(mapping: CsvColumnMapping) {
    errorMsg = "";
    if (stage.name !== "mapping") return;
    const path = stage.filePath;

    try {
      const content = await readCsvText(path);
      const preview = await previewProductCsv({ content, mapping });
      stage = {
        name: "preview",
        preview,
        mapping,
        filePath: path,
      };
    } catch (err) {
      errorMsg = String(err);
    }
  }

  // ── Import commit ───────────────────────────────────────────────────────

  async function handleImport() {
    if (stage.name !== "preview") return;
    importingCommit = true;
    errorMsg = "";

    try {
      const content = await readCsvText(stage.filePath);
      const result: CsvImportResult = await importProductCsv({
        content,
        mapping: stage.mapping,
        strategy: selectedStrategy,
      });
      stage = { name: "result", result, strategy: selectedStrategy };
    } catch (err) {
      errorMsg = String(err);
    } finally {
      importingCommit = false;
    }
  }

  // ── Helpers ─────────────────────────────────────────────────────────────

  function rowBadge(status: CsvPreviewRowStatus): { label: string; cls: string } {
    if (status.kind === "ok") return { label: $LL.csvImport.badge.ok(), cls: "badge-ok" };
    if (status.kind === "duplicate_sku") return { label: $LL.csvImport.badge.dupSku(), cls: "badge-warn" };
    if (status.kind === "duplicate_barcode") return { label: $LL.csvImport.badge.dupBarcode(), cls: "badge-warn" };
    if (status.kind === "missing_required") return { label: $LL.csvImport.badge.missing(), cls: "badge-error" };
    if (status.kind === "invalid") return { label: $LL.csvImport.badge.invalid(), cls: "badge-error" };
    if (status.kind === "unknown_unit") return { label: $LL.csvImport.badge.unknownUnit(), cls: "badge-warn" };
    return { label: "?", cls: "badge-error" };
  }

  function outcomeBadge(outcome: CsvImportRowOutcome): { label: string; cls: string } {
    if (outcome.action === "created") return { label: $LL.csvImport.actions.created(), cls: "badge-ok" };
    if (outcome.action === "updated") return { label: $LL.csvImport.actions.updated(), cls: "badge-info" };
    if (outcome.action === "skipped") return { label: $LL.csvImport.actions.skipped(), cls: "badge-warn" };
    if (outcome.action === "invalid") return { label: $LL.csvImport.actions.invalid(), cls: "badge-error" };
    return { label: "?", cls: "badge-error" };
  }

  function outcomeReason(outcome: CsvImportRowOutcome): string {
    if (outcome.action === "skipped") return outcome.reason;
    if (outcome.action === "invalid") return outcome.reason;
    if (outcome.action === "created") return $LL.csvImport.actions.skuCreated({ sku: outcome.sku });
    if (outcome.action === "updated") return $LL.csvImport.actions.skuUpdated({ sku: outcome.sku });
    return "";
  }

  function rowDetailMessage(status: CsvPreviewRowStatus): string {
    if (status.kind === "duplicate_sku") {
      return $LL.csvImport.detailRow.alreadyHasSku({ sku: status.existing_sku });
    }
    if (status.kind === "duplicate_barcode") {
      return $LL.csvImport.detailRow.barcodeBelongsToOther({ bc: status.existing_barcode });
    }
    if (status.kind === "missing_required") {
      return $LL.csvImport.detailRow.missingField({ field: status.field });
    }
    if (status.kind === "invalid") {
      return status.reason;
    }
    if (status.kind === "unknown_unit") {
      const suggested = status.suggested_keys.join(", ");
      return $LL.csvImport.detailRow.unknownUnitSuggest({ suggested: suggested || $LL.csvImport.badge.unknownUnit() });
    }
    return $LL.csvImport.detailRow.readyToImport();
  }

  const strategies: Array<{ value: ConflictStrategy; label: string; desc: string }> = [
    {
      value: "skip",
      label: $LL.csvImport.conflictOptions.skipDuplicates(),
      desc: $LL.csvImport.conflictOptions.skipDuplicatesDesc(),
    },
    {
      value: "update",
      label: $LL.csvImport.conflictOptions.updateExisting(),
      desc: $LL.csvImport.conflictOptions.updateExistingDesc(),
    },
    {
      value: "review",
      label: $LL.csvImport.conflictOptions.reviewConflicts(),
      desc: $LL.csvImport.conflictOptions.reviewConflictsDesc(),
    },
  ];

  // ── Derived preview counts ───────────────────────────────────────────────

  $: previewCounts =
    stage.name === "preview"
      ? {
          total: stage.preview.total_rows,
          valid: stage.preview.valid_rows,
          dupSku: stage.preview.duplicate_sku_count,
          dupBc: stage.preview.duplicate_barcode_count,
          missing: stage.preview.missing_required_count,
        }
      : { total: 0, valid: 0, dupSku: 0, dupBc: 0, missing: 0 };
</script>

<!-- ── Stage: Select file ─────────────────────────────────────────────────── -->
{#if stage.name === "select"}
  <div class="page">
    <div class="page-header">
      <h1>{$LL.csvImport.pageTitle()}</h1>
      <p class="page-desc">
        {$LL.csvImport.selectFileDesc()}
      </p>
    </div>

    {#if errorMsg}
      <div class="error-banner">{errorMsg}</div>
    {/if}

    <div class="action-cards">
      <button class="action-card primary" on:click={selectAndPreview}>
        <span class="card-icon">📄</span>
        <span class="card-title">{$LL.csvImport.selectFile()}</span>
        <span class="card-desc">{$LL.csvImport.selectFileAction()}</span>
      </button>

      <div class="action-card info">
        <span class="card-icon">ℹ️</span>
        <span class="card-title">{$LL.csvImport.expectedColumns()}</span>
        <span class="card-desc">
          {$LL.csvImport.required()}: <code>sku</code>, <code>description</code><br />
          {$LL.csvImport.optional()}: <code>barcode</code>, <code>category</code>,
          <code>unit</code>, <code>alert_days_before</code>, <code>notes</code>
        </span>
      </div>
    </div>

    <div class="hint-box">
      <strong>{$LL.csvImport.tip()}</strong> {$LL.csvImport.tipText()}
    </div>
  </div>

<!-- ── Stage: Column mapping ─────────────────────────────────────────────── -->
{:else if stage.name === "mapping"}
  <ColumnMapper
    headers={stage.headers}
    initialMapping={stage.mapping}
    onApply={handleMappingConfirm}
    onCancel={() => (stage = { name: "select" })}
  />

<!-- ── Stage: Preview ────────────────────────────────────────────────────── -->
{:else if stage.name === "preview"}
  {@const counts = previewCounts}
  {@const preview = stage.preview}
  <div class="page">
    <div class="page-header">
      <h1>{$LL.csvImport.importPreview()}</h1>
      <div class="header-actions">
        <button class="btn-secondary" on:click={() => selectAndPreview()}>
          {$LL.csvImport.chooseDifferentFile()}
        </button>
      </div>
    </div>

    {#if errorMsg}
      <div class="error-banner">{errorMsg}</div>
    {/if}

    <!-- Summary cards -->
    <div class="summary-cards">
      <div class="summary-card">
        <span class="summary-num">{counts.total}</span>
        <span class="summary-label">{$LL.csvImport.totalRows()}</span>
      </div>
      <div class="summary-card ok">
        <span class="summary-num">{counts.valid}</span>
        <span class="summary-label">{$LL.csvImport.valid()}</span>
      </div>
      {#if counts.dupSku > 0}
        <div class="summary-card warn">
          <span class="summary-num">{counts.dupSku}</span>
          <span class="summary-label">{$LL.csvImport.duplicates.sku()}</span>
        </div>
      {/if}
      {#if counts.dupBc > 0}
        <div class="summary-card warn">
          <span class="summary-num">{counts.dupBc}</span>
          <span class="summary-label">{$LL.csvImport.duplicates.barcode()}</span>
        </div>
      {/if}
      {#if counts.missing > 0}
        <div class="summary-card error">
          <span class="summary-num">{counts.missing}</span>
          <span class="summary-label">{$LL.csvImport.missing()}</span>
        </div>
      {/if}
    </div>

    <!-- Conflict strategy -->
    <section class="section">
      <h2>{$LL.csvImport.conflictStrategy()}</h2>
      <div class="strategy-cards">
        {#each strategies as s}
          <label class="strategy-card" class:selected={selectedStrategy === s.value}>
            <input type="radio" name="strategy" value={s.value} bind:group={selectedStrategy} />
            <span class="strategy-label">{s.label}</span>
            <span class="strategy-desc">{s.desc}</span>
          </label>
        {/each}
      </div>
    </section>

    <!-- Row detail table -->
    {#if preview.rows.length > 0}
      <section class="section">
        <h2>{$LL.csvImport.rowDetails()}</h2>
        <div class="table-wrap">
          <table class="preview-table">
            <thead>
              <tr>
                <th>#</th>
                <th>{$LL.csvImport.sku()}</th>
                <th>{$LL.csvImport.description()}</th>
                <th>{$LL.csvImport.barcode()}</th>
                <th>{$LL.csvImport.status()}</th>
                <th>{$LL.csvImport.detail()}</th>
              </tr>
            </thead>
            <tbody>
              {#each preview.rows as row}
                {@const badge = rowBadge(row.status)}
                {@const detailMsg = rowDetailMessage(row.status)}
                <tr class={badge.cls}>
                  <td class="row-num">{row.row_index}</td>
                  <td class="cell-mono">{row.sku ?? "—"}</td>
                  <td>{row.description ?? "—"}</td>
                  <td class="cell-mono cell-muted">{row.barcode ?? "—"}</td>
                  <td><span class="badge {badge.cls}">{badge.label}</span></td>
                  <td class="detail-cell">{@html detailMsg}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {/if}

    <!-- Import button -->
    <div class="import-actions">
      <button
        class="btn-primary"
        disabled={importingCommit || counts.valid === 0}
        on:click={handleImport}
      >
        {importingCommit ? $LL.csvImport.importing() : $LL.csvImport.importButton({ n: counts.valid })}
      </button>
    </div>
  </div>

<!-- ── Stage: Result ────────────────────────────────────────────────────── -->
{:else if stage.name === "result"}
  {@const result = stage.result}
  <div class="page">
    <div class="page-header">
      <h1>{$LL.csvImport.importComplete()}</h1>
    </div>

    {#if errorMsg}
      <div class="error-banner">{errorMsg}</div>
    {/if}

    <div class="result-cards">
      <div class="result-card ok">
        <span class="result-num">{result.created}</span>
        <span class="result-label">{$LL.csvImport.created()}</span>
      </div>
      {#if stage.strategy === "update" && result.updated > 0}
        <div class="result-card info">
          <span class="result-num">{result.updated}</span>
          <span class="result-label">{$LL.csvImport.updated()}</span>
        </div>
      {/if}
      {#if result.skipped > 0}
        <div class="result-card warn">
          <span class="result-num">{result.skipped}</span>
          <span class="result-label">{$LL.csvImport.skipped()}</span>
        </div>
      {/if}
      {#if result.invalid > 0}
        <div class="result-card error">
          <span class="result-num">{result.invalid}</span>
          <span class="result-label">{$LL.csvImport.invalid()}</span>
        </div>
      {/if}
    </div>

    {#if result.rows.length > 0}
      <section class="section">
        <h2>{$LL.csvImport.importLog()}</h2>
        <div class="table-wrap">
          <table class="preview-table">
            <thead>
              <tr>
                <th>#</th>
                <th>{$LL.csvImport.sku()}</th>
                <th>{$LL.csvImport.description()}</th>
                <th>{$LL.csvImport.barcode()}</th>
                <th>{$LL.csvImport.status()}</th>
                <th>{$LL.csvImport.detail()}</th>
              </tr>
            </thead>
            <tbody>
              {#each result.rows as row}
                {@const badge = outcomeBadge(row.outcome)}
                <tr class={badge.cls}>
                  <td class="row-num">{row.row_index}</td>
                  <td class="cell-mono">{row.sku || "—"}</td>
                  <td>{row.description || "—"}</td>
                  <td class="cell-mono cell-muted">{row.barcode || "—"}</td>
                  <td><span class="badge {badge.cls}">{badge.label}</span></td>
                  <td class="detail-cell">{outcomeReason(row.outcome)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {/if}

    <div class="import-actions">
      <button class="btn-secondary" on:click={() => (stage = { name: "select" })}>
        {$LL.csvImport.importAnotherFile()}
      </button>
    </div>
  </div>
{/if}

<style>
  .page {
    padding: 24px;
    max-width: 960px;
    margin: 0 auto;
  }

  .page-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 20px;
  }

  .page-header h1 {
    margin: 0;
    font-size: 1.4rem;
    color: #1e293b;
  }

  .page-desc {
    margin: 6px 0 0;
    color: #64748b;
    font-size: 0.875rem;
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .action-cards {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 20px;
  }

  .action-card {
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    cursor: pointer;
    min-width: 220px;
    font-family: inherit;
    text-align: left;
    transition: box-shadow 0.15s, border-color 0.15s;
  }

  .action-card.primary {
    border-color: #2563eb;
    background: #eff6ff;
  }

  .action-card.primary:hover {
    box-shadow: 0 2px 12px rgba(37, 99, 235, 0.2);
  }

  .action-card.info {
    background: #f8fafc;
    color: #475569;
  }

  .card-icon {
    font-size: 1.4rem;
  }

  .card-title {
    font-weight: 600;
    font-size: 0.95rem;
    color: #1e293b;
  }

  .action-card.info .card-title {
    color: #475569;
  }

  .card-desc {
    font-size: 0.8rem;
    color: #64748b;
    line-height: 1.5;
  }

  .hint-box {
    background: #fffbeb;
    border: 1px solid #fcd34d;
    border-radius: 7px;
    padding: 12px 16px;
    font-size: 0.85rem;
    color: #78350f;
    margin-bottom: 20px;
  }

  .error-banner {
    background: #fef2f2;
    border: 1px solid #fca5a5;
    color: #991b1b;
    padding: 10px 14px;
    border-radius: 7px;
    margin-bottom: 16px;
    font-size: 0.875rem;
  }

  /* Summary cards */
  .summary-cards,
  .result-cards {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 24px;
  }

  .summary-card,
  .result-card {
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    padding: 14px 20px;
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 90px;
  }

  .summary-card.ok,
  .result-card.ok {
    border-color: #86efac;
    background: #f0fdf4;
  }

  .summary-card.warn,
  .result-card.warn {
    border-color: #fcd34d;
    background: #fffbeb;
  }

  .summary-card.error,
  .result-card.error {
    border-color: #fca5a5;
    background: #fef2f2;
  }

  .result-card.info {
    border-color: #93c5fd;
    background: #eff6ff;
  }

  .summary-num,
  .result-num {
    font-size: 1.8rem;
    font-weight: 700;
    line-height: 1;
    color: #1e293b;
  }

  .summary-label,
  .result-label {
    font-size: 0.75rem;
    color: #64748b;
    margin-top: 4px;
  }

  /* Sections */
  .section {
    margin-bottom: 24px;
  }

  .section h2 {
    font-size: 1rem;
    font-weight: 600;
    color: #334155;
    margin: 0 0 12px;
  }

  /* Strategy cards */
  .strategy-cards {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }

  .strategy-card {
    flex: 1;
    min-width: 180px;
    background: #fff;
    border: 2px solid #e2e8f0;
    border-radius: 8px;
    padding: 12px 14px;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }

  .strategy-card input[type="radio"] {
    display: none;
  }

  .strategy-card.selected {
    border-color: #2563eb;
    background: #eff6ff;
  }

  .strategy-label {
    display: block;
    font-weight: 600;
    font-size: 0.875rem;
    color: #1e293b;
    margin-bottom: 4px;
  }

  .strategy-desc {
    font-size: 0.8rem;
    color: #64748b;
    line-height: 1.4;
  }

  /* Preview table */
  .table-wrap {
    overflow-x: auto;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    margin-bottom: 20px;
  }

  .preview-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .preview-table th {
    background: #f8fafc;
    padding: 8px 10px;
    text-align: left;
    font-weight: 500;
    color: #475569;
    border-bottom: 1px solid #e2e8f0;
    white-space: nowrap;
  }

  .preview-table td {
    padding: 7px 10px;
    border-bottom: 1px solid #f1f5f9;
    vertical-align: middle;
  }

  .preview-table tr:last-child td {
    border-bottom: none;
  }

  .preview-table tr.badge-ok {
    background: #f0fdf4;
  }

  .preview-table tr.badge-warn {
    background: #fffbeb;
  }

  .preview-table tr.badge-error {
    background: #fef2f2;
  }

  .row-num {
    color: #94a3b8;
    font-size: 0.8rem;
    text-align: right;
    width: 36px;
  }

  .cell-mono {
    font-family: monospace;
    font-size: 0.85rem;
    color: #1e293b;
  }

  .cell-muted {
    color: #64748b;
  }

  .detail-cell {
    font-size: 0.8rem;
    color: #475569;
    max-width: 280px;
  }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 500;
    white-space: nowrap;
  }

  .badge-ok {
    background: #dcfce7;
    color: #166534;
  }

  .badge-warn {
    background: #fef9c3;
    color: #854d0e;
  }

  .badge-error {
    background: #fee2e2;
    color: #991b1b;
  }

  .badge-info {
    background: #dbeafe;
    color: #1e40af;
  }

  /* Import actions */
  .import-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
  }

  /* Buttons */
  .btn-primary,
  .btn-secondary {
    padding: 8px 20px;
    border-radius: 7px;
    font-size: 0.875rem;
    font-family: inherit;
    cursor: pointer;
    border: none;
    transition: background 0.15s;
  }

  .btn-primary {
    background: #2563eb;
    color: #fff;
  }

  .btn-primary:hover:not(:disabled) {
    background: #1d4ed8;
  }

  .btn-primary:disabled {
    background: #93c5fd;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: #f1f5f9;
    color: #475569;
    border: 1px solid #e2e8f0;
  }

  .btn-secondary:hover {
    background: #e2e8f0;
  }

  code {
    background: #f1f5f9;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 0.85em;
    color: #475569;
  }
</style>
