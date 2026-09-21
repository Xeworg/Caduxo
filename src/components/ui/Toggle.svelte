<!--
  Toggle.svelte — shared primitive (PR 3 of caduxo-daisyui-redesign).
  Replaces the bespoke `toggle-wrap` / `toggle-track` / `toggle-thumb`
  pattern. Composes DaisyUI `toggle toggle-primary toggle-{size}`. The
  underlying `<input type="checkbox">` stays in the DOM for form
  semantics — DaisyUI's toggle is purely a visual wrapper around the
  native input.

  No hardcoded user-facing copy: the visible `label` is supplied by
  the consumer (typically bound to `$LL.…`). For screen-reader-only
  labels, use `aria-label` instead of `label`.

  Tailwind classes referenced here (for the JIT scanner):
    toggle toggle-primary toggle-sm toggle-md toggle-lg
    motion-reduce:transition-none
-->
<script lang="ts">
  export type ToggleSize = "sm" | "md";

  interface Props {
    /** Controlled checked state. */
    checked: boolean;
    /** Visible label (renders next to the toggle). */
    label?: string;
    /** Plain `aria-label` override when no visible label is supplied. */
    "aria-label"?: string;
    /** Forwarded `aria-describedby` for helper text. */
    "aria-describedby"?: string;
    size?: ToggleSize;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
    id?: string;
  }

  let {
    checked,
    label,
    size = "md",
    disabled = false,
    onchange,
    id,
    ...rest
  }: Props = $props();

  const sizeClass = $derived(size === "sm" ? "toggle-sm" : "toggle-md");

  function handleChange(event: Event) {
    const target = event.currentTarget as HTMLInputElement;
    onchange?.(target.checked);
  }
</script>

{#if label}
  <label class="flex items-center gap-2" for={id}>
    <input
      {id}
      type="checkbox"
      class="toggle toggle-primary {sizeClass} motion-reduce:transition-none"
      checked={checked}
      disabled={disabled}
      aria-describedby={rest["aria-describedby"]}
      onchange={handleChange}
    />
    <span>{label}</span>
  </label>
{:else}
  <input
    {id}
    type="checkbox"
    class="toggle toggle-primary {sizeClass} motion-reduce:transition-none"
    checked={checked}
    disabled={disabled}
    aria-label={rest["aria-label"]}
    aria-describedby={rest["aria-describedby"]}
    onchange={handleChange}
  />
{/if}
