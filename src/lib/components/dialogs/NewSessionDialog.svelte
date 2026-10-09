<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { fetchRecentDirs } from '../../api/commands';
  import { reportError, type FailureCause } from '../../api/logger';
  import { liveSessionsStore } from '../../stores/live.svelte';
  import Dialog from '../common/Dialog.svelte';
  import Icon from '../common/Icon.svelte';

  interface Props {
    onClose: () => void;
    onOpened: (key: string) => void;
  }

  let { onClose, onOpened }: Props = $props();

  let recentDirs = $state<string[]>([]);
  let selected = $state<string | null>(null);
  let opening = $state(false);

  onMount(() => {
    void fetchRecentDirs()
      .then((dirs) => {
        recentDirs = dirs;
        selected ??= dirs[0] ?? null;
      })
      .catch((cause: FailureCause) => reportError('falha ao listar pastas recentes', cause));
  });

  function folderName(path: string): string {
    return path.split('/').filter(Boolean).at(-1) ?? path;
  }

  async function chooseFolder(): Promise<void> {
    const chosen = await open({ directory: true, multiple: false, title: 'Escolha a pasta da sessão' });
    if (typeof chosen === 'string') {
      if (!recentDirs.includes(chosen)) recentDirs = [chosen, ...recentDirs];
      selected = chosen;
    }
  }

  async function start(): Promise<void> {
    if (!selected || opening) return;
    opening = true;
    const session = await liveSessionsStore.open(selected);
    opening = false;
    if (session) onOpened(session.key);
  }
</script>

<Dialog title="Nova sessão" width={560} {onClose}>
  <p class="label">Pasta</p>
  {#if recentDirs.length === 0}
    <p class="hint">Nenhuma pasta recente. Escolha uma pasta para começar.</p>
  {:else}
    <div class="folders" role="radiogroup" aria-label="Pastas recentes">
      {#each recentDirs as directory (directory)}
        <button
          type="button"
          role="radio"
          aria-checked={selected === directory}
          class="folder"
          class:selected={selected === directory}
          onclick={() => (selected = directory)}
          ondblclick={() => void start()}
        >
          <Icon name="folder" size={15} />
          <span class="folder-name">{folderName(directory)}</span>
          <span class="folder-path mono">{directory}</span>
        </button>
      {/each}
    </div>
  {/if}
  <button type="button" class="choose" onclick={() => void chooseFolder()}>
    <Icon name="plus" size={14} />Escolher outra pasta…
  </button>
  {#if liveSessionsStore.lastError}
    <p class="error" role="alert">{liveSessionsStore.lastError}</p>
  {/if}

  {#snippet footer()}
    <button type="button" class="button-secondary" onclick={onClose}>Cancelar</button>
    <button type="button" class="button-primary" disabled={!selected || opening} onclick={() => void start()}>
      <Icon name="terminal" size={14} />{opening ? 'Abrindo…' : 'Abrir sessão'}
    </button>
  {/snippet}
</Dialog>

<style>
  .label {
    margin: 0 0 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .hint {
    margin: 0 0 10px;
    font-size: 12.5px;
    color: var(--muted);
  }

  .folders {
    display: grid;
    gap: 2px;
    margin-bottom: 10px;
  }

  .folder {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border: 1px solid transparent;
    border-radius: 9px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .folder:hover {
    background: var(--surface-2);
  }

  .folder.selected {
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .folder-name {
    font-weight: 550;
  }

  .folder-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
    color: var(--muted);
    direction: rtl;
    text-align: left;
  }

  .choose {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-size: 12.5px;
    font-weight: 550;
    cursor: pointer;
  }

  .error {
    margin: 12px 0 0;
    padding: 8px 10px;
    border-radius: 9px;
    background: color-mix(in srgb, var(--diff-removed) var(--status-badge-alpha), transparent);
    color: var(--diff-removed);
    font-size: 12px;
  }
</style>
