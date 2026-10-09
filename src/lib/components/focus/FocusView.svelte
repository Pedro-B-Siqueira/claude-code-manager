<script lang="ts">
  import type { LiveSessionView } from '../../api/types';
  import StatusBadge from '../sessions/StatusBadge.svelte';
  import FileChangeList from '../sessions/FileChangeList.svelte';
  import ContextMeter from '../sessions/ContextMeter.svelte';

  interface Props {
    sessions: LiveSessionView[];
    activeKey: string | null;
    onSelect: (key: string) => void;
  }

  let { sessions, activeKey, onSelect }: Props = $props();

  const active = $derived(sessions.find((session) => session.key === activeKey) ?? sessions[0]);
</script>

<div class="focus">
  <div class="tabs" role="tablist" aria-label="Sessões">
    {#each sessions as session (session.key)}
      <button
        type="button"
        role="tab"
        class="tab"
        aria-selected={session.key === active?.key}
        class:active={session.key === active?.key}
        onclick={() => onSelect(session.key)}
      >
        <span class="tab-dot" data-status={session.status} aria-hidden="true"></span>
        <span class="tab-label">{session.title}</span>
      </button>
    {/each}
  </div>

  {#if active}
    <div class="body">
      <section class="terminal mono" aria-label="Terminal">
        {#each active.previewLines as line, index (index)}
          <div>{line || ' '}</div>
        {/each}
        <p class="placeholder">Terminal real chega na etapa 3.</p>
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
        </dl>
        <ContextMeter percent={active.contextPercent} />
        <h3>Arquivos</h3>
        <FileChangeList files={active.files} visibleCount={8} />
        <h3>Atividade</h3>
        <p class="hint">Feed de atividade chega na etapa 5.</p>
      </aside>
    </div>
  {/if}
</div>

<style>
  .focus {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    height: 100%;
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
    padding: 14px 16px;
    border-radius: var(--radius-card);
    background: var(--terminal-bg);
    color: var(--terminal-text);
    font-size: 12.5px;
    line-height: 1.55;
    overflow: auto;
    border: 1px solid var(--border);
  }

  .placeholder {
    margin-top: 18px;
    color: var(--muted);
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

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  @media (max-width: 860px) {
    .body {
      grid-template-columns: 1fr;
    }
  }
</style>
