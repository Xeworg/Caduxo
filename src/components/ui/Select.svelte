<!--
  Select.svelte — shared primitive (PR 4 of caduxo-daisyui-redesign).

  Composes DaisyUI `select select-{size}` and `select-error` when
  `invalid`. The native `<select>` element stays in the DOM for form
  semantics (keyboard navigation, mobile OS sheet, screen-reader
  announcements) — the DaisyUI wrapper is a visual overlay.

  Note: DaisyUI v5 emits `select` with a default border — the v4-era
  `select-bordered` modifier is dead and is intentionally NOT emitted.

  No hardcoded user-facing copy: the `options` array supplies the
  `label` for every option (typically bound to `$LL.*` keys), the
  consumer passes `placeholder` text, and any visible surrounding
  label / helper text is supplied via the `label` / `helper` slots
  or wrapping markup.

  Tailwind classes referenced here (for the JIT scanner):
    select select-sm select-md select-lg
    select-error
    motion-reduce:transition-none
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export type SelectSize = "sm" | "md";

  export interface Option {
    value: string;
    label: string;
    /** When true, the option is rendered but unselectable. */
    disabled?: boolean;
  }

  interface Props {
    /** Two-way bindable selected value. */
    value: string;
    options: Option[];
    /** Visual size. `sm` composes `select-sm`; `md` is the default. */
    size?: SelectSize;
    /** Disables interaction; the select renders greyed-out. */
    disabled?: boolean;
    /** When true, the select renders with `select-error` (red border). */
    invalid?: boolean;
    /** Plain accessible label (use when no visible `<label>` is rendered). */
    "aria-label"?: string;
    /** id of a visible `<label>` that labels the select. */
    "aria-labelledby"?: string;
    /** id of an element that describes the select (helper text, error message). */
    "aria-describedby"?: string;
    /** Native `required` attribute — surfaces in form validation. */
    required?: boolean;
    /** Native `name` attribute for form submission. */
    name?: string;
    /** Optional id passthrough for the select element. */
    id?: string;
    /** Fired when the selection changes. */
    onchange?: (value: string) => void;
    /** Optional slot rendered before the native options (used for a placeholder `<option>`). */
    leading?: Snippet;
  }

  let {
    value = $bindable(),
    options,
    size = "md",
    disabled = false,
    invalid = false,
    "aria-label": ariaLabel,
    "aria-labelledby": ariaLabelledBy,
    "aria-describedby": ariaDescribedBy,
    required = false,
    name,
    id,
    onchange,
    leading,
  }: Props = $props();

  const sizeClass = $derived(size === "sm" ? "select-sm" : "select-md");
  const invalidClass = $derived(invalid ? "select-error" : "");

  function handleChange(event: Event) {
    const target = event.currentTarget as HTMLSelectElement;
    value = target.value;
    onchange?.(target.value);
  }
</script>

<select
  {id}
  {name}
  class="select {sizeClass} {invalidClass} motion-reduce:transition-none w-full"
  {disabled}
  {required}
  aria-label={ariaLabel}
  aria-labelledby={ariaLabelledBy}
  aria-describedby={ariaDescribedBy}
  aria-invalid={invalid ? "true" : undefined}
  value={value}
  onchange={handleChange}
>
  {#if leading}
    {@render leading()}
  {/if}
  {#each options as option (option.value)}
    <option value={option.value} disabled={option.disabled}>
      {option.label}
    </option>
  {/each}
</select>