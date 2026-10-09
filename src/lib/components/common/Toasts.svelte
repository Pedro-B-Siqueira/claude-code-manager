<script lang="ts">
  import { toastStore } from '../../stores/toasts.svelte';
  import Icon from './Icon.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each toastStore.items as toast (toast.id)}
    <div class="toast" data-tone={toast.tone} role={toast.tone === 'error' ? 'alert' : 'status'}>
      <Icon name={toast.tone === 'info' ? 'bell' : 'warning'} size={14} />
      <p>{toast.message}</p>
      <button type="button" aria-label="Fechar aviso" onclick={() => toastStore.dismiss(toast.id)}><Icon name="close" size={12} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 90;
    display: grid;
    gap: 8px;
    width: min(420px, calc(100vw - 32px));
  }

  .toast {
    --tone: var(--blue);
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: start;
    gap: 8px;
    padding: 10px 10px 10px 12px;
    border: 1px solid color-mix(in srgb, var(--tone) 40%, var(--border));
    border-radius: 10px;
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-hover);
    animation: toast-in var(--duration-popover) var(--ease) both;
  }

  .toast[data-tone='warning'] {
    --tone: var(--amber);
  }

  .toast[data-tone='error'] {
    --tone: var(--diff-removed);
  }

  .toast :global(svg) {
    color: var(--tone);
    margin-top: 2px;
  }

  p {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  button {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
