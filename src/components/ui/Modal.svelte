<!--
  Modal.svelte — shared primitive (PR 4 of caduxo-daisyui-redesign).

  Backed by the native `<dialog class="modal">` element so we get native
  focus trap, native Escape handling, and a themed backdrop from the
  browser — no hand-rolled focus-trap implementation. The primitive
  wraps `dialog.showModal()` / `dialog.close()` in an effect that
  reacts to the bindable `open` rune.

  The primitive owns:
    - Open / close lifecycle driven by the `open` prop.
    - The native `<dialog>` `cancel` event — the browser fires this
      when the user presses Escape; the dialog is still open at that
      point. The consumer wires `oncancel` to discard in-progress
      edits per the spec.
    - The native `<dialog>` `close` event — the browser fires this
      after the dialog has actually closed. The primitive uses it to
      flip `open` back to `false` (so two-way binding stays in sync)
      and to restore focus to `returnFocusTo` (no-op when the element
      is no longer in the DOM).
    - A `focusin` listener that bounces focus back into the dialog
      when it strays (DaisyUI's modal class does not add focus trap
      on every browser; the primitive enforces it).
    - A backdrop-click guard — clicks on the dialog element itself
      (i.e. the backdrop area, not the inner modal-box) only close
      the modal when `closeOnBackdrop` is true.
    - Scoped CSS for `<dialog>::backdrop` with a `backdrop-blur-sm`
      only when `prefers-reduced-motion: no-preference` is set
      (per design §5.10).

  The consumer owns:
    - Header copy (titles / descriptions).
    - All body / footer content (slots).
    - The `returnFocusTo` element — usually captured by an
      `onClose` handler on the consumer side via `event.currentTarget`.

  Tailwind classes referenced here (for the JIT scanner):
    modal modal-box modal-bottom sm:modal-middle
    modal-action
    btn btn-ghost btn-circle btn-sm
    motion-reduce:transition-none
-->
<script lang="ts" module>
  /**
   * CSS selector for focusable elements inside the dialog. Used by
   * the focus-trap lifecycle and the initial-focus effect.
   */
  const FOCUSABLE_SELECTOR = [
    "a[href]",
    "button:not([disabled])",
    "input:not([disabled])",
    "select:not([disabled])",
    "textarea:not([disabled])",
    "[tabindex]:not([tabindex='-1'])",
  ].join(",");
</script>

