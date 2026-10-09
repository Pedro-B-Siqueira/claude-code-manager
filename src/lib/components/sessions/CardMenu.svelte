<script lang="ts" module>
  export interface MenuAction {
    id: string;
    label: string;
    danger?: boolean;
    run: () => void;
  }
</script>

<script lang="ts">
  import Icon from '../common/Icon.svelte';

  interface Props {
    actions: MenuAction[];
  }

  let { actions }: Props = $props();

  let open = $state(false);
  let root: HTMLDivElement;

  function closeOnOutside(event: MouseEvent): void {
    if (open && event.target instanceof Node && !root.contains(event.target)) open = false;
  }

  function choose(action: MenuAction): void {
    open = false;
    action.run();
  }
</script>

<svelte:window onclick={closeOnOutside} onkeydown={(event) => event.key === 'Escape' && (open = false)} />

<div class="menu-root" bind:this={root}>
  <button type="button" class="trigger" aria-label="Mais ações" aria-haspopup="menu" aria-expanded={open} onclick={() => (open = !open)}>
    <Icon name="more" />
  </button>
  {#if open}
    <div class="menu" role="menu">
      {#each actions as action (action.id)}
        <button type="button" role="menuitem" class="item" class:danger={action.danger} onclick={() => choose(action)}>{action.label}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .menu-root {
    position: relative;
  }

  .trigger {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .trigger:hover,
  .trigger[aria-expanded='true'] {
    background: var(--surface-2);
    color: var(--text);
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 20;
    display: grid;
    min-width: 180px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    box-shadow: var(--shadow-hover);
    animation: menu-in var(--duration-popover) var(--ease) both;
    transform-origin: top right;
  }

  .item {
    padding: 7px 10px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }

  .item:hover,
  .item:focus-visible {
    background: var(--surface-2);
  }

  .item.danger {
    color: var(--diff-removed);
  }

  @keyframes menu-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.97);
    }
  }
</style>
