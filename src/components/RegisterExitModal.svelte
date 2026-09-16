<script lang="ts">
  import { createLotMovement, type LotLocationBalance } from "../lib/lot_movements.js";
  import type { UnitKind } from "../lib/products.js";

  // ── Props ──────────────────────────────────────────────────────────────────

  export let lotId: string;
  export let currentBalances: LotLocationBalance[];
  /**
   * Unit kind resolved from the lot's product catalog link.
   * `null` for legacy/uncatalogued products — treated as decimal.
   */
  export let unitType: UnitKind | null = null;
  export let onClose: () => void;
  export let onCreated: () => void;

  // ── State ──────────────────────────────────────────────────────────────────

  let sourceLocationId = "";
  let exitReason = "";
  let quantity = 0;
  let notes = "";
  let submitting = false;
  let errorMsg = "";

  // Exit reasons (the eight kinds from the spec)
  const EXIT_REASONS = [
    { value: "exit:sale", label: "Venta" },
    { value: "exit:loss", label: "Pérdida" },
    { value: "exit:expired", label: "Vencido" },
    { value: "exit:damaged", label: "Dañado" },
    { value: "exit:internal_use", label: "Consumo interno" },
    { value: "exit:return_to_supplier", label: "Devolución a proveedor" },
    { value: "exit:inventory_adjustment", label: "Ajuste de inventario (salida)" },
    { value: "exit:other", label: "Otro" },
  ];

  // Reasons that require notes
  const REQUIRES_NOTES = ["exit:inventory_adjustment", "exit:other"];

  $: requiresNotes = REQUIRES_NOTES.includes(exitReason);
  $: availableQuantity = sourceLocationId
    ? currentBalances.find((b) => b.location_id === sourceLocationId)?.balance ?? 0
    : 0;

  // ── Unit-aware quantity input rules ───────────────────────────────────────
  $: isIntegerUnit = unitType === "integer";
  $: qtyMin = isIntegerUnit ? 1 : 0.01;
  $: qtyStep = isIntegerUnit ? 1 : 0.01;
  $: qtyInputMode = (isIntegerUnit ? "numeric" : "decimal") as
    | "numeric"
    | "decimal";

  /**
   * Local validation for integer-unit products: catches fractional input
   * before submit so the user gets immediate feedback. Backend enforces the
   * same invariant, but this avoids a round-trip for the common case.
   */
  function isFractionalForIntegerUnit(qty: number): boolean {
    if (!isIntegerUnit) return false;
    if (qty <= 0) return false;
    return !Number.isInteger(qty);
  }

  // ── Submit ────────────────────────────────────────────────────────────────

  async function submit() {
    errorMsg = "";

    if (!sourceLocationId) {
      errorMsg = "Selecciona una ubicación de origen";
      return;
    }
    if (!exitReason) {
      errorMsg = "Selecciona un motivo de salida";
      return;
    }
    if (quantity <= 0) {
      errorMsg = "La cantidad debe ser mayor a 0";
      return;
    }
    if (isFractionalForIntegerUnit(quantity)) {
      errorMsg = `La unidad del producto es de tipo entero; no se permiten cantidades fraccionarias (${quantity})`;
      return;
    }
    if (quantity > availableQuantity) {
      errorMsg = `Solo hay ${availableQuantity} unidades disponibles en esta ubicación`;
      return;
    }
    if (requiresNotes && !notes.trim()) {
      errorMsg = "Se requieren notas para este tipo de salida";
      return;
    }

    submitting = true;
    try {
      await createLotMovement({
        lot_id: lotId,
        kind: exitReason,
        direction: null,
        quantity,
        source_location_id: sourceLocationId,
        destination_location_id: null,
        notes: notes.trim() || null,
      });
      onCreated();
    } catch (e) {
      errorMsg = String(e);
      submitting = false;
    }
  }
</script>

<div class="modal-overlay" role="dialog" aria-modal="true" aria-label="Registrar salida">
  <div class="modal-box">
    <div class="modal-header">
      <h3>Registrar salida</h3>
      <button class="modal-close" on:click={onClose}>✕</button>
    </div>

    <div class="modal-body">
      <div class="form-group">
        <label for="exit-source">Ubicación de origen *</label>
        <select id="exit-source" bind:value={sourceLocationId} disabled={submitting}>
          <option value="">Seleccionar ubicación…</option>
          {#each currentBalances as bal}
            {#if bal.balance > 0}
              <option value={bal.location_id}>{bal.location_id} ({bal.balance} disponibles)</option>
            {/if}
          {/each}
        </select>
      </div>

      <div class="form-group">
        <label for="exit-reason">Motivo de salida *</label>
        <select id="exit-reason" bind:value={exitReason} disabled={submitting}>
          <option value="">Seleccionar motivo…</option>
          {#each EXIT_REASONS as reason}
            <option value={reason.value}>{reason.label}</option>
          {/each}
        </select>
      </div>

      <div class="form-group">
        <label for="exit-qty">Cantidad *</label>
        <input
          id="exit-qty"
          type="number"
          min={qtyMin}
          step={qtyStep}
          inputmode={qtyInputMode}
          max={availableQuantity}
          bind:value={quantity}
          disabled={submitting}
        />
        <span class="hint">Disponibles: {availableQuantity}{isIntegerUnit ? " (solo enteros)" : ""}</span>
      </div>

      <div class="form-group">
        <label for="exit-notes">
          Notas {requiresNotes ? "*" : "(opcional)"}
        </label>
        <textarea
          id="exit-notes"
          rows="3"
          bind:value={notes}
          disabled={submitting}
          placeholder={requiresNotes ? "Se requieren notas para este motivo" : "Notas adicionales…"}
        ></textarea>
      </div>

      {#if errorMsg}
        <div class="alert-error" role="alert">{errorMsg}</div>
      {/if}
    </div>

    <div class="modal-footer">
      <button type="button" class="btn-secondary" on:click={onClose} disabled={submitting}>
        Cancelar
      </button>
      <button
        type="button"
        class="btn-primary"
        on:click={submit}
        disabled={submitting}
      >
        {submitting ? "Guardando…" : "Registrar"}
      </button>
    </div>
  </div>
</div>

<style>
  .form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 14px;
  }

  .form-group label {
    font-size: 0.85rem;
    color: #374151;
    font-weight: 500;
  }

  .form-group select,
  .form-group input,
  .form-group textarea {
    padding: 8px 10px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
  }

  .form-group select:focus,
  .form-group input:focus,
  .form-group textarea:focus {
    outline: 2px solid #3b82f6;
    border-color: #3b82f6;
  }

  .hint {
    font-size: 0.78rem;
    color: #6b7280;
  }

  .alert-error {
    background: #fee2e2;
    color: #991b1b;
    border: 1px solid #fca5a5;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 0.85rem;
    margin-bottom: 12px;
  }
</style>
