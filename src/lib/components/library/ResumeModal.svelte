<script lang="ts">
  import type { SessionListItem } from '../../api/types';
  import { openInVsCode } from '../../sessions/actions';
  import { libraryStore } from '../../stores/library.svelte';
  import Icon from '../common/Icon.svelte';
  import HistoryRow from './HistoryRow.svelte';
  import LibrarySummary from './LibrarySummary.svelte';

  interface Props {
    onClose: () => void;
    onResume: (sessionId: string) => void;
  }

  let { onClose, onResume }: Props = $props();

  interface ListSection {
    key: string;
    label: string;
    items: SessionListItem[];
  }

  const searching = $derived(libraryStore.query.trim().length > 0);

  const sections = $derived.by((): ListSection[] => {
    const groups = libraryStore.groups;
    return [
      { key: 'pinned', label: 'Fixadas', items: groups.pinned },
      { key: 'recent', label: 'Recentes', items: groups.recent },
      ...groups.byProject.map((group) => ({ key: `project:${group.project}`, label: group.project, items: group.sessions })),
    ].filter((section) => section.items.length > 0);
  });

  const results = $derived(
    libraryStore.hits.flatMap((hit) => {
      const item = libraryStore.itemsById.get(hit.sessionId);
      return item ? [{ hit, item }] : [];
    }),
  );

  const navigableIds = $derived(
    searching ? results.map((result) => result.item.id) : [...new Set(sections.flatMap((section) => section.items.map((item) => item.id)))],
  );

  function moveSelection(step: number): void {
    if (navigableIds.length === 0) return;
    const current = libraryStore.selectedId ? navigableIds.indexOf(libraryStore.selectedId) : -1;
    const next = navigableIds[Math.min(navigableIds.length - 1, Math.max(0, current + step))];
    if (next) {
      void libraryStore.select(next);
      document.querySelector(`[data-session-id="${next}"]`)?.scrollIntoView({ block: 'nearest' });
    }
  }

  function handleKey(event: KeyboardEvent): void {
    if (event.key === 'Escape') onClose();
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      moveSelection(1);
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      moveSelection(-1);
    }
  }

  function focusOnMount(element: HTMLInputElement): void {
    element.focus();
  }

  const rowHandlers = {
    onSelect: (sessionId: string) => void libraryStore.select(sessionId),
    onRename: (sessionId: string, name: string | null) => void libraryStore.rename(sessionId, name),
    onTogglePin: (sessionId: string) => void libraryStore.togglePin(sessionId),
  };
</script>

<svelte:window onkeydown={handleKey} />

