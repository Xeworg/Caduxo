<script lang="ts">
  import {
    exportBackupWithDialog,
    validateBackupWithDialog,
    restoreBackup,
    type RestoreValidation,
  } from "../lib/backup_restore.js";

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
        exportSuccess = `Backup saved to ${result.path} (${kb} KB, schema v${result.schemaVersion}).`;
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
  <h1>Backup &amp; Restore</h1>

  <!-- ─── Export ─────────────────────────────────────────────────────────── -->
  <section class="card">
    <h2>Export backup</h2>
    <p>Creates a full copy of your database at a location you choose. The app remains open and usable during export.</p>

    <button class="btn-primary" on:click={handleExport} disabled={exporting}>
      {exporting ? "Exporting…" : "Export database"}
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
    <h2>Restore from backup</h2>
    <p>
      Restoring a backup replaces all current data with the contents of the backup file.
      <strong>This is a destructive operation.</strong> Make sure you have an export of your current data before proceeding.
    </p>

    <div class="warning-banner">
      ⚠️ Restoring a backup cannot be undone. Current data will be permanently replaced.
    </div>

    {#if !validation}
      <button class="btn-secondary" on:click={handleValidate} disabled={validating}>
        {validating ? "Selecting…" : "Select backup file to restore"}
      </button>

      {#if validationError}
        <div class="message error">{validationError}</div>
      {/if}
    {:else if validation.canRestore}
      <!-- Validation passed — show summary -->
      <div class="validation-summary">
        <h3>✅ Backup file validated</h3>
        <ul class="checks-list">
          {#each validation.checks as check}
            <li>{check}</li>
          {/each}
        </ul>

        {#if !showRestoreConfirm}
          <button class="btn-danger" on:click={openRestoreConfirm}>
            Restore from this backup
          </button>
        {:else}
          <!-- Explicit destructive confirmation -->
          <div class="confirm-box">
            <p><strong>Are you sure?</strong> This will permanently replace all current data with the backup.</p>
            <div class="confirm-actions">
              <button class="btn-danger" on:click={handleRestore} disabled={restoring}>
                {restoring ? "Restoring…" : "Yes, replace my data"}
              </button>
              <button class="btn-secondary" on:click={() => { showRestoreConfirm = false; }}>
                Cancel
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
        <h3>❌ Backup file cannot be restored</h3>
        <ul class="checks-list">
          {#each validation.checks as check}
            <li>{check}</li>
          {/each}
        </ul>

        <p>
          <button class="btn-secondary" on:click={() => { validation = null; selectedBackupPath = ""; }}>
            Choose a different file
          </button>
        </p>
      </div>
    {/if}
  </section>

  <!-- ─── Help ───────────────────────────────────────────────────────────── -->
  <section class="card">
    <h2>About backups</h2>
    <dl class="info-list">
      <dt>What is included?</dt>
      <dd>Full database copy including all stores, products, expiry lots, categories, settings, and history.</dd>

      <dt>How often should I back up?</dt>
      <dd>Regular backups are recommended before major changes like CSV imports or bulk lot operations.</dd>

      <dt>Where should I save backups?</dt>
      <dd>Any location you choose — external drive, cloud folder, or local directory. Standard SQLite files can be opened with most database tools.</dd>

      <dt>Current database location</dt>
      <dd>The active database is stored in the app's local data directory. The backup export lets you choose where to save your copy.</dd>
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
