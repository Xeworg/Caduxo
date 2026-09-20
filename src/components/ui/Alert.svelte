<!--
  Alert.svelte — shared primitive (PR 3 of caduxo-daisyui-redesign).
  Composes DaisyUI `alert` with a leading inline-SVG heroicon (no icon
  dependency; all SVGs are inline). The `dismissible` flag surfaces a
  trailing close affordance and emits `ondismiss` to the consumer; the
  consumer owns the dismissal state so the primitive remains
  controlled.

  No hardcoded user-facing copy: the icon is a glyph (no text label),
  the title / body come from `title` (optional string) and the default
  slot, and the action buttons are passed via the `actions` snippet.

  Tailwind classes referenced here (for the JIT scanner):
    alert alert-success alert-warning alert-error alert-info
    alert-soft
    btn btn-ghost btn-circle btn-xs
    motion-reduce:transition-none
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export type AlertVariant = "success" | "warning" | "error" | "info";

  interface Props {
    variant: AlertVariant;
    /** Optional title rendered as a strong label inside the alert. */
    title?: string;
    /** When true, surfaces a trailing close button and calls `ondismiss`. */
    dismissible?: boolean;
    /**
     * Required when `dismissible` is true. The primitive does NOT
     * ship a default string — the consumer owns the accessible
     * label (typically `$LL.common.dismiss()`).
     */
    dismissLabel?: string;
    /** Plain `aria-label` override (rare). */
    "aria-label"?: string;
    /** Required `role` — defaults to `alert` for error / warning,
     *  `status` for success / info. */
    role?: "alert" | "status";
    /** Default body content. */
    children?: Snippet;
    /** Optional action buttons slot (rendered after the body). */
    actions?: Snippet;
    ondismiss?: () => void;
  }

  let {
    variant,
    title,
    dismissible = false,
    dismissLabel,
    "aria-label": ariaLabel,
    role,
    children,
    actions,
    ondismiss,
  }: Props = $props();

  const variantClass = $derived(`alert-${variant}`);

  // role defaults follow established screen-reader semantics: errors
  // and warnings interrupt (`role="alert"`); success and info are
  // polite (`role="status"`).
  const effectiveRole = $derived(
    role ?? (variant === "error" || variant === "warning" ? "alert" : "status"),
  );
</script>

<div
  class="alert {variantClass} alert-soft motion-reduce:transition-none"
  role={effectiveRole}
  aria-label={ariaLabel}
>
  <!-- Inline heroicon glyph; no icon library is added. The icon is
       decorative and hidden from screen readers so the body / title
       carry the message. -->
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    class="h-5 w-5 shrink-0"
    aria-hidden="true"
    focusable="false"
  >
    {#if variant === "success"}
      <circle cx="12" cy="12" r="10"></circle>
      <path d="m9 12 2 2 4-4"></path>
    {:else if variant === "warning"}
      <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z"></path>
      <line x1="12" y1="9" x2="12" y2="13"></line>
      <line x1="12" y1="17" x2="12.01" y2="17"></line>
    {:else if variant === "error"}
      <circle cx="12" cy="12" r="10"></circle>
      <line x1="15" y1="9" x2="9" y2="15"></line>
      <line x1="9" y1="9" x2="15" y2="15"></line>
    {:else}
      <!-- info -->
      <circle cx="12" cy="12" r="10"></circle>
      <line x1="12" y1="16" x2="12" y2="12"></line>
      <line x1="12" y1="8" x2="12.01" y2="8"></line>
    {/if}
  </svg>

  <div class="flex-1">
    {#if title}
      <h3 class="font-semibold">{title}</h3>
    {/if}
    {#if children}
      <div class="text-sm">
        {@render children()}
      </div>
    {/if}
  </div>

  {#if actions}
    <div class="flex items-center gap-2">
      {@render actions()}
    </div>
  {/if}

  {#if dismissible}
    <button
      type="button"
      class="btn btn-ghost btn-circle btn-xs"
      aria-label={dismissLabel}
      onclick={ondismiss}
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="h-4 w-4"
        aria-hidden="true"
        focusable="false"
      >
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>
  {/if}
</div>
