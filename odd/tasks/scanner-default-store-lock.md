# Scanner default store lock

## Goal
Keep Scanner store selection user-friendly while making the chosen store the default locked store for Scanner-created lots.

## Tasks

- [x] Inspect current Scanner store picker and Registration lot creation flow.
- [x] Keep an explicit, friendly active-store selector in Scanner for changing the Scanner context.
- [x] Ensure Scanner Registration creates new lots in the selected active store by default.
- [x] Prevent selecting a different store inside the Scanner lot-creation flow; the Scanner-selected store is locked.
- [x] Validate with focused frontend checks and record evidence.

## Approach

- Preserve the prior scanner-store-fix diff (empty/stale picker,
  Stores-page persistence of `last_selected_store_id`, Scanner UX
  fixes, calendar lot detail raw-id fix, i18n updates, Cargo.lock
  sync). Touch only the allowed edit surfaces.
- Scanner active store context: extend the existing
  `selectActiveStore` empty/stale branch with an **always-visible**
  panel that renders inside the interactive Scanner surface (after
  the page header, before the mode tabs). The panel reads
  `settings.last_selected_store_id`, shows the active store name, and
  — when more than one active store exists — exposes a `Select` that
  persists the new id via `updateSettings({ last_selected_store_id })`.
  Switching clears any in-progress scan via the existing
  `resetMutationForm()` helper because the previously resolved value
  would no longer match the new active store's context.
- Scanner → LotForm lock: extend `LotForm.svelte` with two narrow
  optional props — `lockedStoreId: string | null` and
  `lockedStoreName: string` — instead of duplicating the form. When
  `lockedStoreId` is set, `init()` pre-fills `selectedStoreId` from
  it, the template renders a read-only warning-tinted chip in place
  of the store select, and the validation guard
  `if (!selectedStoreId)` still passes. Edit mode ignores
  `lockedStoreId` (the existing lot already owns its store id).
- Product/Lot catalog unchanged: when no `lockedStoreId` is passed,
  `LotForm` keeps the existing single-store hint or multi-store
  select behaviour. Outside the Scanner tab the form is invoked
  without the lock props and behaves exactly as before.
- i18n: add `scanner.activeStoreContext.{title,body,label,
  lockedBadge,changeNotice,errors.failed}` and
  `lotForm.{lockedStoreLabel,lockedStoreHint}` to both locales and
  regenerate `i18n-types.ts` via `npm run i18n:generate`.

## Evidence

### Scanner active-store context (always visible)

- Before: the active store was visible only via the header copy. A
  select appeared only when `last_selected_store_id` was empty or
  stale (the existing `selectActiveStore` block). Once the page was
  interactive (`pageReady === true`), the user had no in-page
  affordance to see or change the active store without leaving the
  Scanner tab.
- After: the interactive Scanner surface now renders an
  `active-store-context` panel as the first child of the
  `pageReady` branch. The panel renders the active store name (from
  the new `activeStore` derived) plus a `locked-badge` chip and the
  `scanner.activeStoreContext.body()` copy. When more than one
  active store exists, a `Select` calls a new
  `changeActiveStore(next)` handler that persists via
  `updateSettings({ last_selected_store_id: next })`, refreshes
  local `settings`, and clears in-progress scan / lot state via the
  existing `resetMutationForm()` helper. With a single active store
  the panel renders a read-only `active-store-name` line.
- Files: `src/components/ScannerPage.svelte`,
  `src/i18n/en/index.ts`, `src/i18n/es/index.ts`,
  `src/i18n/i18n-types.ts` (regenerated).

### Scanner Registration → LotForm store lock

- Before: Scanner Registration opened `LotForm` with `mode="create"`
  and the product context. The form then defaulted
  `selectedStoreId` to the first store returned by `listStores()`,
  which was not necessarily the Scanner-selected store. The store
  select was rendered whenever more than one store existed, allowing
  the user to accidentally register a lot in a different store.
- After: Scanner Registration passes the Scanner-active store id and
  resolved store name through the new `LotForm.lockedStoreId` /
  `lockedStoreName` props. The form pre-fills `selectedStoreId` from
  the lock, replaces the select with a warning-tinted locked hint
  (`scanner.activeStoreContext.lockedBadge` /
  `lotForm.lockedStoreLabel`), and keeps the existing
  validation/submit path so the locked store reaches the
  `createExpiryLot` payload verbatim. Outside the Scanner tab
  (Product Catalog) the form is invoked without the lock props and
  falls back to the previous behaviour.
- Files: `src/components/ScannerPage.svelte`,
  `src/components/LotForm.svelte`,
  `src/i18n/en/index.ts`, `src/i18n/es/index.ts`,
  `src/i18n/i18n-types.ts` (regenerated).

### i18n + type regeneration

- New keys added in `src/i18n/en/index.ts`:
  `scanner.activeStoreContext.title`, `body`, `label`,
  `lockedBadge`, `changeNotice`, `errors.failed`,
  `lotForm.lockedStoreLabel`, `lotForm.lockedStoreHint`.
- Same keys added in `src/i18n/es/index.ts` (Spanish equivalents).
- `npm run i18n:generate` regenerated `src/i18n/i18n-types.ts` to
  include both new key paths under `scanner` and `lotForm`.

## Validation

- `npm run i18n:generate` — regenerated typesafe-i18n types; output
  reports `generated file: './src/i18n/i18n-types.ts'` and
  `all files are up to date`.
- `npm run check` — `svelte-check --tsconfig ./tsconfig.json
  --threshold error` reports `0 errors and 0 warnings`. New
  `lockedStoreId` / `lockedStoreName` props, the `activeStore` /
  `activeStoreSelectId` deriveds, the `changeActiveStore` handler,
  the always-visible active-store context UI, and the new i18n
  keys type-check.
- No backend / Rust files were touched, so no Rust test command is
  required. The Tauri command surface
  (`updateSettings({ last_selected_store_id })`,
  `createExpiryLot({ store_id })`) is unchanged — the lock is a
  pure frontend affordance over the existing IPC.

## Risks / Follow-ups

- The Scanner active-store context inherits the `pickerStoreOptions`
  filter (`s.is_active === true`) from the missing/stale picker.
  Inactive stores are intentionally excluded so the user cannot
  persist an inactive id (which would re-trigger the backend
  `resolve_scanner_code` "No active store" error). A future
  follow-up could surface inactive stores in the picker with a
  separate `(inactive)` suffix if product asks for it.
- Switching the active store clears any in-progress scan via
  `resetMutationForm()`. If a future Scanner sub-flow carries
  valuable state across an active-store switch, that state should
  be re-evaluated against `resetMutationForm()`.
- The Scanner-locked store name falls back to `lockedStoreId` when
  the parent cannot resolve a name (e.g. stale persisted id). The
  fallback is unreachable today because `pageReady` already guards
  on `activeStoreSelected`, but the fallback is kept so a future
  refactor that opens `LotForm` outside the `pageReady` branch does
  not regress silently.
