# Delta for Caduxo Expiry Tracker

## ADDED Requirements

### Capability: Scanner quick operations

#### Requirement: Scanner top-level tab with operation modes

The Caduxo desktop application MUST expose a top-level navigation entry labelled **Scanner** in every locale. The Scanner surface MUST be reachable from the main `App.svelte` navbar with the same `aria-current="page"` affordance used by every other tab and MUST be available on Windows and Linux builds at parity. The Scanner tab MUST render three selectable operation modes:

- `Sale` — decrements active lot stock for a customer sale.
- `Registration` — registers incoming stock for a known product, or starts quick product creation when the scanned code is unknown.
- `Stock-out` — decrements active lot stock for a non-sale reason drawn from the existing v1 reason vocabulary.

The active mode is a single piece of UI state scoped to the Scanner tab and is persisted in component-local storage for the lifetime of the tab; switching modes MUST NOT discard the already-resolved scan result, the chosen lot, or the in-progress confirmation form. The Scanner tab MUST render a single primary input field that accepts handheld-scanner keyboard input (terminated by `Enter`) and MUST debounce repeated scans within `400 ms` of the previous successful resolution so a single physical scan never produces two side effects.

#### Scenario: Scanner tab appears in the navbar between Reports and Import

- GIVEN the user is on any tab other than Scanner
- WHEN the user activates the Scanner tab in the navbar
- THEN the active tab becomes `scanner`
- AND the Scanner tab gains `aria-current="page"`
- AND the Scanner surface renders the three mode tabs and the scanner input

#### Scenario: switching modes preserves the in-progress scan

- GIVEN the user scanned a valid product barcode in `Sale` mode
- WHEN the user switches to `Registration` mode without confirming
- THEN the resolved product, available lots, and the currently selected lot remain available
- AND switching modes MUST NOT issue a fresh lookup for the previously scanned value

#### Scenario: rapid double-scan is debounced

- GIVEN the user scans a valid value at `t=0`
- WHEN the scanner input fires the same value at `t=200ms`
- THEN the second scan is ignored
- AND only one downstream side effect is produced
- AND the input field is cleared exactly once

#### Scenario: Scanner tab is unavailable until first store exists

- GIVEN the database has no active store
- WHEN the user activates the Scanner tab
- THEN the tab renders a message in the active locale telling the user to create the first store first
- AND the scanner input is disabled
- AND no lookup command is issued

#### Requirement: Scanner input matching priority

The Scanner tab MUST resolve every scan against the catalog in a strict three-step priority order:

1. **Lot code** — exact match against `expiry_lots.batch_code` (active lots only, scoped to the active store).
2. **Product barcode** — exact match against `product_barcodes.barcode` (any active product).
3. **Product SKU** — exact match against `products.sku` (active products only).

A new IPC command `resolve_scanner_code` MUST be introduced and MUST be called by the Scanner tab on every committed scan. The command returns a discriminated result so the Scanner tab can dispatch by mode without re-querying:

- `LotMatch { lot, product }` — the lot code resolved directly to one active lot. `product` is the lot's parent product.
- `ProductMatch { product, lots }` — the value resolved to a product (barcode or SKU); `lots` lists the active lots for that product, ordered according to the active FEFO policy (see `FEFO lot-selection policy`).
- `Unknown { scanned_value }` — neither step matched; the Scanner tab dispatches by mode (see `Sale mode flow`, `Registration mode`, and `Stock-out mode`).

The existing `find_product_by_scan` command is left untouched because the dashboard scan/search surface remains a product-only path; the new Scanner tab MUST NOT call `find_product_by_scan` and MUST NOT reuse its discriminator.

#### Scenario: lot-code match wins over product matches

- GIVEN active lot L has `batch_code = "SUP-2026-XYZ"` for product P
- AND product P also has a barcode `7501234567890`
- WHEN the Scanner input resolves `SUP-2026-XYZ`
- THEN `resolve_scanner_code` returns `LotMatch { lot: L, product: P }`
- AND the product barcode is NOT consulted because step 1 succeeded

#### Scenario: product barcode match when lot code is unknown

