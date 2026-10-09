<script lang="ts">
  import type { LiveSessionView } from '../../api/types';
  import { formatCost, formatMemory, formatTokens } from '../../format';
  import Icon from '../common/Icon.svelte';
  import CardMenu, { type MenuAction } from './CardMenu.svelte';
  import ContextMeter from './ContextMeter.svelte';
  import FileChangeList from './FileChangeList.svelte';
  import StatusBadge from './StatusBadge.svelte';
  import TerminalPreview from './TerminalPreview.svelte';

  interface Props {
    session: LiveSessionView;
    onFocus: (key: string) => void;
    onEnd?: (session: LiveSessionView) => void;
    onResume?: (session: LiveSessionView) => void;
    onRemove?: (session: LiveSessionView) => void;
  }

  let { session, onFocus, onEnd, onResume, onRemove }: Props = $props();

  const menuActions = $derived.by((): MenuAction[] => {
    if (session.exited) {
      return [
        ...(onResume ? [{ id: 'resume', label: 'Retomar sessão', run: () => onResume(session) }] : []),
        ...(onRemove ? [{ id: 'remove', label: 'Remover da grade', run: () => onRemove(session) }] : []),
      ];
    }
    return onEnd ? [{ id: 'end', label: 'Encerrar sessão', danger: true, run: () => onEnd(session) }] : [];
  });
</script>

<article class="card" class:hibernated={session.hibernated} aria-label={session.title}>
  <header class="head">
    <StatusBadge status={session.status} hibernated={session.hibernated} />
    {#if session.origin === 'external'}
      <span class="origin" title="Sessão aberta fora do app">externa</span>
    {/if}
    {#if session.exited}
      <span class="origin" title="O processo terminou; a conversa continua retomável">encerrada</span>
    {/if}
    <span class="menu">
      {#if menuActions.length > 0}<CardMenu actions={menuActions} />{/if}
    </span>
  </header>

  <h3 class="title">{session.title}</h3>

  <div class="meta">
    <span class="repo">{session.repo}</span>
    {#if session.branch}
      <span class="branch mono"><Icon name="branch" size={12} />{session.branch}</span>
    {/if}
    {#if session.isWorktree}
      <span class="worktree" title="Worktree isolado"><Icon name="worktree" size={12} />worktree</span>
    {/if}
  </div>

  {#if session.status === 'permission' && session.statusDetail}
    <p class="permission mono" title="Comando pedido">$ {session.statusDetail}</p>
  {/if}

  <TerminalPreview lines={session.previewLines} />

  <FileChangeList files={session.files} />

  <ContextMeter percent={session.contextPercent} />

  <footer class="foot">
    <span class="stats mono">
      <span title="Tokens da sessão">{formatTokens(session.totalTokens)} tok</span>
      <span aria-hidden="true">·</span>
      <span title="Custo equivalente na API">{formatCost(session.costUsd)}</span>
      {#if session.memoryMb !== null}
        <span aria-hidden="true">·</span>
        <span class="memory" title="Memória do processo claude">
          <Icon name="memory" size={12} />{formatMemory(session.memoryMb)}
        </span>
      {/if}
    </span>
    <span class="actions">
      <button type="button" class="icon-button" aria-label="Abrir no VS Code" title="Abrir no VS Code">
        <Icon name="code" />
      </button>
      <button type="button" class="icon-button" aria-label="Abrir no Finder" title="Abrir no Finder">
        <Icon name="folder" />
      </button>
      <button type="button" class="focus-button" onclick={() => onFocus(session.key)}>
        <Icon name="focus" size={14} />Focar
      </button>
    </span>
  </footer>
</article>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 14px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    transition:
      transform 220ms var(--ease),
      box-shadow 220ms var(--ease),
      border-color 220ms var(--ease);
  }

  .card:hover,
  .card:focus-within {
    transform: translateY(-2px);
    box-shadow: var(--shadow-hover);
  }

  .card.hibernated {
    border-style: dashed;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .origin {
    font-size: 11px;
    color: var(--muted);
    padding: 1px 7px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
  }

  .menu {
    margin-left: auto;
  }

  .title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    font-size: 12px;
    color: var(--muted);
    min-width: 0;
  }

  .repo {
    color: var(--text);
    font-weight: 500;
  }

  .branch,
  .worktree {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .branch {
    font-size: 11.5px;
    max-width: 100%;
  }

  .worktree {
    padding: 0 7px;
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    font-size: 11px;
  }

  .permission {
    margin: 0;
    padding: 6px 9px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--amber) var(--status-badge-alpha), transparent);
    color: var(--amber);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding-top: 4px;
    border-top: 1px solid var(--border);
  }

  .stats {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--muted);
  }

  .memory {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .actions {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  .icon-button,
  .focus-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    height: 28px;
    border: 0;
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    transition:
      background-color 150ms var(--ease),
      color 150ms var(--ease);
  }

  .icon-button {
    width: 28px;
  }

  .focus-button {
    padding: 0 10px;
    color: var(--text);
    background: var(--surface-2);
    font-size: 12px;
    font-weight: 550;
  }

  .icon-button:hover,
  .focus-button:hover {
    background: var(--surface-2);
    color: var(--text);
  }
</style>
