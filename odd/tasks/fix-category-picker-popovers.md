# Fix category picker and themed popover placement

## Goal
Repair real-world form dropdown issues reported on `main`: legacy/themed combobox popovers can open upward over previous fields, the special category picker does not reliably respond to mouse selection, and the inline "Create" row in the CategoryPicker does not stay anchored when the popover content changes after typing.

A follow-up cleanup migrates the remaining user-visible native `<select>`-based pickers to the themed `Listbox` primitive (the OS black dropdown leaked through DaisyUI on Chromium / WebKit) and removes the dead `<datalist>` branch from `Input.svelte`.

## Evidence
- User screenshot: `/tmp/pi-clipboard-66a6697d-76b1-4579-9215-e6db76d672a4.png` — category picker popover in Reports filters opens upward over the preceding fields; user described it as "detached/moved instead of refreshed/anchored naturally" (root cause: the popover was positioned once on open and never re-positioned when the user typed, so the create row appearing/disappearing and the result list shrinking/growing left the popover anchored to a stale height).
- User screenshot: `/tmp/pi-clipboard-3e925709-08f1-403f-8c47-1943ad9d7925.png` — product edit CategoryPicker with existing chips and empty query; user reported the "create category" option does not appear. By design, an empty query shows no create row (it would not make sense to offer "Create ''"); the create row appears when the user types a name with no exact match in the result list. This was confirmed working once the popover re-positions as the user types.
- User report (cleanup trigger): the Configuration page language selector still rendered the OS-native black dropdown on Chromium. Read-only audit confirmed the root cause was `src/components/ui/Select.svelte` wrapping a real `<select>` element. User explicitly requested a sweep of every consumer ("haz que los haga cambie todos") and removal of dead code in `Input.svelte` ("que el codigo muerto lo elimine").

## Affected controls and migration status

The two real-world bugs (popover opening upward over a preceding field; click selection not registering; popover not refreshing after content changes) all live in the shared themed-popover primitives. The `Select.svelte` primitive and `Input.svelte` datalist snippet are out of scope because they wrap a native `<select>` / `<datalist>` on purpose (form-semantics, mobile OS sheet, screen-reader keystroke path).

| Control | File | Type | In-scope? | Fix this PR |
| --- | --- | --- | --- | --- |
| CategoryPicker (multi-select categories) | `src/components/inputs/CategoryPicker.svelte` | Themed Svelte popover (special: chips, async search, inline create) | ✅ | Uses the shared `placePopover()` helper; reactive reposition on results / create-row / loading changes; close-button click now stops bubbling so it cannot re-open the popover it just closed. |
| Combobox — ProductForm barcode type | `src/components/ui/Combobox.svelte` (consumed by `src/components/ProductForm.svelte`) | Themed Svelte popover (PR 8a replacement for `<datalist>`) | ✅ | Uses the shared `placePopover()` helper; reactive reposition on `value` / `options` change. |
| Combobox — ProductForm product unit | same | same | ✅ | inherits the fix from the primitive |
| Combobox — ProductDetailPage | `src/components/ProductDetailPage.svelte` → `src/components/ui/Combobox.svelte` | same | ✅ | inherits the fix from the primitive |
| Listbox — ReportsPage (store / location / urgency) | `src/components/ui/Listbox.svelte` (consumed by `src/components/ReportsPage.svelte`) | Themed Svelte popover (PR 8a.1 replacement for native `<select>`) | ✅ | Uses the shared `placePopover()` helper; reactive reposition on `value` / `options` change. |
| Listbox — DashboardPage (filters) | `src/components/DashboardPage.svelte` → `src/components/ui/Listbox.svelte` | same | ✅ | inherits the fix from the primitive |
| Listbox — ColumnMapper (8 columns) | `src/components/ColumnMapper.svelte` → `src/components/ui/Listbox.svelte` | same | ✅ | inherits the fix from the primitive |
| Listbox — ResolveQuantityDialog (resolution kind) | `src/components/ResolveQuantityDialog.svelte` → `src/components/ui/Listbox.svelte` | same | ✅ | inherits the fix from the primitive |
| Listbox — ArchiveLotDialog (archive reason) | `src/components/ArchiveLotDialog.svelte` → `src/components/ui/Listbox.svelte` | same | ✅ | inherits the fix from the primitive |
| Listbox — RegisterExitModal (motivo) | `src/components/RegisterExitModal.svelte` → `src/components/ui/Listbox.svelte` | same | ✅ | inherits the fix from the primitive |
| `Select.svelte` consumers (AdjustCount, MoveStock, ConfigurationPage, LotForm) | `src/components/ui/Select.svelte` | Wrapped native `<select>` (initial design choice — form semantics + OS sheet) | ❌ (originally) → ✅ (cleanup PR) | Migrated to the themed `Listbox.svelte` primitive in the fix/category-picker-popovers cleanup commit. The "by design" rationale was wrong in practice: the wrapped `<select>` still leaks OS chrome on Chromium / WebKit (the DaisyUI token set cannot override the open-option popup). Every consumer was migrated; see "Native `<select>` migration" below. |
| `Input.svelte` datalist snippet | `src/components/ui/Input.svelte` (snippet slot + `<datalist>` id binding) | Native `<datalist>` autocomplete (intended fallback for consumers that wanted OS-native chrome) | ❌ (originally) → ✅ (cleanup PR) | Dead code — no consumer passes `list=` or fills the `datalist` snippet (every autocomplete-style consumer migrated to the themed `Combobox.svelte` primitive). Removed the `list` prop, the `datalist` snippet slot, the `<datalist>` render block, and the matching header references. See "Dead-code cleanup" below. |
| `DatePicker.svelte` | `src/components/DatePicker.svelte` | Themed calendar popover with the *same* placement bug (`placeBelow = spaceBelow >= popHeight + 8 \|\| spaceBelow >= spaceAbove`) | ❌ (not in allowed edit surfaces) | Not edited in this branch — flagged as follow-up. The shared `placePopover()` helper now exists; a future branch should port DatePicker to it and drop the duplicate `positionPopover()` block. |

