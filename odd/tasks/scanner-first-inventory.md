# Scanner-first inventory operations

Branch: `feat/scanner-first-inventory`

## Goal

Make Scanner the canonical operational surface for lots and stock movements, while keeping Product Detail as a reusable entry point for creating lots. Dashboard becomes a read-oriented surface that opens the relevant lot in Scanner.

## Confirmed design decisions

- A lot represents a product batch and expiry identity, not an individual box or shelf position.
- Initial stock may be distributed among multiple locations within the same store while remaining one lot.
- Distributed creation writes one `entry:initial` for the full quantity and transfers to additional locations in one atomic backend transaction.
- Transfers between stores preserve the same logical lot and its total quantity: they move a balance from an origin-store location to a destination-store location. The receiving store must be able to find and operate a lot when it has a positive local balance, even if it is not the lot's anchor `store_id`.
- A single reusable lot-creation flow serves Scanner and Product Detail. The default remains one location; the operator can opt into multi-location distribution.
- Scanner owns sale, stock-out, registration, and a new lot-context view for transfer, exit, adjustment, balances, and movement history.
- Dashboard does not duplicate mutations; it opens a selected lot in Scanner through in-memory shell navigation. No URL router is introduced.
- Cross-store *initial* distribution, reporting changes, and persistent deep links are out of scope for the first release. Cross-store transfers after lot creation are in scope.

## Tasks

- [x] 1. Map and test the existing lot and movement invariants needed by the new flow, including initial-entry semantics, transfer balances, store ownership, and atomic rollback behavior.
- [x] 2. Add an atomic backend command and DTO for creating one lot with an initial within-store location distribution; cover validation, duplicate locations, quantity totals, integer units, and rollback.
- [x] 3. Extract reusable lot-creation presentation state from `LotForm` and add an accessible optional distribution editor with quantity-total validation and bilingual copy.
- [ ] 4. Route Scanner Registration and Product Detail Add Lot through the same reusable creation flow, preserving Scanner store locking and existing Product Detail behavior for the simple path.
- [ ] 5. Add Scanner lot-context view showing current per-location balances, movement history, and canonical actions for same-store and cross-store transfer, exit, and count adjustment.
- [ ] 6. Make Scanner resolution and Dashboard lot views location-balance aware so a receiving store can find and operate a transferred lot without duplicating the lot identity.
- [ ] 7. Add in-memory Dashboard-to-Scanner navigation that opens the selected lot in Scanner context without duplicating lot mutation UI.
- [ ] 8. Consolidate shared movement client rules and fix known selector/state drift before using the Scanner lot-context actions broadly.
- [ ] 9. Remove superseded Dashboard mutation UI, duplicate Scanner/Modal movement logic, dead navigation state, and obsolete i18n only after their replacements are integrated and covered.
- [ ] 10. Run focused backend/frontend verification, accessibility review, manual operational scenarios, including cross-store transfer and receipt, and document evidence.

## Supported operational possibilities

The target workflow retains every current domain operation while assigning a clear canonical surface.

| Operation | Canonical surface | Data effect |
| --- | --- | --- |
| Create a product from an unknown scan | Scanner Registration using reusable `ProductForm` | Creates a product; no lot or stock movement yet. |
| Create a lot in one location | Scanner Registration or Product Detail using reusable `LotForm` | Creates one lot and one initial ledger entry. |
| Create a lot distributed across several locations in one store | Same reusable creation flow with distribution enabled | Creates one lot, one initial entry for the total, and internal transfers atomically. |
| Sell stock | Scanner Sale | Registers an `exit:sale` from the selected location. |
| Register a non-sale output | Scanner Stock-out / lot context | Registers a typed `exit:*` with required notes where the domain requires them. |
| Move stock inside one store | Scanner lot context | Registers a `transfer` between two locations; total stays unchanged. |
| Transfer stock between stores | Scanner lot context | Registers a `transfer` from an origin-store location to a destination-store location; total and lot identity stay unchanged. |
| Replenish display stock | Scanner lot context | A same-store transfer, normally storage to display. |
| Adjust a physical count | Scanner lot context | Registers `inventory_adjustment` against one location; notes remain required. |
| Resolve a lot | Scanner lot context / existing product detail capability | Records the existing lot-resolution outcome and changes status when applicable. |
| Archive a lot | Existing product-detail capability, later linkable from Scanner context | Applies the existing archive flow and its required reason/notes. |
| Inspect stock and movement history | Scanner lot context; Dashboard is read-oriented | Shows total, balances by location, and immutable ledger history. |
| Open a lot from Dashboard | Dashboard → Scanner context | Navigates in memory; Dashboard does not duplicate mutation forms. |

### Cross-store transfer contract

- The origin location must have enough balance; the destination location must be active.
- The lot remains one logical batch with one total, batch code, expiry date, alert policy, and movement history.
- The receiving store can discover the lot only while it has a positive balance in one of its locations.
- The lot anchor store is retained for backward-compatible records; location balances, not the anchor alone, decide operational visibility.
- Initial distribution remains within one store in v1. To place stock in another store, create it in the origin store and use the explicit transfer action.

