<!--
  LoadingState.svelte — shared primitive (PR 3 of caduxo-daisyui-redesign).
  Three variants:
    - `skeleton` renders N skeleton rows (default 3) for table-style
      placeholders. DaisyUI's `skeleton` class animates a shimmer; the
      global `prefers-reduced-motion: reduce` reset in `src/app.css`
      clamps the animation to 0.001 ms so reduced-motion users see a
      static grey block.
    - `spinner` renders a DaisyUI spinner with an optional
      `aria-label` for assistive tech.
    - `text` renders the localised `label` prop as a polite
      announcement. The `announce` flag toggles `aria-live="polite"`
      on the wrapper for the spinner and text variants; the skeleton
      variant suppresses announcements because each row is visual.

  Tailwind classes referenced here (for the JIT scanner):
    skeleton
    loading loading-spinner loading-md loading-sm
-->
<script lang="ts">
  export type LoadingVariant = "skeleton" | "spinner" | "text";

  interface Props {
    variant?: LoadingVariant;
    /** Number of skeleton rows (skeleton variant only). Defaults to 3. */
    rows?: number;
    /**
     * Visible label and the accessible label. Required for `text`
     * variant; optional for `spinner` and `skeleton` (still
     * recommended so screen readers can announce the loading
     * state). The primitive does NOT ship a default string — the
     * consumer owns the copy (typically `$LL.common.loading()`).
     */
    label?: string;
    /**
     * When true (default for `text` and `spinner`), the wrapper
     * carries `aria-live="polite"` so screen readers announce the
     * loading state. The skeleton variant never announces.
     */
    announce?: boolean;
  }

  let {
    variant = "skeleton",
    rows = 3,
    label,
    announce,
  }: Props = $props();

  // Default the `announce` flag by variant so consumers do not have to
  // pass it explicitly for the common case.
  const effectiveAnnounce = $derived(
    announce ?? (variant !== "skeleton"),
  );

  // Clamp `rows` to [1, 50] so a typo cannot render 10 000 skeleton
  // divs by accident. The clamp keeps the surface cheap.
  const clampedRows = $derived(Math.max(1, Math.min(50, rows)));
</script>

{#if variant === "skeleton"}
  <div
    class="flex w-full flex-col gap-2"
    role="status"
    aria-busy="true"
    aria-label={label}
  >
    {#each Array.from({ length: clampedRows }) as _, i (i)}
      <div class="skeleton h-4 w-full" aria-hidden="true"></div>
    {/each}
  </div>
{:else if variant === "spinner"}
  <div
    class="flex items-center justify-center gap-2"
    role="status"
    aria-busy="true"
    aria-live={effectiveAnnounce ? "polite" : undefined}
    aria-label={label}
  >
    <span
      class="loading loading-spinner loading-md motion-reduce:hidden"
      aria-hidden="true"
    ></span>
    {#if label}
      <span>{label}</span>
    {/if}
  </div>
{:else}
  <!-- text -->
  <p
    role="status"
    aria-busy="true"
    aria-live={effectiveAnnounce ? "polite" : undefined}
  >
    {label}
  </p>
{/if}
