<script lang="ts">
  import { searchLibrary } from '../../api/commands';
  import type { LiveSessionView, SessionListItem } from '../../api/types';
  import { buildItems, GROUP_LABELS, rankItems, type PaletteAction, type PaletteItem } from '../../palette/search';
  import Icon from '../common/Icon.svelte';

  interface Props {
    actions: PaletteAction[];
    live: LiveSessionView[];
    history: SessionListItem[];
    onChoose: (item: PaletteItem) => void;
    onClose: () => void;
  }

  let { actions, live, history, onChoose, onClose }: Props = $props();

  const HISTORY_SEARCH_DEBOUNCE_MS = 140;

  let query = $state('');
  let active = $state(0);
  let textMatches = $state<SessionListItem[]>([]);
  let list: HTMLDivElement;

  const items = $derived(buildItems(actions, live, history));
  const results = $derived.by(() => {
    const ranked = rankItems(items, query);
    const shownIds = new Set(ranked.map((item) => item.id));
    const extra = textMatches
      .map((match): PaletteItem => ({
        id: `history:${match.id}`,
        group: 'history',
        label: match.title,
        detail: 'na conversa',
        keywords: '',
        target: { kind: 'history', sessionId: match.id },
      }))
      .filter((item) => !shownIds.has(item.id))
      .slice(0, 4);
    return [...ranked, ...extra];
  });

  $effect(() => {
    const current = query.trim();
    textMatches = [];
    if (current.length < 3) return;
    const timer = setTimeout(() => {
      void searchLibrary(current)
        .then((hits) => {
          const byId = new Map(history.map((item) => [item.id, item]));
          textMatches = hits.filter((hit) => hit.matchedIn === 'text').flatMap((hit) => byId.get(hit.sessionId) ?? []);
        })
        .catch(() => (textMatches = []));
    }, HISTORY_SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    void query;
    active = 0;
  });

  function choose(item: PaletteItem | undefined): void {
    if (item) onChoose(item);
  }

  function handleKey(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const step = event.key === 'ArrowDown' ? 1 : -1;
      active = (active + step + results.length) % Math.max(results.length, 1);
      list?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' });
    } else if (event.key === 'Enter') {
      event.preventDefault();
      choose(results[active]);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
    }
  }

  function focusOnMount(element: HTMLInputElement): void {
    element.focus();
  }

  function startsGroup(index: number): boolean {
    return index === 0 || results[index - 1]?.group !== results[index]?.group;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events (keys are handled by the input) -->
<div class="backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && onClose()}>
  <div class="palette" role="dialog" aria-modal="true" aria-label="Buscar">
    <label class="search">
      <Icon name="search" size={15} />
      <input
        placeholder="Buscar sessões, arquivos, branches e ações…"
        aria-label="Buscar"
        aria-controls="palette-results"
        aria-activedescendant={results[active] ? `palette-${active}` : undefined}
        bind:value={query}
        onkeydown={handleKey}
        use:focusOnMount
      />
      <kbd class="mono">esc</kbd>
    </label>
    <div class="results" id="palette-results" role="listbox" bind:this={list}>
      {#each results as item, index (item.id)}
        {#if startsGroup(index)}
          <p class="group">{GROUP_LABELS[item.group]}</p>
        {/if}
        <button
          type="button"
          id="palette-{index}"
          role="option"
          aria-selected={index === active}
          class="item"
          class:active={index === active}
          data-index={index}
          onmousemove={() => (active = index)}
          onclick={() => choose(item)}
        >
          <span class="label" class:mono={item.group === 'files' || item.group === 'branches'}>{item.label}</span>
          {#if item.detail}<span class="detail">{item.detail}</span>{/if}
        </button>
      {:else}
        <p class="empty">Nada encontrado para “{query}”.</p>
      {/each}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 12vh 16px 16px;
    background: rgb(0 0 0 / 0.32);
    animation: fade var(--duration-popover) var(--ease) both;
  }

  .palette {
    width: min(620px, 100%);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: 0 24px 64px rgb(0 0 0 / 0.38);
    overflow: hidden;
    animation: rise var(--duration-popover) var(--ease) both;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    height: 48px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
  }

  .search input {
    flex: 1;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 14px;
    user-select: text;
  }

  kbd {
    padding: 1px 6px;
    border-radius: 5px;
    background: var(--surface-2);
    font-size: 11px;
  }

  .results {
    overflow-y: auto;
    padding: 6px;
  }

  .group {
    margin: 8px 8px 4px;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .item {
    display: flex;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .item.active {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }

  .detail {
    margin-left: auto;
    flex-shrink: 0;
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
    color: var(--muted);
  }

  .empty {
    margin: 14px;
    font-size: 12.5px;
    color: var(--muted);
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.98);
    }
  }
</style>