## Shared popover placement helper

A single helper now owns the placement math:

`src/lib/popoverPlacement.ts` exports `placePopover(options)` (pure function, no internal state, no listeners) plus `PlacementOptions` / `PlacementResult` / `PopoverSide` types. The helper:

1. Measures the popover's actual rendered height (`Math.max(offsetHeight, scrollHeight)`) and caps it at the supplied `maxHeight`. The fallback to `maxHeight` covers the very first paint where both measurements can read 0 (right after `isOpen` flips).
2. Prefers opening **below** the trigger when `spaceBelow >= measuredHeight + gap`. The previous short-circuit `|| spaceBelow >= spaceAbove` was removed — that rule forced below placement even when most of the popover would overflow the viewport.
3. Opens **above** otherwise and clamps the top to `margin` pixels from the viewport edge so the popover header never disappears under the viewport edge.
4. Clamps the left coordinate to `[margin, viewportWidth - popoverWidth - margin]` so wide popovers do not spill past the right edge.
5. Width resolution: explicit `width` > `matchTriggerWidth` (uses trigger rect width, floored by `minWidth`) > natural rendered width.
6. Mutates `popover.style.{position,top,left,width}` and returns `{ side, top, left, width, height }` so callers can react.

The helper is re-runnable: each invocation re-measures, so a call after the user types or after the inline create row appears picks up the live rendered size. Multiple calls in a single frame coalesce naturally (the browser paints once).

All three themed primitives — `CategoryPicker`, `Combobox`, `Listbox` — now delegate to the helper with a thin `positionPopover()` wrapper that supplies the CSS `max-height` and width policy:

- `CategoryPicker`: `maxHeight: 320, width: 300` (themed fixed width)
- `Combobox`: `maxHeight: 260, minWidth: 200, matchTriggerWidth: true`
- `Listbox`: `maxHeight: 260, minWidth: 200, matchTriggerWidth: true`

No more hand-rolled `getBoundingClientRect` / `offsetHeight` / `scrollHeight` math in the three primitives. `grep getBoundingClientRect src/components/{inputs/CategoryPicker,ui/Combobox,ui/Listbox}.svelte` returns no matches.

## Reactive reposition on content changes

Each primitive now re-positions the popover whenever its rendered content changes — not only on open and scroll:

