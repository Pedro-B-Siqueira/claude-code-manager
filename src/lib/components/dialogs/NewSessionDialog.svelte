<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { createWorktree, fetchRecentDirs } from '../../api/commands';
  import { ltrPath } from '../../format';
  import { describeFailure, reportError, type FailureCause } from '../../api/logger';
  import type { WorktreePlan } from '../../api/types';
  import { liveSessionsStore } from '../../stores/live.svelte';
  import { settingsStore } from '../../stores/settings.svelte';
  import Dialog from '../common/Dialog.svelte';
  import Icon from '../common/Icon.svelte';
  import WorktreeSection from './WorktreeSection.svelte';

  interface Props {
    onClose: () => void;
    onOpened: (key: string) => void;
  }

  let { onClose, onOpened }: Props = $props();

  let recentDirs = $state<string[]>([]);
  let selected = $state<string | null>(null);
  let opening = $state(false);
  let error = $state<string | null>(null);
  let useWorktree = $state(false);
  let branchPrefix = $state(settingsStore.current.branchPrefixes[0] ?? 'feat-');
  let branchName = $state('');
  let plan = $state<WorktreePlan | null>(null);
  let confirming = $state(false);

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

  function select(path: string): void {
    if (!recentDirs.includes(path)) recentDirs = [path, ...recentDirs];
    selected = path;
    confirming = false;
  }

  async function chooseFolder(): Promise<void> {
    const chosen = await open({ directory: true, multiple: false, title: 'Escolha a pasta da sessão' });
    if (typeof chosen === 'string') select(chosen);
  }

  async function openPlain(cwd: string): Promise<void> {
    const session = await liveSessionsStore.open(cwd);
    if (session) onOpened(session.key);
    else error = liveSessionsStore.lastError;
  }

  async function openInNewWorktree(cwd: string): Promise<void> {
    const session = await createWorktree(cwd, branchPrefix, branchName).catch((cause: FailureCause) => {
      error = `Não foi possível criar o worktree: ${describeFailure(cause)}`;
      return null;
    });
    if (!session) return;
    await liveSessionsStore.reload();
    onOpened(session.key);
  }

  async function start(): Promise<void> {
    if (!selected || opening) return;
    if (useWorktree && !confirming) {
      if (plan) confirming = true;
      return;
    }
    opening = true;
    error = null;
    await (useWorktree ? openInNewWorktree(selected) : openPlain(selected));
    opening = false;
  }
</script>

<Dialog title="Nova sessão" width={600} {onClose}>
  {#if confirming && plan}
    <div class="confirm">
      <p>Confirme a criação do worktree. Este é o único comando que altera o repositório:</p>
      <pre class="mono">git -C {plan.repoRoot} worktree add -b {plan.branch} {plan.path} {plan.base}</pre>
      <p class="hint">Nenhum checkout, reset ou stash é feito no repositório principal. Depois, a sessão abre dentro do worktree.</p>
    </div>
  {:else}
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
            onclick={() => select(directory)}
            ondblclick={() => void start()}
          >
            <Icon name="folder" size={15} />
            <span class="folder-name">{folderName(directory)}</span>
            <span class="folder-path mono">{ltrPath(directory)}</span>
          </button>
        {/each}
      </div>
    {/if}
    <button type="button" class="choose" onclick={() => void chooseFolder()}>
      <Icon name="plus" size={14} />Escolher outra pasta…
    </button>

    {#if selected}
      <WorktreeSection
        cwd={selected}
        prefixes={settingsStore.current.branchPrefixes}
        bind:enabled={useWorktree}
        bind:prefix={branchPrefix}
        bind:name={branchName}
        onPlan={(next) => (plan = next)}
        onSelectFolder={(path) => {
          useWorktree = false;
          select(path);
        }}
      />
    {/if}
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#snippet footer()}
    {#if confirming}
      <button type="button" class="button-secondary" onclick={() => (confirming = false)}>Voltar</button>
      <button type="button" class="button-primary" disabled={opening} onclick={() => void start()}>
        <Icon name="worktree" size={14} />{opening ? 'Criando…' : 'Criar worktree e abrir'}
      </button>
    {:else}
      <button type="button" class="button-secondary" onclick={onClose}>Cancelar</button>
      <button type="button" class="button-primary" disabled={!selected || opening || (useWorktree && !plan)} onclick={() => void start()}>
        <Icon name="terminal" size={14} />{opening ? 'Abrindo…' : useWorktree ? 'Continuar' : 'Abrir sessão'}
      </button>
    {/if}
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
    font-size: 12px;
    color: var(--muted);
  }

  .folders {
    display: grid;
    gap: 2px;
    margin-bottom: 10px;
    max-height: 240px;
    overflow-y: auto;
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

  .confirm p {
    margin: 0 0 10px;
    font-size: 13px;
  }

  .confirm pre {
    margin: 0 0 10px;
    padding: 10px 12px;
    border-radius: 9px;
    background: var(--code-bg);
    color: var(--code-text);
    font-size: 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
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
