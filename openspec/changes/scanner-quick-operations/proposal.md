# Proposal — scanner-quick-operations

## Change metadata

- **Change ID**: `scanner-quick-operations`
- **Domain**: `caduxo-expiry-tracker` (single-domain project)
- **Artifact store**: `both` (session-confirmed)
- **Review budget**: 800 changed lines (project configuration)
- **Delivery strategy**: `ask-on-risk`
- **Chain strategy**: `deferred` — no chained split planned at proposal time
- **Strict TDD**: `false` (project default)
- **Execution mode**: `interactive`

## Problem statement

Caduxo is a local-first desktop expiry tracker, but common stock operations still require navigating through slower product and lot screens. In a small shop, the operator may have a handheld scanner in hand and needs to quickly:

- reduce stock after a sale,
- register incoming stock for an existing or new product,
- remove stock for an existing non-sale reason.

Today those workflows are possible through existing screens, but they are not optimized for repeated scanner-driven use. The app also closes normally when the user closes the window, which makes quick scanner access slower than expected for a desktop utility that should remain available during the workday.

## Outcomes (success criteria)

After this change ships, a Caduxo user can:

1. Open a top-level **Scanner** tab.
2. Choose one of three scanner operation modes: **Sale**, **Registration**, or **Stock-out**.
3. Scan SKU, product barcode, or lot code.
4. In Sale mode, scan an item, default quantity to `1`, adjust quantity if needed, choose/confirm a lot, and confirm before stock is decremented.
5. In Registration mode, scan an existing product and create a new lot for it, or scan an unknown code and start quick product creation with the scanned code preserved.
6. In Stock-out mode, scan an item, choose/confirm a lot, choose one of the existing output/removal reasons, set quantity, and confirm before stock is decremented.
7. Use FEFO as a configurable lot-selection policy: suggest FEFO by default, optionally require FEFO, or leave lot choice manual.
8. Close the main window without losing quick access: by default Caduxo hides to the system tray and can be restored from there.
9. Change close behavior in Configuration so users may choose between minimizing to tray and exiting the app.

## Scope (in scope)

- New top-level navigation entry/tab: **Scanner**.
- Scanner input optimized for handheld scanners that submit keyboard input, usually ending with Enter.
- Code matching against lot code first, product barcode second, and product SKU third.
- Sale mode with explicit confirmation, default quantity `1`, editable quantity, and lot selection/confirmation.
- Registration mode that creates a new lot for an existing product in v1.
- Unknown-code path that starts quick product creation and preserves the scanned code as SKU/barcode candidate.
- Stock-out mode using the existing stock-out/removal reason model.
- Configurable FEFO policy in Configuration:
  - `suggest_fefo` (default),
  - `require_fefo`,
  - `manual_lot_choice`.
- System tray close behavior:
  - default: minimize/hide to tray,
  - configurable alternative: exit application.
- Persistence for FEFO policy and close behavior through the existing settings mechanism.

## Non-goals

This change does NOT implement:

- billing,
- payments,
- invoices,
- accounting,
- full POS behavior,
- automatic checkout,
- multi-device sync,
- online inventory service,
- label printing,
- automatic sale confirmation without an explicit user confirmation button,
- automatically adding registration quantity to an existing lot in v1.

## Confirmed product decisions

| # | Decision | Source |
|---|----------|--------|
| 1 | New tab is named **Scanner** / **Escáner** in UI copy | User confirmation |
| 2 | First version includes three modes: Sale, Registration, Stock-out | User confirmation |
| 3 | The former expired/discard idea becomes broader Stock-out / Salida | User correction |
| 4 | Scanner accepts SKU, product barcode, and lot code | User confirmation |
| 5 | Matching priority is lot code, then product barcode, then product SKU | Proposal decision |
| 6 | FEFO is configurable | User confirmation |
| 7 | Default FEFO policy is suggest FEFO, not require FEFO | User confirmation |
| 8 | Sale defaults quantity to `1`, user can change it | User confirmation |
| 9 | Sale requires explicit confirmation before stock changes | User confirmation |
| 10 | Registration of an existing product creates a new lot in v1 | User confirmation |
| 11 | Stock-out mode reuses existing stock-out reasons | User confirmation |
| 12 | Closing the window minimizes to tray by default | User confirmation |
| 13 | Close behavior is configurable from Configuration | User confirmation |

## Proposed capabilities

This proposal adds scanner-driven quick operations and modifies settings/application lifecycle behavior.

### New capability: Scanner quick operations

- A top-level Scanner tab supports Sale, Registration, and Stock-out modes.
- The active mode determines the action performed after scan resolution.
- Scanner input accepts SKU, product barcode, and lot code.
- Lot-code matches select the lot directly. Product matches show available lots.
- FEFO policy affects product-level lot suggestions/selections.
- All stock mutations require explicit user confirmation.

### Modified capability: Settings

- Configuration exposes FEFO policy.
- Configuration exposes close behavior.
- Both settings persist across app restarts.

### Modified capability: Desktop app lifecycle

- Closing the main window minimizes/hides to tray by default.
- The tray exposes restore and quit actions.
- Users can opt into normal exit-on-close behavior.

## Risks and considerations

- **Scope risk:** scanner workflows touch product lookup, lot selection, stock movements, settings, and Tauri lifecycle. Tasks should likely split UI scanner work from tray/settings work if the forecast exceeds review budget.
- **Data-model risk:** lot code support may require adding a field if the current lot model does not already expose one. The design phase must verify existing schema before implementation.
- **FEFO semantics:** requiring FEFO must not block direct lot-code scans unless product rules explicitly say direct lot scans are still bound by FEFO. The design phase must settle this with the spec scenarios.
- **Tray platform behavior:** Windows and Linux tray behavior may differ. Implementation must verify both priority platforms where possible.

## Next phase

Write the delta spec for `caduxo-expiry-tracker`, then produce design and implementation tasks after reviewing the current data model, settings service, navigation structure, and Tauri lifecycle code.