- `CategoryPicker` (Svelte 4 reactive block `$: if (isOpen) { ... }`): touches `results`, `searching`, `creating`, `createError`, `query` so the block re-runs when the search results update, the loading spinner toggles, the inline create row appears or disappears, the create error appears, or the user types. `tick().then(rAF(positionPopover))` ensures the DOM has been updated and the next paint has occurred before measuring.
- `Combobox` (Svelte 5 `$effect`): touches `value` and `options` so the effect re-runs when the user types a new query or the parent re-supplies the option list.
- `Listbox` (Svelte 5 `$effect`): touches `value` and `options` for the same reasons; options are typically static after mount, but the contract matches the siblings.

`resize` listeners were also added (throttled to 80ms) to all three primitives so a window resize refreshes the position. The previous fix only listened to `scroll`.

## Click reliability

- `CategoryPicker`'s option-row handlers keep the split pattern from the previous fix: `on:mousedown={(e) => e.preventDefault()}` (suppresses focus shift only) + `on:click={() => onItemClick(idx)}` (selection runs in the `click` event, *after* `mouseup`, so the popover cannot be removed mid-gesture and leave the chip-remove button behind it to eat the click).
- The close `×` button inside the trigger previously called `onclick={closePopover}` with no `stopPropagation`. The click bubbled to the `.cp-trigger` div's `on:click={togglePopover}`, which saw the freshly-closed popover and re-opened it — clicking `×` had no effect. The handler now calls `e.stopPropagation()` before closing.
- The selection handler `commitActive()` keeps the order `toggle() → closePopover()` so the chip appears before the popover is removed; combined with the `click`-only handler, this means the click event has fully completed before the DOM change.

## Inline create visibility

- The create row appears when `query.trim() && !hasExactMatch`. Empty query → no create row (intentional — there is nothing to create). Exact match → no create row (it would be a duplicate).
- The reactive reposition block in `CategoryPicker` ensures that when the create row appears, the popover re-measures and re-positions so the new row is visible inside the popover rather than clipped past the bottom edge. This was the root cause of "create category option does not appear" once the user's intent was confirmed.

## Tasks
- [x] Map remaining affected picker/select primitives and explain which old controls need migration. (See table above.)
- [x] Extract shared popover placement helper to `src/lib/popoverPlacement.ts`.
- [x] Refactor `CategoryPicker`, `Combobox`, `Listbox` to call `placePopover()`. Remove the three hand-rolled `positionPopover()` bodies.
- [x] Add reactive reposition on content changes (results / options / query / loading / create row).
- [x] Add throttled `resize` listeners to all three primitives.
- [x] Fix `CategoryPicker` close-button click bubbling so the `×` button actually closes the popover.
- [x] Fix `CategoryPicker` mouse selection reliability (already split in the previous quick fix; confirmed correct pattern still in place).
- [x] Run frontend diagnostics: `npm run check`, targeted greps for `<select` / `<datalist` and duplicate placement code.

### Native `<select>` migration (cleanup commit, fix/category-picker-popovers branch)
- [x] Migrate `ConfigurationPage.svelte` language selector → themed `Listbox` (was a `<Select>`).
- [x] Migrate `ConfigurationPage.svelte` theme switcher → themed `Listbox` (was a `<Select>`).
- [x] Strip stale "Select.svelte wraps the native `<select>`" / `Select.svelte` list comments from `ConfigurationPage.svelte`.
- [x] Migrate `LotForm.svelte` store selector → themed `Listbox` (was a `<Select>`). The disabled `value: ""` placeholder is folded into `storeOptions` so the empty-selection visual state stays intact.
- [x] Migrate `LotForm.svelte` location picker → themed `Listbox` (was a `<Select>`). The optional "no location" entry stays as a real (non-disabled) choice at `value: ""` so the submit-time validation contract is preserved.
- [x] Migrate `MoveStockModal.svelte` source location → themed `Listbox` (was a `<Select>`). Empty placeholder folded into `sourceLocationOptions` as `disabled: true`.
- [x] Migrate `MoveStockModal.svelte` destination location → themed `Listbox` (was a `<Select>`). Empty placeholder folded into `destinationLocationOptions` as `disabled: true`.
- [x] Migrate `RegisterExitModal.svelte` exit-reason selector → themed `Listbox` (was a `<Select>`). Source-location selector was already on `Listbox`; only the motivo was on `<Select>`. Empty placeholder folded into `exitReasonOptions` as `disabled: true`.
- [x] Migrate `AdjustCountModal.svelte` location selector → themed `Listbox` (was a `<Select>`). Empty placeholder folded into `locationOptions` as `disabled: true`.
- [x] Decide fate of `src/components/ui/Select.svelte`: zero imports remain after the migration. Left in place (no `rm` in this branch); report the unused state so a future branch can delete it.
- [x] **Post-#16 sync follow-up (this branch):** with `main` now carrying `src/components/ScannerPage.svelte` (`scanner-quick-operations` PR merged as #16) and the two new ConfigurationPage controls (`currentFefoPolicy` / `currentCloseBehavior` from `scanner-quick-operations` PR 3), the previous migration missed them. Re-ran the migration and then deleted the now-truly-orphan `src/components/ui/Select.svelte` (explicit parent task, file in allowed edit surfaces, zero active consumers verified by grep + `npm run check`). Details: "Post-#16 sync follow-up" section below.

