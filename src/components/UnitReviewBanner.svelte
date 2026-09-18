<script lang="ts">
  import {
    unitAuditBannerState,
    dismissUnitAuditBanner,
  } from "../lib/unit_definitions.js";
  import { LL } from "../i18n/i18n-svelte.js";

  /** Called when the user clicks the "Review" button. */
  export let onReview: () => void;

  let loading = true;
  let showBanner = false;
  let unrecognizedCount = 0;
  let dismissing = false;

  /** Plural-aware banner message — picks singular or plural form. */
  $: bannerMessageFn = unrecognizedCount === 1
    ? $LL.unitReview.unitCount_singular_alt
    : $LL.unitReview.unitCount_plural_alt;
  $: bannerMessage = bannerMessageFn({ n: unrecognizedCount });

  async function refresh() {
    loading = true;
    try {
      const state = await unitAuditBannerState();
      showBanner = state.show_banner;
      unrecognizedCount = state.unrecognized_count;
    } catch {
      showBanner = false;
    } finally {
      loading = false;
    }
  }

  async function dismiss() {
    dismissing = true;
    try {
      await dismissUnitAuditBanner();
      showBanner = false;
    } catch {
      // Keep banner visible on error.
    } finally {
      dismissing = false;
    }
  }

  /** Call this from the parent (DashboardPage) on mount. */
  export async function init() {
    await refresh();
  }
</script>

{#if !loading && showBanner}
  <div class="unit-banner" role="status">
    <span class="banner-icon">⚠️</span>
    <span class="banner-text">
      {@html bannerMessage}
    </span>
    <button
      type="button"
      class="btn-primary btn-sm banner-btn"
      on:click={onReview}
    >
      {$LL.unitReview.review()}
    </button>
    <button
      type="button"
      class="btn-ghost btn-sm"
      disabled={dismissing}
      on:click={dismiss}
      title={$LL.unitReview.dismiss()}
    >
      {dismissing ? $LL.unitReview.inProgress() : $LL.unitReview.dismiss()}
    </button>
  </div>
{/if}

<style>
  .unit-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    background: #fef9c3;
    border: 1px solid #fde047;
    border-radius: 8px;
    padding: 10px 14px;
    font-size: 0.9rem;
    color: #713f12;
    margin-bottom: 12px;
  }

  .banner-icon {
    font-size: 1rem;
  }

  .banner-text {
    flex: 1;
  }

  .banner-btn {
    background: #a16207;
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 5px 12px;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .banner-btn:hover {
    background: #854d0e;
  }

  .btn-ghost {
    background: transparent;
    border: none;
    color: #713f12;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .btn-ghost:hover {
    background: rgba(113, 63, 18, 0.1);
  }

  .btn-ghost:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
