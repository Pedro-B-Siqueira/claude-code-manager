<script lang="ts">
  import { onMount } from 'svelte';
  import FocusView from './lib/components/focus/FocusView.svelte';
  import FilterChips from './lib/components/layout/FilterChips.svelte';
  import Sidebar from './lib/components/layout/Sidebar.svelte';
  import TopBar from './lib/components/layout/TopBar.svelte';
  import ViewToggle from './lib/components/layout/ViewToggle.svelte';
  import SessionGrid from './lib/components/sessions/SessionGrid.svelte';
  import { liveSessionsStore } from './lib/stores/live.svelte';
  import { settingsStore } from './lib/stores/settings.svelte';
  import { uiStore } from './lib/stores/ui.svelte';

  onMount(() => {
    void settingsStore.load();
  });

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
      pinned={liveSessionsStore.pinned}
      projects={liveSessionsStore.projects}
      claudeGaugeDetected={true}
      onFocus={(key) => uiStore.focusSession(key)}
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
            <SessionGrid sessions={visibleSessions} onFocus={(key) => uiStore.focusSession(key)} />
          {:else}
            <FocusView
              sessions={liveSessionsStore.sessions}
              activeKey={uiStore.focusedSessionKey}
              onSelect={(key) => (uiStore.focusedSessionKey = key)}
            />
          {/if}
        </div>
      {/key}
    </main>
  </div>
</div>

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
