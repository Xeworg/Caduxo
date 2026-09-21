<!--
  EmptyState.svelte — shared primitive (PR 3 of caduxo-daisyui-redesign).
  Composes a centered column with a 64-px heroicon (when supplied), the
  consumer-supplied title in `text-lg font-medium`, the consumer-
  supplied body in `text-sm`, and an optional action slot. No
  hardcoded copy: every visible string enters through props / slots.

  Tailwind classes referenced here (for the JIT scanner):
    text-base-content/70
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  // The icon name is mapped to an inline-SVG heroicon so the
  // primitive stays zero-dependency. Consumers pass a string from the
  // closed set below — no SVGs leak out of this file.
  export type EmptyStateIcon =
    | "inbox"
    | "calendar"
    | "document"
    | "tag"
    | "search"
    | "warning"
    | "info"
    | "none";

  interface Props {
    title: string;
    body?: string;
    icon?: EmptyStateIcon;
    actions?: Snippet;
    /** Optional accessible label override (rare; the title is the default). */
    "aria-label"?: string;
  }

  let {
    title,
    body,
    icon = "inbox",
    actions,
    ...rest
  }: Props = $props();
</script>

<div
  class="flex flex-col items-center justify-center gap-2 px-4 py-8 text-center text-base-content/70"
  aria-label={rest["aria-label"]}
>
  {#if icon !== "none"}
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      class="h-16 w-16 opacity-60"
      aria-hidden="true"
      focusable="false"
    >
      {#if icon === "inbox"}
        <polyline points="22 12 16 12 14 15 10 15 8 12 2 12"></polyline>
        <path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11Z"></path>
      {:else if icon === "calendar"}
        <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
        <line x1="16" y1="2" x2="16" y2="6"></line>
        <line x1="8" y1="2" x2="8" y2="6"></line>
        <line x1="3" y1="10" x2="21" y2="10"></line>
      {:else if icon === "document"}
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z"></path>
        <polyline points="14 2 14 8 20 8"></polyline>
      {:else if icon === "tag"}
        <path d="M20.59 13.41 13.42 20.58a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82Z"></path>
        <line x1="7" y1="7" x2="7.01" y2="7"></line>
      {:else if icon === "search"}
        <circle cx="11" cy="11" r="8"></circle>
        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
      {:else if icon === "warning"}
        <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z"></path>
        <line x1="12" y1="9" x2="12" y2="13"></line>
        <line x1="12" y1="17" x2="12.01" y2="17"></line>
      {:else}
        <!-- info -->
        <circle cx="12" cy="12" r="10"></circle>
        <line x1="12" y1="16" x2="12" y2="12"></line>
        <line x1="12" y1="8" x2="12.01" y2="8"></line>
      {/if}
    </svg>
  {/if}

  <p class="text-lg font-medium">{title}</p>
  {#if body}
    <p class="max-w-md text-sm">{body}</p>
  {/if}

  {#if actions}
    <div class="mt-2 flex items-center gap-2">
      {@render actions()}
    </div>
  {/if}
</div>