<script lang="ts">
  import type { Snippet } from "svelte";

  export type ModalSize = "sm" | "md" | "wide";

  interface Props {
    /**
     * Two-way bindable open state. Setting to `true` calls
     * `dialog.showModal()`; setting to `false` calls
     * `dialog.close()`. The primitive flips it back to `false`
     * when the dialog closes (Escape, backdrop click, native
     * close button, programmatic `close()`).
     */
    open: boolean;
    /** Width variant — `sm` ≈ 320 px, `md` default, `wide` ≈ 720 px. */
    size?: ModalSize;
    /**
     * When true (default), clicking on the dialog backdrop closes
     * the modal. Set to false for forms where accidental backdrop
     * clicks would discard in-progress edits.
     */
    closeOnBackdrop?: boolean;
    /**
     * When true (default), pressing Escape triggers the native
     * `cancel` event (consumer hooks `oncancel` to discard edits).
     * Set to false to suppress Escape entirely.
     */
    closeOnEscape?: boolean;
    /**
     * When true, the modal renders a trailing close button in the
     * header. The button's accessible label is supplied by the
     * consumer via `closeLabel` — the primitive never ships a
     * default string.
     */
    showClose?: boolean;
    /** Accessible label for the close button (required when `showClose`). */
    closeLabel?: string;
    /**
     * Element to receive focus when the modal closes. The
     * primitive no-ops the restoration when the element is not
     * in the DOM at the time of close (per spec's
     * "focus restoration edge cases" mitigation).
     */
    returnFocusTo?: HTMLElement | null;
    /** `aria-labelledby` target for the modal title (e.g. an `<h2 id>`). */
    titleId?: string;
    /** `aria-describedby` target for an optional description paragraph. */
    descriptionId?: string;
    /** Optional id passthrough for the dialog element. */
    id?: string;
    /** Plain `aria-label` fallback when no `titleId` is supplied. */
    "aria-label"?: string;
    /**
     * Fires when the user presses Escape while the modal is open.
     * The dialog is still open at this point — call `onclose()` or
     * flip `open = false` from the consumer to dismiss. The
     * primitive does NOT auto-close on cancel (so the consumer can
     * confirm with the user before discarding edits).
     */
    oncancel?: (event: Event) => void;
    /**
     * Fires after the modal has actually closed (Escape confirmed,
     * backdrop click, close button, or programmatic close). The
     * dialog is already hidden at this point; use this to flip
     * consumer state.
     */
    onclose?: () => void;
    /** Default body content. */
    children?: Snippet;
    /** Optional footer slot rendered in `modal-action`. */
    footer?: Snippet;
  }

  let {
    open = $bindable(),
    size = "md",
    closeOnBackdrop = true,
    closeOnEscape = true,
    showClose = false,
    closeLabel,
    returnFocusTo = null,
    titleId,
    descriptionId,
    id,
    "aria-label": ariaLabel,
    oncancel,
    onclose,
    children,
    footer,
  }: Props = $props();

  let dialog: HTMLDialogElement | undefined = $state();

  // Map our size variant onto DaisyUI's `modal-box` width utilities.
  // DaisyUI's `modal-box` ships at ~32rem on lg+; we use Tailwind
  // utility classes to widen (wide) or narrow (sm) without breaking
  // the responsive contract.
  const sizeClass = $derived.by((): string => {
    switch (size) {
      case "sm":
        return "max-w-sm";
      case "wide":
        return "max-w-3xl";
      default:
        return "max-w-md";
    }
  });

  // Stable id when the consumer does not supply one. SSR-safe because
  // the dialog is only opened in the browser (showModal requires a
  // user gesture or a microtask after the first interaction).
  const dialogId = $derived(
    id ?? `modal-${Math.random().toString(36).slice(2, 10)}`,
  );

  // ── Open / close lifecycle ────────────────────────────────────────
  // React to `open` flips: open the dialog via `showModal()` when it
  // becomes true; close it via `close()` when it becomes false. The
  // `<dialog>` `close` handler below flips `open` back to false so
  // the parent's `bind:open` stays in sync.
  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      // showModal() throws if the dialog is already open — the
      // `!dialog.open` guard covers that case.
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  function handleClose() {
    // The browser fired the native `close` event — the dialog is
    // already hidden. Flip `open` so the parent's two-way binding
    // stays in sync, then restore focus if the trigger element is
    // still in the DOM.
    if (open) open = false;
    onclose?.();
    if (returnFocusTo && document.body.contains(returnFocusTo)) {
      // Wrap the call in a microtask so the browser has time to
      // finish its own focus-restoration logic first (some browsers
      // restore focus to <body> before our listener runs).
      queueMicrotask(() => {
        try {
          returnFocusTo.focus();
        } catch {
          // Defensive — focus() can throw if the element was
          // removed between the contains() check and the focus()
          // call (race condition during hot reload).
        }
      });
    }
  }

  function handleCancel(event: Event) {
    // The browser fires `cancel` when the user presses Escape. The
    // dialog is still open at this point. We suppress the default
    // when `closeOnEscape` is false so Escape does nothing.
    if (!closeOnEscape) {
      event.preventDefault();
      return;
    }
    oncancel?.(event);
    // Note: we do NOT auto-flip `open` here — the native close()
    // fires next and our `handleClose` listener takes over. The
    // consumer can call `dialog.close()` from `oncancel` if it
    // wants to confirm and then dismiss.
  }

  function handleClick(event: MouseEvent) {
    // Backdrop click — the native dialog fires a `click` whose
    // target IS the dialog element when the user clicks outside
    // the inner modal-box. We close only when allowed.
    if (!closeOnBackdrop) return;
    if (event.target === dialog) {
      dialog.close();
    }
  }

  // ── Focus-trap lifecycle ───────────────────────────────────────────
  // DaisyUI's `modal` class positions the dialog in the top layer
  // but does not enforce a focus trap on every browser. We listen
  // for `focusin` on the document and bounce focus back inside the
  // dialog when it strays (Tab past the last focusable, Shift+Tab
  // before the first focusable, or programmatic focus moves).
  $effect(() => {
    if (!dialog) return;

    function handleFocusIn(event: FocusEvent) {
      if (!dialog || !dialog.open) return;
      const target = event.target as Node | null;
      if (target && dialog.contains(target)) return;
      // Focus strayed outside the dialog — bounce it back.
      const focusable = dialog.querySelector<HTMLElement>(
        FOCUSABLE_SELECTOR,
      );
      if (focusable) {
        focusable.focus();
      } else {
        // No focusable element — fall back to the dialog itself so
        // the browser does not move focus to <body>.
        dialog.focus();
      }
    }

    document.addEventListener("focusin", handleFocusIn);
    return () => {
      document.removeEventListener("focusin", handleFocusIn);
    };
  });

  // ── Initial focus when the modal opens ───────────────────────────
  // When the dialog opens, focus the first focusable element so
  // keyboard users land somewhere useful (the close button, the
  // first form input, the first action button, …).
  $effect(() => {
    if (!dialog) return;
    if (!dialog.open) return;
    const focusable = dialog.querySelector<HTMLElement>(FOCUSABLE_SELECTOR);
    if (focusable) {
      // Defer to the next microtask so the dialog has finished its
      // own open animation before we move focus.
      queueMicrotask(() => focusable.focus());
    }
  });
