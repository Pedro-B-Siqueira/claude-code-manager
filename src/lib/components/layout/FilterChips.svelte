<script lang="ts">
  import { FILTER_LABELS, FILTER_ORDER, type FilterCounts, type SessionFilter } from '../../sessions/status';

  interface Props {
    active: SessionFilter;
    counts: FilterCounts;
    onSelect: (filter: SessionFilter) => void;
  }

  let { active, counts, onSelect }: Props = $props();
</script>

<div class="chips" role="radiogroup" aria-label="Filtrar sessões">
  {#each FILTER_ORDER as filter (filter)}
    <button
      type="button"
      class="chip"
      role="radio"
      aria-checked={active === filter}
      class:active={active === filter}
      onclick={() => onSelect(filter)}
    >
      {FILTER_LABELS[filter]}
      <span class="count mono">{counts[filter]}</span>
    </button>
  {/each}
</div>

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 11px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    cursor: pointer;
    transition:
      background-color 150ms var(--ease),
      color 150ms var(--ease),
      border-color 150ms var(--ease);
  }

  .chip:hover {
    color: var(--text);
  }

  .chip.active {
    border-color: var(--text);
    background: var(--text);
    color: var(--bg);
  }

  .count {
    font-size: 11px;
    opacity: 0.75;
  }
</style>
