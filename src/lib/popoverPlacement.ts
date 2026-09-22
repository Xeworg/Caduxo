/**
 * Shared popover placement helper used by the themed Svelte popover
 * primitives (`CategoryPicker`, `Combobox`, `Listbox`).
 *
 * The helper measures the popover's actual rendered content, decides
 * which side of the trigger has more room, prefers opening *below*
 * the trigger when the rendered content fits there, and clamps the
 * result to the viewport with a small safety margin so the popover
 * never disappears under the viewport edge.
 *
 * Why a single helper:
 *
 * - Three primitives used to hand-roll the same placement math
 *   (`getBoundingClientRect`, `offsetHeight` / `scrollHeight`, viewport
 *   clamp, below-vs-above decision). Diverging copies produced visible
 *   regressions (popovers opening above short lists, popovers left
 *   un-refreshed after results changed, mismatched widths).
 * - The helper is a *pure* function: no internal state, no listeners.
 *   Components own their own lifecycle and decide when to call it
 *   (open, scroll, resize, results changed, create row appeared, etc.).
 *   Re-running is safe and cheap — the helper re-measures every call
 *   so the position tracks the live rendered content.
 *
 * The helper does NOT touch positioning that is intrinsically
 * component-specific (focus management, keyboard navigation, click
 * handlers). It is intentionally small and typed.
 */

/** Which side of the trigger the popover was placed on. */
export type PopoverSide = "above" | "below";

/** Inputs to {@link placePopover}. */
export interface PlacePopoverOptions {
    /** The trigger element the popover is anchored to. */
    trigger: HTMLElement;
    /** The popover element to position. Required; the helper measures
     *  its actual rendered size before deciding the side. */
    popover: HTMLElement;
    /** Upper bound for the popover height (must match the CSS
     *  `max-height`). The measured height is capped at this value so
     *  the side decision is not driven by a fully-scrolled popover's
     *  scrollHeight alone. */
    maxHeight: number;
    /** When set, the popover's left edge is clamped so the popover
     *  stays within `margin` pixels of the trigger's left edge. If
     *  the popover would overflow the right viewport edge, the left
     *  coordinate is pulled back so the popover still fits. */
    minWidth?: number;
    /** When true, the popover's rendered width matches the trigger's
     *  width (floored by `minWidth`). Mirrors the WAI-ARIA 1.2
     *  select-only combobox contract where the listbox width tracks
     *  the combobox trigger width. */
    matchTriggerWidth?: boolean;
    /** Explicit popover width (pixels). Takes precedence over
     *  `matchTriggerWidth`; use when the popover has a fixed width
     *  contract (e.g. `CategoryPicker`'s themed 300px width). */
    width?: number;
    /** Viewport safety margin in pixels. The popover is clamped so
     *  it stays within `margin` pixels of every viewport edge.
     *  Defaults to 8. */
    margin?: number;
    /** Gap between the trigger and the popover in pixels. Defaults
     *  to 4. */
    gap?: number;
}

/** Result of {@link placePopover}. Returned for callers that want to
 *  react to which side was chosen (e.g. to flip a caret arrow). */
export interface PlacementResult {
    /** The side the popover was placed on. */
    side: PopoverSide;
    /** Applied viewport-relative top (CSS pixels). */
    top: number;
    /** Applied viewport-relative left (CSS pixels). */
    left: number;
    /** Applied width (CSS pixels). */
    width: number;
    /** Measured height the helper used for the side decision (CSS
     *  pixels, capped at `maxHeight`). */
    height: number;
}

/**
 * Compute and apply the popover's viewport-relative top/left/width so
 * the popover stays anchored to the trigger and prefers opening below
 * when the rendered content fits.
 *
 * The function mutates `popover.style.{top,left,position,width}` and
 * also returns the resolved placement so callers can react.
 *
 * Safe to call repeatedly: the popover is re-measured on every
 * invocation so the position tracks the live rendered content (e.g.
 * after a search results update, after an inline create row appears
 * or disappears, after a tab switch). Multiple calls in a single
 * frame coalesce naturally because the browser only paints once.
 *
 * @example
 * placePopover({ trigger: triggerEl, popover: popoverEl, maxHeight: 320, width: 300 });
 */
export function placePopover(options: PlacePopoverOptions): PlacementResult {
    const {
        trigger,
        popover,
        maxHeight,
        minWidth = 0,
        matchTriggerWidth = false,
        width: explicitWidth,
        margin = 8,
        gap = 4,
    } = options;

    const rect = trigger.getBoundingClientRect();

    // Use the larger of offsetHeight / scrollHeight so a fully-scrolled
    // popover (where scrollHeight > offsetHeight because content clips
    // behind `overflow-y: auto`) still picks the visible content size.
    // Fall back to `maxHeight` on the very first paint where both
    // measurements can read 0 (e.g. right after `isOpen` flips).
    const measuredHeight = Math.max(
        popover.offsetHeight,
        popover.scrollHeight,
    );
    const effectiveHeight =
        measuredHeight > 0 ? Math.min(maxHeight, measuredHeight) : maxHeight;

    const vw = window.innerWidth;
    const vh = window.innerHeight;

    // Side decision: prefer below when the rendered content fits there
    // with the configured gap. Otherwise open above. The previous
    // implementation short-circuited with `|| spaceBelow >= spaceAbove`
    // which forced below placement even when most of the popover would
    // overflow the viewport — the new rule only looks at whether the
    // popover *fits* below, not which side has more raw space.
    const spaceBelow = vh - rect.bottom;
    const placeBelow = spaceBelow >= effectiveHeight + gap;

    let top: number;
    if (placeBelow) {
        const desiredTop = rect.bottom + gap;
        // Even when below placement fits, the popover can still spill
        // past the bottom on tall content; clamp to keep the header
        // visible.
        top = Math.min(desiredTop, vh - effectiveHeight - margin);
    } else {
        const desiredTop = rect.top - effectiveHeight - gap;
        top = Math.max(margin, desiredTop);
    }

    // Width: explicit override > match trigger > natural width.
    let popoverWidth: number;
    if (explicitWidth !== undefined) {
        popoverWidth = explicitWidth;
    } else if (matchTriggerWidth) {
        popoverWidth = Math.max(rect.width, minWidth);
    } else {
        popoverWidth = Math.max(
            popover.offsetWidth || rect.width,
            rect.width,
            minWidth,
        );
    }

    const left = Math.max(margin, Math.min(rect.left, vw - popoverWidth - margin));

    popover.style.position = "fixed";
    popover.style.top = `${top}px`;
    popover.style.left = `${left}px`;
    popover.style.width = `${popoverWidth}px`;

    return {
        side: placeBelow ? "below" : "above",
        top,
        left,
        width: popoverWidth,
        height: effectiveHeight,
    };
}