</script>

<!--
  The dialog element. We listen for the native `cancel` event
  (Escape press — dialog still open) and `close` (dialog actually
  closed). The `click` listener implements backdrop-click-to-close
  per `closeOnBackdrop`.

  NOTE: the `class="modal modal-bottom sm:modal-middle"` Tailwind
  utilities come from DaisyUI v5 — the v5 modal class supports the
  `modal-bottom` variant for narrow viewports and the
  `sm:modal-middle` variant to centre on wider screens.
-->
<dialog
  bind:this={dialog}
  id={dialogId}
  class="modal modal-bottom sm:modal-middle"
  aria-labelledby={titleId}
  aria-describedby={descriptionId}
  aria-label={!titleId ? ariaLabel : undefined}
  oncancel={handleCancel}
  onclose={handleClose}
  onclick={handleClick}
>
  <div class="modal-box {sizeClass} motion-reduce:transition-none">
    {#if showClose}
      <form method="dialog" class="absolute right-2 top-2">
        <button
          type="submit"
          class="btn btn-ghost btn-circle btn-sm"
          aria-label={closeLabel}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="h-4 w-4"
            aria-hidden="true"
            focusable="false"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </form>
    {/if}
    {#if children}
      {@render children()}
    {/if}
    {#if footer}
      <div class="modal-action">
        {@render footer()}
      </div>
    {/if}
  </div>
</dialog>

<!--
  Scoped backdrop styles. The blur is gated on
  `@media (prefers-reduced-motion: no-preference)` because the
  backdrop-filter render itself triggers a paint transition on
  some platforms (per design §5.10).

  The `:backdrop` pseudo-element fires when the `<dialog>` is open
  in the top layer — DaisyUI v5 applies its own backdrop colour,
  and our blur supplements it on capable platforms.
-->
<style>
  dialog.modal::backdrop {
    background-color: color-mix(in oklch, black 40%, transparent);
  }

  @media (prefers-reduced-motion: no-preference) {
    dialog.modal::backdrop {
      backdrop-filter: blur(4px);
      -webkit-backdrop-filter: blur(4px);
    }
  }

  /* Reduce-motion users: fall back to a solid (non-blur) backdrop. */
  @media (prefers-reduced-motion: reduce) {
    dialog.modal::backdrop {
      backdrop-filter: none;
      -webkit-backdrop-filter: none;
    }
  }
</style>