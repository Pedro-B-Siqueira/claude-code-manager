<script lang="ts">
  import type { SessionListItem } from '../../api/types';
  import type { ProjectGroup } from '../../stores/live.svelte';
  import Icon from '../common/Icon.svelte';

  interface Props {
    pinned: SessionListItem[];
    projects: ProjectGroup[];
    claudeGaugeDetected: boolean;
    onResume: () => void;
    onOpenPinned: (sessionId: string) => void;
  }

  let { pinned, projects, claudeGaugeDetected, onResume, onOpenPinned }: Props = $props();
</script>

<aside class="sidebar">
  <div class="primary-actions">
    <button type="button" class="new-session">
      <Icon name="plus" />Nova sessão
    </button>
    <button type="button" class="resume-session" onclick={onResume}>
      <Icon name="history" />Retomar sessão
    </button>
  </div>

  <nav class="sections" aria-label="Sessões">
    <section>
      <h2>Fixadas</h2>
      {#if pinned.length === 0}
        <p class="hint">Fixe sessões para acesso rápido.</p>
      {:else}
        <ul>
          {#each pinned as session (session.id)}
            <li>
              <button type="button" class="row" title={session.title} onclick={() => onOpenPinned(session.id)}>
                <Icon name="pin" size={13} />
                <span class="row-label">{session.title}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section>
      <h2>Projetos</h2>
      <ul>
        {#each projects as project (project.repo)}
          <li class="row project">
            <span class="row-label">{project.repo}</span>
            <span class="count mono" title="Sessões ativas">{project.activeCount}</span>
          </li>
        {/each}
      </ul>
    </section>
  </nav>

  <footer class="footer">
    <p class="footer-line">
      <Icon name="gauge" size={13} />
      ClaudeGauge: {claudeGaugeDetected ? 'detectado' : 'não detectado'}
    </p>
    <p class="footer-line">
      <Icon name="shield" size={13} />
      <span>Hooks via <code class="mono">--settings</code> · config global intocada</span>
    </p>
  </footer>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 18px;
    min-height: 0;
    padding: 14px 12px 12px;
    border-right: 1px solid var(--border);
    background: var(--bg);
    overflow-y: auto;
  }

  .primary-actions {
    display: grid;
    gap: 6px;
  }

  .new-session,
  .resume-session {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 12px;
    border-radius: var(--radius-button);
    font-weight: 550;
    cursor: pointer;
    transition:
      filter 150ms var(--ease),
      background-color 150ms var(--ease);
  }

  .new-session {
    border: 0;
    background: var(--accent);
    color: var(--accent-ink);
  }

  .new-session:hover {
    filter: brightness(1.06);
  }

  .resume-session {
    border: 1px solid var(--border);
    background: var(--surface);
  }

  .resume-session:hover {
    background: var(--surface-2);
  }

  .sections {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 18px;
    align-content: start;
    flex: 1;
  }

  h2 {
    margin: 0 0 6px;
    padding: 0 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 1px;
  }

  section {
    min-width: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-height: 28px;
    padding: 0 8px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    font-size: 12.5px;
    text-align: left;
  }

  button.row {
    cursor: pointer;
  }

  button.row:hover {
    background: var(--surface-2);
  }

  .row-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    font-size: 11px;
    color: var(--muted);
  }

  .hint {
    margin: 0;
    padding: 0 8px;
    font-size: 12px;
    color: var(--muted);
  }

  .footer {
    display: grid;
    gap: 4px;
    padding: 10px 8px 0;
    border-top: 1px solid var(--border);
  }

  .footer-line {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 11px;
    color: var(--muted);
  }

  code {
    font-size: 10.5px;
  }

  @media (max-width: 860px) {
    .sidebar {
      border-right: 0;
      border-bottom: 1px solid var(--border);
      overflow: visible;
    }
  }
</style>
