<script lang="ts">
  import { isContextHigh } from '../../sessions/status';

  interface Props {
    percent: number | null;
  }

  let { percent }: Props = $props();

  const clampedPercent = $derived(percent === null ? 0 : Math.min(100, Math.max(0, percent)));
  const high = $derived(percent !== null && isContextHigh(percent));
</script>

<div class="meter" class:high title="Janela de contexto usada">
  <span class="caption">Contexto</span>
  <div
    class="track"
    role="meter"
    aria-label="Contexto usado"
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={percent ?? undefined}
  >
    <div class="fill" style:width="{clampedPercent}%"></div>
  </div>
  <span class="value mono">{percent === null ? '—' : `${Math.round(clampedPercent)}%`}</span>
</div>

<style>
  .meter {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 8px;
    font-size: 11.5px;
    color: var(--muted);
  }

  .track {
    height: 5px;
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: inherit;
    background: var(--muted);
    transition: width var(--duration-meter) var(--ease);
  }

  .high .fill {
    background: var(--amber);
  }

  .high .value {
    color: var(--amber);
  }
</style>