- GIVEN no active lot has `batch_code = "7501234567890"`
- AND product P has barcode `7501234567890`
- WHEN the Scanner input resolves `7501234567890`
- THEN `resolve_scanner_code` returns `ProductMatch { product: P, lots: [...] }`
- AND the SKU lookup is NOT consulted because step 2 succeeded

#### Scenario: product SKU match when barcode is unknown

- GIVEN no lot or barcode matches `ABC-001`
- AND product P has `sku = "ABC-001"`
- WHEN the Scanner input resolves `ABC-001`
- THEN `resolve_scanner_code` returns `ProductMatch { product: P, lots: [...] }`

#### Scenario: lot-code match is scoped to the active store

- GIVEN the active store is `S1`
- AND lot L1 with `batch_code = "X"` belongs to store `S2`
- AND lot L2 with `batch_code = "X"` belongs to store `S1`
- WHEN the Scanner input resolves `X`
- THEN `resolve_scanner_code` returns `LotMatch { lot: L2, product: ... }`
- AND `L1` is NOT considered

#### Scenario: barcode already attached to an archived product does not match

- GIVEN product P is archived (`is_active = false`)
- AND P has barcode `7501234567890`
- WHEN the Scanner input resolves `7501234567890`
- THEN `resolve_scanner_code` does NOT return `ProductMatch` referencing P
- AND the SKU lookup is tried next
- AND if the SKU is also unknown, `Unknown` is returned

#### Scenario: unknown scan in any mode preserves the original scanned value

- GIVEN the scanner input resolves to `Unknown`
- WHEN the Scanner tab receives the result
- THEN `scanned_value` is the trimmed value as the user submitted it
- AND no upper-casing, lower-casing, or whitespace mutation has occurred
- AND the original value is available for pre-filling the quick-create form in Registration mode

#### Requirement: FEFO lot-selection policy

The Scanner tab MUST honour a configurable FEFO (First-Expired, First-Out) lot-selection policy that is exposed in the Settings surface and persisted across restarts (see `Settings: FEFO policy persisted`). The policy has exactly three values:

- `suggest_fefo` (default) — when the scanner resolves to a product with multiple active lots, the Scanner tab pre-selects the FEFO lot (earliest non-expired `expiry_date`, ties broken by `created_at` then `id`) but the user MAY override the selection before confirming.
- `require_fefo` — the Scanner tab MUST always pre-select the FEFO lot and MUST disable the lot picker so the user cannot override. The picker is rendered as a read-only chip with a translated notice that the FEFO policy is enforced.
- `manual_lot_choice` — the Scanner tab does NOT pre-select a lot; the user MUST choose one before the confirm button becomes enabled.

The policy applies whenever the resolved result is `ProductMatch`. For `LotMatch` the policy does not apply because the lot is already identified by the scanned code; the FEFO selection MUST NOT override a direct lot-code match.

#### Scenario: default suggest_fefo pre-selects the earliest expiry

- GIVEN the FEFO policy is `suggest_fefo` (the default on fresh installs)
- AND product P has three active lots with `expiry_date` values `2026-09-01`, `2026-08-15`, and `2027-01-10`
- WHEN the scanner resolves to `ProductMatch` for P
- THEN the lot with `expiry_date = 2026-08-15` is pre-selected
- AND the lot picker is editable
- AND the user MAY choose a different lot before confirming

#### Scenario: require_fefo disables the lot picker

- GIVEN the FEFO policy is `require_fefo`
- AND product P has three active lots
- WHEN the scanner resolves to `ProductMatch` for P
- THEN the lot with the earliest `expiry_date` is pre-selected
- AND the lot picker is disabled
- AND the Scanner tab renders a translated notice that FEFO is required

#### Scenario: manual_lot_choice requires explicit selection

- GIVEN the FEFO policy is `manual_lot_choice`
- WHEN the scanner resolves to `ProductMatch` for P
- THEN no lot is pre-selected
- AND the confirm button is disabled until the user chooses a lot
- AND the unavailable confirm state is rendered with the same pattern used by other gated forms

#### Scenario: lot-code match bypasses the FEFO policy

