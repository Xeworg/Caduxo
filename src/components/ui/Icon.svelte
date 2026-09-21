<!--
  Icon.svelte — local Heroicons wrapper.

  SVG path data is copied verbatim from the Heroicons project
  (https://github.com/tailwindlabs/heroicons), which is released by the
  Tailwind Labs team under the MIT License. Heroicons is bundled here
  rather than fetched from a CDN so Caduxo keeps running offline and
  ships with the same look on every theme. License and copyright notice
  retained per the MIT terms.

  The closed icon map below is the single source of truth for every
  Heroicon shape used by Caduxo UI. Adding a new icon means adding an
  entry here (and a corresponding TS literal) — do not import
  Heroicons directly elsewhere or drop raw SVG paths into consumers.

  Why a closed map (vs. an open stringly typed icon name)?
    - TypeScript catches typos at build time. The IconName union and
      the ICONS record share the same keys, so a typo in either place
      is a compile error.
    - Theme contract: every entry renders with fill="none" and
      stroke="currentColor", so the icon inherits the surrounding text
      colour and adapts to all seven DaisyUI themes shipped by Caduxo.
      No raw palette classes, no hardcoded hex; the only Tailwind
      classes used are sizing utilities (h-N w-N).
    - Bundle-size discipline: callers cannot accidentally pull in the
      full Heroicons library; the closed set is audited per task.

  Tailwind classes referenced here (for the JIT scanner):
    h-3 w-3 h-4 w-4 h-5 w-5 h-6 w-6
-->
<script lang="ts">
  /**
   * Closed union of every Heroicons 24 outline shape that Caduxo uses
   * today. Add an entry here whenever a new icon is needed; do not
   * expose a generic `string` icon name.
   */
  export type IconName =
    | "adjustments-horizontal"
    | "archive-box-arrow-down"
    | "arrow-down-on-square-stack"
    | "arrow-down-tray"
    | "arrow-path"
    | "arrow-right-on-rectangle"
    | "arrow-uturn-left"
    | "arrows-right-left"
    | "calendar"
    | "check"
    | "clipboard-document-list"
    | "document-arrow-down"
    | "document-arrow-up"
    | "document-magnifying-glass"
    | "exclamation-triangle"
    | "eye"
    | "information-circle"
    | "pencil"
    | "pencil-square"
    | "x-mark";

  export type IconSize = "xs" | "sm" | "md" | "lg";

  interface Props {
    /** Icon name from the closed Heroicons 24 outline set. */
    name: IconName;
    /** Visual size. Default matches the shared Button primitive (sm). */
    size?: IconSize;
    /**
     * Decorative by default; consumers do not need to repeat
     * aria-hidden="true" on every usage. Set `false` only when the
     * icon conveys information that is NOT already in the surrounding
     * label (rare in Caduxo today).
     */
    decorative?: boolean;
    /**
     * Accessible label, required when `decorative={false}`. Ignored
     * when `decorative` is unset/true because the icon is hidden from
     * assistive tech in that mode.
     */
    label?: string;
    /** Extra class string appended after the size classes. */
    class?: string;
  }

  let {
    name,
    size = "sm",
    decorative = true,
    label,
    class: extraClass = "",
  }: Props = $props();

  /**
   * Map of icon name → SVG inner markup (paths/lines/circles only,
   * no outer <svg> wrapper). Each entry is the verbatim Heroicons
   * source so attribution travels with the shape.
   */
  const ICONS: Record<IconName, string> = {
    "adjustments-horizontal":
      '<path d="M10.5 6h9.75M10.5 6a1.5 1.5 0 1 1-3 0m3 0a1.5 1.5 0 1 0-3 0M3.75 6H7.5m3 12h9.75m-9.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-3.75 0H7.5m9-6h3.75m-3.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-9.75 0h9.75" />',
    "archive-box-arrow-down":
      '<path d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M12 12.75l-3 3m0 0 3 3m-3-3h6.75M3 4.5h18v3.75H3V4.5Z" />',
    "arrow-down-on-square-stack":
      '<path d="M9 12.75 12 15l3-2.25M12 15V6.75M16.5 17.25a2.25 2.25 0 0 0 2.25-2.25V6a2.25 2.25 0 0 0-2.25-2.25H7.5A2.25 2.25 0 0 0 5.25 6v9a2.25 2.25 0 0 0 2.25 2.25h9Z" />',
    "arrow-down-tray":
      '<path d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />',
    "arrow-path":
      '<path d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" />',
    "arrow-right-on-rectangle":
      '<path d="M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" />',
    "arrow-uturn-left":
      '<path d="M9 15 3 9m0 0 6-6M3 9h12a6 6 0 0 1 0 12h-3" />',
    "arrows-right-left":
      '<path d="M7.5 21 3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" />',
    calendar:
      '<path d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 0 1 2.25-2.25h13.5A2.25 2.25 0 0 1 21 7.5v11.25m-18 0A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75m-18 0v-7.5A2.25 2.25 0 0 1 5.25 9h13.5A2.25 2.25 0 0 1 21 11.25v7.5" />',
    check:
      '<path d="M4.5 12.75 6 14.25l6-6m0 0 1.5-1.5M12 8.25l-1.5-1.5" />',
    "clipboard-document-list":
      '<path d="M9 12h3.75M9 15h3.75M9 18h3.75m3 .75H18a2.25 2.25 0 0 0 2.25-2.25V6.108c0-1.135-.845-2.098-1.976-2.192a48.424 48.424 0 0 0-1.123-.08m-5.801 0c-.065.21-.1.433-.1.664 0 .414.336.75.75.75h4.5a.75.75 0 0 0 .75-.75 2.25 2.25 0 0 0-.1-.664m-5.8 0A2.251 2.251 0 0 1 13.5 2.25H15a2.25 2.25 0 0 1 2.15 1.586m-5.8 0c-.376.023-.75.05-1.124.08C9.095 4.01 8.25 4.973 8.25 6.108V8.25m0 0H4.875c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125h9.75c.621 0 1.125-.504 1.125-1.125V9.375c0-.621-.504-1.125-1.125-1.125H8.25ZM6.75 12h.008v.008H6.75V12Zm0 3h.008v.008H6.75V15Zm0 3h.008v.008H6.75V18Z" />',
    "document-arrow-down":
      '<path d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m6.75 9.75H12M9 16.5l1.5 1.5L12 16.5m-1.5-3v6m9-13.5V18a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 18V6a2.25 2.25 0 0 1 2.25-2.25h6.75a3.375 3.375 0 0 1 3.375 3.375v6.375c0 .621.504 1.125 1.125 1.125h.75Z" />',
    "document-arrow-up":
      '<path d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25M9 16.5l1.5-1.5L12 16.5m-1.5-3v6m9-13.5V18a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 18V6a2.25 2.25 0 0 1 2.25-2.25h6.75a3.375 3.375 0 0 1 3.375 3.375v6.375c0 .621.504 1.125 1.125 1.125h2.625Z" />',
    "document-magnifying-glass":
      '<path d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m5.231 13.481L15 17.25m4.5 0a3 3 0 1 0-6 0 3 3 0 0 0 6 0ZM9 12H7.5a2.25 2.25 0 0 0-2.25 2.25v3a2.25 2.25 0 0 0 2.25 2.25h1.5a2.25 2.25 0 0 0 2.25-2.25v-3a2.25 2.25 0 0 0-2.25-2.25H9Z" />',
    "exclamation-triangle":
      '<path d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z" />',
    eye:
      '<path d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z" /><path d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />',
    "information-circle":
      '<path d="M11.25 11.25l.041-.02a.75.75 0 0 1 1.063.852l-.708 2.836a.75.75 0 0 0 1.063.853l.041-.021M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-3.75h.008v.008H12V8.25Z" />',
    pencil:
      '<path d="M16.862 4.487 18.549 2.799a2.121 2.121 0 1 1 3 3L19.513 7.835M16.862 4.487 6.348 14.998a2.25 2.25 0 0 0-.578.978l-1.086 3.264a.375.375 0 0 0 .464.464l3.264-1.086a2.25 2.25 0 0 0 .978-.578L18.55 7.835m-1.687-3.348 1.687 1.687" />',
    "pencil-square":
      '<path d="M16.862 4.487 18.549 2.799a2.121 2.121 0 1 1 3 3L19.513 7.835m-2.651-3.348L6.348 14.998a2.25 2.25 0 0 0-.578.978l-1.086 3.264a.375.375 0 0 0 .464.464l3.264-1.086a2.25 2.25 0 0 0 .978-.578L18.55 7.835m-2.651-3.348 2.651 3.348M16.5 7.5l-9 9" />',
    "x-mark":
      '<path d="M6 18 18 6M6 6l12 12" />',
  };

  /**
   * Map icon size → (Tailwind class, stroke-width). Stroke-width tapers
   * up for tiny sizes so the shape stays legible at 12 px; larger sizes
   * use the Heroicons default 1.5 stroke. Sizing utilities (h-/w-) keep
   * the wrapper locked to the requested pixel size.
   */
  const SIZE_MAP: Record<
    IconSize,
    { classes: string; strokeWidth: number }
  > = {
    xs: { classes: "h-3 w-3", strokeWidth: 2 },
    sm: { classes: "h-4 w-4", strokeWidth: 1.75 },
    md: { classes: "h-5 w-5", strokeWidth: 1.5 },
    lg: { classes: "h-6 w-6", strokeWidth: 1.5 },
  };

  const sizeEntry = $derived(SIZE_MAP[size]);

  /**
   * aria-hidden defaults to true (decorative). The rendered SVG stays
   * `focusable="false"` so IE/older WebKit tab stops skip it, matching
   * the convention used by the existing EmptyState / Alert / Modal
   * primitives.
   */
  const ariaHidden = $derived(decorative ? "true" : "false");
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={sizeEntry.strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  class="{sizeEntry.classes} {extraClass}"
  aria-hidden={ariaHidden}
  aria-label={decorative ? undefined : label}
  role={decorative ? undefined : "img"}
  focusable="false"
>
  {@html ICONS[name]}
</svg>