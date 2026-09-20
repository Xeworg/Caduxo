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
  import Button from "./ui/Button.svelte";
  import Input from "./ui/Input.svelte";
  import Badge from "./ui/Badge.svelte";
  import Alert from "./ui/Alert.svelte";

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
    <Alert variant="error">{error}</Alert>
  {/if}

  {#if loading}
    <p class="loading">{$LL.common.loading()}</p>
  {:else if groups.length === 0}
    <div class="empty-state">
      <p>{$LL.unitReview.allRecognized()}</p>
      <Button variant="primary" onclick={onDone}>
        {$LL.unitReview.backToDashboard()}
      </Button>
    </div>
  {:else}
    <div class="group-list">
      {#each groups as group (group.raw_value)}
        {@const inProgress = actionInProgress === group.raw_value}
        <div class="group-card" class:disabled={inProgress}>
          <div class="group-header">
            <span class="raw-value">"{group.raw_value}"</span>
            <Badge semantic="neutral" size="sm">
              {group.product_count === 1
                ? $LL.unitReview.unitCount_singular({ n: group.product_count })
                : $LL.unitReview.unitCount_plural({ n: group.product_count })}
            </Badge>
          </div>

          <div class="action-row">
            <!-- Map to preset (DaisyUI collapse-arrow) -->
            <details class="preset-dropdown collapse collapse-arrow">
              <summary class="collapse-title">
                {$LL.unitReview.mapToPresetDropdown()}
              </summary>
              <div class="collapse-content">
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

            <!-- Keep as custom (DaisyUI collapse-arrow) -->
            <details class="custom-form collapse collapse-arrow">
              <summary class="collapse-title">
                {$LL.unitReview.createCustomUnit()}
              </summary>
              <div class="collapse-content">
                <Input
                  id="name-{group.raw_value}"
                  value={group.raw_value}
                  label={$LL.unitReview.displayName()}
                />
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
                <Button
                  variant="primary"
                  size="sm"
                  disabled={inProgress}
                  onclick={() => {
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
                </Button>
              </div>
            </details>

            <!-- Leave for later -->
            <Button
              variant="ghost"
              size="sm"
              disabled={inProgress}
              onclick={() => leaveForLater(group.raw_value)}
            >
              {$LL.unitReview.leaveForLater()}
            </Button>
          </div>
        </div>
      {/each}
    </div>

    {#if actionResult}
      <Alert variant="success">
        {successMessage}
        {#if assignedUnitMsg}
          {@html assignedUnitMsg}
        {/if}
      </Alert>
    {/if}

    <Button variant="secondary" onclick={onDone}>
      {$LL.unitReview.done()}
    </Button>
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
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
    font-size: 0.9rem;
  }

  /* The error / success banners are rendered via the `Alert.svelte`
     primitive so the bespoke `.alert-error` / `.alert-success` styling
     rules are intentionally omitted — DaisyUI `alert-{variant} alert-soft`
     provides the same visual contract. */

  .loading {
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
    font-style: italic;
  }

  .empty-state {
    text-align: center;
    padding: 40px 20px;
    color: var(--color-base-content);
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
    border: 1px solid var(--color-base-200);
    border-radius: 8px;
    padding: 14px;
    background: var(--color-base-100);
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
    color: var(--color-base-content);
  }

  .action-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: flex-start;
  }

  /* The collapse-arrow wrapper inherits DaisyUI's collapse border + arrow
     indicator. The preset list and custom-unit form live inside
     `.collapse-content` so the legacy `.dropdown-panel` / `.custom-panel`
     absolute-positioning hacks are no longer needed. The preset / custom
     dropdown classes stay so consumers can target specific overrides
     without having to re-derive the DaisyUI class list. */
  .preset-dropdown,
  .custom-form {
    flex: 1;
    min-width: 240px;
  }

  .dropdown-hint {
    font-size: 0.75rem;
    font-weight: 600;
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
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
    color: var(--color-base-content);
  }

  .dropdown-item:hover:not(:disabled) {
    background: var(--color-base-200);
  }

  .dropdown-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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
    color: var(--color-base-content);
  }
</style>