- GIVEN the FEFO policy is `manual_lot_choice`
- AND the scanner resolves `SUP-2026-XYZ` to `LotMatch { lot: L }`
- WHEN the Scanner tab receives the result
- THEN L is the selected lot
- AND no lot picker is rendered
- AND the FEFO policy is not consulted for `LotMatch`

#### Requirement: Sale mode flow with explicit confirmation

In `Sale` mode the Scanner tab MUST follow this exact sequence after a successful resolution:

1. Default the quantity to `1`. The quantity input MUST be editable and MUST reject values `<= 0` and values greater than the selected lot's per-location balance (or total quantity when the lot has a single source location). For integer-unit products the input MUST render with `step="1"` and `min="1"` and MUST reject fractional values.
2. When the resolution is `ProductMatch` and the product has more than one active lot, apply the FEFO policy (see `FEFO lot-selection policy`) to determine the pre-selected lot.
3. When the resolution is `LotMatch` the lot is already chosen.
4. Render a `Confirm` button that is disabled until both the lot and the quantity are valid.
5. On `Confirm`, the Scanner tab MUST call `create_lot_movement` with `kind = "exit:sale"`, the selected lot id, the lot's current source location (when the lot has a single balance; otherwise the user MUST choose a location), and the chosen quantity. The confirm button MUST be re-disabled until the IPC call returns.
6. After a successful IPC call the Scanner tab MUST clear the input, MUST reset the quantity to `1`, and MUST render a translated success notice that includes the SKU of the product and the sold quantity. Stock MUST NOT be mutated before the user activates `Confirm`.

The Scanner tab MUST NOT render, request, or persist any billing, payment, invoice, or accounting field in v1 (per the project non-goals in `openspec/config.yaml`).

#### Scenario: Sale confirms before stock is decremented

- GIVEN the scanner resolved to `LotMatch { lot: L }` with `quantity = 5`
- AND the quantity input is left at `1`
- WHEN the user activates `Confirm`
- THEN the Scanner tab calls `create_lot_movement` with `kind = "exit:sale"`, `quantity = 1`
- AND the lot's `quantity` becomes `4` only after the IPC call returns success
- AND no movement row is written if the IPC call fails

#### Scenario: Sale confirm is disabled until quantity and lot are valid

- GIVEN the scanner resolved to `ProductMatch` with multiple active lots
- AND the FEFO policy is `manual_lot_choice`
- WHEN the Scanner tab renders the form
- THEN the `Confirm` button is disabled
- AND the button becomes enabled exactly when the user chooses a lot and enters a positive quantity that does not exceed the lot's balance

#### Scenario: Sale quantity cannot exceed the lot balance

- GIVEN lot L has `quantity = 3`
- WHEN the user types `5` into the quantity input
- THEN the form rejects the value with an inline error in the active locale
- AND the `Confirm` button is disabled
- AND no IPC call is issued

#### Scenario: Sale success resets the form for the next scan

- GIVEN the user confirmed a sale of `1` unit of product `ABC-001`
- WHEN the IPC call returns success
- THEN the scanner input is cleared
- AND the quantity resets to `1`
- AND a translated success notice reads, in the active locale, something equivalent to `Sold 1 × ABC-001`
- AND the next scan resumes the empty-form state

#### Requirement: Registration mode for existing and new products

In `Registration` mode the Scanner tab MUST follow this exact sequence after a successful resolution:

1. When the resolution is `LotMatch` or `ProductMatch`, the Scanner tab MUST open the existing-lot creation flow for that product: it MUST pre-fill the lot creation form with the resolved product, the active store, and the scanned value as a candidate `batch_code` (when the user scanned a lot code that does not yet exist as a lot, the value is preserved verbatim as `batch_code`). The new lot's quantity defaults to the user's typed value; the expiry date is mandatory; the unit is taken from the product's `default_unit_id` (or the resolved `default_unit` text). On submit the existing `create_expiry_lot` IPC command MUST be called and the Scanner tab MUST NOT auto-add the registered quantity to any existing lot. The registered lot is a brand-new `expiry_lots` row with its own `entry:initial` movement.
2. When the resolution is `Unknown`, the Scanner tab MUST start the quick product creation flow. The scanned value MUST be preserved verbatim and exposed to the form as a candidate `sku`/`primary_barcode` pair (the form renders two distinct fields and lets the user pick which slot receives the scanned value; both may be empty if the user decides to retype). The Scanner tab MUST call `create_product` followed by `add_product_barcode_on_create` only when the user confirms a barcode slot, reusing the typed wrapper already introduced for the dashboard scan-search surface.
3. After a successful submission the Scanner tab MUST clear the input and MUST render a translated success notice naming the new lot's `batch_code` (case 1) or the new product's SKU and barcode (case 2).

