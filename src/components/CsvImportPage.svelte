<!--
  CsvImportPage.svelte — CSV import stage machine (PR 8b of
  caduxo-daisyui-redesign).

  Migration to shared UI primitives:
    - Button.svelte for every stage button (select file / choose
      different / import / import another).
    - Alert.svelte for the error banner (`.error-banner`) and the
      tip box (`.hint-box` → info variant).
    - DaisyUI `radio radio-primary radio-sm` wrappers for the
      conflict-strategy radios (native <input> stays in the DOM for
      form semantics).
    - The action-card clickable surface (`.action-card.primary` +
      `.action-card.info`) becomes a Card.svelte clickable surface
      with DaisyUI primitives.
    - The summary / result card grids keep their bespoke layout —
      PR 9 owns the data-table migration for the preview + import
      log tables.

  Tailwind classes referenced here (for the JIT scanner):
    btn btn-primary btn-secondary btn-ghost btn-sm
    card card-body bg-base-100 bg-base-200
    alert alert-error alert-info alert-warning alert-success alert-soft
    radio radio-primary radio-sm
    flex items-center gap-2
-->
<script lang="ts">
  import {
    type CsvColumnMapping,
    type CsvPreviewResponse,
    type CsvPreviewRow,
    type CsvPreviewRowStatus,
    type CsvImportResult,
    type CsvImportRowOutcome,
    type CsvImportRowResult,
    type ConflictStrategy,
    previewProductCsv,
    importProductCsv,
    readCsvText,
    pickCsvFile,
  } from "../lib/csv.js";
  import ColumnMapper from "./ColumnMapper.svelte";
  import Button from "./ui/Button.svelte";
  import Alert from "./ui/Alert.svelte";
  import { LL } from "../i18n/i18n-svelte.js";
  import { humanizeError } from "../lib/errors.js";

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
      errorMsg = humanizeError(err);
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
      errorMsg = humanizeError(err);
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
      errorMsg = humanizeError(err);
    } finally {
      importingCommit = false;
    }
  }

  // ── Helpers ─────────────────────────────────────────────────────────────

  /**
   * Stable reason codes emitted by the backend for CSV row reasons. Must
   * stay in lock-step with the `REASON_CODE_*` constants in
   * `src-tauri/src/services/csv_io.rs`. See the matching `csvImport.reasonCodes`
   * i18n namespace for the localized strings.
   */
  const REASON_CODES = {
    requiredFieldSku: "required_field_sku",
    requiredFieldDescription: "required_field_description",
    skuAlreadyExists: "sku_already_exists",
    skuConflictManual: "sku_conflict_manual",
    barcodeBelongsToOtherProduct: "barcode_belongs_to_other_product",
    barcodeConflictManual: "barcode_conflict_manual",
    alertDaysNegative: "alert_days_negative",
    alertDaysTooLarge: "alert_days_too_large",
    alertDaysRangeInvalid: "alert_days_range_invalid",
    dbError: "db_error",
    genericValidation: "generic_validation",
  } as const;

  /**
   * Resolves a backend reason code to a localized string, falling back to
   * the raw `fallback` text when the code is missing or unknown. Context
   * fields (`sku`, `barcode`) are interpolated into the matching
   * parameterized keys when present.
   */
  function localizeReason(
    code: string | null | undefined,
    fallback: string,
    ctx: { sku?: string; barcode?: string },
  ): string {
    switch (code) {
      case REASON_CODES.requiredFieldSku:
        return $LL.csvImport.reasonCodes.requiredFieldSku();
      case REASON_CODES.requiredFieldDescription:
        return $LL.csvImport.reasonCodes.requiredFieldDescription();
      case REASON_CODES.skuAlreadyExists:
        return $LL.csvImport.reasonCodes.skuAlreadyExists({
          sku: ctx.sku ?? "",
        });
      case REASON_CODES.skuConflictManual:
        return $LL.csvImport.reasonCodes.skuConflictManual();
      case REASON_CODES.barcodeBelongsToOtherProduct:
        return $LL.csvImport.reasonCodes.barcodeBelongsToOtherProduct({
          barcode: ctx.barcode ?? "",
        });
      case REASON_CODES.barcodeConflictManual:
        return $LL.csvImport.reasonCodes.barcodeConflictManual();
      case REASON_CODES.alertDaysNegative:
        return $LL.csvImport.reasonCodes.alertDaysNegative();
      case REASON_CODES.alertDaysTooLarge:
        return $LL.csvImport.reasonCodes.alertDaysTooLarge();
      case REASON_CODES.alertDaysRangeInvalid:
        return $LL.csvImport.reasonCodes.alertDaysRangeInvalid();
      case REASON_CODES.dbError:
        return $LL.csvImport.reasonCodes.dbError();
      case REASON_CODES.genericValidation:
        return $LL.csvImport.reasonCodes.genericValidation();
      default:
        return fallback;
    }
  }

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

  function outcomeReason(row: CsvImportRowResult): string {
    const outcome = row.outcome;
    if (outcome.action === "skipped") {
      return localizeReason(outcome.reason_code, outcome.reason, {
        sku: row.sku ?? "",
        barcode: row.barcode ?? "",
      });
    }
    if (outcome.action === "invalid") {
      return localizeReason(outcome.reason_code, outcome.reason, {
        sku: row.sku ?? "",
        barcode: row.barcode ?? "",
      });
    }
    if (outcome.action === "created") return $LL.csvImport.actions.skuCreated({ sku: outcome.sku });
    if (outcome.action === "updated") return $LL.csvImport.actions.skuUpdated({ sku: outcome.sku });
    return "";
  }

  function rowDetailMessage(row: CsvPreviewRow): string {
    const status = row.status;
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
      return localizeReason(status.reason_code, status.reason, {
        sku: row.sku ?? "",
        barcode: row.barcode ?? "",
      });
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
      <Alert variant="error">{errorMsg}</Alert>
    {/if}

    <div class="action-cards">
      <button
        type="button"
        class="action-card action-card-primary"
        on:click={selectAndPreview}
      >
        <span class="card-icon" aria-hidden="true">📄</span>
        <span class="card-title">{$LL.csvImport.selectFile()}</span>
        <span class="card-desc">{$LL.csvImport.selectFileAction()}</span>
      </button>

      <div class="action-card action-card-info">
        <span class="card-icon" aria-hidden="true">ℹ️</span>
        <span class="card-title">{$LL.csvImport.expectedColumns()}</span>
        <span class="card-desc">
          {$LL.csvImport.required()}: <code>sku</code>, <code>description</code><br />
          {$LL.csvImport.optional()}: <code>barcode</code>, <code>category</code>,
          <code>unit</code>, <code>alert_days_before</code>, <code>notes</code>
        </span>
      </div>
    </div>

    <div class="banner-stack">
      <Alert variant="info" role="status">
        <strong>{$LL.csvImport.tip()}</strong> {$LL.csvImport.tipText()}
      </Alert>
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
        <Button variant="secondary" size="sm" onclick={() => selectAndPreview()}>
          {$LL.csvImport.chooseDifferentFile()}
        </Button>
      </div>
    </div>

    {#if errorMsg}
      <Alert variant="error">{errorMsg}</Alert>
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
            <input
              type="radio"
              class="radio radio-primary radio-sm"
              name="strategy"
              value={s.value}
              bind:group={selectedStrategy}
            />
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
                {@const detailMsg = rowDetailMessage(row)}
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
      <Button
        variant="primary"
        size="md"
        disabled={importingCommit || counts.valid === 0}
        loading={importingCommit}
        onclick={handleImport}
      >
        {importingCommit ? $LL.csvImport.importing() : $LL.csvImport.importButton({ n: counts.valid })}
      </Button>
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
      <Alert variant="error">{errorMsg}</Alert>
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
                  <td class="detail-cell">{outcomeReason(row)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {/if}

    <div class="import-actions">
      <Button variant="secondary" size="md" onclick={() => (stage = { name: "select" })}>
        {$LL.csvImport.importAnotherFile()}
      </Button>
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
    color: var(--color-base-content);
  }

  .page-desc {
    margin: 6px 0 0;
    color: var(--color-secondary);
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
    background: var(--color-base-100);
    border: 1px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
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
    color: var(--color-base-content);
  }

  .action-card-primary {
    border-color: var(--color-primary);
    background: color-mix(in oklch, var(--color-primary) 6%, var(--color-base-100));
  }

  .action-card-primary:hover {
    box-shadow: 0 2px 12px color-mix(in oklch, var(--color-primary) 22%, transparent);
  }

  .action-card-info {
    background: color-mix(in oklch, var(--color-base-200) 70%, transparent);
    color: var(--color-secondary);
  }

  .card-icon {
    font-size: 1.4rem;
  }

  .card-title {
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--color-base-content);
  }

  .action-card-info .card-title {
    color: var(--color-secondary);
  }

  .card-desc {
    font-size: 0.8rem;
    color: var(--color-secondary);
    line-height: 1.5;
  }

  .banner-stack {
    margin-bottom: 20px;
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
    background: var(--color-base-100);
    border: 1px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
    border-radius: 10px;
    padding: 14px 20px;
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 90px;
  }

  .summary-card.ok,
  .result-card.ok {
    border-color: color-mix(in oklch, var(--color-success) 50%, transparent);
    background: color-mix(in oklch, var(--color-success) 8%, transparent);
  }

  .summary-card.warn,
  .result-card.warn {
    border-color: color-mix(in oklch, var(--color-warning) 50%, transparent);
    background: color-mix(in oklch, var(--color-warning) 10%, transparent);
  }

  .summary-card.error,
  .result-card.error {
    border-color: color-mix(in oklch, var(--color-error) 50%, transparent);
    background: color-mix(in oklch, var(--color-error) 8%, transparent);
  }

  .result-card.info {
    border-color: color-mix(in oklch, var(--color-info) 50%, transparent);
    background: color-mix(in oklch, var(--color-info) 8%, transparent);
  }

  .summary-num,
  .result-num {
    font-size: 1.8rem;
    font-weight: 700;
    line-height: 1;
    color: var(--color-base-content);
  }

  .summary-label,
  .result-label {
    font-size: 0.75rem;
    color: var(--color-secondary);
    margin-top: 4px;
  }

  /* Sections */
  .section {
    margin-bottom: 24px;
  }

  .section h2 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-secondary);
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
    background: var(--color-base-100);
    border: 2px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
    border-radius: 8px;
    padding: 12px 14px;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
    color: var(--color-base-content);
  }

  .strategy-card.selected {
    border-color: var(--color-primary);
    background: color-mix(in oklch, var(--color-primary) 6%, transparent);
  }

  .strategy-label {
    display: block;
    font-weight: 600;
    font-size: 0.875rem;
    color: var(--color-base-content);
    margin-bottom: 4px;
  }

  .strategy-desc {
    font-size: 0.8rem;
    color: var(--color-secondary);
    line-height: 1.4;
  }

  /* Preview table */
  .table-wrap {
    overflow-x: auto;
    border: 1px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
    border-radius: 8px;
    margin-bottom: 20px;
  }

  .preview-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .preview-table th {
    background: color-mix(in oklch, var(--color-base-200) 70%, transparent);
    padding: 8px 10px;
    text-align: left;
    font-weight: 500;
    color: var(--color-secondary);
    border-bottom: 1px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
    white-space: nowrap;
  }

  .preview-table td {
    padding: 7px 10px;
    border-bottom: 1px solid color-mix(in oklch, var(--color-base-200) 90%, transparent);
    vertical-align: middle;
    color: var(--color-base-content);
  }

  .preview-table tr:last-child td {
    border-bottom: none;
  }

  .preview-table tr.badge-ok {
    background: color-mix(in oklch, var(--color-success) 6%, transparent);
  }

  .preview-table tr.badge-warn {
    background: color-mix(in oklch, var(--color-warning) 10%, transparent);
  }

  .preview-table tr.badge-error {
    background: color-mix(in oklch, var(--color-error) 8%, transparent);
  }

  .row-num {
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    font-size: 0.8rem;
    text-align: right;
    width: 36px;
  }

  .cell-mono {
    font-family: monospace;
    font-size: 0.85rem;
    color: var(--color-base-content);
  }

  .cell-muted {
    color: var(--color-secondary);
  }

  .detail-cell {
    font-size: 0.8rem;
    color: var(--color-secondary);
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
    background: color-mix(in oklch, var(--color-success) 18%, transparent);
    color: var(--color-success);
  }

  .badge-warn {
    background: color-mix(in oklch, var(--color-warning) 22%, transparent);
    color: var(--color-warning);
  }

  .badge-error {
    background: color-mix(in oklch, var(--color-error) 18%, transparent);
    color: var(--color-error);
  }

  .badge-info {
    background: color-mix(in oklch, var(--color-info) 18%, transparent);
    color: var(--color-info);
  }

  /* Import actions */
  .import-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
  }

  code {
    background: color-mix(in oklch, var(--color-base-200) 70%, transparent);
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 0.85em;
    color: var(--color-secondary);
  }
</style>
