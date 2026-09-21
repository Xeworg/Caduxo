# Design — caduxo-daisyui-redesign

> Technical approach for the Tailwind v4 + DaisyUI redesign of
> `caduxo-expiry-tracker`. This document defines *how* the proposal and
> spec are implemented: concrete file paths, plugin ordering, primitive
> contracts, motion tokens, theme persistence wiring, migration order,
> and the chained-PR shape. It does not introduce requirements that
> contradict the spec — every requirement under
> `openspec/changes/caduxo-daisyui-redesign/specs/caduxo-expiry-tracker/spec.md`
> remains authoritative; this design is the implementation blueprint
> that satisfies them.

## 1. Stack additions and dependency wiring

### 1.1 New dependencies

Added to `package.json` `dependencies`:

- `tailwindcss@^4`
- `@tailwindcss/vite@^4` (Tailwind v4 Vite plugin)
- `daisyui@^5`

`tailwindcss` and `@tailwindcss/vite` are added to `dependencies`
rather than `devDependencies` because Tailwind v4 ships the runtime
used by the generated `app.css`; the `@tailwindcss/vite` plugin
operates at build time but is published as a runtime package, so this
matches the upstream convention and keeps lockfile intent clear.

### 1.2 Vite plugin ordering

`vite.config.ts` becomes:

```ts
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  clearScreen: false,
  build: { target: "es2022" },
  server: { strictPort: true, port: 1420 },
});
```

`tailwindcss()` is registered **before** `svelte()` so that Tailwind's
class-collection pass sees every Svelte component's class usage during
the same build pass that compiles the components. Reversing the order
does not break the build but causes Tailwind to miss classes referenced
only inside Svelte files, leading to "ghost classes" that render
un-styled. The Phase 1 foundation gate (Section 11) explicitly verifies
this ordering with a manual Tauri/dev-browser launch.

### 1.3 `src/app.css` shape

The new stylesheet becomes the single CSS entry point. It owns:

1. The Tailwind v4 import and DaisyUI plugin registration.
2. The custom `caduxo-light` theme block.
3. The curated `themes: ["caduxo-light", "dark"]` allow-list for v1.
4. Project-scoped motion tokens (Section 7).
5. Global a11y utilities (`sr-only`, `:focus-visible` outline guard).
6. A baseline `prefers-reduced-motion: reduce` reset for animated
   primitives.

Sketch (illustrative, not final implementation):

```css
@import "tailwindcss";
@plugin "daisyui" {
  themes: caduxo-light --default, dark;
  root: ":root";
  logs: false;
}

@plugin "daisyui/theme" {
  name: "caduxo-light";
  default: true;
  prefersdark: false;
  color-scheme: light;
  /* semantic overrides derived from #2563eb / #f6f8fb /
     existing urgency palette — see Section 3 */
  --color-primary: oklch(54% 0.18 264);
  --color-primary-content: oklch(98% 0.005 264);
  --color-base-100: oklch(98% 0.005 240);
  --color-base-200: oklch(96% 0.008 240);
  --color-base-300: oklch(92% 0.01 240);
  --color-base-content: oklch(20% 0.02 264);
  --color-secondary: oklch(60% 0.06 240);
  --color-accent: oklch(70% 0.12 200);
  --color-neutral: oklch(28% 0.02 264);
  --color-info: oklch(70% 0.12 230);
  --color-success: oklch(70% 0.16 145);
  --color-warning: oklch(80% 0.18 75);
  --color-error: oklch(62% 0.22 25);
  /* matching -content tokens derived for AA contrast */
  --radius-selector: 0.5rem;
  --radius-field: 0.5rem;
  --radius-box: 0.75rem;
  --size-selector: 0.25rem;
  --size-field: 0.25rem;
  --border: 1px;
  --depth: 1;
  --noise: 0;
}

/* project motion tokens — see Section 7 */
@theme {
  --ease-out-soft: cubic-bezier(0.16, 1, 0.3, 1);
  --duration-fast: 120ms;
  --duration-base: 180ms;
  --duration-slow: 240ms;
}

/* reduced-motion reset — Section 7.4 */
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.001ms !important;
    scroll-behavior: auto !important;
  }
}
```

### 1.4 `src/main.ts` import switch

`src/main.ts` is updated to import `./app.css` instead of `./style.css`.
The legacy `src/style.css` is **retained** during the migration and
deleted only after the Phase 12 grep gate in Section 11 confirms zero
production references to its classes (`.shell`, `.hero`, `.eyebrow`,
`.cards`).

## 2. Shared UI primitive architecture

### 2.1 Directory layout

```
src/components/ui/
├── Button.svelte
├── Card.svelte
├── Modal.svelte
├── Table.svelte
├── Badge.svelte
├── Alert.svelte
├── EmptyState.svelte
├── LoadingState.svelte
├── Tabs.svelte
├── Select.svelte
├── Input.svelte
├── Toggle.svelte
├── Tooltip.svelte
└── theme/
    └── themeStore.svelte.ts   # active-theme rune + helpers
```

Primitives live in their own directory so they are not mistaken for
page components. The `ui/theme/` subdirectory hosts the theme rune;
it is the only non-component file in the directory.

### 2.2 Props / slots conventions

All primitives follow these rules, enforced by the spec and verified
during the verify phase:

1. **Snake_case props, kebab-case attrs.** The project uses Svelte 5
   runes with TypeScript; props are declared in `<script lang="ts">`
   and the public surface uses kebab-case in markup (e.g.
   `<Button variant="primary">`, `<Modal size="wide">`). Boolean
   shorthand props (`disabled`, `loading`, `open`) follow the same
   convention.
2. **Slots over props for content.** Every primitive that has variable
   body text or action content exposes named slots — never props. For
   example `<Card>`, `<Modal>`, `<Alert>`, `<EmptyState>` all expose
   a `children` default slot and named slots for actions. This keeps
   i18n text where it belongs (inside the consumer) and avoids
   primitives owning copy.
