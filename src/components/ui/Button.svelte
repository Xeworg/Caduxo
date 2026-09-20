<!--
  Button.svelte — shared primitive (PR 3 of caduxo-daisyui-redesign).
  Composes DaisyUI button variants without inlining user-visible copy.
  No hardcoded strings; consumers pass labels via the default `children`
  snippet, an `aria-label` for icon-only usage, and optional `iconStart`
  / `iconEnd` snippets for leading / trailing icons.

  Tailwind classes referenced here (for the JIT scanner):
    btn-primary btn-secondary btn-ghost btn-outline btn-error btn-warning
    btn-success btn-link btn-square btn-circle
    btn-xs btn-sm btn-md btn-lg
    loading loading-spinner loading-sm
    motion-safe:animate-none motion-reduce:transition-none
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export type ButtonVariant =
    | "primary"
    | "secondary"
    | "ghost"
    | "outline"
    | "danger"
    | "warning"
    | "success"
    | "link"
    | "icon";

  export type ButtonSize = "xs" | "sm" | "md" | "lg";

  interface Props {
    /**
     * Visual + semantic role. The `icon` variant is square-shaped and
     * requires an `aria-label` because it carries no visible text.
     */
    variant?: ButtonVariant;
    /**
     * Visual size. Icon-only buttons ignore this and stay square at the
     * closest equivalent.
     */
    size?: ButtonSize;
    type?: "button" | "submit" | "reset";
    /** Disables interaction; the click handler is not invoked. */
    disabled?: boolean;
    /**
     * Renders a spinner, sets `aria-busy`, and prevents re-submission
     * until cleared. Use this for in-flight async actions.
     */
    loading?: boolean;
    /** Required when `variant === "icon"` — no visible text label. */
    "aria-label"?: string;
    /** Optional leading icon slot. */
    iconStart?: Snippet;
    /** Optional trailing icon slot. */
    iconEnd?: Snippet;
    onclick?: (event: MouseEvent) => void;
    /** Default slot — visible label. Required unless `aria-label` is set. */
    children?: Snippet;
    /** Optional `aria-describedby` for helper text. */
    "aria-describedby"?: string;
    /** Optional id for label association (rare; for form-style usage). */
    id?: string;
  }

  let {
    variant = "primary",
    size = "md",
    type = "button",
    disabled = false,
    loading = false,
    iconStart,
    iconEnd,
    onclick,
    children,
    id,
    ...rest
  }: Props = $props();

  // Map our variant + size union onto DaisyUI classes. We compute the
  // class string at the top level so Tailwind's scanner sees the literal
  // tokens (btn-primary, btn-xs, …) during the build pass.
  const variantClass = $derived.by((): string => {
    // The `icon` variant is handled separately because it composes
    // `btn-square` + a size-equivalent and requires an aria-label.
    if (variant === "icon") return "btn-square";
    return `btn-${variant}`;
  });

  const sizeClass = $derived.by((): string => {
    if (variant === "icon") {
      // Square icon-only buttons collapse size to a square equivalent.
      if (size === "xs") return "btn-xs";
      if (size === "sm") return "btn-sm";
      if (size === "lg") return "btn-lg";
      return "btn-md";
    }
    return `btn-${size}`;
  });

  const isInteractive = $derived(!disabled && !loading);
</script>

<button
  {id}
  {type}
  class="btn {variantClass} {sizeClass} motion-reduce:transition-none"
  aria-busy={loading ? "true" : undefined}
  aria-disabled={!isInteractive ? "true" : undefined}
  aria-label={rest["aria-label"]}
  aria-describedby={rest["aria-describedby"]}
  disabled={!isInteractive}
  onclick={isInteractive ? onclick : undefined}
>
  {#if loading}
    <span
      class="loading loading-spinner loading-sm"
      aria-hidden="true"
    ></span>
  {:else if iconStart}
    {@render iconStart()}
  {/if}
  {#if children}
    {@render children()}
  {/if}
  {#if iconEnd && !loading}
    {@render iconEnd()}
  {/if}
</button>