## Acceptance criteria

- A user can create one lot with a total quantity distributed across two or more active locations in one store.
- The persisted lot total equals the ledger-derived total, and every distributed creation either fully commits or fully rolls back.
- Same-batch, same-expiry stock distributed between storage and display remains one lot with visible per-location balances.
- Scanner is the canonical place to inspect a lot and record its operational changes.
- Product Detail opens the same reusable creation flow and retains a simple single-location path.
- Dashboard can open a lot in Scanner context without reproducing transfer, exit, or adjustment controls.
- Superseded mutation components, duplicate helpers, stale navigation paths, tests, and i18n keys are removed only after the canonical Scanner flow replaces every caller; no dead code path remains reachable.
- Location selectors stay synchronized and prevent invalid empty selections; integer-unit quantities reject fractional values before submission.
- All new user-visible text is available in English and Spanish, and the new distribution controls remain keyboard-accessible.

## Non-goals

- Cross-store initial distribution.
- Splitting identical batch/expiry stock into multiple lots merely because it occupies multiple locations.
- URL routing or restoration of pending Scanner context after application restart.
- Report/CSV redesign for displaying location distributions.
- Changing the existing product lifecycle model.

## Replacement and cleanup rule

This feature is a replacement, not an additive second workflow. Once the Scanner lot-context surface is live and callers have moved:

- Dashboard-local mutation modals and their state are deleted rather than hidden.
- Duplicated Scanner Stock-out and modal validation/reason helpers are consolidated behind the canonical movement form and then removed.
- Obsolete callbacks, imports, navigation state, tests, and untranslated i18n keys are removed in the same work unit as their final caller.
- A component is removed only after a repository-wide reference search, focused behavior checks, and readback confirm that no supported entry point depends on it.
- Shared primitives and distinct domain actions (lot creation, resolution, archival) remain when still used; cleanup never means deleting behavior merely because it is not in Scanner yet.

## Planned verification

- Focused Rust service/command tests for distributed creation and movement invariants.
- `cargo test --lib` and `cargo build` in `src-tauri`.
- i18n generation, Svelte type checks, production frontend build, and `git diff --check`.
- Keyboard/focus and screen-reader-label review of the distribution editor.
- Manual scenarios: receive into storage + display, sell from display, replenish from storage, adjust one location, and open a dashboard lot in Scanner.

## Evidence

### Task 1 — invariant mapping (in progress)

- Existing focused Rust evidence passed: `create_lot_movement_emits_initial_entry_on_lot_creation`, `create_lot_movement_transfer_preserves_lot_total`, and `create_lot_movement_transfer_accepts_cross_store_destination`.
- Command: `cd src-tauri && cargo test --lib --package caduxo --no-fail-fast -- --nocapture create_lot_movement_emits_initial_entry_on_lot_creation create_lot_movement_transfer_preserves_lot_total create_lot_movement_transfer_accepts_cross_store_destination`.
- Result: 3 passed, 0 failed.
- Confirmed invariant: distributed creation must emit exactly one `entry:initial` for the total and transfers for additional locations. Multiple initial entries would double-count the lot total during ledger reconciliation.
- Confirmed gap: cross-store transfers already work and retain the origin anchor store, but Scanner and Dashboard currently filter by that anchor and therefore cannot discover a transferred lot in the receiving store. Task 6 addresses this.
- Task 1 completion: no existing test exercises a SQL failure after `BEGIN` and asserts rollback; Task 2 therefore owns five focused service tests, including an inactive destination after the first allocation and zero persisted lot/movement rows after failure.
- Task 1 completion: Scanner and Dashboard currently scope discovery to `expiry_lots.store_id`; Task 6 must derive store visibility from positive ledger balances in `store_locations`.

### Task 2 — atomic distributed creation (complete)

- Added `create_expiry_lot_distributed`: one initial entry for the total plus transfers to additional active locations, within one transaction.
- The service rejects empty or duplicate allocations, total mismatches, inactive/non-store locations, non-positive quantities, and fractional quantities for integer-unit products.
- Focused verification passed after the final formatter pass: 7 distributed-creation tests, 6 distribution-message tests, and `git diff --check`.
- Full independent verification passed earlier: 757 Rust library tests, frontend type checks, and release build. `cargo clippy` retains one unrelated pre-existing error and nine warnings in `src/domain/lot_movements.rs`.
- Work-unit commit: `d0e35ca feat(lots): support distributed initial stock`.

### Task 3 — reusable distribution editor (complete)

- Added an opt-in, create-only `DistributionEditor` that preserves the existing simple and edit paths in `LotForm`.
- The editor maintains ordered anchor allocations, blocks invalid totals/rows/duplicates/fractional integer quantities, and dispatches the distributed backend wrapper only when enabled.
- English/Spanish copy and generated i18n types are in parity. Independent frontend verification passed: `npm run check`, `npm run build`, and `git diff --check`.
- Work-unit commit: `204c59c feat(lots): add initial distribution editor`.