#### Scenario: existing product receives a new lot in v1

- GIVEN the scanner resolved to `ProductMatch { product: P }`
- WHEN the Scanner tab opens the lot creation form
- THEN the form is pre-filled with `product_id = P.id` and the active store
- AND the existing active lots for P are NOT shown as a selection (Registration creates a new lot, not an adjustment)
- AND on submit the new lot has its own `expiry_lots` row with its own `entry:initial` movement
- AND no `lot_movements` row touches an existing lot

#### Scenario: unknown scan starts quick-create with the scanned value preserved

- GIVEN the scanner resolved to `Unknown { scanned_value: "7509876543210" }`
- WHEN the Scanner tab opens the quick-create form
- THEN the form renders the SKU and barcode fields side-by-side
- AND both fields are empty
- AND a small affordance (for example, two radio buttons labelled `Use as SKU` / `Use as barcode`) lets the user route the scanned value into exactly one slot
- AND after routing, the scanned value is bound to the chosen slot verbatim
- AND the other slot stays empty
- AND the form rejects submission while the scanned value is un-routed

#### Scenario: scanned lot code becomes the new lot's batch_code

- GIVEN the scanner resolved `SUPPLIER-XYZ-2026` to `Unknown`
- WHEN the Scanner tab starts the quick product creation flow
- AND the user creates the product
- AND the user proceeds to the lot creation step
- THEN the lot creation form pre-fills `batch_code = "SUPPLIER-XYZ-2026"`
- AND the user may keep, change, or clear that value
- AND the auto-generator defined under `lot registration` is NOT invoked because the field is non-blank

#### Scenario: Registration success resets the scanner

- GIVEN the user successfully created a new lot via Registration mode
- WHEN the Scanner tab receives the success response
- THEN the input is cleared
- AND the lot form is closed
- AND a translated success notice reads, in the active locale, the new lot's `batch_code`
- AND the next scan resumes the empty-form state

#### Requirement: Stock-out mode reusing existing reason vocabulary

In `Stock-out` mode the Scanner tab MUST follow this exact sequence after a successful resolution:

