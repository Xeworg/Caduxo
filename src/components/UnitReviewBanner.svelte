<script lang="ts">
  import {
    unitAuditBannerState,
    dismissUnitAuditBanner,
  } from "../lib/unit_definitions.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import Button from "./ui/Button.svelte";
  import Icon from "./ui/Icon.svelte";

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
    <span class="banner-icon" aria-hidden="true">
      <Icon name="exclamation-triangle" size="md" />
    </span>
    <span class="banner-text">
      {@html bannerMessage}
    </span>
    <Button variant="warning" size="sm" onclick={onReview}>
      {#snippet iconStart()}
        <Icon name="exclamation-triangle" size="sm" />
      {/snippet}
      {$LL.unitReview.review()}
    </Button>
    <Button
      variant="ghost"
      size="sm"
      onclick={dismiss}
      disabled={dismissing}
      loading={dismissing}
      aria-label={$LL.unitReview.dismiss()}
    >
      {$LL.unitReview.dismiss()}
    </Button>
  </div>
{/if}

<style>
  /* Themed banner — uses `--color-warning` + `color-mix` so the surface
     adapts to `caduxo-light` and `dark` without bespoke light-hex
     overrides (PR 13 cleanup). */
  .unit-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    background: color-mix(in oklch, var(--color-warning) 12%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-warning) 40%, transparent);
    border-radius: 8px;
    padding: 10px 14px;
    font-size: 0.9rem;
    color: color-mix(in oklch, var(--color-warning) 80%, var(--color-base-content));
    margin-bottom: 12px;
  }

  .banner-icon {
    font-size: 1rem;
    display: inline-flex;
    align-items: center;
    color: color-mix(in oklch, var(--color-warning) 70%, var(--color-base-content));
  }

  .banner-text {
    flex: 1;
  }
</style>
