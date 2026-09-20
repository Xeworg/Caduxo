<!--
  Card.svelte — shared primitive (PR 3 of caduxo-daisyui-redesign).
  Composes DaisyUI `card` with optional `header` / `footer` / default
  slots and a `tone` prop for status-driven tinting. Optional `labelled`
  flag surfaces `role="region"` + `aria-labelledby` when a region-level
  accessible name is needed.

  Tailwind classes referenced here (for the JIT scanner):
    card card-body card-title card-actions
    bg-base-200
    border-warning border-error border-success border-info
    border-base-300
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export type CardTone =
    | "default"
    | "muted"
    | "warning"
    | "error"
    | "success"
    | "info";

  interface Props {
    /**
     * Visual tone. `muted` swaps the background to `bg-base-200` for
     * secondary surfaces; the status tones add a matching border
     * colour.
     */
    tone?: CardTone;
    /**
     * When true, the card surfaces `role="region"` and is associated
     * with its `labelledBy` id via `aria-labelledby`.
     */
    labelled?: boolean;
    /** Required when `labelled` is true. */
    "aria-labelledby"?: string;
    /** Plain `aria-label` alternative when no header id is available. */
    "aria-label"?: string;
    /** Optional small heading id (separate from `aria-labelledby`). */
    titleId?: string;
    /** Default body content. */
    children?: Snippet;
    /** Optional header slot rendered inside `card-body` before children. */
    header?: Snippet;
    /** Optional footer slot rendered inside `card-body` after children. */
    footer?: Snippet;
    /** Optional id passthrough. */
    id?: string;
  }

  let {
    tone = "default",
    labelled = false,
    titleId,
    children,
    header,
    footer,
    id,
    ...rest
  }: Props = $props();

  const toneClass = $derived.by((): string => {
    switch (tone) {
      case "muted":
        return "bg-base-200";
      case "warning":
        return "border-warning";
      case "error":
        return "border-error";
      case "success":
        return "border-success";
      case "info":
        return "border-info";
      default:
        return "border-base-300";
    }
  });

  const isRegion = $derived(Boolean(labelled));
</script>

<section
  {id}
  class="card bg-base-100 {toneClass} shadow-sm"
  role={isRegion ? "region" : undefined}
  aria-labelledby={isRegion ? rest["aria-labelledby"] : undefined}
  aria-label={!isRegion ? rest["aria-label"] : undefined}
>
  <div class="card-body">
    {#if header || titleId}
      <header class="card-title" id={titleId}>
        {#if header}
          {@render header()}
        {/if}
      </header>
    {/if}
    {#if children}
      {@render children()}
    {/if}
    {#if footer}
      <div class="card-actions justify-end">
        {@render footer()}
      </div>
    {/if}
  </div>
</section>
