<script lang="ts">
  import type { ActivityItem } from '../../api/types';
  import { activityLabel, activityTime } from '../../activity/labels';
  import { relativePath } from '../../format';
  import { diffHover } from '../../stores/diff-popover.svelte';

  interface Props {
    items: ActivityItem[];
    basePath: string | null;
  }

  let { items, basePath }: Props = $props();
</script>

{#if items.length === 0}
  <p class="empty">Nenhuma atividade registrada ainda.</p>
{:else}
  <ol class="feed">
    {#each items as item (item.id)}
      <li>
        <button
          type="button"
          class="row"
          data-kind={item.kind}
          use:diffHover={item.editId !== null ? { kind: 'edit', editId: item.editId, basePath } : null}
        >
          <span class="time mono">{activityTime(item.timestamp)}</span>
          <span class="action">{activityLabel(item)}</span>
          <span class="target" class:mono={item.kind !== 'prompt'} title={item.target ?? ''}>
            {item.target ? (item.kind === 'prompt' ? item.target : relativePath(item.target, basePath)) : '—'}
          </span>
          {#if item.fromSubagent}<span class="badge">subagente</span>{/if}
        </button>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .feed {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 1px;
  }

  .row {
    display: grid;
    grid-template-columns: 40px 64px minmax(0, 1fr) auto;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    padding: 4px 6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    font-size: 12px;
    text-align: left;
    cursor: default;
  }

  .row:hover,
  .row:focus-visible {
    background: var(--surface-2);
  }

  .time {
    font-size: 11px;
    color: var(--muted);
  }

  .action {
    font-weight: 550;
  }

  .row[data-kind='edit'] .action {
    color: var(--green);
  }

  .row[data-kind='prompt'] .action {
    color: var(--accent);
  }

  .target {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 11.5px;
  }

  .badge {
    padding: 0 6px;
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    font-size: 10.5px;
    color: var(--muted);
  }

  .empty {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
</style>