3. **i18n discipline.** Primitives MUST NOT call `$LL.*` directly.
   Visible text enters the primitive through slots or through a
   `label` / `description` prop that the consumer binds to the i18n
   store. This keeps primitives reusable in non-i18n contexts and
   keeps the translation tree the only source of user-visible copy.
4. **Class composition via Tailwind.** Primitives compose their
   visual surface from DaisyUI utility classes directly in markup
   (e.g. `class="btn btn-primary btn-sm"`) rather than from per-
   component scoped CSS. The exception is the `Modal` and `Tooltip`
   primitives, which need a small amount of scoped CSS to position
   the popover and to expose the focus-trap lifecycle hook (see
   Section 5).
5. **Accessibility metadata.** Every interactive primitive requires an
   `aria-label` (or `aria-labelledby`) when its accessible name does
   not come from visible text. `<Button variant="icon">` requires
   `aria-label` at the TypeScript level (enforced by a `Snippet` or
   comment-annotated prop with no default).

### 2.3 Primitive API contracts

The contracts below are the canonical shape for v1. Each contract maps
to one or more spec requirements under "Shared UI primitives".

#### `Button.svelte`

```svelte
<script lang="ts" generics="Variant extends ButtonVariant">
  type ButtonVariant =
    | "primary" | "secondary" | "ghost" | "outline"
    | "danger"  | "warning"  | "success" | "link"  | "icon";
  type ButtonSize = "xs" | "sm" | "md" | "lg";

  interface Props {
    variant?: Variant;
    size?: ButtonSize;
    type?: "button" | "submit" | "reset";
    disabled?: boolean;
    loading?: boolean;
    /** Required when variant === "icon". */
    "aria-label"?: string;
    /** Optional leading icon snippet. */
    iconStart?: import("svelte").Snippet;
    /** Optional trailing icon snippet. */
    iconEnd?: import("svelte").Snippet;
    onclick?: (event: MouseEvent) => void;
    children?: import("svelte").Snippet;
  }
</script>
```

The component composes `btn`, `btn-{variant}`, optional `btn-{size}`,
and `loading loading-spinner loading-sm` (when `loading` is true).
Icon-only buttons require `aria-label`. The `loading` state renders
the spinner and disables interaction; a re-submission guard prevents
double activation.

#### `Card.svelte`

```svelte
interface Props {
  /** Surfaces role="region" + aria-labelledby when true. */
  labelled?: boolean;
  /** Forwarded aria-label / aria-labelledby. */
  "aria-label"?: string;
  "aria-labelledby"?: string;
  /** Optional tone modifier. */
  tone?: "default" | "muted" | "warning" | "error" | "success";
  children: import("svelte").Snippet;
  /** Optional header slot for title/actions. */
  header?: import("svelte").Snippet;
  /** Optional footer slot for actions/meta. */
  footer?: import("svelte").Snippet;
}
```

Composes `card`, `card-body`, with optional `bg-base-200` for the
muted tone and the matching `border-{tone}` modifier for status
tones. The `header` / `footer` slots become `card-title` /
`card-actions` rows.

#### `Modal.svelte`

```svelte
interface Props {
  open: boolean;
  size?: "sm" | "md" | "wide";
  /** When true, clicking on the dialog backdrop closes the modal.
   *  Defaults to true for confirmation dialogs, false for forms. */
  closeOnBackdrop?: boolean;
  /** When true, Escape closes the modal. Defaults to true. */
  closeOnEscape?: boolean;
  /** Optional close affordance — a trailing button in the header. */
  showClose?: boolean;
  /** Trigger element captured for focus restoration. */
  returnFocusTo?: HTMLElement | null;
  titleId?: string;
  descriptionId?: string;
  children: import("svelte").Snippet;
  /** Optional footer slot for actions (Cancel/Save). */
  footer?: import("svelte").Snippet;
}
```

The primitive owns a `<dialog class="modal">` element. It uses
`dialog.showModal()` on `open=true` and `dialog.close()` on `open=
false`. It registers an `onClose` handler on the `<dialog>` element
to detect backdrop-click (a `click` whose target is the dialog
itself) and an `onKeydown` handler for Escape (which the dialog
fires natively before the keydown listener — the primitive listens
for `cancel` and surfaces a `cancel` callback the consumer can hook
to discard in-progress edits). Focus is restored to
`returnFocusTo` (or the `document.activeElement` snapshot taken when
the modal opened) on close. The modal also enforces
`inert`-like focus-trap behaviour by listening for `focusin` and
bouncing focus back inside the dialog if it strays.

#### `Table.svelte`

```svelte
interface Props {
  zebra?: boolean;
  stickyHeader?: boolean;
  size?: "dense" | "default";
  /** Caption for accessibility; becomes the visible `<caption>`. */
  caption?: string;
  /** aria-describedby for the loading/empty slot. */
  describedBy?: string;
  /** Empty / loading slots are mutually exclusive and only one
   *  should be supplied at a time. */
  empty?: import("svelte").Snippet;
  loading?: import("svelte").Snippet;
  children: import("svelte").Snippet;
}
```

Composes `table`, `table-zebra` (when `zebra`), `table-pin-rows`
(when `stickyHeader`), and `table-sm` (when `dense`). The `caption`
slot is rendered inside `<caption class="sr-only">` by default and
becomes visible when `captionVisible` is passed. Numeric columns
should use the project's `num` CSS utility (see Section 4.4) rather
than a prop on the primitive.

#### `Badge.svelte`

```svelte
type Urgency = "expired" | "today" | "alert" | "soon" | "normal";
type Semantic = "success" | "warning" | "error" | "info" | "neutral";

interface Props {
  urgency?: Urgency;
  semantic?: Semantic;
  /** When true, prepends a status dot with the matching semantic. */
  dot?: boolean;
  size?: "sm" | "md";
  children: import("svelte").Snippet;
}
```

Either `urgency` or `semantic` may be supplied; `urgency` takes
precedence when both are set (and is the recommended path for
urgency surfaces). The dot uses the DaisyUI `status status-{...}`
class family. The expired variant optionally renders the pulse
animation defined in Section 7.5.

#### `Alert.svelte`

