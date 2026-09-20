<script lang="ts">
  import {
    listUnrecognizedUnits,
    listUnitDefinitions,
    applyUnitReviewAction,
    type UnrecognizedUnitGroup,
    type UnitDefinitionResponse,
    type UnitReviewActionResult,
  } from "../lib/unit_definitions.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import { humanizeError } from "../lib/errors.js";

  /** Called when the user dismisses or finishes the review. */
  export let onDone: () => void;

  let loading = true;
  let groups: UnrecognizedUnitGroup[] = [];
  let unitCatalog: UnitDefinitionResponse[] = [];
  let error = "";
  let actionInProgress: string | null = null;
  let actionResult: UnitReviewActionResult | null = null;

  /** Plural-aware subtitle message. */
  $: subtitleFn = groups.length === 0
    ? $LL.unitReview.noUnrecognizedUnits
    : groups.length === 1
    ? $LL.unitReview.unrecognizedFound
    : $LL.unitReview.unrecognizedFound;
  $: subtitleValues = groups.length === 1
    ? $LL.unitReview.unrecognizedValue_singular
    : $LL.unitReview.unrecognizedValue_plural;
  $: subtitleMessage = subtitleFn({
    count: groups.length,
    values: subtitleValues,
  });

  /** Plural-aware success message. */
  $: successFn = actionResult && actionResult.updated_count === 1
    ? $LL.unitReview.productsUpdated_singular
    : $LL.unitReview.productsUpdated_plural;
  $: successMessage = successFn ? successFn({ count: actionResult!.updated_count }) : "";
  $: assignedUnitMsg = actionResult?.assigned_unit
    ? $LL.unitReview.unitAssigned({ name: actionResult.assigned_unit.display_name })
    : "";

  async function init() {
    loading = true;
    error = "";
    try {
      [groups, unitCatalog] = await Promise.all([
        listUnrecognizedUnits(),
        listUnitDefinitions(),
      ]);
    } catch (e) {
      error = humanizeError(e);
    } finally {
      loading = false;
    }
  }

  /** Maps a raw value to an existing preset unit. */
  async function mapToPreset(rawValue: string, presetId: string) {
    actionInProgress = rawValue;
    actionResult = null;
    try {
      const result = await applyUnitReviewAction({
        MapToPreset: { raw_value: rawValue, preset_id: presetId },
      });
      actionResult = result;
      groups = groups.filter((g) => g.raw_value !== rawValue);
    } catch (e) {
      error = humanizeError(e);
    } finally {
      actionInProgress = null;
    }
  }

  /** Creates a custom unit and assigns it to all products with this raw value. */
  async function keepAsCustom(rawValue: string, displayName: string, kind: "integer" | "decimal") {
    actionInProgress = rawValue;
    actionResult = null;
    try {
      const result = await applyUnitReviewAction({
        KeepAsCustom: { raw_value: rawValue, display_name: displayName, kind },
      });
      actionResult = result;
      groups = groups.filter((g) => g.raw_value !== rawValue);
    } catch (e) {
      error = humanizeError(e);
    } finally {
      actionInProgress = null;
    }
  }

  /** Marks a raw value for later review (no-op in this slice). */
  async function leaveForLater(rawValue: string) {
    actionInProgress = rawValue;
    actionResult = null;
    try {
      const result = await applyUnitReviewAction({
        LeaveForLater: { raw_value: rawValue },
      });
      actionResult = result;
      groups = groups.filter((g) => g.raw_value !== rawValue);
    } catch (e) {
      error = humanizeError(e);
    } finally {
      actionInProgress = null;
    }
  }

  /** Preset units grouped by kind for the "Map to preset" dropdown. */
  $: presetsByKind = {
    integer: unitCatalog.filter((u) => u.kind === "integer" && !u.archived_at),
    decimal: unitCatalog.filter((u) => u.kind === "decimal" && !u.archived_at),
  };

  // Load data on mount.
  init();
</script>