<!-- svelte-ignore a11y_click_events_have_key_events (Escape is handled on the window) -->
<div class="backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && onClose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="resume-title">
    <header class="head">
      <h2 id="resume-title">Retomar sessão</h2>
      <button type="button" class="close" aria-label="Fechar" onclick={onClose}><Icon name="close" /></button>
    </header>

    <div class="body">
      <div class="list-pane">
        <label class="search">
          <Icon name="search" size={14} />
          <input
            type="search"
            placeholder="Buscar por título, conversa, arquivo, branch, tag…"
            aria-label="Buscar no histórico"
            value={libraryStore.query}
            oninput={(event) => libraryStore.setQuery(event.currentTarget.value)}
            use:focusOnMount
          />
        </label>

        {#if libraryStore.progress.running}
          <p class="progress" role="status">
            Indexando histórico… {libraryStore.progress.filesDone}/{libraryStore.progress.filesTotal}
          </p>
        {/if}

        <div class="list">
          {#if searching}
            {#if results.length === 0}
              <p class="empty">Nada encontrado para “{libraryStore.query}”.</p>
            {/if}
            {#each results as { hit, item } (item.id)}
              <HistoryRow
                {item}
                selected={item.id === libraryStore.selectedId}
                matchedIn={hit.matchedIn}
                snippet={hit.snippet}
                {...rowHandlers}
              />
            {/each}
          {:else if sections.length === 0}
            <p class="empty">Nenhuma sessão no histórico ainda.</p>
          {:else}
            {#each sections as section (section.key)}
              <h3 class="section-label">{section.label}</h3>
              {#each section.items as item (item.id)}
                <HistoryRow {item} selected={item.id === libraryStore.selectedId} {...rowHandlers} />
              {/each}
            {/each}
          {/if}
        </div>
      </div>

      <div class="detail-pane">
        {#if libraryStore.summary && libraryStore.summary.item.id === libraryStore.selectedId}
          {@const sessionId = libraryStore.summary.item.id}
          <div class="detail-scroll">
            <LibrarySummary
              summary={libraryStore.summary}
              knownTags={libraryStore.knownTags}
              knownCategories={libraryStore.knownCategories}
              onTagsChange={(tags) => void libraryStore.setTags(sessionId, tags)}
              onCategoryChange={(category) => void libraryStore.setCategory(sessionId, category)}
            />
          </div>
          <footer class="detail-actions">
            <button
              type="button"
              class="secondary"
              disabled={!libraryStore.summary.cwdExists}
              onclick={() => libraryStore.summary && void openInVsCode(libraryStore.summary.item.cwd ?? '', libraryStore.summary.item.branch)}
            >
              <Icon name="code" size={14} />Abrir no VS Code
            </button>
            <button
              type="button"
              class="primary"
              disabled={!libraryStore.summary.cwdExists}
              title={libraryStore.summary.cwdExists ? 'Abre claude --resume no terminal embutido' : 'A pasta original não existe mais'}
              onclick={() => onResume(sessionId)}
            >
              <Icon name="terminal" size={14} />Retomar no terminal
            </button>
          </footer>
        {:else}
          <div class="placeholder">
            <Icon name="history" size={22} />
            <p>Selecione uma sessão para ver o resumo.</p>
            <p class="hint">Use ↑ ↓ para navegar. Dois cliques no título renomeiam.</p>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgb(0 0 0 / 0.42);
    animation: fade-in var(--duration-popover) var(--ease) both;
  }

  .dialog {
    display: flex;
    flex-direction: column;
    width: min(1060px, 100%);
    height: min(700px, 100%);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--bg);
    box-shadow: 0 24px 64px rgb(0 0 0 / 0.35);
    overflow: hidden;
    animation: dialog-in var(--duration-popover) var(--ease) both;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px 12px 18px;
    border-bottom: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
  }

  .close {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .close:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .body {
    display: grid;
    grid-template-columns: minmax(300px, 400px) minmax(0, 1fr);
    flex: 1;
    min-height: 0;
  }

  .list-pane {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    padding: 12px;
    border-right: 1px solid var(--border);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    height: 34px;
    border: 1px solid var(--border);
    border-radius: var(--radius-button);
    background: var(--surface);
    color: var(--muted);
  }

  .search:focus-within {
    border-color: var(--accent);
  }

  .search input {
    flex: 1;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    user-select: text;
  }

  .progress {
    margin: 0;
    font-size: 11.5px;
    color: var(--muted);
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 1px;
  }

  .section-label {
    margin: 10px 10px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .section-label:first-child {
    margin-top: 2px;
  }

  .empty {
    margin: 16px 10px;
    font-size: 12.5px;
    color: var(--muted);
  }

  .detail-pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--surface);
  }

  .detail-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 18px 20px;
  }

  .detail-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }

  .primary,
  .secondary {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 34px;
    padding: 0 14px;
    border-radius: var(--radius-button);
    font-weight: 550;
    cursor: pointer;
  }

  .primary {
    border: 0;
    background: var(--accent);
    color: var(--accent-ink);
  }

  .secondary {
    border: 1px solid var(--border);
    background: var(--surface);
  }

  .primary:disabled,
  .secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .placeholder {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 6px;
    color: var(--muted);
    text-align: center;
  }

  .placeholder p {
    margin: 0;
  }

  .hint {
    font-size: 12px;
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }

  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
  }

  @media (max-width: 760px) {
    .body {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(0, 1fr) minmax(0, 1fr);
    }

    .list-pane {
      border-right: 0;
      border-bottom: 1px solid var(--border);
    }
  }
</style>
