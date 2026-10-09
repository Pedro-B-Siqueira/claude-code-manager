<script lang="ts">
  import type { Theme } from '../../api/types';
  import Icon from '../common/Icon.svelte';

  interface Props {
    theme: Theme;
    needsYouCount: number;
    onToggleTheme: () => void;
    onShowNeedsYou: () => void;
    onOpenPalette: () => void;
    onOpenSettings: () => void;
  }

  let { theme, needsYouCount, onToggleTheme, onShowNeedsYou, onOpenPalette, onOpenSettings }: Props = $props();

  const themeLabel = $derived(theme === 'dark' ? 'Mudar para tema claro' : 'Mudar para tema escuro');
</script>

<header class="topbar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <span class="mark" aria-hidden="true"></span>
    <span class="name">Claude Code Manager</span>
  </div>

  <button type="button" class="search" aria-label="Buscar (⌘K)" onclick={onOpenPalette}>
    <Icon name="search" size={14} />
    <span class="search-text">Buscar sessões, arquivos, branches…</span>
    <kbd class="mono">⌘K</kbd>
  </button>

  <div class="right" data-tauri-drag-region>
    <button
      type="button"
      class="needs-you"
      class:active={needsYouCount > 0}
      onclick={onShowNeedsYou}
      disabled={needsYouCount === 0}
    >
      <Icon name="bell" size={14} />
      {needsYouCount}
      {needsYouCount === 1 ? 'precisa de você' : 'precisam de você'}
    </button>

    <button type="button" class="theme" aria-label={themeLabel} title={themeLabel} onclick={onToggleTheme}>
      <Icon name={theme === 'dark' ? 'sun' : 'moon'} />
    </button>
    <button type="button" class="theme" aria-label="Configurações (⌘,)" title="Configurações (⌘,)" onclick={onOpenSettings}>
      <Icon name="settings" />
    </button>
  </div>
</header>

<style>
  .topbar {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    height: 48px;
    padding: 0 12px 0 var(--titlebar-inset);
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 650;
    letter-spacing: -0.01em;
    white-space: nowrap;
  }

  .mark {
    width: 14px;
    height: 14px;
    border-radius: 4px;
    background: var(--accent);
  }

  .search {
    justify-self: center;
    display: flex;
    align-items: center;
    gap: 8px;
    width: min(440px, 100%);
    height: 30px;
    padding: 0 6px 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-button);
    background: var(--surface);
    color: var(--muted);
    cursor: pointer;
  }

  .search-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    font-size: 12.5px;
  }

  kbd {
    padding: 1px 6px;
    border-radius: 5px;
    background: var(--surface-2);
    font-size: 11px;
  }

  .right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .needs-you {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    white-space: nowrap;
    cursor: pointer;
  }

  .needs-you.active {
    border-color: transparent;
    background: color-mix(in srgb, var(--blue) var(--status-badge-alpha), transparent);
    color: var(--blue);
    font-weight: 550;
  }

  .needs-you:disabled {
    cursor: default;
  }

  .theme {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .theme:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  @media (max-width: 860px) {
    .search-text {
      display: none;
    }

    .search {
      width: auto;
      justify-self: end;
    }
  }
</style>
