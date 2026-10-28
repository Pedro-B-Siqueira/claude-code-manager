<script lang="ts">
  import { shortcutLabel } from '../../app/shortcuts';
  import Icon from '../common/Icon.svelte';

  interface Props {
    onOpen: () => void;
  }

  let { onOpen }: Props = $props();
</script>

<div class="jarvis">
  <button type="button" class="open" onclick={onOpen}><Icon name="sparkle" size={14} />Modo Jarvis</button>
  <kbd aria-hidden="true">{shortcutLabel('jarvis')}</kbd>
</div>

<style>
  .jarvis {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .open {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 12px;
    border: 1px solid color-mix(in srgb, var(--blue) 45%, var(--border));
    border-radius: var(--radius-button);
    background: color-mix(in srgb, var(--blue) 8%, transparent);
    color: var(--blue);
    font-size: 12.5px;
    font-weight: 550;
    cursor: pointer;
    transition: background-color 150ms var(--ease);
  }

  .open:hover {
    background: color-mix(in srgb, var(--blue) 14%, transparent);
  }

  kbd {
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: 11px;
  }
</style>
