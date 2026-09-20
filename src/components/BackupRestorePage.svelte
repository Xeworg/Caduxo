<!--
  BackupRestorePage.svelte — backup export + restore validation flow
  (PR 8b of caduxo-daisyui-redesign).

  Migration to shared UI primitives:
    - Button.svelte for every action button (export / validate /
      restore / choose different / cancel).
    - Alert.svelte for the export success / export error / validation
      error / restore error surfaces (replaces `.message.success`
      and `.message.error` divs).
    - Alert.svelte for the destructive-operation warning banner
      (replaces `.warning-banner`).
    - The validation summary list (`.checks-list`) and the
      destructive confirm block (`.confirm-box`) preserve their
      bespoke layout because they are the canonical restore-confirmation
      surface (the surrounding buttons, alerts, and section chrome
      all migrate to primitives).
    - No input / select / radio controls exist on this page; PR 9
      owns the info-list (`.info-list`) and checks-list (`.checks-list`)
      data-table migration.

  Tailwind classes referenced here (for the JIT scanner):
    btn btn-primary btn-secondary btn-danger btn-ghost btn-sm
    alert alert-success alert-error alert-warning alert-soft
    flex items-center gap-2
-->
<script lang="ts">
  import {
    exportBackupWithDialog,
    validateBackupWithDialog,
    restoreBackup,
    type RestoreValidation,
  } from "../lib/backup_restore.js";
  import Button from "./ui/Button.svelte";
  import Alert from "./ui/Alert.svelte";
  import { LL } from "../i18n/i18n-svelte.js";
  import { humanizeError } from "../lib/errors.js";

  // ─── State ────────────────────────────────────────────────────────────────

  let exporting = false;
  let exportError = "";
  let exportSuccess = "";

  let validating = false;
  let validation: RestoreValidation | null = null;
  let selectedBackupPath = "";
  let validationError = "";

  let restoring = false;
  let restoreError = "";
  let showRestoreConfirm = false;

  // ─── Handlers ─────────────────────────────────────────────────────────────

  async function handleExport() {
    exporting = true;
    exportError = "";
    exportSuccess = "";

    try {
      const result = await exportBackupWithDialog();
      if (result) {
        const kb = (result.bytes / 1024).toFixed(1);
        exportSuccess = $LL.backupRestore.exportSuccess({
          path: result.path,
          kb,
          schema: result.schemaVersion,
        });
      }
      // null means cancelled
    } catch (e) {
      exportError = humanizeError(e);
    } finally {
      exporting = false;
    }
  }

  async function handleValidate() {
    validating = true;
    validation = null;
    selectedBackupPath = "";
    validationError = "";
    showRestoreConfirm = false;
    restoreError = "";

    try {
      const result = await validateBackupWithDialog();
      if (result) {
        selectedBackupPath = result.path;
        validation = result.validation;
        if (!result.validation.canRestore) {
          showRestoreConfirm = false;
        }
      }
    } catch (e) {
      validationError = humanizeError(e);
    } finally {
      validating = false;
    }
  }

  function openRestoreConfirm() {
    showRestoreConfirm = true;
    restoreError = "";
  }

  async function handleRestore() {
    restoring = true;
    restoreError = "";

    try {
      await restoreBackup({
        backupPath: selectedBackupPath,
        confirmed: true,
      });
      // On success, reload the page to reflect restored data
      window.location.reload();
    } catch (e) {
      restoreError = humanizeError(e);
    } finally {
      restoring = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  /**
   * Stable check codes emitted by the backend. Must stay in lock-step with
   * the `CHECK_CODE_*` constants in
   * `src-tauri/src/services/backup_restore.rs`. See the matching
   * `backupRestore.checks` i18n namespace for the localized strings.
   */
  const CHECK_CODES = {
    fileNotFound: "file_not_found",
    sqliteHeaderValid: "sqlite_header_valid",
    sqliteHeaderInvalid: "sqlite_header_invalid",
    requiredTablesPresent: "required_tables_present",
    requiredTablesMissing: "required_tables_missing",
    schemaVersionDetected: "schema_version_detected",
    integrityCheckOk: "integrity_check_ok",
    integrityCheckFailed: "integrity_check_failed",
    schemaCheckFailed: "schema_check_failed",
    schemaVersionInvalid: "schema_version_invalid",
    schemaVersionCompatible: "schema_version_compatible",
  } as const;

  /**
   * Number of tables the backend requires in a Caduxo backup. Mirrors
   * `REQUIRED_TABLES.len()` in `services::backup_restore`; kept as a
   * constant here so the localized "All N required tables present" message
   * shows the same number the backend emits in the English fallback.
   */
  const REQUIRED_TABLES_COUNT = 8;

  /**
   * Resolves a backend check code to a localized string, falling back to
   * the raw `fallback` text when the code is missing or unknown. Schema
   * version placeholders are extracted from the fallback so the localized
   * "Schema version: N" message carries the same N the backend reports.
   */
  function localizeCheck(
    code: string | null | undefined,
    fallback: string,
  ): string {
    switch (code) {
      case CHECK_CODES.fileNotFound:
        return $LL.backupRestore.checks.fileNotFound();
      case CHECK_CODES.sqliteHeaderValid:
        return $LL.backupRestore.checks.sqliteHeaderValid();
      case CHECK_CODES.sqliteHeaderInvalid:
        return $LL.backupRestore.checks.sqliteHeaderInvalid();
      case CHECK_CODES.requiredTablesPresent:
        return $LL.backupRestore.checks.requiredTablesPresent({
          n: REQUIRED_TABLES_COUNT,
        });
      case CHECK_CODES.requiredTablesMissing:
        return $LL.backupRestore.checks.requiredTablesMissing();
      case CHECK_CODES.schemaVersionDetected: {
        const match = fallback.match(/Schema version:\s*(-?\d+)/);
        return $LL.backupRestore.checks.schemaVersionDetected({
          v: match?.[1] ?? "0",
        });
      }
      case CHECK_CODES.integrityCheckOk:
        return $LL.backupRestore.checks.integrityCheckOk();
      case CHECK_CODES.integrityCheckFailed:
        return $LL.backupRestore.checks.integrityCheckFailed();
      case CHECK_CODES.schemaCheckFailed:
        return $LL.backupRestore.checks.schemaCheckFailed();
      case CHECK_CODES.schemaVersionInvalid: {
        const match = fallback.match(/Schema version\s*(-?\d+)/);
        return $LL.backupRestore.checks.schemaVersionInvalid({
          v: match?.[1] ?? "0",
        });
      }
      case CHECK_CODES.schemaVersionCompatible:
        return $LL.backupRestore.checks.schemaVersionCompatible();
      default:
        return fallback;
    }
  }
</script>

<div class="page">
  <h1>{$LL.backupRestore.pageTitle()}</h1>

  <!-- ─── Export ─────────────────────────────────────────────────────────── -->
  <section class="card">
    <h2>{$LL.backupRestore.exportSection()}</h2>
    <p>{$LL.backupRestore.exportDesc()}</p>

    <Button
      variant="primary"
      size="sm"
      onclick={handleExport}
      disabled={exporting}
      loading={exporting}
    >
      {exporting ? $LL.backupRestore.exporting() : $LL.backupRestore.exportButton()}
    </Button>

    {#if exportSuccess}
      <div class="status-stack">
        <Alert variant="success" role="status">{exportSuccess}</Alert>
      </div>
    {/if}

    {#if exportError}
      <div class="status-stack">
        <Alert variant="error">{exportError}</Alert>
      </div>
    {/if}
  </section>

  <!-- ─── Restore ───────────────────────────────────────────────────────── -->
  <section class="card">
    <h2>{$LL.backupRestore.restoreSection()}</h2>
    <p>
      {$LL.backupRestore.restoreDesc()}
      <strong>{$LL.backupRestore.destructiveOp()}</strong>
      {$LL.backupRestore.restoreWarning()}
    </p>

    <div class="banner-stack">
      <Alert variant="warning" role="alert">
        {$LL.backupRestore.restoreDangerBanner()}
      </Alert>
    </div>

    {#if !validation}
      <Button
        variant="secondary"
        size="sm"
        onclick={handleValidate}
        disabled={validating}
        loading={validating}
      >
        {validating ? $LL.backupRestore.selecting() : $LL.backupRestore.selectBackupFile()}
      </Button>

      {#if validationError}
        <div class="status-stack">
          <Alert variant="error">{validationError}</Alert>
        </div>
      {/if}
    {:else if validation.canRestore}
      <!-- Validation passed — show summary -->
      <div class="validation-summary">
        <h3>{$LL.backupRestore.backupValidated()}</h3>
        <ul class="checks-list">
          {#each validation.checks as check, i}
            <li>{localizeCheck(validation.checkCodes[i], check)}</li>
          {/each}
        </ul>

        {#if !showRestoreConfirm}
          <Button variant="danger" size="sm" onclick={openRestoreConfirm}>
            {$LL.backupRestore.restoreButton()}
          </Button>
        {:else}
          <!-- Explicit destructive confirmation -->
          <div class="confirm-box">
            <p><strong>{$LL.common.confirm()}</strong> {$LL.backupRestore.restoreConfirmPrompt()}</p>
            <div class="confirm-actions">
              <Button
                variant="danger"
                size="sm"
                onclick={handleRestore}
                disabled={restoring}
                loading={restoring}
              >
                {restoring ? $LL.backupRestore.restoring() : $LL.backupRestore.restoreData()}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onclick={() => { showRestoreConfirm = false; }}
              >
                {$LL.common.cancel()}
              </Button>
            </div>
            {#if restoreError}
              <div class="status-stack">
                <Alert variant="error">{restoreError}</Alert>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {:else}
      <!-- Validation failed -->
      <div class="validation-summary">
        <h3>{$LL.backupRestore.cannotRestore()}</h3>
        <ul class="checks-list">
          {#each validation.checks as check, i}
            <li>{localizeCheck(validation.checkCodes[i], check)}</li>
          {/each}
        </ul>

        <p>
          <Button
            variant="secondary"
            size="sm"
            onclick={() => { validation = null; selectedBackupPath = ""; }}
          >
            {$LL.backupRestore.chooseDifferentFile()}
          </Button>
        </p>
      </div>
    {/if}
  </section>

  <!-- ─── Help ───────────────────────────────────────────────────────────── -->
  <section class="card">
    <h2>{$LL.backupRestore.aboutSection()}</h2>
    <dl class="info-list">
      <dt>{$LL.backupRestore.includesQ()}</dt>
      <dd>{$LL.backupRestore.includesA()}</dd>

      <dt>{$LL.backupRestore.howOftenQ()}</dt>
      <dd>{$LL.backupRestore.howOftenA()}</dd>

      <dt>{$LL.backupRestore.whereQ()}</dt>
      <dd>{$LL.backupRestore.whereA()}</dd>

      <dt>{$LL.backupRestore.locationQ()}</dt>
      <dd>{$LL.backupRestore.locationA()}</dd>
    </dl>
  </section>
</div>

<style>
  .page {
    max-width: 720px;
    margin: 0 auto;
    padding: 1.5rem;
  }

  h1 {
    font-size: 1.5rem;
    margin-bottom: 1.5rem;
    color: var(--color-base-content);
  }

  h2 {
    font-size: 1.15rem;
    margin-bottom: 0.75rem;
    color: var(--color-base-content);
  }

  h3 {
    font-size: 1rem;
    margin-bottom: 0.5rem;
    color: var(--color-base-content);
  }

  .card {
    background: var(--color-base-100);
    border: 1px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
    border-radius: 8px;
    padding: 1.25rem;
    margin-bottom: 1rem;
  }

  .card p {
    color: var(--color-secondary);
    line-height: 1.6;
    margin-bottom: 0.75rem;
  }

  .banner-stack,
  .status-stack {
    margin-top: 0.75rem;
    margin-bottom: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .validation-summary {
    background: color-mix(in oklch, var(--color-base-200) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
    border-radius: 6px;
    padding: 1rem;
  }

  .checks-list {
    list-style: none;
    padding: 0;
    margin: 0 0 0.75rem;
    font-size: 0.875rem;
  }

  .checks-list li {
    padding: 0.2rem 0;
    color: var(--color-secondary);
  }

  .confirm-box {
    background: color-mix(in oklch, var(--color-error) 6%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-error) 35%, transparent);
    border-radius: 6px;
    padding: 1rem;
    margin-top: 0.5rem;
  }

  .confirm-box p {
    margin-bottom: 0.75rem;
  }

  .confirm-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .info-list {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.5rem 1rem;
    font-size: 0.875rem;
  }

  dt {
    font-weight: 600;
    color: var(--color-base-content);
    white-space: nowrap;
  }

  dd {
    color: var(--color-secondary);
    margin: 0;
    line-height: 1.5;
  }
</style>
