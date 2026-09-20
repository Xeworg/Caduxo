<!--
  Input.svelte — shared primitive (PR 4 of caduxo-daisyui-redesign).

  Composes DaisyUI `input input-bordered input-{size}` with
  `input-error` when `invalid`, paired with `label`,
  `label-text-alt`, and an optional helper / error description. The
  primitive renders the visible label + required marker + helper
  text via DaisyUI's field pattern (the DaisyUI form-control
  layout) so consumers do not have to hand-roll the spacing.

  When the consumer supplies a `list` prop, the primitive renders
  a `<datalist id={list}>` slot the consumer fills — used by
  `ProductForm` to keep its barcode-type and unit-definition
  autocomplete semantics verbatim (per design §4.2 / spec).

  No hardcoded user-facing copy: every visible string enters
  through props (`label`, `helper`) or slots (`leading`, `trailing`).

  Tailwind classes referenced here (for the JIT scanner):
    input input-bordered input-sm input-md input-lg
    input-error
    label label-text label-text-alt
    form-control
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
    /** When true, surfaces a `*` after the label (via `label-text-alt`). */
    required?: boolean;
    /** Optional helper text rendered below the input. */
    helper?: string;
    /** When true, the input renders with `input-error` (red border). */
    invalid?: boolean;
    /** Optional `id` of a `<datalist>` for autocomplete suggestions. */
    list?: string;
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
    /**
     * Optional `<datalist>` snippet — rendered after the input as a
     * sibling. The consumer fills it with `<option>` elements. The
     * datalist's id is the consumer-supplied `list` prop value, so
     * the native autocomplete wiring works out of the box.
     */
    datalist?: Snippet;
  }

  let {
    value = $bindable(),
    type = "text",
    label,
    required = false,
    helper,
    invalid = false,
    list,
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
    datalist,
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

<label class="form-control w-full" for={id}>
  <div class="label">
    <span class="label-text">
      {label}
      {#if required}
        <span class="text-error" aria-hidden="true">*</span>
        <span class="sr-only">(required)</span>
      {/if}
    </span>
  </div>
  <input
    {id}
    {name}
    {type}
    {placeholder}
    {disabled}
    {maxlength}
    {minlength}
    {list}
    class="input input-bordered {sizeClass} {invalidClass} motion-reduce:transition-none w-full"
    aria-label={ariaLabel}
    aria-describedby={effectiveDescribedBy}
    aria-invalid={invalid ? "true" : undefined}
    aria-required={required ? "true" : undefined}
    value={value}
    oninput={handleInput}
    {onblur}
  />
  {#if helper}
    <div class="label">
      <span class="label-text-alt text-sm opacity-70" id={helperId}>
        {helper}
      </span>
    </div>
  {/if}
  {#if datalist}
    {@render datalist()}
  {/if}
</label>