<!--
  Badge.svelte — shared primitive (PR 3 of caduxo-daisyui-redesign).
  Two parallel APIs:
    - `urgency` for the expiry-tracker domain (expired | today | alert |
      soon | normal). Maps to a DaisyUI semantic modifier so urgency
      surfaces stay semantically distinct.
    - `semantic` for general status feedback (success | warning | error
      | info | neutral).
  When both are supplied, `urgency` takes precedence. The optional
  `dot` flag renders a leading status dot. The `expired` urgency
  variant is wired up for the `motion-safe:animate-urgency-pulse`
  utility, whose keyframes land in PR 12 (task 12.1.1). Reduced-motion
  users see the badge as static because the global reset clamps
  every animation / transition; the motion-safe variant is the
  narrow allow-list that keeps the pulse only for users who have not
  requested reduced motion.

  Tailwind classes referenced here (for the JIT scanner):
    badge badge-error badge-warning badge-success badge-info badge-neutral
    badge-sm
    status status-error status-warning status-success status-info status-neutral
    motion-safe:animate-urgency-pulse
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export type BadgeUrgency =
    | "expired"
    | "today"
    | "alert"
    | "soon"
    | "normal";

  export type BadgeSemantic =
    | "success"
    | "warning"
    | "error"
    | "info"
    | "neutral";

  export type BadgeSize = "sm" | "md";

  interface Props {
    /** Urgency variant (overrides `semantic` when both are supplied). */
    urgency?: BadgeUrgency;
    /** Semantic variant. */
    semantic?: BadgeSemantic;
    /** When true, prepends a status dot in the matching colour. */
    dot?: boolean;
    size?: BadgeSize;
    children?: Snippet;
  }

  let { urgency, semantic, dot = false, size = "md", children }: Props =
    $props();

  // Map urgency → semantic for both the badge colour and the dot
  // colour so the two parallel APIs stay visually consistent. The
  // mapping is explicit (rather than a default) so a future urgency
  // token easily extends without surprising the existing surfaces.
  const effective = $derived.by((): BadgeSemantic => {
    if (urgency) {
      switch (urgency) {
        case "expired":
          return "error";
        case "today":
          return "error";
        case "alert":
          return "warning";
        case "soon":
          return "warning";
        case "normal":
          return "success";
      }
    }
    return semantic ?? "neutral";
  });

  const semanticClass = $derived(`badge-${effective}`);
  const statusClass = $derived(`status-${effective}`);
  const sizeClass = $derived(size === "sm" ? "badge-sm" : "");

  // The pulse utility is only emitted once the keyframes are defined
  // in PR 12. Until then the class is a no-op; consumers can render
  // this primitive today without waiting for the animation to land.
  const pulseClass = $derived(
    urgency === "expired" ? "motion-safe:animate-urgency-pulse" : "",
  );
</script>

<span
  class="badge {semanticClass} {sizeClass} {pulseClass} gap-1"
>
  {#if dot}
    <span class="status {statusClass} status-sm" aria-hidden="true"></span>
  {/if}
  {#if children}
    {@render children()}
  {/if}
</span>
