<script lang="ts">
  import type { LiveSessionView } from '../../api/types';
  import SessionCard from './SessionCard.svelte';

  interface Props {
    sessions: LiveSessionView[];
    onFocus: (key: string) => void;
  }

  let { sessions, onFocus }: Props = $props();
</script>

{#if sessions.length === 0}
  <div class="empty">
    <p class="empty-title">Nenhuma sessão neste filtro</p>
    <p class="empty-hint">Use “Nova sessão” ou “Retomar sessão” na barra lateral.</p>
  </div>
{:else}
  <div class="grid">
    {#each sessions as session (session.key)}
      <SessionCard {session} {onFocus} />
    {/each}
  </div>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 14px;
    align-content: start;
  }

  .empty {
    display: grid;
    place-items: center;
    gap: 4px;
    padding: 64px 16px;
    border: 1px dashed var(--border);
    border-radius: var(--radius-card);
    color: var(--muted);
    text-align: center;
  }

  .empty-title {
    margin: 0;
    color: var(--text);
    font-weight: 550;
  }

  .empty-hint {
    margin: 0;
    font-size: 12px;
  }
</style>
