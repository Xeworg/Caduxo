<!--
  RegisterExitModal.svelte — Migrated to Modal.svelte primitive in
  caduxo-daisyui-redesign PR 7b. Shell markup replaced with the
  shared primitive; the source-location picker and the motivo
  picker both render through the themed `Listbox` primitive (no
  native `<select>` so OS black dropdowns cannot leak through).
  Quantity input and notes textarea stay inline (textareas and
  number spinners are out of scope for the themed primitives).
  Business state, validation, submit handlers, and visible copy
  preserved verbatim.

  Task 2.7d: Stock-out reasons are now fetched from the catalog API
  and submitted as `exit_reason_id`. The backend derives the movement
  kind. Notes requirement is driven by the selected reason's
  `movement_kind`. Movement history shows immutable snapshot `reason`
  plus `movement_kind`; archived reasons render as plain disabled text.
-->
<script lang="ts">
  import { createLotMovement, type LotLocationBalance } from "../lib/lot_movements.js";
import {
  isFractionalForIntegerUnit,
  notesRequiredByMovementKind,
  qtyAttrs,
} from "../lib/movementRules.js";
import {
  listStockOutReasons,
  isReasonActive,
  type StockOutReason,
} from "../lib/stock_out_reasons.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import { locale } from "../i18n/locale.svelte.js";
  import type { UnitKind } from "../lib/products.js";
  import { humanizeError } from "../lib/errors.js";
  import Modal from "./ui/Modal.svelte";
  import Listbox from "./ui/Listbox.svelte";
  import Button from "./ui/Button.svelte";

  // ── Props ──────────────────────────────────────────────────────────────────

  interface Props {
    lotId: string;
    currentBalances: LotLocationBalance[];
    /** All active store locations — resolved to human-readable names. */
    allLocations: { id: string; name: string; store_id: string; store_name?: string }[];
    /** Unit kind from the lot's product catalog link. `null` for legacy products. */
    unitType: UnitKind | null;
    onClose: () => void;
    onCreated: () => void;
  }

  const {
    lotId,
    currentBalances,
    allLocations,
    unitType,
    onClose,
    onCreated,
  }: Props = $props();

  // ── State ──────────────────────────────────────────────────────────────────

  /** Backs the `<Modal>` primitive via two-way binding. */
  let visible = $state(true);
  /** Element to receive focus when the modal closes (the trigger row). */
  let returnFocusTo: HTMLElement | null = $state(null);

  let sourceLocationId = $state("");
  /** ID of the selected stock-out reason from the catalog. */
  let selectedReasonId = $state("");
  /** The resolved reason object for notes-requirement and archived-display logic. */
  let selectedReason: StockOutReason | null = $state(null);
  let quantity = $state(0);
  let notes = $state("");
  let submitting = $state(false);
  let errorMsg = $state("");
  let loadingReasons = $state(false);
  let reasonsLoadError = $state("");
  /** All stock-out reasons fetched from the catalog. */
  let allReasons: StockOutReason[] = $state([]);

  /** True when the reason picker and submit should be disabled due to load failure. */
  let reasonsUnavailable = $derived(loadingReasons || !!reasonsLoadError);

  // ── Reactive derived state ─────────────────────────────────────────────────

  /** Notes are required when the selected reason's movement_kind demands them. */
  let requiresNotes = $derived.by(() => {
    if (!selectedReason) return false;
    return notesRequiredByMovementKind(selectedReason.movement_kind);
  });

  /**
   * Active reasons available for selection in the dropdown.
   * Filtered using `archived_at === null` (NOT `is_active`).
   * The backend uses `archived_at` to track soft-delete state.
   */
  let activeReasons = $derived(allReasons.filter(isReasonActive));

  let availableQuantity = $derived(
    sourceLocationId
      ? currentBalances.find((b) => b.location_id === sourceLocationId)?.balance ?? 0
      : 0,
  );

  let isIntegerUnit = $derived(unitType === "integer");
  let _qtyAttrs = $derived(qtyAttrs(isIntegerUnit));

  /**
   * Options for the stock-out reason Listbox.
   * Includes an empty placeholder and active reasons only — archived
   * reasons are never selectable.
   */
  let exitReasonOptions = $derived([
    { value: "", label: $LL.lotMovements.modal.selectExitReason(), disabled: true },
    ...activeReasons.map((r) => ({
      value: r.id,
      label: r.display_name,
    })),
  ]);

  /**
   * When the selected reason is archived (archived_at !== null), append a
   * disabled plain-text option so the historical snapshot remains legible
   * in the dropdown but cannot be re-selected.
   */
  let archivedSelectedOption = $derived.by(() => {
    if (!selectedReason || isReasonActive(selectedReason)) return [];
    return [{ value: selectedReason.id, label: selectedReason.display_name, disabled: true }];
  });

  /**
   * Combined reason options: active reasons first, then the archived
   * selected reason (if any) as a disabled trailing entry.
   */
  let allReasonOptions = $derived([...exitReasonOptions, ...archivedSelectedOption]);

  /**
   * Source-location options for the `<Listbox>` primitive. Filters
   * balances to those with stock available and labels each entry
   * with the resolved location name (from `allLocations`) and the
   * available-balance badge. When a location id is not found in
   * `allLocations` the raw id is used as a fallback so the row
   * stays inspectable rather than disappearing silently.
   */
  let sourceLocationOptions = $derived([
    { value: "", label: $LL.lotMovements.modal.selectLocation(), disabled: true },
    ...currentBalances
      .filter((bal) => bal.balance > 0)
      .map((bal) => {
        const resolved = allLocations.find((l) => l.id === bal.location_id);
        const displayName = resolved?.name ?? bal.location_id;
        return {
          value: bal.location_id,
          label: `${displayName} (${$LL.lotMovements.modal.availableOption({ balance: bal.balance })})`,
        };
      }),
  ]);

  // ── Load reasons on mount ─────────────────────────────────────────────────

  import { onMount } from "svelte";

  onMount(async () => {
    await loadReasons();
  });

  async function loadReasons(): Promise<void> {
    loadingReasons = true;
    reasonsLoadError = "";
    try {
      allReasons = await listStockOutReasons();
    } catch (e) {
      reasonsLoadError = $LL.lotMovements.errors.loadStockOutReasonsFailed({
        msg: humanizeError(e),
      });
    } finally {
      loadingReasons = false;
    }
  }

  function retryReasons(): void {
    void loadReasons();
  }

  /**
   * Handles reason Listbox changes. When the user selects an active
   * reason, resolves the full reason object for notes-requirement logic.
   * When the user clears the selection (empty string), resets the reason.
   */
  function handleReasonChange(reasonId: string) {
    selectedReasonId = reasonId;
    if (!reasonId) {
      selectedReason = null;
      return;
    }
    selectedReason = allReasons.find((r) => r.id === reasonId) ?? null;
  }

  // ── Submit ────────────────────────────────────────────────────────────────

  async function submit() {
    errorMsg = "";

    if (!sourceLocationId) {
      errorMsg = $LL.lotMovements.modal.selectSourceLocationError();
      return;
    }
    if (!selectedReasonId) {
      errorMsg = $LL.lotMovements.modal.selectExitReasonError();
      return;
    }
    if (quantity <= 0) {
      errorMsg = $LL.lotMovements.modal.quantityPositiveError();
      return;
    }
    if (isFractionalForIntegerUnit(quantity, isIntegerUnit)) {
      errorMsg = $LL.lotMovements.modal.integerQuantityError({ quantity });
      return;
    }
    if (quantity > availableQuantity) {
      errorMsg = $LL.lotMovements.modal.availableQuantityError({ available: availableQuantity });
      return;
    }
    if (requiresNotes && !notes.trim()) {
      errorMsg = $LL.lotMovements.modal.exitNotesRequiredError();
      return;
    }

    submitting = true;
    try {
      // Submit `exit_reason_id`; the backend derives the movement kind.
      // `kind` is left empty because it is ignored when `exit_reason_id`
      // is present.
      await createLotMovement(
        {
          lot_id: lotId,
          kind: "",
          direction: null,
          quantity,
          source_location_id: sourceLocationId,
          destination_location_id: null,
          notes: notes.trim() || null,
          exit_reason_id: selectedReasonId,
        },
        locale.current,
      );
      onCreated();
    } catch (e) {
      errorMsg = humanizeError(e);
      submitting = false;
    }
  }

  /**
   * The modal's `oncancel` callback wires Escape to the consumer's
   * close handler so the "discard in-progress edits on Escape"
   * semantic is preserved verbatim.
   */
  function handleCancel() {
    if (submitting) return;
    onClose();
  }

  function handleClose() {
    onClose();
  }
