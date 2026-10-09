<script lang="ts">
  import type { MatchSource, SessionListItem } from '../../api/types';
  import { formatRelativeTime } from '../../format';
  import Icon from '../common/Icon.svelte';
  import HighlightedSnippet from './HighlightedSnippet.svelte';

  interface Props {
    item: SessionListItem;
    selected: boolean;
    matchedIn?: MatchSource | null;
    snippet?: string | null;
    onSelect: (sessionId: string) => void;
    onRename: (sessionId: string, name: string | null) => void;
    onTogglePin: (sessionId: string) => void;
  }

  let { item, selected, matchedIn = null, snippet = null, onSelect, onRename, onTogglePin }: Props = $props();

  const MATCH_LABELS: Record<MatchSource, string> = {
    title: 'título',
    project: 'projeto',
    branch: 'branch',
    file: 'arquivo',
    tag: 'tag',
    text: 'conversa',
  };

  let editing = $state(false);
  let draftName = $state('');

  function startRename(): void {
    draftName = item.title;
    editing = true;
  }

  function commitRename(): void {
    if (!editing) return;
    editing = false;
    const trimmed = draftName.trim();
    if (trimmed !== item.title) onRename(item.id, trimmed === '' ? null : trimmed);
  }

  function handleRenameKey(event: KeyboardEvent): void {
    if (event.key === 'Enter') commitRename();
    if (event.key === 'Escape') {
      event.stopPropagation();
      editing = false;
    }
  }

  function focusOnMount(element: HTMLInputElement): void {
    element.focus();
    element.select();
  }
</script>

<div class="row" class:selected data-session-id={item.id}>
  {#if editing}
    <input
      class="rename"
      aria-label="Novo nome da sessão"
      bind:value={draftName}
      onkeydown={handleRenameKey}
      onblur={commitRename}
      use:focusOnMount
    />
  {:else}
    <button type="button" class="main" aria-current={selected} onclick={() => onSelect(item.id)} ondblclick={startRename}>
      <span class="title">{item.title}</span>
      <span class="meta">
        <span class="project">{item.project}</span>
        {#if item.branch}
          <span class="mono branch">{item.branch}</span>
        {/if}
        <span class="time">{formatRelativeTime(item.updatedAt)}</span>
      </span>
      {#if matchedIn}
        <span class="match">
          <span class="match-source">{MATCH_LABELS[matchedIn]}</span>
          {#if snippet}<HighlightedSnippet text={snippet} />{/if}
        </span>
      {/if}
    </button>
  {/if}
  <span class="actions">
    <button type="button" class="icon-button" aria-label="Renomear" title="Renomear" onclick={startRename}>
      <Icon name="pencil" size={14} />
    </button>
    <button
      type="button"
      class="icon-button pin"
      class:pinned={item.pinned}
      aria-label={item.pinned ? 'Desafixar' : 'Fixar'}
      aria-pressed={item.pinned}
      title={item.pinned ? 'Desafixar' : 'Fixar'}
      onclick={() => onTogglePin(item.id)}
    >
      <Icon name="pin" size={14} />
    </button>
  </span>
</div>

<style>
  .row {
    position: relative;
    display: flex;
    align-items: stretch;
    border-radius: 9px;
    transition: background-color 150ms var(--ease);
  }

  .row:hover {
    background: var(--surface-2);
  }

  .row.selected {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .main {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 2px;
    padding: 8px 10px;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .title {
    font-size: 13px;
    font-weight: 550;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    display: flex;
    gap: 8px;
    font-size: 11.5px;
    color: var(--muted);
    min-width: 0;
  }

  .project {
    flex-shrink: 0;
    white-space: nowrap;
  }

  .branch {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .time {
    margin-left: auto;
    flex-shrink: 0;
  }

  .match {
    display: flex;
    gap: 6px;
    font-size: 11.5px;
    color: var(--muted);
    min-width: 0;
  }

  .match-source {
    flex-shrink: 0;
    padding: 0 6px;
    border-radius: var(--radius-pill);
    background: var(--surface-2);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: 6px;
    opacity: 0;
    transition: opacity 150ms var(--ease);
  }

  .row:hover .actions,
  .row:focus-within .actions,
  .row.selected .actions {
    opacity: 1;
  }

  .actions:has(.pinned) {
    opacity: 1;
  }

  .icon-button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .icon-button:hover {
    background: var(--surface);
    color: var(--text);
  }

  .pin.pinned {
    color: var(--accent);
  }

  .rename {
    flex: 1;
    margin: 6px;
    padding: 5px 8px;
    border: 1px solid var(--accent);
    border-radius: 7px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 13px;
    user-select: text;
  }
</style>
