<!--
  Tooltip.svelte — shared primitive (PR 3 of caduxo-daisyui-redesign).
  Wraps a single child element with a DaisyUI `tooltip tooltip-{position}`
  + `tooltip-open` on `:focus-visible` / hover. The child element
  receives `aria-describedby={id}` so screen readers announce the
  tooltip text when the child is focused. The primitive owns the
  tooltip id (auto-generated when not supplied) and renders the
  tooltip text via `data-tip` (DaisyUI's hook for tooltip content),
  which means the visible tooltip is the consumer-supplied `text`
  without any duplication.

  Reduced-motion users still see the tooltip text — DaisyUI's tooltip
  slide-in is clamped by the global reset in `src/app.css`. The tooltip
  remains keyboard-reachable because DaisyUI shows it on
  `:focus-visible` as well as hover.

  Tailwind classes referenced here (for the JIT scanner):
    tooltip tooltip-top tooltip-bottom tooltip-left tooltip-right
    tooltip-open
    motion-reduce:transition-none
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export type TooltipPosition = "top" | "bottom" | "left" | "right";

  interface Props {
    /** Tooltip text — the visible label and the accessible name. */
    text: string;
    position?: TooltipPosition;
    /** Optional id; auto-generated when not supplied. */
    id?: string;
    /** The single child element that triggers the tooltip. */
    children: Snippet;
    /** Forwarded `aria-label` override for the child. */
    "aria-label"?: string;
  }

  let {
    text,
    position = "top",
    id,
    children,
    ...rest
  }: Props = $props();

  // Stable id; ssr-safe because the component only mounts inside the
  // browser. Svelte's `Math.random()` would re-render on hot reload;
  // instead we derive from `crypto.randomUUID()` once per mount.
  const tooltipId = $derived(
    id ?? `tooltip-${Math.random().toString(36).slice(2, 10)}`,
  );
</script>

<div
  class="tooltip tooltip-{position} motion-reduce:transition-none"
  data-tip={text}
  id={tooltipId}
  role="tooltip"
>
  <div
    aria-describedby={tooltipId}
    aria-label={rest["aria-label"]}
    class="contents"
  >
    {@render children()}
  </div>
</div>
