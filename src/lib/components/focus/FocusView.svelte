<script lang="ts">
  import { fetchActivity } from '../../api/commands';
  import { onLibraryChanged } from '../../api/events';
  import { reportWarning, type FailureCause } from '../../api/logger';
  import type { ActivityItem, LiveSessionView, Theme } from '../../api/types';
  import { formatCost, formatMemory, formatTokens } from '../../format';
  import ActivityFeed from '../sessions/ActivityFeed.svelte';
  import Icon from '../common/Icon.svelte';
  import ContextMeter from '../sessions/ContextMeter.svelte';
  import FileChangeList from '../sessions/FileChangeList.svelte';
  import ResizeHandle from '../common/ResizeHandle.svelte';
  import { FOCUS_PANEL_LIMITS } from '../../stores/layout.svelte';
  import StatusBadge from '../sessions/StatusBadge.svelte';
  import XtermView from '../terminal/XtermView.svelte';

  interface Props {
    sessions: LiveSessionView[];
    activeKey: string | null;
    theme: Theme;
    scrollback: number;
    panelWidth: number;
    onPanelResize: (width: number) => void;
    onSelect: (key: string) => void;
    onResume: (session: LiveSessionView) => void;
    onWake: (session: LiveSessionView) => Promise<boolean>;
    onNewSession: () => void;
  }

  let { sessions, activeKey, theme, scrollback, panelWidth, onPanelResize, onSelect, onResume, onWake, onNewSession }: Props = $props();

  // Opening a hibernated session wakes it once; a failure waits for an explicit retry.
  let wakingKey = $state<string | null>(null);
  let wakeFailedKey = $state<string | null>(null);

  function wake(session: LiveSessionView): void {
    wakingKey = session.key;
    wakeFailedKey = null;
    void onWake(session).then((woke) => {
      if (!woke) wakeFailedKey = session.key;
    });
  }

  $effect(() => {
    const current = active;
    if (!current) return;
    if (current.hibernated && wakingKey !== current.key) wake(current);
    if (!current.hibernated && wakingKey === current.key) wakingKey = null;
  });

  const active = $derived(sessions.find((session) => session.key === activeKey) ?? sessions[0]);

  const ACTIVITY_LIMIT = 200;
  type PanelTab = 'summary' | 'activity';
  let panelTab = $state<PanelTab>('summary');
  let activity = $state<ActivityItem[]>([]);

  async function loadActivity(sessionId: string): Promise<void> {
    const items = await fetchActivity(sessionId, ACTIVITY_LIMIT).catch((cause: FailureCause) => {
      reportWarning('falha ao carregar a atividade', cause);
      return [];
    });
    if (active?.sessionId === sessionId) activity = items;
  }

  $effect(() => {
    const sessionId = active?.sessionId;
    if (panelTab !== 'activity' || !sessionId) return;
    void loadActivity(sessionId);
    const unlisten = onLibraryChanged(() => void loadActivity(sessionId));
    return () => void unlisten.then((stop) => stop());
  });
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
        {:else if active.hibernated && wakeFailedKey === active.key}
          <div class="ended">
            <p>Não foi possível acordar esta sessão. A conversa continua salva.</p>
            <button type="button" class="button-primary" onclick={() => wake(active)}>
              <Icon name="history" size={14} />Tentar de novo
            </button>
          </div>
        {:else if active.hibernated}
          <div class="ended">
            <p>Sessão hibernada para economizar memória. Acordando com <code class="mono">--resume</code>…</p>
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

      <ResizeHandle
        label="Largura do painel lateral"
        value={panelWidth}
        min={FOCUS_PANEL_LIMITS.min}
        max={FOCUS_PANEL_LIMITS.max}
        direction={-1}
        onResize={onPanelResize}
      />

      <aside class="panel">
        <div class="panel-tabs" role="tablist" aria-label="Painel da sessão">
          <button type="button" role="tab" aria-selected={panelTab === 'summary'} class:active={panelTab === 'summary'} onclick={() => (panelTab = 'summary')}>Resumo</button>
          <button type="button" role="tab" aria-selected={panelTab === 'activity'} class:active={panelTab === 'activity'} onclick={() => (panelTab = 'activity')}>Atividade</button>
        </div>
        {#if panelTab === 'summary'}
          <StatusBadge status={active.status} hibernated={active.hibernated} />
          {#if active.statusDetail}<p class="detail mono">{active.statusDetail}</p>{/if}
          <dl>
            <dt>Repositório</dt>
            <dd>{active.repo}</dd>
            <dt>Branch</dt>
            <dd class="mono">{active.branch ?? '—'}</dd>
            <dt>Pasta</dt>
            <dd class="mono">{active.cwd}</dd>
            <dt>Tokens</dt>
            <dd class="mono">{formatTokens(active.totalTokens)} · {formatCost(active.costUsd)}</dd>
            {#if active.memoryMb !== null}
              <dt>Memória</dt>
              <dd class="mono">{formatMemory(active.memoryMb)}</dd>
            {/if}
          </dl>
          <ContextMeter percent={active.contextPercent} />
          <h3>Arquivos</h3>
          <FileChangeList files={active.files} totalCount={active.filesTotal} sessionId={active.sessionId} basePath={active.cwd} visibleCount={12} />
        {:else}
          <ActivityFeed items={activity} basePath={active.cwd} />
        {/if}
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
    grid-template-columns: minmax(0, 1fr) 12px var(--focus-panel-width, 300px);
    flex: 1;
    min-height: 0;
    transition: grid-template-columns var(--duration-view) var(--ease);
  }

  .body > :global(.handle) {
    margin: 0 1px;
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

  .panel-tabs {
    display: flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-button);
    background: var(--bg);
  }

  .panel-tabs button {
    flex: 1;
    height: 26px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    cursor: pointer;
  }

  .panel-tabs button.active {
    background: var(--surface-2);
    color: var(--text);
    font-weight: 550;
  }

  .detail {
    margin: 0;
    padding: 6px 9px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--amber) var(--status-badge-alpha), transparent);
    color: var(--amber);
    font-size: 11.5px;
    overflow-wrap: anywhere;
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
      gap: 12px;
    }

    .body > :global(.handle) {
      display: none;
    }
  }
</style>
