<script lang="ts">
  import { shortcutLabel } from '../../app/shortcuts';
  import { dndzone, type DndEvent } from 'svelte-dnd-action';
  import { flip } from 'svelte/animate';
  import type { LiveSessionView } from '../../api/types';
  import SessionCard from './SessionCard.svelte';

  interface Props {
    sessions: LiveSessionView[];
    reorderable: boolean;
    onReorder: (sessionIds: string[]) => void;
    onFocus: (key: string) => void;
    onEnd: (session: LiveSessionView) => void;
    onResume: (session: LiveSessionView) => void;
    onRemove: (session: LiveSessionView) => void;
    onWake: (session: LiveSessionView) => void;
  }

  let { sessions, reorderable, onReorder, onFocus, onEnd, onResume, onRemove, onWake }: Props = $props();

  const FLIP_MS = 250;

  /** `isDndShadowItem` marks the placeholder svelte-dnd-action renders at the drop position. */
  type GridItem = LiveSessionView & { id: string; isDndShadowItem?: boolean };

  const reducedMotion = typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  const flipDuration = reducedMotion ? 0 : FLIP_MS;

  let items = $state<GridItem[]>([]);
  let dragging = $state(false);

  $effect(() => {
    if (!dragging) items = sessions.map((session) => ({ ...session, id: session.key }));
  });

  function consider(event: CustomEvent<DndEvent<GridItem>>): void {
    dragging = true;
    items = event.detail.items;
  }

  function finalize(event: CustomEvent<DndEvent<GridItem>>): void {
    items = event.detail.items;
    dragging = false;
    onReorder(items.map((item) => item.sessionId));
  }

  /** The card being dragged turns translucent and slightly smaller. */
  function styleDragged(element: HTMLElement | undefined): void {
    if (!element) return;
    element.style.opacity = '0.4';
    element.style.transform = `${element.style.transform} scale(0.97)`;
    element.style.outline = 'none';
  }
</script>

{#if sessions.length === 0}
  <div class="empty">
    <p class="empty-title">Nenhuma sessão neste filtro</p>
    <p class="empty-hint">Use “Nova sessão” ({shortcutLabel('new-session')}) ou “Retomar sessão” na barra lateral.</p>
  </div>
{:else}
  <div
    class="grid"
    use:dndzone={{ items, flipDurationMs: flipDuration, dragDisabled: !reorderable, dropTargetStyle: {}, transformDraggedElement: styleDragged, zoneTabIndex: -1 }}
    onconsider={consider}
    onfinalize={finalize}
  >
    {#each items as item (item.id)}
      <div class="slot" class:drop-preview={item.isDndShadowItem} animate:flip={{ duration: flipDuration }}>
        <SessionCard session={item} {onFocus} {onEnd} {onResume} {onRemove} {onWake} />
      </div>
    {/each}
  </div>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 14px;
    align-content: start;
    outline: none;
  }

  .slot {
    min-width: 0;
  }

  .slot.drop-preview {
    border-radius: var(--radius-card);
    outline: 1.5px dashed color-mix(in srgb, var(--accent) 70%, transparent);
    outline-offset: 2px;
  }

  .slot.drop-preview :global(.card) {
    opacity: 0.5;
  }

  .empty {
    display: grid;
    place-items: center;
    gap: 4px;
    padding: 64px 16px;
    border: 1px dashed var(--border);
    border-radius: var(--radius-card);
    color: var(--muted);
    text-align: center;
  }

  .empty-title {
    margin: 0;
    color: var(--text);
    font-weight: 550;
  }

  .empty-hint {
    margin: 0;
    font-size: 12px;
  }
</style>