### Dead-code cleanup (`Input.svelte`)
- [x] Remove `list?: string` prop from `Input.svelte` (no consumer passes it).
- [x] Remove `datalist?: Snippet` prop + `Snippet` import (no consumer fills it).
- [x] Remove `{#if datalist}{@render datalist()}{/if}` render block.
- [x] Remove `{list}` attribute from the underlying `<input>` element.
- [x] Update the header comment to describe the post-cleanup contract (note: `ProductForm.svelte` has a single historical `<datalist>` reference in its own header comment describing the migration to `Combobox`; no live `<datalist>` consumer exists).

## Diagnostics
- `npm run check` — passes (0 errors, 0 warnings).
- Targeted greps (post-cleanup):
  - `<select\s` in `src/` — only inside `src/components/ui/Select.svelte` itself (the unused primitive). Zero `<select>` usages remain in any active component. All matches in other files are in stale documentation comments (`ReportsPage.svelte:9`, `Listbox.svelte` header explaining the migration, etc.).
  - `<datalist\s` / `datalist=` in `src/` — only in two header comments that describe the historical migration to the themed `Combobox` primitive (`ProductForm.svelte:10`, `Combobox.svelte` header). Zero live `<datalist>` elements, zero `datalist=` snippet fillers, and zero `<Input list=...>` consumers.
  - `import.*ui/Select\.svelte` / `<Select\b` in `src/` — zero (after the cleanup commit). The only `<Select>` mentions left are migration-history comments inside the migrated files.
  - `getBoundingClientRect` / `offsetHeight` / `scrollHeight` in `src/components/` — only in `DatePicker.svelte` (out of scope; follow-up below). The three themed primitives have no measurement code of their own.
- No commit / push performed (per parent scope).

## Follow-ups (out of scope for this branch)
- **DatePicker → shared helper.** `src/components/DatePicker.svelte` still has its own `positionPopover()` block with the old `placeBelow = spaceBelow >= popHeight + 8 || spaceBelow >= spaceAbove` rule and an `offsetHeight`-only measurement. When the surface is reopened, port it to `placePopover({ maxHeight: 320, matchTriggerWidth: true })` and drop the duplicate block. The DatePicker already has `resize` handling and scroll throttling, which the helper-consumers inherit for free once they wire their own listener.
- **Stale `<Select>` comment in `themeStore.svelte.ts`.** Line 50 still says "the display order in the Configuration page `<Select>`" in a JSDoc block. The Configuration page now uses `Listbox`. The comment is outside this branch's allowed edit surfaces; a doc-only follow-up should rename it.
- **Long-term, the `placePopover()` helper could grow into a Svelte action** (`use:popoverPlacement={{ trigger, maxHeight, width }}`) so each primitive just declares `bind:this={popoverEl}` and the action handles open / scroll / resize / content-change reposition. Not done now because the primitives already have bespoke open/close lifecycle that the action would have to coordinate with; the current pure-function helper is the smaller, safer change.

## Post-#16 sync follow-up (this branch)

After PR #16 (`scanner-quick-operations`) merged into `main`, `fix/category-picker-popovers` was synced and the previous cleanup commit missed two surfaces:

