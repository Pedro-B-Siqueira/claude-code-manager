<script lang="ts">
  import Icon from '../../lib/components/common/Icon.svelte';
  import type { SceneSummary } from '../data/summary';
  import StatusTag from './StatusTag.svelte';

  interface Props {
    summary: SceneSummary;
    /** Repouso exists only where the app can lock the screen (macOS). */
    canLock: boolean;
    onLock: () => void;
    onExit: () => void;
  }

  let { summary, canLock, onLock, onExit }: Props = $props();
</script>

<header class="hud">
  <div class="overview">
    <div>
      <h1>Modo Jarvis</h1>
      <p class="headline">{summary.headline}</p>
    </div>
    {#if summary.tags.length > 0}
      <ul class="tags" aria-label="Sessões por status">
        {#each summary.tags as tag (tag.status)}
          <li><StatusTag status={tag.status} label={tag.label} /></li>
        {/each}
      </ul>
    {/if}
  </div>
  <div class="actions">
    {#if canLock}
      <button type="button" class="button-secondary compact" onclick={onLock}><Icon name="lock" size={14} />Repouso</button>
    {/if}
    <button type="button" class="ghost" onclick={onExit}><Icon name="exit" size={14} />Sair</button>
    <kbd>Esc</kbd>
  </div>
</header>

<style>
  .hud {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 24px;
    pointer-events: none;
  }

  .overview,
  .actions {
    display: flex;
    align-items: center;
    gap: 18px;
    pointer-events: auto;
  }

  .overview {
    align-items: flex-start;
  }

  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }

  .headline {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 12.5px;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 2px 0 0;
    padding: 0;
    list-style: none;
  }

  .actions {
    gap: 10px;
  }

  .compact {
    height: 32px;
  }

  .ghost {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-button);
    background: transparent;
    font-weight: 550;
    cursor: pointer;
  }

  .ghost:hover {
    background: var(--surface-2);
  }

  kbd {
    padding: 2px 7px;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: 11.5px;
  }
</style>