1. When the resolution is `ProductMatch`, apply the FEFO policy (see `FEFO lot-selection policy`) to determine the pre-selected lot.
2. When the resolution is `LotMatch` the lot is already chosen.
3. Render a reason picker populated with the eight v1 exit reasons: `exit:waste`, `exit:expired`, `exit:damaged`, `exit:internal_consumption`, `exit:return_to_supplier`, `exit:inventory_adjustment`, `exit:other`. `exit:sale` MUST NOT be listed in the reason picker because Stock-out is the non-sale path; Sales are handled exclusively by `Sale` mode.
4. The quantity input follows the same rules as `Sale` mode (default `1`, editable, integer-unit aware, capped by the selected lot's balance).
5. The `Confirm` button is disabled until lot, reason, and valid quantity are all chosen. The notes field is required when the reason is `exit:inventory_adjustment` or `exit:other` (matching the existing `RegisterExitModal` rule in `src/components/RegisterExitModal.svelte`).
6. On `Confirm` the Scanner tab MUST call `create_lot_movement` with `kind = <the selected reason>`, the selected lot id, the lot's source location (when the lot has a single balance; otherwise the user MUST choose one), and the chosen quantity. Stock MUST NOT be mutated before the user activates `Confirm`.
7. After a successful IPC call the Scanner tab MUST clear the input, reset the quantity to `1`, clear the reason picker, and render a translated success notice that includes the SKU, the chosen reason label, and the removed quantity.

#### Scenario: Stock-out uses the v1 reason vocabulary

- GIVEN the scanner resolved to `LotMatch` for lot L
- WHEN the reason picker renders
- THEN the picker lists the seven non-sale v1 exit reasons
- AND `exit:sale` is NOT present
- AND the labels are routed through `$LL.lotMovements.exitReasons.*` (or its Scanner-tab equivalent) in the active locale

#### Scenario: Stock-out confirm is gated by lot, reason, and quantity

- GIVEN the user has chosen lot L and quantity `1` but no reason
- WHEN the Scanner tab renders the form
- THEN the `Confirm` button is disabled
- AND the button becomes enabled exactly when a non-sale reason is selected and the quantity is valid
- AND when the reason is `exit:inventory_adjustment` or `exit:other`, the notes field is also required

#### Scenario: Stock-out success resets the form

- GIVEN the user confirmed a stock-out of `2` units of product `ABC-001` with reason `Damaged`
- WHEN the IPC call returns success
- THEN the input is cleared
- AND the quantity resets to `1`
- AND the reason picker is cleared
- AND the selected lot remains visible as a hint but is NOT pre-selected for the next scan
- AND a translated success notice reads the SKU, the reason label, and the quantity in the active locale

#### Scenario: Stock-out cannot reference the Sale reason

- GIVEN the user is in Stock-out mode
- WHEN the reason picker renders
- THEN no option has the value `exit:sale`
- AND if a stale form state (for example, a draft picked from a previous session) tries to submit `kind = "exit:sale"` from Stock-out mode, the IPC command rejects the value with a `CommandError::Validation` translated into the active locale

#### Requirement: Stock mutation requires explicit confirmation

The Scanner tab MUST NOT mutate lot stock through any IPC command before the user activates the `Confirm` button on a fully-valid Sale, Registration-lot-submission, or Stock-out form. The rule covers at minimum:

- `create_lot_movement` for `exit:*` kinds (Sale and Stock-out).
- `create_expiry_lot` for Registration (the new lot is the mutation).
- `create_product` followed by `add_product_barcode_on_create` for the unknown-code Registration path.

The `Confirm` button MUST be re-disabled from the moment the user activates it until the IPC response returns, and the input MUST be cleared only after a successful response. On IPC failure the form MUST keep its current state, surface the translated backend error in an inline error slot, and MUST NOT retry automatically.

#### Scenario: confirm fires only one movement

- GIVEN the form is valid and the `Confirm` button is enabled
- WHEN the user activates `Confirm` once
- THEN the Scanner tab issues exactly one `create_lot_movement` IPC call
- AND the button is disabled while the call is in flight
- AND no second call is fired by debounce, retry, or keyboard handler

#### Scenario: IPC failure keeps the form state

- GIVEN the user activated `Confirm` with a valid form
- WHEN the IPC call returns an error
- THEN the Scanner tab does NOT clear the input
- AND the form preserves the chosen lot, quantity, and (for Stock-out) reason
- AND the inline error slot renders the translated backend message
- AND the `Confirm` button is re-enabled so the user may retry

### Capability: Desktop app lifecycle

#### Requirement: default close behavior hides to system tray

When the user closes the main Caduxo window through the operating system's window controls (title-bar close, `Alt+F4` on Windows, window-menu close on Linux, `Cmd+Q` is NOT covered because that exits the whole process), the Tauri runtime MUST intercept `WindowEvent::CloseRequested` and, by default, prevent the close, hide the window with `window.hide()`, and leave the process alive in the system tray. The hide MUST be the default behaviour on first launch and MUST continue to be the default until the user picks an alternative in Configuration.

The application MUST register a tray icon that exposes the application brand and two menu entries (`Restore` and `Quit`). The tray icon MUST persist while the process is alive, MUST be visible on Windows and on the supported Linux desktop environments at parity, and MUST be removed on process exit. Activating `Restore` from the tray menu MUST call `window.show()` and MUST call `window.set_focus()` so the window becomes the active foreground window. Activating `Quit` MUST terminate the process cleanly after closing the window.

#### Scenario: closing the window hides to tray by default

- GIVEN the app has just launched with no persisted close-behavior setting
- WHEN the user closes the main window via the OS window controls
- THEN `WindowEvent::CloseRequested` is intercepted
- AND `window.hide()` is invoked
- AND the process stays alive
- AND the tray icon remains visible
- AND no application-exit event is fired

#### Scenario: tray Restore brings the window back

- GIVEN the main window is hidden in the tray
- WHEN the user activates `Restore` from the tray menu
- THEN the main window is shown
- AND the window receives keyboard focus
- AND the OS task-bar/dock entry reflects the active state

#### Scenario: tray Quit terminates the process

- GIVEN the main window is hidden in the tray
- WHEN the user activates `Quit` from the tray menu
- THEN the main window is closed
- AND the tray icon is removed
- AND the process exits with status `0` after running the existing shutdown hooks (database pool close, structured-log flush)

#### Scenario: close on a platform without tray support fails safely

- GIVEN the running platform does not support the system tray (per a Tauri runtime capability check)
- WHEN the user closes the main window
- THEN the close MUST NOT be silently swallowed
- AND the application MUST fall back to a normal application exit so the user is not stranded with a hidden process they cannot restore
- AND the fallback MUST be logged at `warn` level with the platform reason

#### Requirement: configurable close behavior

The close-window behaviour MUST be configurable through the Settings surface and persisted across restarts (see `Settings: close behavior persisted`). Two values are supported in v1:

- `minimize_to_tray` — the default; intercepts `WindowEvent::CloseRequested`, calls `window.hide()`, and keeps the process alive.
- `exit_application` — does NOT intercept `WindowEvent::CloseRequested`; the window close tears down the process through the existing Tauri shutdown path.

Switching the value MUST take effect on the next close-window event; in-flight windows MUST NOT have their behaviour retroactively changed mid-session.

#### Scenario: exit_application tears down the process

- GIVEN the close-behavior setting is `exit_application`
- WHEN the user closes the main window
- THEN `WindowEvent::CloseRequested` is NOT intercepted
- AND the window is closed
- AND the process exits through the existing Tauri shutdown path
- AND no tray icon remains after exit

#### Scenario: switching to minimize_to_tray takes effect on the next close

- GIVEN the close-behavior setting is `exit_application`
- AND the main window is currently open
- WHEN the user changes the setting to `minimize_to_tray` in Configuration
- THEN the persisted `app_settings.close_behavior` row is updated
- AND the currently open window is NOT immediately hidden
- AND the next close-window event hides the window to the tray

#### Scenario: switching to exit_application takes effect on the next close

- GIVEN the close-behavior setting is `minimize_to_tray`
- AND the main window is currently open
- WHEN the user changes the setting to `exit_application` in Configuration
- THEN the persisted `app_settings.close_behavior` row is updated
- AND the next close-window event exits the application
- AND the tray icon remains visible until the exit (the window is currently shown)

### Capability: Settings

#### Requirement: FEFO policy persisted in app_settings

`SettingsResponse` MUST include a new non-optional `scanner_fefo_policy: FefoPolicy` field. `SettingsUpdate` MUST accept an optional `scanner_fefo_policy?: FefoPolicy` field whose presence causes only that key to be updated; partial updates MUST NOT touch `last_selected_store_id`, `require_initial_location_on_lot_create`, `language`, `theme`, or `close_behavior`.

`FefoPolicy` is a Rust enum with exactly three variants — `SuggestFefo`, `RequireFefo`, `ManualLotChoice` — wire-serialised as the snake_case strings `"suggest_fefo"`, `"require_fefo"`, and `"manual_lot_choice"`. The settings repository MUST expose `get_scanner_fefo_policy(pool) -> FefoPolicy` (defaulting to `SuggestFefo` when the row is absent or unrecognised, matching the existing `language` fallback pattern) and `set_scanner_fefo_policy(pool, value)` helpers. The IPC handler MUST reject values outside the three-variant set with a `CommandError::Validation` translated into the active locale; the persisted row stays untouched on rejection.

The Scanner tab MUST read the policy from `SettingsResponse` on mount and on every `updateSettings` success, and MUST apply it per `FEFO lot-selection policy`.

#### Scenario: missing row behaves as suggest_fefo

- GIVEN no `scanner_fefo_policy` row exists in `app_settings`
- WHEN `getSettings` is invoked
- THEN the response carries `scanner_fefo_policy = "suggest_fefo"`

#### Scenario: partial update of only the FEFO policy

- GIVEN the existing settings include `language = "es"` and `theme = "dark"`
- WHEN the frontend calls `updateSettings({ scanner_fefo_policy: Some("require_fefo") })`
- THEN `app_settings.scanner_fefo_policy` is set to `"require_fefo"`
- AND `app_settings.language` remains `"es"`
- AND `app_settings.theme` remains `"dark"`

#### Scenario: invalid value is rejected

- GIVEN the IPC handler receives
  `updateSettings({ scanner_fefo_policy: Some("force_fefo") })`
- WHEN the handler validates the payload
- THEN the update is rejected with a `CommandError::Validation` translated into the active locale
- AND the persisted `scanner_fefo_policy` value is unchanged

#### Scenario: scanner tab reads the policy from SettingsResponse on mount

- GIVEN `app_settings.scanner_fefo_policy` is `"require_fefo"`
- WHEN the user opens the Scanner tab
- THEN the lot picker behaves per `require_fefo` rules immediately
- AND the Scanner tab does NOT issue a separate IPC call to read the policy

#### Requirement: close behavior persisted in app_settings

`SettingsResponse` MUST include a new non-optional `close_behavior: CloseBehavior` field. `SettingsUpdate` MUST accept an optional `close_behavior?: CloseBehavior` field whose presence causes only that key to be updated.

`CloseBehavior` is a Rust enum with exactly two variants — `MinimizeToTray`, `ExitApplication` — wire-serialised as the snake_case strings `"minimize_to_tray"` and `"exit_application"`. The settings repository MUST expose `get_close_behavior(pool) -> CloseBehavior` (defaulting to `MinimizeToTray` when the row is absent or unrecognised) and `set_close_behavior(pool, value)` helpers. The IPC handler MUST reject values outside the two-variant set with a `CommandError::Validation` translated into the active locale; the persisted row stays untouched on rejection.

The Tauri runtime MUST read the persisted `close_behavior` on startup before installing the `WindowEvent::CloseRequested` handler so the configured behaviour is in effect from the very first close attempt. The Configuration page MUST expose a translated selector with the same optimistic-update + rollback-on-failure pattern used by the existing language selector.

#### Scenario: missing row behaves as minimize_to_tray

- GIVEN no `close_behavior` row exists in `app_settings`
- WHEN `getSettings` is invoked
- THEN the response carries `close_behavior = "minimize_to_tray"`
- AND the first close-window event hides the window to the tray

#### Scenario: partial update of only the close behavior

- GIVEN the existing settings include `language = "es"` and `require_initial_location_on_lot_create = true`
- WHEN the frontend calls `updateSettings({ close_behavior: Some("exit_application") })`
- THEN `app_settings.close_behavior` is set to `"exit_application"`
- AND `app_settings.language` and `app_settings.require_initial_location_on_lot_create` are unchanged

#### Scenario: invalid value is rejected

- GIVEN the IPC handler receives
  `updateSettings({ close_behavior: Some("kill") })`
- WHEN the handler validates the payload
- THEN the update is rejected with a `CommandError::Validation` translated into the active locale
- AND the persisted `close_behavior` value is unchanged

#### Scenario: configuration selector rolls back on IPC failure

- GIVEN the active close behavior is `minimize_to_tray`
- AND the persisted `close_behavior` value is `"minimize_to_tray"`
- WHEN the user selects `exit_application`
- AND `updateSettings({ close_behavior: Some("exit_application") })` returns an error
- THEN the selector returns to `minimize_to_tray`
- AND the persisted `close_behavior` value remains `"minimize_to_tray"`
- AND a translated "could not save" message is rendered in the inline error slot

#### Scenario: persisted value survives restart

- GIVEN `app_settings.close_behavior` is `"exit_application"`
- WHEN the user restarts the application
- THEN `getSettings` returns `close_behavior = "exit_application"`
- AND the first close-window event tears down the process without hiding