```svelte
type AlertVariant = "success" | "error" | "warning" | "info";

interface Props {
  variant: AlertVariant;
  /** When true, the alert is dismissible; exposes a close slot. */
  dismissible?: boolean;
  "aria-label"?: string;
  title?: string;
  children: import("svelte").Snippet;
  actions?: import("svelte").Snippet;
}
```

Composes `alert alert-{variant}` with a leading icon (heroicons
inline SVG), the title in `font-semibold`, the body in the default
slot, and an optional `actions` slot for inline buttons. The
`dismissible` affordance is a `btn btn-ghost btn-circle btn-xs`
positioned at the trailing edge and wired through a `dismiss` event.

#### `EmptyState.svelte`

```svelte
interface Props {
  title: string;
  body?: string;
  /** Optional inline-SVG heroicon name. */
  icon?: string;
  actions?: import("svelte").Snippet;
}
```

Composes a centered column with `text-base-content/70` styling, a
64-px heroicon (when supplied), the title in `text-lg font-medium`,
the body in `text-sm`, and the optional action slot. The Spanish
copy pass uses this surface to verify longer translations fit
(Section 8.3).

#### `LoadingState.svelte`

```svelte
interface Props {
  variant?: "skeleton" | "spinner" | "text";
  rows?: number;          // skeleton rows when variant === "skeleton"
  label?: string;         // text fallback label
  /** When true, the loading surface is announced via aria-live. */
  announce?: boolean;
}
```

Composes DaisyUI `skeleton` for the skeleton variant (rows default
to 3), `loading loading-spinner` for the spinner variant, or a
localized `<p>` for the text variant. The `announce` flag sets
`aria-live="polite"` on the wrapper; reduced-motion users receive
the static skeleton regardless of variant.

#### `Tabs.svelte`

```svelte
type TabStyle = "bordered" | "lifted" | "boxed";

interface TabItem {
  id: string;
  label: string;
  panel: import("svelte").Snippet;
}

interface Props {
  items: TabItem[];
  activeId: string;
  style?: TabStyle;
  onchange?: (id: string) => void;
  "aria-label": string;
}
```

Composes DaisyUI `tabs tabs-{style}` with `tab tab-active`. The
primitive handles `ArrowLeft` / `ArrowRight` to move focus and
`Home` / `End` to jump, `Enter` / `Space` to activate, and emits
`onchange` when the active tab changes. `role="tablist"` lives on
the wrapper; each tab is `role="tab"`, each panel is `role="tabpanel"`
with `aria-labelledby` pointing at the tab.

#### `Select.svelte`

```svelte
interface Option {
  value: string;
  label: string;
  disabled?: boolean;
}

interface Props {
  value: string;
  options: Option[];
  "aria-label"?: string;
  "aria-labelledby"?: string;
  size?: "sm" | "md";
  disabled?: boolean;
  invalid?: boolean;
  onchange?: (value: string) => void;
}
```

Composes DaisyUI `select select-bordered select-{size}` with
`select-error` when `invalid`. The native `<select>` element stays
in the DOM for form semantics; the visible surface is the DaisyUI
wrapper. This primitive is the v1 path for every migrated select
(see Section 4.1 for the listbox fallback decision).

#### `Input.svelte`

```svelte
interface Props {
  value: string;
  type?: "text" | "search" | "number" | "email" | "url" | "password";
  label: string;
  /** Required marker — surfaces a `*` after the label. */
  required?: boolean;
  helper?: string;
  invalid?: boolean;
  /** Optional datalist id to wire a <datalist> for autocomplete. */
  list?: string;
  "aria-label"?: string;
  "aria-describedby"?: string;
  size?: "sm" | "md" | "lg";
  disabled?: boolean;
  oninput?: (value: string) => void;
  onblur?: (event: FocusEvent) => void;
}
```

Composes DaisyUI `input input-bordered input-{size}` with
`input-error` when `invalid`, paired with `label`, `label-text`,
and `label-text-alt` for the required marker and helper text. When
`list` is supplied, the primitive renders a `<datalist id={list}>`
slot the consumer fills (used by `ProductForm` to keep its
barcode-type and unit-definition autocomplete).

#### `Toggle.svelte`

```svelte
interface Props {
  checked: boolean;
  label: string;
  "aria-label"?: string;
  "aria-describedby"?: string;
  size?: "sm" | "md";
  disabled?: boolean;
  onchange?: (checked: boolean) => void;
}
```

Composes DaisyUI `toggle toggle-primary toggle-{size}`. The
underlying `<input type="checkbox">` element stays in the DOM for
form semantics.

#### `Tooltip.svelte`

```svelte
interface Props {
  text: string;
  /** Placement relative to the trigger. */
  position?: "top" | "bottom" | "left" | "right";
  /** Optional id for keyboard activation. */
  id?: string;
  children: import("svelte").Snippet;
}
```

Composes DaisyUI `tooltip tooltip-{position}` and `tooltip-open`
when the trigger has focus or hover. Keyboard activation is wired
through focus and `aria-describedby={id}` on the child. Reduced-
motion disables the slide-in.

## 3. Theme system

### 3.1 Theme set shipped at v1 GA

The DaisyUI plugin is configured with `themes: caduxo-light --default,
dark` so that the production bundle ships exactly two themes. Per the
proposal assumptions, accent themes are deferred to a post-v1
follow-up. `caduxo-light` is `default: true` so DaisyUI applies it to
`:root` automatically; `dark` is available for explicit opt-in via
`data-theme="dark"` on `<html>`.

### 3.2 `caduxo-light` brand theme

The theme tokens are derived from the existing palette:

