<!--
  Input.svelte — shared primitive (PR 4 of caduxo-daisyui-redesign).

  Composes DaisyUI `input input-{size}` with `input-error` when
  `invalid`, wrapped in DaisyUI v5's `fieldset` + `fieldset-legend`
  pattern (the v4-era `form-control` / `label` / `label-text` /
  `label-text-alt` classes are NOT emitted by DaisyUI v5). The
  primitive renders the visible label + required marker + helper
  text via the fieldset pattern so consumers do not have to hand-
  roll the spacing.

  Note: an earlier revision surfaced a `<datalist>` snippet slot
  bound to a `list` prop, but every consumer that needed native
  autocomplete semantics has since migrated to the themed
  `Combobox.svelte` primitive. The `list` prop, the `datalist`
  snippet, and the `<datalist>` render block were all dead code;
  they were removed in the fix/category-picker-popovers branch
  cleanup so the primitive owns only the contracts it actually
  serves today.

  No hardcoded user-facing copy: every visible string enters
  through props (`label`, `helper`) or slots (`leading`, `trailing`).

  Note: DaisyUI v5 emits `input` with a default border — the v4-era
  `input-bordered` modifier is dead and is intentionally NOT emitted.

  Tailwind classes referenced here (for the JIT scanner):
    input input-sm input-md input-lg
    input-error
    fieldset fieldset-legend
    label
    motion-reduce:transition-none
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export type InputType =
    | "text"
    | "search"
    | "number"
    | "email"
    | "url"
    | "password";

  export type InputSize = "sm" | "md" | "lg";

  interface Props {
    /** Two-way bindable input value. */
    value: string;
    type?: InputType;
    /** Visible label rendered above the input. */
    label: string;
    /** When true, surfaces a `*` after the legend (visible marker) plus an
     *  sr-only `(required)` annotation so screen readers announce the
     *  required state. */
    required?: boolean;
    /** Optional helper text rendered below the input. */
    helper?: string;
    /** When true, the input renders with `input-error` (red border). */
    invalid?: boolean;
    size?: InputSize;
    disabled?: boolean;
    /** Native placeholder text. */
    placeholder?: string;
    /** Native `name` attribute for form submission. */
    name?: string;
    /** Native `maxlength` attribute. */
    maxlength?: number;
    /** Native `minlength` attribute. */
    minlength?: number;
    /** Plain accessible label override (rare). */
    "aria-label"?: string;
    /** id of an element that describes the input (helper text, error message). */
    "aria-describedby"?: string;
    /** Optional id passthrough for the input element. */
    id?: string;
    /** Fired on every keystroke. */
    oninput?: (value: string) => void;
    /** Fired when the input loses focus. */
    onblur?: (event: FocusEvent) => void;
  }

  let {
    value = $bindable(),
    type = "text",
    label,
    required = false,
    helper,
    invalid = false,
    size = "md",
    disabled = false,
    placeholder,
    name,
    maxlength,
    minlength,
    "aria-label": ariaLabel,
    "aria-describedby": ariaDescribedBy,
    id,
    oninput,
    onblur,
  }: Props = $props();

  const sizeClass = $derived.by((): string => {
    switch (size) {
      case "sm":
        return "input-sm";
      case "lg":
        return "input-lg";
      default:
        return "input-md";
    }
  });

  const invalidClass = $derived(invalid ? "input-error" : "");

  // Helper text id is used for `aria-describedby` so screen readers
  // announce the helper text when the input receives focus. Auto-
  // generated per instance so multiple inputs on a page do not
  // collide.
  const helperId = $derived(
    id ? `${id}-helper` : `input-helper-${Math.random().toString(36).slice(2, 10)}`,
  );

  // Compose `aria-describedby`: the consumer-supplied value first,
  // then the auto-generated helper id when helper text is present.
  const effectiveDescribedBy = $derived.by((): string | undefined => {
    const ids: string[] = [];
    if (ariaDescribedBy) ids.push(ariaDescribedBy);
    if (helper) ids.push(helperId);
    return ids.length > 0 ? ids.join(" ") : undefined;
  });

  function handleInput(event: Event) {
    const target = event.currentTarget as HTMLInputElement;
    value = target.value;
    oninput?.(target.value);
  }
</script>

<fieldset class="fieldset">
  <legend class="fieldset-legend">
    {label}
    {#if required}
      <span class="text-error" aria-hidden="true">*</span>
      <span class="sr-only">(required)</span>
    {/if}
  </legend>
  <input
    {id}
    {name}
    {type}
    {placeholder}
    {disabled}
    {maxlength}
    {minlength}
    class="input {sizeClass} {invalidClass} motion-reduce:transition-none w-full"
    aria-label={ariaLabel}
    aria-describedby={effectiveDescribedBy}
    aria-invalid={invalid ? "true" : undefined}
    aria-required={required ? "true" : undefined}
    value={value}
    oninput={handleInput}
    {onblur}
  />
  {#if helper}
    <p class="label" id={helperId}>{helper}</p>
  {/if}
</fieldset>