</script>

<Modal
  bind:open={visible}
  size="md"
  showClose
  closeLabel={$LL.lotMovements.modal.close()}
  {returnFocusTo}
  oncancel={handleCancel}
  onclose={handleClose}
  aria-label={$LL.lotMovements.registerExit()}
>
  {#snippet children()}
    <header class="dialog-header">
      <h3>{$LL.lotMovements.registerExit()}</h3>
    </header>

    <div class="dialog-body">
      {#if loadingReasons}
        <p class="loading-hint">{$LL.common.loading()}</p>
      {:else if reasonsLoadError}
        <p class="reasons-error">{reasonsLoadError}</p>
        <Button
          variant="outline"
          size="sm"
          onclick={retryReasons}
          disabled={loadingReasons}
        >
          {$LL.common.retry()}
        </Button>
      {/if}

      <div class="form-group">
        <label for="exit-source">{$LL.lotMovements.modal.sourceLocation()}</label>
        <Listbox
          id="exit-source"
          value={sourceLocationId}
          options={sourceLocationOptions}
          disabled={submitting || reasonsUnavailable}
          aria-label={$LL.lotMovements.modal.sourceLocation()}
          onchange={(v) => (sourceLocationId = v)}
        />
      </div>

      <div class="form-group">
        <label for="exit-reason">{$LL.lotMovements.modal.exitReason()}</label>
        <Listbox
          id="exit-reason"
          value={selectedReasonId}
          options={allReasonOptions}
          disabled={submitting || reasonsUnavailable}
          aria-label={$LL.lotMovements.modal.exitReason()}
          onchange={(v) => handleReasonChange(v)}
        />
      </div>

      <div class="form-group">
        <label for="exit-qty">{$LL.lotMovements.modal.quantity()}</label>
        <input
          id="exit-qty"
          type="number"
          class="input input-md w-full motion-reduce:transition-none"
          min={_qtyAttrs.min}
          step={_qtyAttrs.step}
          inputmode={_qtyAttrs.inputmode}
          max={availableQuantity}
          bind:value={quantity}
          disabled={submitting || reasonsUnavailable}
          aria-label={$LL.lotMovements.modal.quantity()}
        />
        <span class="hint">{$LL.lotMovements.available({ available: availableQuantity })}{isIntegerUnit ? $LL.lotMovements.integerNote() : ""}</span>
      </div>

      <div class="form-group">
        <label for="exit-notes">
          {requiresNotes ? $LL.lotMovements.modal.notesRequired() : $LL.lotMovements.modal.notesOptional()}
        </label>
        <textarea
          id="exit-notes"
          class="textarea w-full motion-reduce:transition-none"
          rows="3"
          bind:value={notes}
          disabled={submitting || reasonsUnavailable}
          placeholder={requiresNotes ? $LL.lotMovements.modal.notesRequiredPlaceholder() : $LL.lotMovements.modal.notesOptionalPlaceholder()}
        ></textarea>
      </div>

      {#if errorMsg}
        <div class="alert-error" role="alert">{errorMsg}</div>
      {/if}
    </div>
  {/snippet}

  {#snippet footer()}
    <Button
      variant="ghost"
      onclick={handleClose}
      disabled={submitting}
    >
      {$LL.lotMovements.modal.cancel()}
    </Button>
    <Button
      variant="primary"
      onclick={submit}
      disabled={submitting || reasonsUnavailable || !selectedReasonId}
      loading={submitting}
    >
      {submitting ? $LL.lotMovements.modal.saving() : $LL.lotMovements.modal.registerExitSubmit()}
    </Button>
  {/snippet}
</Modal>

<style>
  /* ── Header ────────────────────────────────────────────────────────────── */
  .dialog-header {
    display: flex;
    align-items: center;
    margin-bottom: 16px;
  }

  .dialog-header h3 {
    margin: 0;
    font-size: 1rem;
    color: var(--color-base-content);
  }

  /* ── Body ──────────────────────────────────────────────────────────────── */
  .dialog-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 14px;
  }

  .form-group label {
    font-size: 0.85rem;
    color: var(--color-base-content);
    font-weight: 500;
  }

  .hint {
    font-size: 0.78rem;
    color: var(--color-base-content);
    opacity: 0.7;
  }

  .alert-error {
    background: color-mix(in oklch, var(--color-error) 12%, transparent);
    color: var(--color-error);
    border: 1px solid color-mix(in oklch, var(--color-error) 30%, transparent);
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 0.85rem;
    margin-bottom: 12px;
  }

  .loading-hint {
    font-size: 0.85rem;
    color: var(--color-base-content);
    opacity: 0.6;
    margin: 0 0 8px;
  }

  .reasons-error {
    font-size: 0.82rem;
    color: var(--color-error);
    margin: 0 0 8px;
  }
</style>
