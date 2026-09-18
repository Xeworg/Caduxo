<script lang="ts">
  import {
    exportBackupWithDialog,
    validateBackupWithDialog,
    restoreBackup,
    type RestoreValidation,
  } from "../lib/backup_restore.js";
  import { LL } from "../i18n/i18n-svelte.js";

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
      exportError = String(e);
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
      validationError = String(e);
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
      restoreError = String(e);
    } finally {
      restoring = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<div class="page">
  <h1>{$LL.backupRestore.pageTitle()}</h1>

  <!-- ─── Export ─────────────────────────────────────────────────────────── -->
  <section class="card">
    <h2>{$LL.backupRestore.exportSection()}</h2>
    <p>{$LL.backupRestore.exportDesc()}</p>

    <button class="btn-primary" on:click={handleExport} disabled={exporting}>
      {exporting ? $LL.backupRestore.exporting() : $LL.backupRestore.exportButton()}
    </button>

    {#if exportSuccess}
      <div class="message success">{exportSuccess}</div>
    {/if}

    {#if exportError}
      <div class="message error">{exportError}</div>
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

    <div class="warning-banner">
      {$LL.backupRestore.restoreDangerBanner()}
    </div>

    {#if !validation}
      <button class="btn-secondary" on:click={handleValidate} disabled={validating}>
        {validating ? $LL.backupRestore.selecting() : $LL.backupRestore.selectBackupFile()}
      </button>

      {#if validationError}
        <div class="message error">{validationError}</div>
      {/if}
    {:else if validation.canRestore}
      <!-- Validation passed — show summary -->
      <div class="validation-summary">
        <h3>{$LL.backupRestore.backupValidated()}</h3>
        <ul class="checks-list">
          {#each validation.checks as check}
            <li>{check}</li>
          {/each}
        </ul>

        {#if !showRestoreConfirm}
          <button class="btn-danger" on:click={openRestoreConfirm}>
            {$LL.backupRestore.restoreButton()}
          </button>
        {:else}
          <!-- Explicit destructive confirmation -->
          <div class="confirm-box">
            <p><strong>{$LL.common.confirm()}</strong> {$LL.backupRestore.restoreConfirmPrompt()}</p>
            <div class="confirm-actions">
              <button class="btn-danger" on:click={handleRestore} disabled={restoring}>
                {restoring ? $LL.backupRestore.restoring() : $LL.backupRestore.restoreData()}
              </button>
              <button class="btn-secondary" on:click={() => { showRestoreConfirm = false; }}>
                {$LL.common.cancel()}
              </button>
            </div>
            {#if restoreError}
              <div class="message error">{restoreError}</div>
            {/if}
          </div>
        {/if}
      </div>
    {:else}
      <!-- Validation failed -->
      <div class="validation-summary">
        <h3>{$LL.backupRestore.cannotRestore()}</h3>
        <ul class="checks-list">
          {#each validation.checks as check}
            <li>{check}</li>
          {/each}
        </ul>

        <p>
          <button class="btn-secondary" on:click={() => { validation = null; selectedBackupPath = ""; }}>
            {$LL.backupRestore.chooseDifferentFile()}
          </button>
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
  }

  h2 {
    font-size: 1.15rem;
    margin-bottom: 0.75rem;
  }

  h3 {
    font-size: 1rem;
    margin-bottom: 0.5rem;
  }

  .card {
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 1.25rem;
    margin-bottom: 1rem;
  }

  .card p {
    color: #64748b;
    line-height: 1.6;
    margin-bottom: 0.75rem;
  }

  .warning-banner {
    background: #fff7ed;
    border: 1px solid #fed7aa;
    border-radius: 6px;
    padding: 0.75rem 1rem;
    margin-bottom: 0.75rem;
    color: #c2410c;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .validation-summary {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
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
    color: #475569;
  }

  .confirm-box {
    background: #fef2f2;
    border: 1px solid #fecaca;
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
  }

  .info-list {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.5rem 1rem;
    font-size: 0.875rem;
  }

  dt {
    font-weight: 600;
    color: #334155;
    white-space: nowrap;
  }

  dd {
    color: #64748b;
    margin: 0;
    line-height: 1.5;
  }

  .btn-primary,
  .btn-secondary,
  .btn-danger {
    padding: 0.5rem 1rem;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: background 0.15s;
  }

  .btn-primary {
    background: #2563eb;
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: #1d4ed8;
  }

  .btn-secondary {
    background: #f1f5f9;
    color: #334155;
    border: 1px solid #cbd5e1;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #e2e8f0;
  }

  .btn-danger {
    background: #dc2626;
    color: white;
  }

  .btn-danger:hover:not(:disabled) {
    background: #b91c1c;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .message {
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    margin-top: 0.5rem;
    font-size: 0.875rem;
  }

  .success {
    background: #f0fdf4;
    border: 1px solid #bbf7d0;
    color: #166534;
  }

  .error {
    background: #fef2f2;
    border: 1px solid #fecaca;
    color: #991b1b;
  }
</style>