<div class="review-page">
  <div class="review-header">
    <h2>{$LL.unitReview.pageTitle()}</h2>
    <p class="subtitle">
      {subtitleMessage}
    </p>
  </div>

  {#if error}
    <div class="alert-error" role="alert">{error}</div>
  {/if}

  {#if loading}
    <p class="loading">{$LL.common.loading()}</p>
  {:else if groups.length === 0}
    <div class="empty-state">
      <p>{$LL.unitReview.allRecognized()}</p>
      <button type="button" class="btn-primary" on:click={onDone}>
        {$LL.unitReview.backToDashboard()}
      </button>
    </div>
  {:else}
    <div class="group-list">
      {#each groups as group (group.raw_value)}
        {@const inProgress = actionInProgress === group.raw_value}
        <div class="group-card" class:disabled={inProgress}>
          <div class="group-header">
            <span class="raw-value">"{group.raw_value}"</span>
            <span class="product-count">
              {group.product_count === 1
                ? $LL.unitReview.unitCount_singular({ n: group.product_count })
                : $LL.unitReview.unitCount_plural({ n: group.product_count })}
            </span>
          </div>

          <div class="action-row">
            <!-- Map to preset -->
            <details class="preset-dropdown">
              <summary class="btn-outline btn-sm">
                {$LL.unitReview.mapToPresetDropdown()}
                <span class="caret">▾</span>
              </summary>
              <div class="dropdown-panel">
                <p class="dropdown-hint">{$LL.unitReview.integerPresets()}</p>
                {#each presetsByKind.integer as preset (preset.id)}
                  <button
                    type="button"
                    class="dropdown-item"
                    disabled={inProgress}
                    on:click={() => mapToPreset(group.raw_value, preset.id)}
                  >
                    {preset.display_name} ({preset.key})
                  </button>
                {/each}
                <p class="dropdown-hint">{$LL.unitReview.decimalPresets()}</p>
                {#each presetsByKind.decimal as preset (preset.id)}
                  <button
                    type="button"
                    class="dropdown-item"
                    disabled={inProgress}
                    on:click={() => mapToPreset(group.raw_value, preset.id)}
                  >
                    {preset.display_name} ({preset.key})
                  </button>
                {/each}
              </div>
            </details>

            <!-- Keep as custom -->
            <details class="custom-form">
              <summary class="btn-outline btn-sm">
                {$LL.unitReview.createCustomUnit()}
                <span class="caret">▾</span>
              </summary>
              <div class="custom-panel">
                <label class="small-label">
                  {$LL.unitReview.displayName()}
                  <input
                    type="text"
                    value={group.raw_value}
                    id="name-{group.raw_value}"
                  />
                </label>
                <div class="kind-radios">
                  <label class="radio-label">
                    <input type="radio" name="kind-{group.raw_value}" value="integer" checked />
                    {$LL.unitReview.integer()}
                  </label>
                  <label class="radio-label">
                    <input type="radio" name="kind-{group.raw_value}" value="decimal" />
                    {$LL.unitReview.decimal()}
                  </label>
                </div>
                <button
                  type="button"
                  class="btn-primary btn-sm"
                  disabled={inProgress}
                  on:click={() => {
                    const nameInput = document.getElementById(
                      `name-${group.raw_value}`,
                    ) as HTMLInputElement;
                    const kindInput = document.querySelector(
                      `input[name="kind-${group.raw_value}"]:checked`,
                    ) as HTMLInputElement;
                    keepAsCustom(
                      group.raw_value,
                      nameInput?.value ?? group.raw_value,
                      (kindInput?.value as "integer" | "decimal") ?? "integer",
                    );
                  }}
                >
                  {inProgress ? $LL.unitReview.inProgress() : $LL.unitReview.createAndAssign()}
                </button>
              </div>
            </details>

            <!-- Leave for later -->
            <button
              type="button"
              class="btn-ghost btn-sm"
              disabled={inProgress}
              on:click={() => leaveForLater(group.raw_value)}
            >
              {$LL.unitReview.leaveForLater()}
            </button>
          </div>
        </div>
      {/each}
    </div>

    {#if actionResult}
      <div class="alert-success" role="status">
        {successMessage}
        {#if assignedUnitMsg}
          {@html assignedUnitMsg}
        {/if}
      </div>
    {/if}

    <button type="button" class="btn-secondary" on:click={onDone}>
      {$LL.unitReview.done()}
    </button>
  {/if}
</div>

<style>
  .review-page {
    max-width: 720px;
    margin: 0 auto;
    padding: 20px;
  }

  .review-header {
    margin-bottom: 20px;
  }

  .review-header h2 {
    margin: 0 0 6px;
    font-size: 1.2rem;
  }

  .subtitle {
    margin: 0;
    color: #6b7280;
    font-size: 0.9rem;
  }

  .alert-error {
    background: #fee2e2;
    border: 1px solid #fca5a5;
    color: #991b1b;
    padding: 10px 14px;
    border-radius: 8px;
    margin-bottom: 16px;
    font-size: 0.9rem;
  }

  .alert-success {
    background: #dcfce7;
    border: 1px solid #86efac;
    color: #166534;
    padding: 10px 14px;
    border-radius: 8px;
    margin-top: 16px;
    font-size: 0.9rem;
  }

  .loading {
    color: #6b7280;
    font-style: italic;
  }

  .empty-state {
    text-align: center;
    padding: 40px 20px;
    color: #374151;
  }

  .empty-state p {
    margin-bottom: 16px;
  }

  .group-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .group-card {
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 14px;
    background: #fff;
    transition: opacity 0.2s;
  }

  .group-card.disabled {
    opacity: 0.6;
    pointer-events: none;
  }

  .group-header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 10px;
  }

  .raw-value {
    font-weight: 600;
    font-size: 1rem;
    color: #111827;
  }

  .product-count {
    font-size: 0.85rem;
    color: #6b7280;
  }

  .action-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: flex-start;
  }

  .btn-outline {
    background: #fff;
    border: 1px solid #d1d5db;
    color: #374151;
    border-radius: 6px;
    padding: 5px 10px;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .btn-outline:hover {
    background: #f9fafb;
    border-color: #9ca3af;
  }

  .btn-ghost {
    background: transparent;
    border: none;
    color: #6b7280;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 5px 8px;
    border-radius: 4px;
  }

  .btn-ghost:hover {
    background: #f3f4f6;
    color: #374151;
  }

  .btn-ghost:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-sm {
    font-size: 0.85rem;
  }

  .caret {
    font-size: 0.7rem;
    margin-left: 4px;
  }

  .preset-dropdown,
  .custom-form {
    position: relative;
  }

  .dropdown-panel {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 8px;
    margin-top: 4px;
    min-width: 200px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    z-index: 10;
    position: absolute;
  }

  .dropdown-hint {
    font-size: 0.75rem;
    font-weight: 600;
    color: #9ca3af;
    margin: 4px 0 2px;
    text-transform: uppercase;
  }

  .dropdown-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 8px;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: 4px;
    font-size: 0.85rem;
    color: #374151;
  }

  .dropdown-item:hover:not(:disabled) {
    background: #f3f4f6;
  }

  .dropdown-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .custom-panel {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 10px;
    margin-top: 4px;
    min-width: 240px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    z-index: 10;
    position: absolute;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .small-label {
    font-size: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .small-label input[type="text"] {
    border: 1px solid #d1d5db;
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 0.85rem;
    width: 100%;
  }

  .kind-radios {
    display: flex;
    gap: 10px;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.85rem;
    cursor: pointer;
  }
</style>