| Token             | Source                                  | Approx. oklch         |
|-------------------|-----------------------------------------|-----------------------|
| `primary`         | existing brand blue `#2563eb`           | `oklch(54% 0.18 264)` |
| `primary-content` | white on the brand blue                 | `oklch(98% 0.005 264)`|
| `base-100`        | existing base background `#f6f8fb`      | `oklch(98% 0.005 240)`|
| `base-200`        | one step darker than base-100           | `oklch(96% 0.008 240)`|
| `base-300`        | border / divider tone                    | `oklch(92% 0.01 240)` |
| `base-content`    | slate-800 `#1e293b`                     | `oklch(20% 0.02 264)` |
| `secondary`       | slate-500 family                         | `oklch(60% 0.06 240)` |
| `accent`          | cyan-400 (the existing brand accent)    | `oklch(70% 0.12 200)` |
| `neutral`         | slate-700                               | `oklch(28% 0.02 264)` |
| `info`            | existing informational blue             | `oklch(70% 0.12 230)` |
| `success`         | existing success green                   | `oklch(70% 0.16 145)` |
| `warning`         | existing warning amber                   | `oklch(80% 0.18 75)`  |
| `error`           | existing error red                       | `oklch(62% 0.22 25)`  |
| `-content` of each semantic | matched for AA contrast      | derived               |

The exact oklch values are tuned during the foundation phase against
the verify report's manual contrast pass (Section 11). The values
above are starting points derived from the existing hex values via
oklch conversion; the verify pass confirms WCAG-AA on body text,
headings, inputs, badges, alerts, and primary button labels.

### 3.3 Theme store

`src/components/ui/theme/themeStore.svelte.ts` exposes the active
theme rune and the theme helpers:

```ts
import type { SettingsResponse, SettingsUpdate } from "../../../lib/stores.js";
import { getSettings, updateSettings } from "../../../lib/stores.js";

export type ThemeName = "caduxo-light" | "dark";
export type ThemeSource = "manual" | "persisted" | "os" | "fallback";

export const theme = $state<{ current: ThemeName; source: ThemeSource }>({
  current: "caduxo-light",
  source: "fallback",
});

/** Bundled theme set — single source of truth for the switcher. */
export const AVAILABLE_THEMES: ThemeName[] = ["caduxo-light", "dark"];

/** Read & apply the persisted theme on launch. See Section 3.4. */
export async function initTheme(): Promise<ThemeName> { /* … */ }

/** Optimistic apply with rollback on IPC failure. See Section 3.5. */
export async function setTheme(next: ThemeName): Promise<void> { /* … */ }
```

### 3.4 Persistence: `app_settings.theme`

The Tauri `SettingsResponse` shape (`src/lib/stores.ts`) is extended
with:

```ts
export interface SettingsResponse {
  /* existing fields… */
  /** Effective active theme. Falls back to OS preference on fresh installs. */
  theme: ThemeName;
  /** True only when the user persisted a theme preference. Mirrors
   *  `language_configured` so the Configuration page can label the
   *  source of the active theme honestly. */
  theme_configured: boolean;
}

export interface SettingsUpdate {
  /* existing fields… */
  theme?: ThemeName;
}
```

The Rust backend (out of design scope but specified here for clarity)
extends `app_settings` with a `theme` text column, defaulting to
`NULL`. `get_settings` reports `theme_configured = theme IS NOT NULL`.
The frontend treats unsupported stored values as the fallback
(`caduxo-light`) without surfacing an error (per the spec's
"unsupported stored theme value falls back to caduxo-light" scenario).

This piggybacks on the existing `settings` command surface — no new
Tauri command is introduced. The frontend change is limited to
`SettingsResponse`, `SettingsUpdate`, and the theme rune/store.

### 3.5 `initTheme` precedence

`initTheme()` mirrors the `initLocale()` pattern in `locale.svelte.ts`
and resolves the active theme in this exact precedence:

```
manual override (in-session, only via Configuration page)   > n/a on boot
persisted app_settings.theme                                 > 1
OS prefers-color-scheme (when app_settings.theme is absent)  > 2
fallback caduxo-light                                        > 3
```

Pseudocode:

```ts
export async function initTheme(): Promise<ThemeName> {
  try {
    const s = await getSettings();
    if (s.theme_configured && isAvailableTheme(s.theme)) {
      applyTheme(s.theme);
      theme.source = "persisted";
      return s.theme;
    }
  } catch {
    // store not ready — fall through
  }

  const os = readPrefersColorScheme(); // "dark" | "light"
  const next: ThemeName = os === "dark" ? "dark" : "caduxo-light";
  applyTheme(next);
  theme.source = os === "dark" ? "os" : "fallback";
  return next;
}
```

The Configuration page theme switcher reads `theme.source` to label
the active source ("Manual", "Persisted", "OS preference", "Default")
so the user can tell *why* they got the theme they got (per the spec
scenario "switcher shows the source of the active theme").

### 3.6 `setTheme` optimistic update + rollback

`setTheme(next)` mirrors `setLocale(next)`:

1. Snapshot `prev = theme.current`.
2. Optimistically apply the new theme via `applyTheme(next)` and set
   `theme.source = "manual"`.
3. Call `updateSettings({ theme: next })`.
4. On IPC failure, revert `applyTheme(prev)`, restore
   `theme.source`, and surface an inline error in the switcher.

`applyTheme(name)` writes `document.documentElement.setAttribute(
"data-theme", name)` — DaisyUI's documented contract for switching
themes at runtime.

### 3.7 OS preference re-read

`initTheme()` re-reads `matchMedia("(prefers-color-scheme: dark)")` on
every app start. There is no client-side cache that shadows the
persisted setting. When `app_settings.theme` exists, OS preference is
ignored (per the spec scenario "stored theme wins over OS preference").

## 4. Native UI replacement strategy

### 4.1 Select vs custom listbox decision

The proposal assumption locks v1 on a single primitive
(`Select.svelte`) that wraps the native `<select>`. The decision
rationale:

- The native `<select>` keeps full keyboard support (Tab, first-letter
  jump, type-ahead), screen-reader semantics, and mobile OS sheet
  behaviour for free.
- DaisyUI's themed surface (`select select-bordered`) replaces the OS
  chrome with the Caduxo visual vocabulary.
- A custom listbox would require re-implementing first-letter jump,
  type-ahead, and the OS-level mobile sheet — out of scope for v1.

The listbox fallback path is reserved for screens where the option
set is large (>20 items) or where the option needs rich rendering
(e.g. category picker with chips). In v1 the only screen approaching
the threshold is the language picker (currently two entries — well
within select territory), so the v1 primitive is sufficient. A future
follow-up can extract `Listbox.svelte` if a screen warrants it.

