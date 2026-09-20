<!--
  Table.svelte — shared primitive (PR 4 of caduxo-daisyui-redesign).

  Composes DaisyUI `table` + the variant utilities (`table-zebra`,
  `table-pin-rows`, `table-sm`). The primitive provides structural
  slots; the consumer composes `<thead>` / `<tbody>` rows directly.

  Slot conventions (mutually exclusive):
    - `body`   — the consumer's data rows. Rendered when the table
                 has data to display.
    - `empty`  — empty-state content (typically an `EmptyState`).
    - `loading` — loading-state content (typically a `LoadingState`).

  The consumer MUST provide at most ONE of `body`, `empty`, or
  `loading` per render. The primitive renders the supplied slot in
  priority order (body > empty > loading) so a single render cycle
  shows the appropriate section. The contract is a documentation /
  review-time enforcement — the primitive does not switch between
  slots dynamically; the consumer is responsible for picking which
  slot to supply based on its own state.

  Numeric columns should add `class="num"` directly on the `<td>`
  element (the utility is defined in `src/app.css` per design §4.4)
  so the alignment composes with every other utility on the cell.

  Tailwind classes referenced here (for the JIT scanner):
    table table-zebra table-pin-rows table-sm
    overflow-x-auto
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export type TableSize = "dense" | "default";

  interface Props {
    /** Zebra-stripe the rows. */
    zebra?: boolean;
    /** Pin the `<thead>` to the top of a scroll container. */
    stickyHeader?: boolean;
    /** `dense` composes DaisyUI `table-sm`; `default` is the standard density. */
    size?: TableSize;
    /** Visible `<caption>`. When unset, the table is captionless (decorative caption via sr-only fallback below). */
    caption?: string;
    /** Optional id reference (e.g. an `aria-describedby` for the empty / loading slot). */
    describedBy?: string;
    /**
     * Body rows (the data state). Rendered when supplied.
     * Mutually exclusive with `empty` and `loading`.
     */
    body?: Snippet;
    /**
     * Empty-state content (rendered when no data is available).
     * Mutually exclusive with `body` and `loading`.
     */
    empty?: Snippet;
    /**
     * Loading-state content (rendered while data is being fetched).
     * Mutually exclusive with `body` and `empty`.
     */
    loading?: Snippet;
    /** Header row content (rendered inside `<thead>`). */
    head?: Snippet;
    /** Optional id passthrough. */
    id?: string;
    /** Forwarded `aria-label` when no `caption` is supplied. */
    "aria-label"?: string;
    /**
     * When true, wraps the table in `overflow-x-auto` so wide tables
     * scroll horizontally inside the parent instead of overflowing
     * the page (per design §4.10). Default: false.
     */
    scrollable?: boolean;
  }

  let {
    zebra = false,
    stickyHeader = false,
    size = "default",
    caption,
    describedBy,
    body,
    empty,
    loading,
    head,
    id,
    "aria-label": ariaLabel,
    scrollable = false,
  }: Props = $props();

  const tableClasses = $derived.by((): string => {
    const classes: string[] = ["table"];
    if (zebra) classes.push("table-zebra");
    if (stickyHeader) classes.push("table-pin-rows");
    if (size === "dense") classes.push("table-sm");
    return classes.join(" ");
  });

  // Body slot priority: body > empty > loading. The consumer supplies
  // exactly one per the mutually-exclusive contract. We treat the
  // resolved snippet as `loading` when the consumer supplied the
  // `loading` slot and not `body` / `empty` — that surfaces
  // `aria-busy="true"` on the tbody so assistive tech announces the
  // loading state.
  const bodySnippet = $derived(body ?? empty ?? loading);
  const bodyIsLoading = $derived(!body && !empty && Boolean(loading));
</script>

<!--
  The wrapper div is only rendered when `scrollable` is true — for
  narrow-table surfaces (e.g. dashboard lot picker) the table
  composes inline. The wrapper is the place where themed scrollbar
  utilities attach in PR 11.

  Tailwind classes referenced here (for the JIT scanner):
    overflow-x-auto scrollbar-thin scrollbar-thumb-base-300
-->
{#if scrollable}
  <div class="overflow-x-auto scrollbar-thin scrollbar-thumb-base-300">
    <table
      {id}
      class={tableClasses}
      aria-label={!caption ? ariaLabel : undefined}
      aria-describedby={describedBy}
    >
      {#if caption}
        <caption>{caption}</caption>
      {/if}
      {#if head}
        <thead>
          {@render head()}
        </thead>
      {/if}
      {#if bodySnippet}
        <tbody
          aria-busy={bodyIsLoading ? "true" : undefined}
          aria-describedby={describedBy}
        >
          {@render bodySnippet()}
        </tbody>
      {/if}
    </table>
  </div>
{:else}
  <table
    {id}
    class={tableClasses}
    aria-label={!caption ? ariaLabel : undefined}
    aria-describedby={describedBy}
  >
    {#if caption}
      <caption>{caption}</caption>
    {/if}
    {#if head}
      <thead>
        {@render head()}
      </thead>
    {/if}
    {#if bodySnippet}
      <tbody
        aria-busy={bodyIsLoading ? "true" : undefined}
        aria-describedby={describedBy}
      >
        {@render bodySnippet()}
      </tbody>
    {/if}
  </table>
{/if}