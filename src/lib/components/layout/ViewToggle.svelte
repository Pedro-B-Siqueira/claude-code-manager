<script lang="ts">
  import type { ViewMode } from '../../stores/ui.svelte';
  import Icon from '../common/Icon.svelte';

  interface Props {
    mode: ViewMode;
    onChange: (mode: ViewMode) => void;
  }

  let { mode, onChange }: Props = $props();
</script>

<div class="toggle" role="radiogroup" aria-label="Modo de visualização">
  <button type="button" role="radio" aria-checked={mode === 'grid'} class:active={mode === 'grid'} onclick={() => onChange('grid')}>
    <Icon name="grid" size={14} />Grade
  </button>
  <button type="button" role="radio" aria-checked={mode === 'focus'} class:active={mode === 'focus'} onclick={() => onChange('focus')}>
    <Icon name="columns" size={14} />Foco
  </button>
</div>

<style>
  .toggle {
    display: inline-flex;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-button);
    background: var(--surface);
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    cursor: pointer;
    transition:
      background-color 150ms var(--ease),
      color 150ms var(--ease);
  }

  button.active {
    background: var(--surface-2);
    color: var(--text);
    font-weight: 550;
  }
</style>