### 4.2 Native `<datalist>` in `ProductForm`

`Input.svelte` exposes a `list` prop that, when supplied, renders a
`<datalist id={list}>` slot the consumer fills. `ProductForm.svelte`
keeps its existing `<datalist>` markup and only swaps the visual
wrapper for the primitive — autocomplete semantics (keyboard, screen
reader) are preserved verbatim.

### 4.3 Modal migration

All eight-plus modals — `MoveStockModal`, `RegisterExitModal`,
`AdjustCountModal`, `ArchiveLotDialog`, `ResolveQuantityDialog`, plus
the inline overlays inside `DashboardPage` — are migrated to
`Modal.svelte`. The migration steps per modal:

1. Replace the legacy `.modal-overlay` / `.modal-box` markup with
   `<Modal bind:open={visible} …>`.
2. Move the existing close handler to `onClose`; preserve the
   "discard in-progress edits on Escape" semantics by hooking the
   `cancel` callback (the dialog's native `cancel` event fires before
   `close` and is the right hook for cancellation).
3. Replace the `✕` close button with `showClose`.
4. Move header / body / footer markup into the corresponding slots.
5. Preserve all existing business state, validation, and submit
   handlers — only the shell changes.

The native `<dialog>` API gives focus trap and Escape handling for
free. The primitive's `closeOnBackdrop` flag preserves the
click-outside-to-close behaviour (default true for confirmation
dialogs; false for forms where accidental close would lose data).

### 4.4 Numeric alignment for tables

A tiny utility class lives in `src/app.css`:

```css
@utility num {
  font-variant-numeric: tabular-nums;
  text-align: end;
}
```

Tables use `<td class="num">` for right-aligned numeric columns so
that decimals and thousands separators line up across rows. This is
preferred over a primitive prop because it composes cleanly with
every other utility on the cell.

### 4.5 Native checkboxes, radios, toggles

- **Checkboxes / radios:** keep the native `<input type="checkbox">`
  / `<input type="radio">` element in the DOM for form semantics;
  wrap the visual surface with DaisyUI `checkbox checkbox-primary
  checkbox-sm` / `radio radio-primary radio-sm`. The `<label>`
  parent provides the click target and the accessible name.
- **Toggle:** replace the bespoke `toggle-wrap` / `toggle-track` /
  `toggle-thumb` CSS in `ConfigurationPage` with
  `<Toggle checked={…} onchange={…} label={…} />`. The toggle
  primitive renders DaisyUI's `toggle` directly — there is no need
  to preserve the underlying checkbox because the toggle component
  already wraps one internally and exposes the `checked` rune.

### 4.6 `<details>` / `<summary>` in `UnitReviewPage`

The "Map to preset" and "Keep as custom" panels migrate to DaisyUI
`collapse collapse-arrow` (the DaisyUI primitive designed to replace
`<details>`). The native `<details>` element is replaced because the
spec requires proper ARIA roles (`role="region"` with `aria-
labelledby`) — `collapse` exposes those roles natively. Keyboard
ergonomics are preserved: `Enter` / `Space` toggle the panel,
`Tab` moves through the panel content in source order.

### 4.7 DatePicker and CategoryPicker popovers

`DatePicker.svelte` and `inputs/CategoryPicker.svelte` are restyled in
place. Their popover root elements acquire `class="dropdown
dropdown-content …"` (along with the existing positioning classes).
The hand-rolled outside-click listener, scroll/resize listeners, and
fixed positioning stay unchanged — only the visual surface is
swapped. Behaviour, keyboard contracts, ARIA semantics, and the
existing `clearable` / year-range / `Uncategorized` rules are
preserved exactly.

The proposal assumption defers extracting a shared `Popover.svelte`
primitive to a follow-up. If a follow-up extracts one, both pickers
gain a single source of truth for positioning and outside-click
handling; the design keeps that path viable (the pickers are already
isolated from the rest of the visual stack).

### 4.8 Tooltips replacing `title=`

Every `title="…"` attribute on icon-only buttons (the
`UnitReviewBanner` dismiss button, dashboard action buttons, calendar
navigation arrows) is replaced by `<Tooltip text={$LL…}>` with the
icon button as the child. The tooltip is now keyboard-reachable (the
child receives focus and the tooltip opens via `:focus-visible`),
not hover-only as the native `title` is.

### 4.9 App shell navigation

`App.svelte` migrates from its bespoke `.nav` markup to DaisyUI
`navbar bg-base-200` with `navbar-start` (the Caduxo brand),
`navbar-center` (the tab buttons), and `navbar-end` (overflow actions
on narrow viewports). On viewports ≤ 720 px, the tab buttons collapse
behind a `dropdown` trigger so the navigation remains reachable. The
`activeTab` state and the surface badge logic are unchanged.

### 4.10 Themed scrollbars

Wide tables get `class="overflow-x-auto scrollbar-thin
scrollbar-thumb-base-300"` so the scrollbar picks up the active
theme. Reduced-motion users see the same scrollbar (scrollbars do
not animate).

## 5. Effects and motion

### 5.1 Motion token inventory

Defined in `src/app.css` via Tailwind v4 `@theme`:

| Token                | Value                    | Used by                                         |
|----------------------|--------------------------|-------------------------------------------------|
| `--duration-fast`    | `120ms`                  | hover/focus colour transitions                   |
| `--duration-base`    | `180ms`                  | dropdown / popover slide-in, button hover       |
| `--duration-slow`    | `240ms`                  | modal show/hide, card hover lift                |
| `--duration-pulse`   | `1800ms`                 | urgency pulse (expired only)                    |
| `--ease-out-soft`    | `cubic-bezier(0.16,1,0.3,1)` | default ease-out for all transitions         |
| `--ease-in-out-soft` | `cubic-bezier(0.4,0,0.2,1)` | modal show/hide                              |

These tokens are referenced via Tailwind utilities:
`transition-colors duration-fast ease-out-soft` etc. Components MUST
NOT hand-write `transition: ... 0.18s` literals — the verify pass
gates on that.

