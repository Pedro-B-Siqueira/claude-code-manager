<script lang="ts">
  import { onMount } from 'svelte';
  import FocusView from './lib/components/focus/FocusView.svelte';
  import FilterChips from './lib/components/layout/FilterChips.svelte';
  import Sidebar from './lib/components/layout/Sidebar.svelte';
  import TopBar from './lib/components/layout/TopBar.svelte';
  import ViewToggle from './lib/components/layout/ViewToggle.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { quitApp, takeNotifiedSession } from './lib/api/commands';
  import { onCloseRequested, onTrayFocusSession } from './lib/api/events';
  import type { LiveSessionView } from './lib/api/types';
  import ConfirmDialog from './lib/components/dialogs/ConfirmDialog.svelte';
  import NewSessionDialog from './lib/components/dialogs/NewSessionDialog.svelte';
  import ResumeModal from './lib/components/library/ResumeModal.svelte';
  import DiffPopover from './lib/components/sessions/DiffPopover.svelte';
  import Toasts from './lib/components/common/Toasts.svelte';
  import SessionGrid from './lib/components/sessions/SessionGrid.svelte';
  import { appInfoStore } from './lib/stores/app-info.svelte';
  import { libraryStore } from './lib/stores/library.svelte';
  import { liveSessionsStore } from './lib/stores/live.svelte';
  import { settingsStore } from './lib/stores/settings.svelte';
  import { uiStore } from './lib/stores/ui.svelte';

  onMount(() => {
    void settingsStore.load();
    void libraryStore.start();
    void liveSessionsStore.start();
    void appInfoStore.refresh();
    void onCloseRequested((runningSessions) => (uiStore.confirmation = { kind: 'quit', runningSessions }));
    void onTrayFocusSession((key) => uiStore.focusSession(key));
    void getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) void openNotifiedSession();
    });
    if (import.meta.env.DEV) void import('./lib/dev/scenario').then(({ runDevScenario }) => runDevScenario());
  });

  function openResume(sessionId: string | null = null): void {
    uiStore.resumeOpen = true;
    if (sessionId) void libraryStore.select(sessionId);
  }

  /** Clicking a notification activates the app; open the session it was about. */
  async function openNotifiedSession(): Promise<void> {
    const key = await takeNotifiedSession().catch(() => null);
    if (key && liveSessionsStore.find(key)) uiStore.focusSession(key);
  }

  async function resumeById(sessionId: string): Promise<void> {
    const session = await liveSessionsStore.resume(sessionId);
    if (!session) return;
    uiStore.resumeOpen = false;
    uiStore.focusSession(session.key);
  }

  function resumeSession(session: LiveSessionView): void {
    void liveSessionsStore.close(session.key).then(() => resumeById(session.sessionId));
  }

  function confirmPending(): void {
    const pending = uiStore.confirmation;
    uiStore.confirmation = null;
    if (pending?.kind === 'end-session') void liveSessionsStore.close(pending.session.key);
    if (pending?.kind === 'quit') void quitApp();
  }

  const visibleSessions = $derived(liveSessionsStore.filtered(uiStore.filter));
</script>

<div class="shell">
  <TopBar
    theme={settingsStore.current.theme}
    needsYouCount={liveSessionsStore.needsYouCount}
    onToggleTheme={() => void settingsStore.toggleTheme()}
    onShowNeedsYou={() => uiStore.showFilter('needsYou')}
  />

  <div class="workspace">
    <Sidebar
      pinned={libraryStore.pinned}
      projects={liveSessionsStore.projects}
      appInfo={appInfoStore.current}
      onNewSession={() => (uiStore.newSessionOpen = true)}
      onResume={() => openResume()}
      onOpenPinned={(sessionId) => openResume(sessionId)}
    />

    <main class="main">
      <div class="main-header">
        <h1>
          Sessões ativas
          <span class="total mono">{liveSessionsStore.sessions.length}</span>
        </h1>
        <ViewToggle mode={uiStore.viewMode} onChange={(mode) => (uiStore.viewMode = mode)} />
      </div>

      {#if uiStore.viewMode === 'grid'}
        <FilterChips
          active={uiStore.filter}
          counts={liveSessionsStore.counts}
          onSelect={(filter) => uiStore.showFilter(filter)}
        />
      {/if}

      {#key uiStore.viewMode}
        <div class="content view-enter">
          {#if uiStore.viewMode === 'grid'}
            <SessionGrid
              sessions={visibleSessions}
              onFocus={(key) => uiStore.focusSession(key)}
              onEnd={(session) => (uiStore.confirmation = { kind: 'end-session', session })}
              onResume={resumeSession}
              onRemove={(session) => void liveSessionsStore.close(session.key)}
            />
          {:else}
            <FocusView
              sessions={liveSessionsStore.sessions}
              activeKey={uiStore.focusedSessionKey}
              theme={settingsStore.current.theme}
              scrollback={settingsStore.current.scrollbackLines}
              onSelect={(key) => (uiStore.focusedSessionKey = key)}
              onResume={resumeSession}
              onNewSession={() => (uiStore.newSessionOpen = true)}
            />
          {/if}
        </div>
      {/key}
    </main>
  </div>
</div>

<DiffPopover />
<Toasts />

{#if uiStore.resumeOpen}
  <ResumeModal onClose={() => (uiStore.resumeOpen = false)} onResume={(sessionId) => void resumeById(sessionId)} />
{/if}

{#if uiStore.newSessionOpen}
  <NewSessionDialog
    onClose={() => (uiStore.newSessionOpen = false)}
    onOpened={(key) => {
      uiStore.newSessionOpen = false;
      uiStore.focusSession(key);
    }}
  />
{/if}

{#if uiStore.confirmation?.kind === 'end-session'}
  <ConfirmDialog
    title="Encerrar sessão?"
    message="O terminal de “{uiStore.confirmation.session.title}” será fechado. A conversa continua salva e pode ser retomada depois."
    confirmLabel="Encerrar"
    danger
    onConfirm={confirmPending}
    onCancel={() => (uiStore.confirmation = null)}
  />
{:else if uiStore.confirmation?.kind === 'quit'}
  <ConfirmDialog
    title="Sair do Claude Code Manager?"
    message={`${uiStore.confirmation.runningSessions === 1 ? 'Há 1 sessão aberta' : `Há ${uiStore.confirmation.runningSessions} sessões abertas`}. Os terminais serão encerrados; as conversas continuam salvas e podem ser retomadas depois.`}
    confirmLabel="Encerrar e sair"
    danger
    onConfirm={confirmPending}
    onCancel={() => (uiStore.confirmation = null)}
  />
{/if}

<style>
  .shell {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
  }

  .workspace {
    display: grid;
    grid-template-columns: 248px minmax(0, 1fr);
    min-height: 0;
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
    padding: 18px 20px 20px;
    overflow-y: auto;
  }

  .main-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  h1 {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 0;
    font-size: 18px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }

  .total {
    font-size: 13px;
    font-weight: 500;
    color: var(--muted);
  }

  .content {
    flex: 1;
    min-height: 0;
  }

  @media (max-width: 860px) {
    .workspace {
      grid-template-columns: 1fr;
      grid-template-rows: auto minmax(0, 1fr);
      overflow-y: auto;
    }

    .main {
      padding: 16px;
      overflow: visible;
    }
  }
</style>
