<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  interface Props {
    title: string;
    width?: number;
    onClose: () => void;
    children: Snippet;
    footer?: Snippet;
  }

  let { title, width = 520, onClose, children, footer }: Props = $props();

  const titleId = `dialog-title-${Math.random().toString(36).slice(2, 8)}`;

  function handleKey(event: KeyboardEvent): void {
    if (event.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={handleKey} />

<!-- svelte-ignore a11y_click_events_have_key_events (Escape is handled on the window) -->
<div class="backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && onClose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby={titleId} style:width="min({width}px, 100%)">
    <header>
      <h2 id={titleId}>{title}</h2>
      <button type="button" class="close" aria-label="Fechar" onclick={onClose}><Icon name="close" /></button>
    </header>
    <div class="content">{@render children()}</div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgb(0 0 0 / 0.42);
    animation: backdrop-in var(--duration-popover) var(--ease) both;
  }

  .dialog {
    display: flex;
    flex-direction: column;
    max-height: min(640px, 100%);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--bg);
    box-shadow: 0 24px 64px rgb(0 0 0 / 0.35);
    overflow: hidden;
    animation: dialog-in var(--duration-popover) var(--ease) both;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px 12px 18px;
    border-bottom: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
  }

  .close {
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

  .close:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .content {
    padding: 16px 18px;
    overflow-y: auto;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }

  @keyframes backdrop-in {
    from {
      opacity: 0;
    }
  }

  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
  }
</style>