### 5.2 Modal fade + scale

Composed via DaisyUI's built-in modal animation (`@keyframes
modal-pop`). The animation is automatically disabled when
`prefers-reduced-motion: reduce` is set (DaisyUI respects the media
query internally for its built-in animations). The primitive does
not add any custom animation; it relies on DaisyUI's default and
supplements it with `backdrop-blur-sm` on the `<dialog>::backdrop`
(also disabled by reduced-motion via Section 5.4).

### 5.3 Dropdown / popover slide-in

DaisyUI's `dropdown-content` ships a subtle slide-down animation.
The same reduced-motion behaviour applies. The pickers' hand-rolled
popovers add `class="dropdown-content"` so they inherit the
animation; the verify pass confirms reduced-motion users see no
animation.

### 5.4 Reduced-motion reset

The global reset in `src/app.css` (Section 1.3) clamps every
animation and transition to `0.001ms` when
`prefers-reduced-motion: reduce`. This is the safety net that
guarantees no animated surface escapes the gate; primitives that
opt into custom animations still respect the global reset.

### 5.5 Urgency pulse (expired only)

Defined as a Tailwind keyframe in `src/app.css`:

```css
@theme {
  --animate-urgency-pulse: urgency-pulse var(--duration-pulse)
    var(--ease-in-out-soft) infinite;
}

