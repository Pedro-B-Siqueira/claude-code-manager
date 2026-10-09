<script lang="ts">
  import type { LiveSessionView, Theme } from '../../api/types';
  import { formatCost, formatTokens } from '../../format';
  import Icon from '../common/Icon.svelte';
  import ContextMeter from '../sessions/ContextMeter.svelte';
  import FileChangeList from '../sessions/FileChangeList.svelte';
  import StatusBadge from '../sessions/StatusBadge.svelte';
  import XtermView from '../terminal/XtermView.svelte';

  interface Props {
    sessions: LiveSessionView[];
    activeKey: string | null;
    theme: Theme;
    scrollback: number;
    onSelect: (key: string) => void;
    onResume: (session: LiveSessionView) => void;
    onNewSession: () => void;
  }

  let { sessions, activeKey, theme, scrollback, onSelect, onResume, onNewSession }: Props = $props();

  const active = $derived(sessions.find((session) => session.key === activeKey) ?? sessions[0]);
</script>

{#if !active}
  <div class="empty">
    <Icon name="terminal" size={22} />
    <p>Nenhuma sessão aberta.</p>
    <button type="button" class="button-primary" onclick={onNewSession}><Icon name="plus" size={14} />Nova sessão</button>
  </div>
{:else}
  <div class="focus">
    <div class="tabs" role="tablist" aria-label="Sessões">
      {#each sessions as session (session.key)}
        <button
          type="button"
          role="tab"
          class="tab"
          aria-selected={session.key === active.key}
          class:active={session.key === active.key}
          title={session.title}
          onclick={() => onSelect(session.key)}
        >
          <span class="tab-dot" data-status={session.exited ? 'idle' : session.status} aria-hidden="true"></span>
          <span class="tab-label">{session.title}</span>
        </button>
      {/each}
    </div>

    <div class="body">
      <section class="terminal" aria-label="Terminal">
        {#if active.origin === 'external'}
          <div class="ended">
            <p>Esta sessão está aberta em outro terminal{active.pid ? ` (pid ${active.pid})` : ''}. O app só acompanha o andamento.</p>
            {#if active.previewLines.length > 0}
              <pre class="last-words mono">{active.previewLines.join('\n')}</pre>
            {/if}
          </div>
        {:else if active.exited}
          <div class="ended">
            <p>O terminal desta sessão foi encerrado. A conversa continua salva.</p>
            <button type="button" class="button-primary" onclick={() => onResume(active)}>
              <Icon name="history" size={14} />Retomar sessão
            </button>
          </div>
        {:else}
          {#key active.key}
            <XtermView sessionKey={active.key} {theme} {scrollback} />
          {/key}
        {/if}
      </section>

      <aside class="panel">
        <StatusBadge status={active.status} hibernated={active.hibernated} />
        <h3>Resumo</h3>
        <dl>
          <dt>Repositório</dt>
          <dd>{active.repo}</dd>
          <dt>Branch</dt>
          <dd class="mono">{active.branch ?? '—'}</dd>
          <dt>Pasta</dt>
          <dd class="mono">{active.cwd}</dd>
          <dt>Tokens</dt>
          <dd class="mono">{formatTokens(active.totalTokens)} · {formatCost(active.costUsd)}</dd>
        </dl>
        <ContextMeter percent={active.contextPercent} />
        <h3>Arquivos</h3>
        <FileChangeList files={active.files} visibleCount={8} />
      </aside>
    </div>
  </div>
{/if}

<style>
  .focus {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    height: 100%;
  }

  .empty {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 10px;
    height: 100%;
    min-height: 280px;
    color: var(--muted);
  }

  .empty p {
    margin: 0;
  }

  .tabs {
    display: flex;
    gap: 4px;
    overflow-x: auto;
    padding-bottom: 2px;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    max-width: 240px;
    height: 30px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .tab.active {
    background: var(--surface);
    color: var(--text);
  }

  .tab-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--muted);
    flex-shrink: 0;
  }

  .tab-dot[data-status='working'] {
    background: var(--accent);
  }
  .tab-dot[data-status='permission'] {
    background: var(--amber);
  }
  .tab-dot[data-status='waiting'] {
    background: var(--blue);
  }
  .tab-dot[data-status='done'] {
    background: var(--green);
  }

  .body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: 12px;
    flex: 1;
    min-height: 0;
  }

  .terminal {
    min-height: 320px;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--terminal-bg);
    overflow: hidden;
  }

  .ended {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 12px;
    height: 100%;
    padding: 24px;
    color: var(--muted);
    text-align: center;
  }

  .ended p {
    margin: 0;
  }

  .last-words {
    max-width: 640px;
    margin: 0;
    padding: 12px 14px;
    border-radius: 9px;
    background: var(--surface);
    color: var(--text);
    font-size: 12px;
    text-align: left;
    white-space: pre-wrap;
    user-select: text;
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    overflow-y: auto;
  }

  h3 {
    margin: 6px 0 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 10px;
    margin: 0;
    font-size: 12px;
  }

  dt {
    color: var(--muted);
  }

  dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  @media (max-width: 860px) {
    .body {
      grid-template-columns: 1fr;
    }
  }
</style>
