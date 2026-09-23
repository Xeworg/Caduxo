<!--
  Badge.svelte — shared primitive (PR 3 of caduxo-daisyui-redesign).
  Two parallel APIs:
    - `urgency` for the expiry-tracker domain (expired | today | alert |
      soon | normal). Maps to a DaisyUI semantic modifier so urgency
      surfaces stay semantically distinct.
    - `semantic` for general status feedback (success | warning | error
      | info | neutral | retired).
  When both are supplied, `urgency` takes precedence. The optional
  `dot` flag renders a leading status dot. The `expired` urgency
  variant composes `motion-safe:animate-urgency-pulse` (PR 12 wires
  the `urgency-pulse` keyframes + the `--animate-urgency-pulse` token
  in `src/app.css`). The pulse is opacity-only and runs at the
  project `--duration-pulse` cadence with `--ease-in-out-soft`. The
  `motion-safe:` variant is the narrow allow-list: reduced-motion
  users see the badge as a static element via the global reset in
  `src/app.css` (the reset clamps every animation / transition as a
  back-stop even when the `motion-safe:` variant fires, because the
  pulse is gated on both conditions).

  Tailwind classes referenced here (for the JIT scanner):
    badge badge-error badge-warning badge-success badge-info badge-neutral
    badge-retired badge-sm
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
    | "neutral"
    | "retired";

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
  const statusClass = $derived(
    effective === "retired" ? "status-warning" : `status-${effective}`,
  );
  const sizeClass = $derived(size === "sm" ? "badge-sm" : "");

  // The pulse utility is emitted only for the `expired` urgency
  // variant. The `urgency-pulse` keyframes are defined in
  // `src/app.css` (PR 12) and the `--animate-urgency-pulse` token
  // there exposes the Tailwind v4 utility. The `motion-safe:`
  // variant ensures the pulse only fires for users who have NOT
  // requested reduced motion; the global reduced-motion reset in
  // `src/app.css` is the back-stop.
  const pulseClass = $derived(
    urgency === "expired" ? "motion-safe:animate-urgency-pulse" : "",
  );
</script>

<span
  class="badge {semanticClass} {sizeClass} {pulseClass} gap-1 whitespace-nowrap"
>
  {#if dot}
    <span class="status {statusClass} status-sm" aria-hidden="true"></span>
  {/if}
  {#if children}
    {@render children()}
  {/if}
</span>

<style>
  .badge-retired {
    background: color-mix(in oklch, var(--color-warning) 18%, transparent);
    border-color: color-mix(in oklch, var(--color-warning) 28%, transparent);
    color: color-mix(
      in oklch,
      var(--color-warning) 80%,
      var(--color-base-content)
    );
  }
</style>