@keyframes urgency-pulse {
  0%, 100% { opacity: 1; }
  50%      { opacity: 0.55; }
}
```

`Badge.svelte` (when `urgency === "expired"`) and the leading dot
inside the dashboard's `.urgency-card-expired` apply
`motion-safe:animate-urgency-pulse`. Opacity-only — no scale or
translation — so the pulse never triggers vestibular sensitivity.
The pulse is never hover-triggered; it runs from mount.

### 5.6 Card hover lift

Urgency cards and report type cards compose `transition-transform
duration-slow ease-out-soft` with `hover:-translate-y-0.5
hover:shadow-lg motion-reduce:hover:translate-y-0`. The
`motion-reduce:` variant disables the translation while keeping
the shadow change as a static affordance.

### 5.7 Button press feedback

`Button.svelte` composes `active:translate-y-px motion-reduce:active:
translate-y-0` for the press feedback. Disabled state removes both
hover and active affordances.

### 5.8 Skeleton shimmer

`LoadingState.svelte` (variant `skeleton`) renders DaisyUI's
`skeleton` class. DaisyUI's skeleton animation is automatically
disabled by the global reduced-motion reset, so the skeleton becomes
a static grey block when reduced motion is requested — no extra
work is needed in the primitive.

### 5.9 App-shell gradient band

`App.svelte` renders a thin gradient strip
(`bg-gradient-to-br from-primary/5 to-base-100`) behind the navbar
on the dashboard landing tab only. It is a static background — no
animation — and so does not need a reduced-motion gate. The strip is
a decorative affordance; it does not communicate state.

### 5.10 Modal backdrop blur

The `<dialog>::backdrop` receives `backdrop-blur-sm` via scoped CSS
in `Modal.svelte`. The blur is gated on
`@media (prefers-reduced-motion: no-preference) { … }` because the
blur render itself triggers a paint transition on some platforms.

## 6. Per-screen migration strategy

The 13 phases in `docs/daisyui-redesign-plan.md` map cleanly to the
foundation, primitives, page migrations, motion, and cleanup
boundaries below. Each phase is one chained PR; oversized phases
are split further by the tasks phase.

| Phase | Scope | Files (illustrative) | Forecast |
|-------|-------|-----------------------|----------|
| 0  | Baseline screenshots / notes               | `docs/redesign-baseline.md`               | <100 |
| 1  | Foundation: deps + Vite + `app.css` + theme + global reset | `package.json`, `vite.config.ts`, `src/app.css`, `src/main.ts`, `src/components/ui/theme/themeStore.svelte.ts` | ~150 |
| 2  | Shared primitives (batch A: Button, Card, Badge, Alert, EmptyState, LoadingState, Toggle, Tooltip) | `src/components/ui/*.svelte` (8 files)   | ~250 |
| 2b | Shared primitives (batch B: Modal, Table, Tabs, Select, Input) | `src/components/ui/*.svelte` (5 files)   | ~200 |
| 3  | App shell navbar + theme switcher on Configuration | `src/App.svelte`, `src/components/ConfigurationPage.svelte` (partial) | ~250 |
| 4  | Dashboard polish                            | `src/components/DashboardPage.svelte`     | ~350 |
| 5  | Modal migration batch (move/exit/adjust/archive/resolve + Dashboard inline overlays) | 5 modal files + DashboardPage partial | ~400 — split if exceeded |
| 6  | Forms (Product, Lot, Reports filters, Backup/Restore, CSV import) | 6 files                       | ~400 — split if exceeded |
| 7  | Tables (Lot movements, Reports, CSV preview, Stores, Backup info, Calendar day-detail) | 6 files            | ~350 |
| 8  | Calendar + DatePicker + CategoryPicker polish | 4 files                                   | ~350 |
| 9  | Toast host (deferred)                       | —                       | follow-up |
| 10 | Responsive pass at 1024 / 720 / 480 px     | touch every migrated file | ~250 |
| 11 | Motion inventory wiring (token imports + per-component gating) | every primitive file | ~150 |
| 12 | Final cleanup (delete `src/style.css`, retire one-off CSS, update docs) | sweep                     | ~150 |

The forecast is intentionally conservative; the tasks phase produces
the exact split per Phase.

### 6.1 Responsive strategy

Three breakpoints drive the responsive pass:

- **≥ 1024 px** — desktop layout. Navbar is fully expanded. Tables
  render at default density with sticky headers.
- **720 – 1023 px** — narrow desktop / tablet. Wide tables become
  scrollable (`overflow-x-auto`). Navbar stays expanded; tab buttons
  shrink to `btn-sm`.
- **< 720 px** — small viewport. Navbar collapses the tab buttons
  behind a `dropdown` trigger; modals become `modal-bottom` (a
  DaisyUI variant that pins to the bottom of the viewport); the
  dashboard urgency cards stack vertically.

No horizontal page overflow is introduced; intentional table wrappers
are explicit (`<div class="overflow-x-auto">`) and the verify pass
gates on them.

### 6.2 Per-screen migration order rationale

Phases 3 → 8 migrate the most visible surfaces first (app shell,
dashboard, modals) so that stakeholder review sees progress early.
Forms migrate after modals so that the input primitives are stable
before they touch a wide surface area. Tables migrate after forms so
that `Table.svelte` benefits from the form primitives' feedback
loops. Calendar and the custom widgets migrate last among the page
phases because they carry the most domain logic and benefit from
the stable primitive layer underneath them. Motion (Phase 11) lands
just before cleanup (Phase 12) so the motion tokens are imported
everywhere before the legacy CSS is retired.

## 7. Accessibility strategy

### 7.1 Focus rings

DaisyUI's default outline ring renders on every interactive primitive
in both `caduxo-light` and `dark`. The primitives do not override the
ring; the verify pass visually confirms the ring remains visible
against the active background. `Modal.svelte` retains the ring
inside the dialog so trapped focus is always visible.

### 7.2 Keyboard reachability

Every primitive is reachable by `Tab` in source order. There are no
`tabindex="-1"` overrides; the only `tabindex` adjustments are on
the calendar day cells (managed by `CalendarMonth.svelte`, unchanged
in this change) and on the popovers (which use `tabindex="-1"` on
the popover root but expose their interactive children via `Tab`).

### 7.3 Escape, click-outside, and focus restoration

`Modal.svelte` and the popovers expose:

- `closeOnEscape` (default true for modals, true for popovers) —
  Escape closes the surface.
- `closeOnBackdrop` (default true for confirmation modals, false for
  forms; always true for popovers) — clicking the backdrop closes.
- Focus restoration to the trigger element on close.

Modals that previously cancelled in-progress edits on Escape hook
the dialog's `cancel` event (which fires before `close` and is the
documented cancellation hook for `<dialog>`).

### 7.4 Contrast

The `caduxo-light` and `dark` theme blocks are tuned for WCAG-AA
contrast on body text, headings, inputs, badges, alerts, and primary
button labels. The verify report (Section 11) includes a manual
contrast pass on every main surface in both themes. The pass gates
the PR that introduces the theme block.

### 7.5 Status communicated through multiple channels

Urgency badges carry colour + text + (for the expired variant) a
leading status dot. Alerts carry colour + icon + text. Loading /
loaded / empty / error states are distinguished by the surface type
(spinner vs skeleton vs empty-state component), not by colour alone.
The verify pass confirms the surfaces remain distinguishable when
desaturated (e.g. browser high-contrast).

### 7.6 DatePicker and CategoryPicker keyboard contracts

`DatePicker.svelte` and `inputs/CategoryPicker.svelte` keep their
existing keyboard contracts verbatim (per the spec's
"DatePicker and CategoryPicker keyboard contracts are preserved"
requirement). The visual migration does not modify any `<input>`,
`<button>`, `<datalist>`, or ARIA attribute on these surfaces.

## 8. i18n strategy for new visual strings

### 8.1 Catalog additions

The `src/i18n/en/index.ts` and `src/i18n/es/index.ts` files gain a
new top-level `theme` namespace and entries under the existing
namespaces for new copy:

- `theme.caduxoLight` — display name for `caduxo-light`.
- `theme.dark` — display name for `dark`.
- `theme.source.manual`, `theme.source.persisted`, `theme.source.os`,
  `theme.source.fallback` — labels for the active-source indicator
  in the switcher.
- `settings.theme.title`, `settings.theme.description` — section
  copy on the Configuration page.
- New empty / loading / error copy under existing namespaces
  (`common.loadingLots`, `dashboard.emptyLots`, etc.) where the
  migrated empty / loading surface introduces copy that the legacy
  surface did not have.

Every new key lands in both `en` and `es` before the PR that uses
it opens for review. `npm run i18n:generate` (run by `prebuild` /
`predev`) regenerates the type catalogue and is part of the CI
gate. The PR template for visual phases requires the author to
list the new keys in the description.

### 8.2 Localised theme labels

The Configuration page theme switcher reads
`$LL.theme.caduxoLight()` / `$LL.theme.dark()` so labels follow the
active locale. This requires no new logic — the primitive simply
expects the consumer to bind `label` to the i18n store.

### 8.3 Spanish copy layout pass

The verify report for the first form-and-table batch (Phases 6/7)
includes a manual layout pass on the active Spanish locale at
1280 px viewport: every new empty / loading / error / theme label
renders without mid-word wrapping or overflow. The pass gates the
PR; any wrapping fix is part of the same PR (no follow-up).

## 9. Verification gates per phase

Every chained PR runs:

```bash
npm run i18n:generate   # if i18n catalogue changes
npm run check           # svelte-check + tsc
npm run build           # vite build
```

Plus, for visual phases:

- A manual Tauri / dev-browser launch in both `caduxo-light` and
  `dark` themes.
- A keyboard pass on every migrated primitive (Tab order, Escape,
  Enter / Space activation).
- A screenshot pass on every main surface, recorded in the verify
  report and compared against the Phase 0 baseline.
- A reduced-motion toggle pass (`prefers-reduced-motion: reduce`
  via browser devtools or OS settings) confirming every animated
  surface reaches the same end state.

## 10. Chained PR structure

The implementation branch is `feat/daisyui-redesign`. Each chained
PR follows the same template:

```
feat(ui): <phase title>

Phase: <N> of docs/daisyui-redesign-plan.md
Plan: docs/daisyui-redesign-plan.md#phase-n
Spec: openspec/changes/caduxo-daisyui-redesign/specs/caduxo-expiry-tracker/spec.md

<one-paragraph summary>
<list of files touched>
<verification commands run + result>
<manual smoke notes>
```

### 10.1 Work unit boundaries

- **No business-logic changes.** Visual phases never touch a Tauri
  command, a store function, or a domain helper. The theme
  persistence change (Phase 1 + Phase 3) is the only backend-
  adjacent work; it lands as its own PR (Phase 1 wire-up of the
  IPC contract, Phase 3 wire-up of the switcher) and does not
  ship in the same PR as any visual surface change.
- **No mixed surfaces.** A PR that touches two unrelated surfaces
  (e.g. the dashboard and the calendar) is split. The split rule is
  page-level, not feature-level — a single page's migration stays
  together even if it touches multiple primitives.
- **No half-migrated state.** Every PR that opens a surface lands
  the full migration of that surface (all primitives, all local CSS
  retired). A PR that leaves a surface in a visibly mixed state is
  rejected at review.
- **i18n strings land with the surface.** A new visible string
  enters the i18n catalogue in the same PR that introduces the
  surface, not in a follow-up.

### 10.2 Risk mitigations

| Risk | Mitigation in the delivery structure |
|------|----------------------------------------|
| Review budget overflow | Forecast per phase (Section 6); split by page if the forecast exceeds 400 lines; the tasks phase enforces the split before the PR opens. |
| Half-migrated UI | Phase 2 lands all primitives before Phase 3; subsequent phases migrate by full page; legacy CSS is retired only in Phase 12 after the verify pass. |
| Tailwind preflight regression | Phase 1 foundation gate verifies a Tauri/dev-browser launch with no widespread breakage before Phase 2 begins; the gate is explicit in the verify report. |
| WebKitGTK quirk | Phase 1 verifies `npm run dev` and (when Tauri is available) `npm run tauri dev` both launch cleanly. |
| Native dialog behaviour divergence | Phase 5 modal migration manually verifies Escape, focus trap, and focus restoration for every migrated modal in the verify report. |
| Calendar / DatePicker regression | Phase 8 verify pass includes a manual keyboard pass on every canonical DatePicker and CategoryPicker scenario, recorded against the spec. |
| i18n drift | Every PR that introduces a visible string includes the en + es key additions; `npm run i18n:generate && npm run check` is a CI gate. |
| Theme persistence IPC failure | Phase 3 verify pass exercises the "switcher rolls back on IPC failure" scenario; the optimistic-update path is unit-style verified by the manual smoke. |
| CSS bundle growth | Phase 1 verify pass records the `npm run build` CSS size; Phase 12 re-measures and the verify report diffs them; a >20% regression blocks the merge. |
| Plugin ordering | Phase 1 foundation gate runs both `npm run dev` and `npm run build`; HMR is verified during the manual launch. |
| Font inconsistency | Phase 1 locks the canonical font (`Inter` is retained, applied via the DaisyUI theme `font-sans` token). |
| Urgency pulse sensitivity | Phase 4 (urgency badges) + Phase 5 (motion wiring) verify pass toggles reduced-motion on/off and confirms the pulse is opacity-only, gated, and absent on hover. |
| Native `<dialog>` focus restoration edge cases (trigger no longer in DOM) | `Modal.svelte` no-ops the restoration when `returnFocusTo` is not in the DOM (checks `document.body.contains` before restoring); verify pass exercises the trigger-removed case. |
| Theme switcher source label confusion | Configuration page renders the active source label next to the active theme; verify pass exercises every source value (`manual`, `persisted`, `os`, `fallback`). |

## 11. Open design notes (resolved or pending)

The following design points surfaced during this phase. Each is
closed here so the tasks phase can proceed without re-litigation.

1. **Resolves the `Popover` extraction question** — deferred to a
   follow-up per the proposal assumption. v1 restyles DatePicker and
   CategoryPicker in place; the pickers remain candidates for a
   shared `Popover.svelte` later.
2. **Resolves the toast host question** — deferred to a follow-up
   per the proposal assumption. v1 keeps inline success / error
   patterns; `ToastHost.svelte` is post-v1.
3. **Resolves the dark-mode polish question** — v1 ships a tuned
   `caduxo-light` plus a hand-checked `dark` that meets AA contrast
   on the main surfaces; per-screen dark polish beyond DaisyUI's
   defaults is post-v1.
4. **Resolves the font question** — Inter (currently in
   `src/style.css`) is retained as the canonical font, applied via
   the DaisyUI theme `font-sans` token so DaisyUI's defaults do not
   override it.
5. **Resolves the calendar grid `td` alignment question** — numeric
   columns use the `num` utility (Section 4.4); calendar day cells
   remain center-aligned for the date number and end-aligned for
   the right-edge badge dots (which is the current behaviour
   `CalendarMonth.svelte` already implements — no change needed).
6. **Resolves the App.svelte `activeTab` migration question** — the
   tab-based shell stays; the `activeTab` rune moves into the
   `navbar-center` slot and the tab buttons become DaisyUI-themed
   buttons. The router-based refactor remains out of scope.
7. **Resolves the `app_settings.theme` IPC contract** — the design
   adds `theme` and `theme_configured` to `SettingsResponse` /
   `SettingsUpdate` (Section 3.4). The Rust backend change is
   implied; the design does not own the Rust code but pins the
   frontend contract.

## 12. Implementation readiness checklist (for the tasks phase)

The tasks phase may begin when the following are true:

- [ ] Foundation phase (Phase 1) is forecast at ≤ 400 lines and
      scoped to deps + Vite + `app.css` + theme block + global
      reduced-motion reset + `themeStore.svelte.ts` skeleton.
- [ ] Each subsequent phase has a forecast (file count × approx.
      diff size) that fits within the 400-line budget or is split
      into per-page sub-phases.
- [ ] The chain sequence names the PR that introduces the theme
      persistence IPC contract (Phase 1 backend contract) and the
      PR that wires the switcher (Phase 3).
- [ ] The verify report template lists `npm run i18n:generate`,
      `npm run check`, `npm run build`, manual smoke, manual
      keyboard pass, manual reduced-motion pass, manual screenshot
      pass, and (for theme phases) the contrast pass.

When all four boxes are checked, the tasks phase produces
`openspec/changes/caduxo-daisyui-redesign/tasks.md` with the work
unit breakdown.