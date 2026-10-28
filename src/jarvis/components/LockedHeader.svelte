<script lang="ts">
  import Icon from '../../lib/components/common/Icon.svelte';
  import { formatClock, type LockedSummary } from '../data/lock-text';

  interface Props {
    since: number;
    summary: LockedSummary;
  }

  let { since, summary }: Props = $props();
</script>

<header class="locked">
  <div class="title">
    <span class="badge"><Icon name="lock" size={16} /></span>
    <div>
      <h1>Repouso ativo</h1>
      <p>desde {formatClock(new Date(since))}</p>
    </div>
  </div>
  <div class="summary">
    <span>{summary.sessions}</span>
    {#if summary.waiting}<span class="waiting">{summary.waiting}</span>{/if}
  </div>
</header>

<style>
  .locked {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 18px 24px;
  }

  .title {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .badge {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--jarvis-structure) 22%, transparent);
    color: var(--jarvis-structure);
  }

  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
  }

  p {
    margin: 1px 0 0;
    color: var(--muted);
    font-size: 12px;
  }

  .summary {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    font-size: 12.5px;
  }

  .waiting {
    padding: 2px 8px;
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--jarvis-waiting) 16%, transparent);
    color: var(--jarvis-waiting);
    font-weight: 550;
  }
</style>