1. `src/components/ScannerPage.svelte` is a brand-new file on `main` (it didn't exist when the cleanup commit ran). It imports and uses `Select.svelte` seven times — two in the store-picker surface (`storeSelectId` active-store selector with `disabled: true` placeholder, `activeStoreSelectId` always-visible context), two in the Sale panel (`selectedLotId` lot picker with `disabled: lotPickerDisabled`, `locationId` source-location picker with `disabled: availableBalances.length === 1`), three in the Stock-out panel (`selectedLotId` lot picker, `exitReason` reason picker with `required` + `disabled: true` placeholder, `locationId` source-location picker).
2. `src/components/ConfigurationPage.svelte` had already lost its `Select` import (the language/theme migration stripped it), but the two new `scanner-quick-operations` controls — `currentFefoPolicy` (three-option FEFO policy selector) and `currentCloseBehavior` (two-option close-window behaviour selector) — still rendered through `<Select>`. `npm run check` failed on those two `Cannot find name 'Select'` references.

Re-ran the migration following the same pattern the cleanup commit established in `LotForm`, `MoveStockModal`, `RegisterExitModal`, `AdjustCountModal`, and the existing ConfigurationPage language/theme controls:

- `Select` import swapped for `Listbox` in `ScannerPage.svelte`.
- All seven `<Select ... />` blocks renamed to `<Listbox ... />`. Each block preserved its existing props verbatim: `value`, `options`, `size` (where used), `disabled`, `required` (exit-reason block), `aria-label`, and `onchange`. The `onchange={(v: string) => (varName = v)}` callbacks keep the explicit two-way binding style already in the file rather than switching to `bind:value` — minimal-diff rule.
- Folding the placeholder rows into `options` was already done in the existing code (the store-picker prepends `{ value: "", label: $LL.lotForm.selectStorePlaceholder(), disabled: true }`, and the exit-reason selector prepends `{ value: "", label: $LL.scanner.stockOut.reasonPlaceholder(), disabled: true }`); no additional folding needed.
- The two ConfigurationPage `<Select>` blocks replaced with `<Listbox>`; their preceding HTML comments updated from "the visible surface is the DaisyUI `select select-md` shell" to "rendered through the themed `Listbox` primitive so the open picker inherits the DaisyUI tokens (no OS-native black dropdown on Chromium / WebKit)" so the inline doc matches the language/theme comments that already describe Listbox.

After the migration, `grep -RE "from [\"']\.{1,2}/ui/Select\.svelte[\"']|<Select\b" src/ --include="*.svelte"` returned zero live consumers (only four historical migration comments in already-migrated files plus `themeStore.svelte.ts:50`'s stale doc-comment follow-up). With the condition met, `src/components/ui/Select.svelte` was deleted (`rm src/components/ui/Select.svelte`) per the parent task's explicit instruction.

### Post-sync diagnostics
- `npm run check` — passes (0 errors, 0 warnings).
- `grep -R "from \"./ui/Select.svelte\"\|from '../ui/Select.svelte'\|<Select" -n src/components` — only historical migration comments (4 hits, all in already-migrated files: `AdjustCountModal.svelte`, `MoveStockModal.svelte` ×2, `RegisterExitModal.svelte`; plus the stale JSDoc follow-up in `themeStore.svelte.ts:50`). Zero live `<Select>` usages.
- `grep -R "<select" -n src/components` — only historical migration comments describing the migration away from native `<select>` (ArchiveLotDialog, ColumnMapper, ReportsPage, Listbox header ×4, AdjustCountModal, ConfigurationPage ×2, LotForm ×2, RegisterExitModal). Zero live `<select>` elements.
- `grep -R "datalist\|list=" -n src/components src/lib` — only historical migration comments in `ProductForm.svelte`, `Combobox.svelte` header, and `Input.svelte` header explaining the removal of the `<datalist>` snippet slot. Zero live `<datalist>` elements or `list=` attributes.
- `src/components/ui/Select.svelte` no longer exists; `ls src/components/ui/ | grep -i select` returns empty.

No commit / push performed (per parent scope).

## App window minimum + zoom guard (this branch)

A real-world test session on `fix/category-picker-popovers` reported two desktop-window issues that this branch now mitigates:

1. The native Tauri window had no `minWidth` / `minHeight`, so the OS allowed the window to be resized below the app's responsive floor (the navbar tabs would clip, the dashboard urgent-card grid would squeeze, and the reports filter row would wrap into unusable layouts).
2. The user could "hacer como un zoom en la interfaz" — accidental Ctrl/Cmd + scroll-wheel or Ctrl/Cmd + `+` / `-` / `=` / `0` shortcuts inside the webview zoomed the document and deformed the layout. On WebKitGTK (Tauri's Linux backend) the Ctrl + wheel gesture is enabled by default; on Chromium / WebKit / WebView2 the keyboard shortcuts are the standard browser zoom bindings.

The mitigation is layered: a native window minimum, a responsive navbar that switches to an overflow dropdown whenever the viewport cannot accommodate every Spanish tab label, an explicit OS-level disable of Tauri’s zoom hotkeys, and a frontend zoom guard. None of this is an accessibility policy change — Caduxo is a fixed-layout desktop application, not a responsive website, so users have no reason to zoom the webview and the resulting deformation is a real bug.

### Revision (corrected after the first attempt)

The first attempt on this branch added `.app-shell { min-width: 960px }` plus `:global(html), :global(body) { min-width: 960px }` as a CSS guard against runtimes that ignore the native Tauri `minWidth`. That turned out to be the bug the user reported next:

- The native Tauri `minWidth: 960` did not take effect at runtime in the user’s environment, so the window reported ≈ 873 px wide.
- Because the document/shell were pinned to 960 px, the body became wider than the viewport and the page entered body-level horizontal scroll.
- The desktop navbar row had its `display: none` switch at `@media (max-width: 720px)` (below both 873 px and the native 960 px floor), so the desktop row was still rendered.
- Net effect: nine Spanish tab buttons in a row wider than the viewport, with the right-side entries clipped off-screen and unreachable — and no horizontal scrollbar obvious inside the navbar itself because the overflow belonged to the body, not the nav.

The corrected approach drops the body/shell `min-width` pinning entirely and raises the navbar overflow breakpoint so the overflow dropdown becomes the constrained-width UX. The native Tauri window still clamps the OS-resize range (`minWidth: 960`); individual page components that need a minimum content width (DashboardPage’s urgent-card grid, ConfigurationPage’s two-column row, etc.) keep their own internal `min-width` rules and scroll inside their containers as needed. No global document pin.

### Window minimum

`src-tauri/tauri.conf.json` `app.windows[0]` now declares:

```json
"minWidth": 960,
"minHeight": 640,
"zoomHotkeysEnabled": false
```

- `minWidth: 960` / `minHeight: 640` is the conservative desktop minimum, aligned with the 1100 × 720 default. It is the OS-level guard against resizing the window below the app floor.
- `zoomHotkeysEnabled: false` is set explicitly (the schema default is already `false`, but the explicit value documents intent): on Windows it maps directly to WebView2’s `IsZoomControlEnabled`; on macOS / Linux it disables the Tauri-injected zoom-hotkey polyfill (which would otherwise intercept Ctrl/Cmd + `-/=` at scale 1.0–5.0). The JS-side guard below is still required because the OS itself may forward Ctrl + wheel to the WebView (notably on WebKitGTK) outside of Tauri’s polyfill.

### CSS guard

**`src/App.svelte` no longer pins `.app-shell` / `html` / `body` to a minimum width.** The previous `.app-shell { min-width: 960px }` and `:global(html), :global(body) { min-width: 960px }` rules were removed because they caused body-level horizontal scrolling at constrained widths and hid the nav buttons. The responsive overflow dropdown (next section) now owns the constrained-width UX, while the native Tauri window’s `minWidth: 960` keeps the OS from resizing the window below the app floor in the first place.

If the runtime ever again ignores the native `minWidth` (an embedded preview, a future debug surface, a non-standard backend), the navbar will simply switch to the overflow dropdown at the new 1024 px breakpoint — every destination stays reachable.

### Responsive overflow

`src/App.svelte` raises the navbar `display: none` / `display: flex` switch from `@media (max-width: 720px)` to `@media (max-width: 1024px)`:

- **Above 1024 px**: the desktop row `.app-tabs` is rendered with all nine Spanish tab buttons (Panel, Tiendas, Productos, Calendario, Reportes, Escáner, Importar, Respaldo, Configuración) plus the brand area in `.navbar-start`. The row comfortably fits because the brand area + nine `btn btn-sm` ghost buttons + gaps add up to roughly 850–1000 px depending on font and padding.
- **At and below 1024 px**: the desktop row hides and the DaisyUI `navbar-end` dropdown (`.app-overflow`) shows. The dropdown renders the same tab list inside a `menu menu-sm dropdown-content` so every destination stays one menu click away.

This covers the user-reported ≈ 873 px case (overflow dropdown shows) and matches common "small desktop" breakpoints. The breakpoint is intentionally higher than the native 960 px floor so the overflow menu takes over whenever the viewport is too narrow to fit the desktop tabs cleanly — the user sees the overflow menu rather than a clipped row.

### Zoom guard

`src/App.svelte` registers the guard via `onMount` (returned cleanup removes every listener on component destroy) so registration is bound to the component lifecycle and does not depend on Svelte’s reactive-tracking path:

- **One-shot reset on mount**: `document.documentElement.style.zoom = "1"` is assigned once. `style.zoom` is non-standard (CSS Zoom draft) but supported on Chromium / WebKit / WebView2 / WebKitGTK — the four backends Tauri ships — and is typed as `string` on `CSSStyleDeclaration`, so the assignment type-checks. The Tauri-side `setZoom()` webview API was deliberately not used because it would require adding `webview:allow-set-webview-zoom` to `src-tauri/capabilities/default.json` (out of this branch’s allowed edit surfaces). The CSS-reset approach covers every runtime we currently ship without a permission change.
- **`wheel` listeners on both `window` and `document`, capture phase, `{ passive: false, capture: true }`** — call `event.preventDefault()` whenever `event.ctrlKey || event.metaKey`. Capture phase fires before any inner bubbling handler can call `stopPropagation`, registering on both targets defends against any future code path that attaches a `wheel` handler to one but not the other. `passive: false` is required so `preventDefault` actually cancels the browser zoom (WebKitGTK / Chromium treat Ctrl + wheel as zoom-by-default; passive listeners cannot suppress it).
- **`keydown` listeners on both `window` and `document`, capture phase, `{ capture: true }`** — block any keydown where `event.ctrlKey || event.metaKey` AND `event.code` is one of `Equal`, `Minus`, `Digit0`, `NumpadAdd`, `NumpadSubtract`, `Numpad0`. Using `event.code` (the physical-key identifier) instead of `event.key` (the produced character) means AZERTY, Dvorak, and numpad layouts all map to the same blocked keys — the previous `event.key === "="` / `"+"` / `"-"` / `"0"` check would have missed `NumpadAdd` and `NumpadSubtract` on a US-international layout and broken on a German layout where Shift+0 produces “)”.
- The modifier check is `ctrlKey || metaKey`, so every other Ctrl/Cmd shortcut (`Ctrl+S`, `Ctrl+R`, `Ctrl+P`, `Cmd+Q`, `Cmd+W`, …) and every plain keypress (normal typing, scanner Enter, form shortcuts) passes through untouched.

The `onMount`-returned cleanup removes every listener on component destroy, so the guard never leaks into HMR or future routing.

### Tasks
- [x] Add `minWidth: 960` / `minHeight: 640` to `src-tauri/tauri.conf.json` `app.windows[0]`.
- [x] Add `zoomHotkeysEnabled: false` to `src-tauri/tauri.conf.json` `app.windows[0]` (explicit default; disables WebView2 zoom hotkeys on Windows and the Tauri-injected polyfill on macOS/Linux).
- [x] **Revision:** remove the `.app-shell { min-width: 960px }` rule from `src/App.svelte` (it caused body-level horizontal scrolling at constrained widths and clipped the desktop nav row).
- [x] **Revision:** remove the `:global(html), :global(body) { min-width: 960px }` rule from `src/App.svelte` (same reason; the responsive overflow menu now owns the constrained-width UX).
- [x] **Revision:** raise the navbar overflow breakpoint from `@media (max-width: 720px)` to `@media (max-width: 1024px)` so the overflow dropdown shows whenever the viewport is too narrow for the nine Spanish tab labels + brand area, including the user-reported ≈ 873 px case.
- [x] **Revision:** replace the `$effect`-mounted zoom guard with an `onMount`-installed one so listener setup is bound to the component lifecycle rather than the reactive-tracking path.
- [x] **Revision:** register zoom-blocking `wheel` and `keydown` listeners in the capture phase (`{ passive: false, capture: true }` for `wheel`, `{ capture: true }` for `keydown`) on BOTH `window` and `document`, so the guard fires before any inner bubbling handler can call `stopPropagation` and is robust to which target future code paths attach to.
- [x] **Revision:** extend the blocked-key set with the numpad variants (`NumpadAdd`, `NumpadSubtract`, `Numpad0`) and switch from `event.key` to `event.code` so locale-dependent layouts (AZERTY, Dvorak, numpad) all map to the same blocked keys.
- [x] **Revision:** force `document.documentElement.style.zoom = "1"` once on mount as a typed, no-permission-needed reset for any inherited OS-level zoom level.
- [x] **Linux/KDE pinch revision:** add capture-phase WebKit `gesturestart` / `gesturechange` / `gestureend` listeners on both `window` and `document`, with non-passive `preventDefault()` and zoom reset, because KDE/WebKitGTK touchpad pinch can bypass the Ctrl+wheel path.
- [x] **Linux/KDE pinch revision:** add a global `html, body { touch-action: pan-x pan-y; }` CSS input policy in `src/app.css` so normal panning remains available while browser pinch/scale gestures are excluded.
- [x] Update the file-header comment in `src/App.svelte` to describe the corrected approach (overflow breakpoint, dropped CSS pinning, onMount zoom guard, capture-phase + numpad variants, and WebKitGTK pinch gesture handling) so future maintainers see the rationale.
- [x] Run `npm run check` — passes (0 errors, 0 warnings).

### Diagnostics
- `npm run check` — passes (0 errors, 0 warnings).
- `grep -n 'minWidth\|minHeight\|zoomHotkeysEnabled' src-tauri/tauri.conf.json` — all three keys present on `app.windows[0]`.
- `grep -n 'min-width' src/App.svelte` — zero matches in the `<style>` block (the global `.app-shell` and `:global(html), :global(body)` rules were removed). The only `min-width` mentions left are in the header comment and the inline comment explaining why the rule was removed.
- `grep -n '@media (max-width' src/App.svelte` — one active `@media` rule: `@media (max-width: 1024px)` (the raised breakpoint). The other matches are references in the file-header comment and inline comments explaining the change.
- `grep -nE 'addEventListener\("wheel"|addEventListener\("keydown"|capture:\s*true|passive:\s*false' src/App.svelte` — every listener is registered with `passive: false, capture: true` (wheel) or `capture: true` (keydown) and every `removeEventListener` carries the matching `capture: true`.
- `grep -n 'NumpadAdd\|NumpadSubtract\|Numpad0\|Equal\|Minus\|Digit0' src/App.svelte` — all six blocked key codes present in the `ZOOM_KEY_CODES` set.
- `grep -n 'onMount' src/App.svelte` — the zoom guard is now installed inside `onMount(...)` with a returned cleanup; no `$effect` rune remains in the file.
- `grep -n '<select\b\|<datalist\b\|<Select\b' src/components src/App.svelte` — unchanged from the Post-#16 sync diagnostics (zero live native elements; only historical migration comments). The window/zoom guard and the navbar-overflow revision did not touch any picker primitive.
- Pre-existing working-tree changes (the eleven `src/components/**` modifications, the deleted `src/components/ui/Select.svelte`, the new `src/lib/popoverPlacement.ts`) are preserved untouched.

### Side effect of dropping the `$effect` block

Replacing the `$effect(...)-mounted zoom guard with `onMount(...)-installed listeners had one incidental benefit: the file is no longer in Svelte 5 “rune mode”. Pre-fix, the `$effect` rune put the file in rune mode, which made the bare `let activeTab: Tab = "stores"` declaration non-reactive (`svelte-check` flagged it with the `non_reactive_update` warning), so tab clicks mutated the variable but did not trigger a re-render. With no runes in the file, the `let` declaration follows Svelte 4-style compile-time reactive bindings, `activeTab = next` re-renders the navbar, and `npm run check` reports zero warnings. This was not the focus of the revision but it removes a real user-visible bug where clicking a tab button did not visually switch the active tab.

No commit / push performed (per parent scope).