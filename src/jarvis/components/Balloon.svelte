<script lang="ts">
  import { STATUS_LABELS, type JarvisSession } from '../data/jarvis-session';
  import { clipAction } from '../data/launch';
  import StatusTag from './StatusTag.svelte';

  interface Props {
    session: JarvisSession;
    elapsed: string;
    selected: boolean;
    /** False in Repouso: the balloon only reads. */
    interactive: boolean;
    showAction: boolean;
    /** Loaded when the balloon expands; null while loading. */
    recentEvents: string[] | null;
    onToggle: () => void;
    onOpen: () => void;
  }

  let { session, elapsed, selected, interactive, showAction, recentEvents, onToggle, onOpen }: Props = $props();
</script>

<article class="balloon" class:selected>
  {#if interactive}
    <button type="button" class="head" aria-expanded={selected} onclick={onToggle}>
      <span class="name">{session.name}</span>
      <span class="time mono">{elapsed}</span>
    </button>
  {:else}
    <div class="head">
      <span class="name">{session.name}</span>
      <span class="time mono">{elapsed}</span>
    </div>
  {/if}
  <div class="meta">
    <StatusTag status={session.status} label={STATUS_LABELS[session.status]} />
    <span class="repo mono">{session.repo}</span>
  </div>
  {#if showAction && session.action}
    <p class="action mono">{clipAction(session.action)}</p>
  {/if}
  {#if selected && interactive}
    <div class="details">
      {#if session.branch}<span class="branch mono">{session.branch}</span>{/if}
      {#each recentEvents ?? [] as event, index (index)}
        <span class="event">{event}</span>
      {/each}
      <div class="open">
        <button type="button" class="button-secondary small" onclick={onOpen}>Abrir sessão</button>
      </div>
    </div>
  {/if}
</article>

<style>
  .balloon {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 272px;
    padding: 12px 14px;
    border: 1px solid var(--jarvis-balloon-border);
    border-radius: 12px;
    background: var(--jarvis-balloon-bg);
    box-shadow: 0 10px 28px rgb(0 0 0 / 0.38);
    color: var(--jarvis-text);
  }

  .balloon.selected {
    border-color: var(--jarvis-selected);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
  }

  button.head {
    cursor: pointer;
  }

  .name {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    font-size: 13.5px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .time {
    flex: none;
    color: var(--jarvis-muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .repo,
  .branch {
    min-width: 0;
    overflow: hidden;
    color: var(--jarvis-muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .action {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }

  .details {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 8px;
    border-top: 1px solid var(--jarvis-balloon-border);
  }

  .event {
    color: var(--jarvis-muted);
    font-size: 11.5px;
    line-height: 1.5;
  }

  .open {
    display: flex;
    justify-content: flex-end;
    padding-top: 4px;
  }

  .small {
    height: 28px;
    padding: 0 10px;
    font-size: 12px;
  }
</style>
