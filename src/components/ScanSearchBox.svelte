<script lang="ts">
  import {
    findProductByScan,
    type ScanSearchResult,
    type ScanFoundResult,
    type ScanNotFoundResult,
  } from "../lib/products.js";
  import { LL } from "../i18n/i18n-svelte.js";

  // ── Props ─────────────────────────────────────────────────────────────────────

  /** Text shown as placeholder in the input field. */
  export let placeholder = "";
  /** Called when a barcode or SKU matches — caller decides the next step. */
  export let onFound: (productId: string, hasLots: boolean) => void;
  /** Called when nothing matched — caller opens quick-create. */
  export let onNotFound: (scannedValue: string) => void;

  // ── State ─────────────────────────────────────────────────────────────────────

  let scanValue = "";
  let scanning = false;
  let errorMsg = "";

  // ── Handler ──────────────────────────────────────────────────────────────────

  /**
   * Triggers on every Enter keypress in the scan input.
   *
   * Flow:
   * 1. Trim and guard empty input.
   * 2. Call `findProductByScan` (barcode-first, then SKU-second exact lookup).
   * 3. On Found → call `onFound(productId, hasLots)`.
   * 4. On NotFound → call `onNotFound(scannedValue)`.
   *
   * No logging of raw SKU / barcode / scan strings per engineering safety rules.
   */
  async function handleScan() {
    const value = scanValue.trim();
    if (!value) return;

    scanning = true;
    errorMsg = "";
    try {
      const result: ScanSearchResult = await findProductByScan(value);
      if (result.match_type === "not_found") {
        onNotFound(value);
      } else {
        const found = result as ScanFoundResult;
        onFound(found.product.id, found.has_lots);
      }
      // Clear the input after a successful scan regardless of outcome.
      scanValue = "";
    } catch (e) {
      errorMsg = $LL.scan.searchError({ msg: String(e) });
    } finally {
      scanning = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      handleScan();
    }
  }
</script>

<div class="scan-box">
  <input
    type="text"
    class="scan-input"
    class:scanning
    bind:value={scanValue}
    {placeholder}
    disabled={scanning}
    autocomplete="off"
    autocorrect="off"
    autocapitalize="off"
    spellcheck="false"
    on:keydown={handleKeydown}
    aria-label={$LL.scan.ariaLabel()}
    title={$LL.scan.title()}
  />

  {#if scanning}
    <span class="scan-spinner" aria-hidden="true"></span>
  {/if}

  {#if errorMsg}
    <span class="scan-error" role="alert">{errorMsg}</span>
  {/if}
</div>

<style>
  .scan-box {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    max-width: 400px;
  }

  .scan-input {
    width: 100%;
    padding: 8px 40px 8px 12px;
    border: 2px solid #bfdbfe;
    border-radius: 8px;
    font-size: 0.95rem;
    font-family: inherit;
    background: #fff;
    color: #1e293b;
    transition: border-color 0.15s, box-shadow 0.15s;
    outline: none;
  }

  .scan-input:focus {
    border-color: #2563eb;
    box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.12);
  }

  .scan-input.scanning {
    background: #f8fafc;
    color: #94a3b8;
  }

  .scan-input:disabled {
    cursor: default;
  }

  /* Spinner overlay */
  .scan-spinner {
    position: absolute;
    right: 10px;
    width: 16px;
    height: 16px;
    border: 2px solid #d1d5db;
    border-top-color: #2563eb;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    pointer-events: none;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .scan-error {
    position: absolute;
    right: 0;
    top: calc(100% + 4px);
    background: #fef2f2;
    border: 1px solid #fca5a5;
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 0.78rem;
    color: #dc2626;
    white-space: nowrap;
    z-index: 10;
  }
</style>
