<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import { quitApp, setUiVisible, takeNotifiedSession } from './lib/api/commands';
  import { onCloseRequested, onTrayFocusSession } from './lib/api/events';
  import type { AppSettings, LiveSessionView } from './lib/api/types';
  import { PALETTE_ACTIONS } from './lib/app/palette-actions';
  import { handlePaletteChoice } from './lib/app/palette-choice';
  import { shortcutFor } from './lib/app/shortcuts';
  import ResizeHandle from './lib/components/common/ResizeHandle.svelte';
  import Toasts from './lib/components/common/Toasts.svelte';
  import ConfirmDialog from './lib/components/dialogs/ConfirmDialog.svelte';
  import NewSessionDialog from './lib/components/dialogs/NewSessionDialog.svelte';
  import SettingsDialog from './lib/components/dialogs/SettingsDialog.svelte';
  import FocusView from './lib/components/focus/FocusView.svelte';
  import FilterChips from './lib/components/layout/FilterChips.svelte';
  import Sidebar from './lib/components/layout/Sidebar.svelte';
  import TopBar from './lib/components/layout/TopBar.svelte';
  import ViewToggle from './lib/components/layout/ViewToggle.svelte';
  import ResumeModal from './lib/components/library/ResumeModal.svelte';
  import CommandPalette from './lib/components/palette/CommandPalette.svelte';
  import DiffPopover from './lib/components/sessions/DiffPopover.svelte';
  import SessionGrid from './lib/components/sessions/SessionGrid.svelte';
  import type { PaletteItem } from './lib/palette/search';
  import { matchesFilter } from './lib/sessions/status';
  import { appInfoStore } from './lib/stores/app-info.svelte';
  import { layoutStore, orderSessions, SIDEBAR_LIMITS } from './lib/stores/layout.svelte';
  import { libraryStore } from './lib/stores/library.svelte';
  import { liveSessionsStore } from './lib/stores/live.svelte';
  import { settingsStore } from './lib/stores/settings.svelte';
  import { uiStore } from './lib/stores/ui.svelte';

  onMount(() => {
    void settingsStore.load();
    void layoutStore.load();
    void libraryStore.start();
    void liveSessionsStore.start();
    void appInfoStore.refresh();
    void onCloseRequested((runningSessions) => (uiStore.confirmation = { kind: 'quit', runningSessions }));
    void onTrayFocusSession((key) => uiStore.focusSession(key));
    void getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) void openNotifiedSession();
    });
    document.addEventListener('visibilitychange', () => void setUiVisible(!document.hidden).catch(() => undefined));
    if (import.meta.env.DEV) void import('./lib/dev/scenario').then(({ runDevScenario }) => runDevScenario());
  });

  async function wakeSession(session: LiveSessionView): Promise<boolean> {
    return (await liveSessionsStore.wake(session.key)) !== null;
  }

  const orderedSessions = $derived(orderSessions(liveSessionsStore.sessions, layoutStore.gridOrder));
  const visibleSessions = $derived(orderedSessions.filter((session) => matchesFilter(session, uiStore.filter)));

  function openResume(sessionId: string | null = null, query: string | null = null): void {
    uiStore.resumeOpen = true;
    if (query !== null) libraryStore.setQuery(query);
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

  function runAction(actionId: string): void {
    const actions: Record<string, () => void> = {
      'new-session': () => (uiStore.newSessionOpen = true),
      resume: () => openResume(),
      'needs-you': () => uiStore.showFilter('needsYou'),
      grid: () => (uiStore.viewMode = 'grid'),
      focus: () => (uiStore.viewMode = 'focus'),
      theme: () => void settingsStore.toggleTheme(),
      settings: () => (uiStore.settingsOpen = true),
    };
    actions[actionId]?.();
  }

  function choosePaletteItem(item: PaletteItem): void {
    uiStore.paletteOpen = false;
    handlePaletteChoice(item, {
      runAction,
      focusLive: (key) => uiStore.focusSession(key),
      openHistory: (sessionId, query) => openResume(sessionId || null, query),
    });
  }

  function handleShortcut(event: KeyboardEvent): void {
    const command = shortcutFor(event);
    if (!command) return;
    event.preventDefault();
    event.stopPropagation();
    if (command.kind === 'palette') uiStore.paletteOpen = !uiStore.paletteOpen;
    if (command.kind === 'new-session') uiStore.newSessionOpen = true;
    if (command.kind === 'settings') uiStore.settingsOpen = true;
    if (command.kind === 'session-index') {
      const session = orderedSessions[command.index];
      if (session) uiStore.focusSession(session.key);
    }
  }

  function saveSettings(next: AppSettings): void {
    uiStore.settingsOpen = false;
    void settingsStore.save(next).then(() => appInfoStore.refresh());
  }
</script>

<svelte:window onkeydowncapture={handleShortcut} />

<div class="shell" style:--sidebar-width="{layoutStore.panels.sidebarWidth}px" style:--focus-panel-width="{layoutStore.panels.focusPanelWidth}px">
  <TopBar
    theme={settingsStore.current.theme}
    needsYouCount={liveSessionsStore.needsYouCount}
    onToggleTheme={() => void settingsStore.toggleTheme()}
    onShowNeedsYou={() => uiStore.showFilter('needsYou')}
    onOpenPalette={() => (uiStore.paletteOpen = true)}
    onOpenSettings={() => (uiStore.settingsOpen = true)}
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
    <div class="sidebar-handle">
      <ResizeHandle
        label="Largura da barra lateral"
        value={layoutStore.panels.sidebarWidth}
        min={SIDEBAR_LIMITS.min}
        max={SIDEBAR_LIMITS.max}
        direction={1}
        onResize={(sidebarWidth) => layoutStore.resize({ sidebarWidth })}
      />
    </div>

    <main class="main">
      <div class="main-header">
        <h1>
          Sessões ativas
          <span class="total mono">{liveSessionsStore.sessions.length}</span>
        </h1>
        <ViewToggle mode={uiStore.viewMode} onChange={(mode) => (uiStore.viewMode = mode)} />
      </div>

      {#if uiStore.viewMode === 'grid'}
        <FilterChips active={uiStore.filter} counts={liveSessionsStore.counts} onSelect={(filter) => uiStore.showFilter(filter)} />
      {/if}

      {#key uiStore.viewMode}
        <div class="content view-enter">
          {#if uiStore.viewMode === 'grid'}
            <SessionGrid
              sessions={visibleSessions}
              reorderable={uiStore.filter === 'all'}
              onReorder={(sessionIds) => layoutStore.reorder(sessionIds)}
              onFocus={(key) => uiStore.focusSession(key)}
              onEnd={(session) => (uiStore.confirmation = { kind: 'end-session', session })}
              onResume={resumeSession}
              onRemove={(session) => void liveSessionsStore.close(session.key)}
              onWake={wakeSession}
            />
          {:else}
            <FocusView
              sessions={orderedSessions}
              activeKey={uiStore.focusedSessionKey}
              scrollback={settingsStore.current.scrollbackLines}
              panelWidth={layoutStore.panels.focusPanelWidth}
              onPanelResize={(focusPanelWidth) => layoutStore.resize({ focusPanelWidth })}
              onSelect={(key) => (uiStore.focusedSessionKey = key)}
              onResume={resumeSession}
              onWake={wakeSession}
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

{#if uiStore.paletteOpen}
  <CommandPalette
    actions={PALETTE_ACTIONS}
    live={orderedSessions}
    history={libraryStore.items}
    onChoose={choosePaletteItem}
    onClose={() => (uiStore.paletteOpen = false)}
  />
{/if}

{#if uiStore.settingsOpen}
  <SettingsDialog
    settings={settingsStore.current}
    claudeGaugeHook={appInfoStore.current?.claudegauge.hookInstalled ?? false}
    onSave={saveSettings}
    onClose={() => (uiStore.settingsOpen = false)}
  />
{/if}

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
    message={`${uiStore.confirmation.runningSessions === 1 ? '1 sessão ainda está trabalhando' : `${uiStore.confirmation.runningSessions} sessões ainda estão trabalhando`}. Os terminais serão encerrados; as conversas continuam salvas e podem ser retomadas depois.`}
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
    grid-template-columns: var(--sidebar-width) 1px minmax(0, 1fr);
    min-height: 0;
    transition: grid-template-columns var(--duration-view) var(--ease);
  }

  .sidebar-handle {
    display: flex;
    background: var(--border);
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

    .sidebar-handle {
      display: none;
    }

    .main {
      padding: 16px;
      overflow: visible;
    }
  }
</style>
