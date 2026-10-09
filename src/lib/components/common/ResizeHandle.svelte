<script lang="ts">
  interface Props {
    label: string;
    value: number;
    min: number;
    max: number;
    /** +1 when dragging right grows the panel, -1 when it shrinks it. */
    direction: 1 | -1;
    onResize: (value: number) => void;
  }

  let { label, value, min, max, direction, onResize }: Props = $props();

  const KEYBOARD_STEP = 16;
  let dragging = $state(false);

  function clamp(next: number): number {
    return Math.min(max, Math.max(min, Math.round(next)));
  }

  function start(event: PointerEvent): void {
    const handle = event.currentTarget;
    if (!(handle instanceof HTMLElement)) return;
    const originX = event.clientX;
    const originValue = value;
    dragging = true;
    document.body.classList.add('resizing');
    handle.setPointerCapture(event.pointerId);
    const move = (moveEvent: PointerEvent) => onResize(clamp(originValue + (moveEvent.clientX - originX) * direction));
    const stop = () => {
      dragging = false;
      document.body.classList.remove('resizing');
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', stop);
      handle.removeEventListener('pointercancel', stop);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', stop);
    handle.addEventListener('pointercancel', stop);
  }

  function nudge(event: KeyboardEvent): void {
    const step = event.key === 'ArrowRight' ? KEYBOARD_STEP : event.key === 'ArrowLeft' ? -KEYBOARD_STEP : 0;
    if (step === 0) return;
    event.preventDefault();
    onResize(clamp(value + step * direction));
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (a focusable separator with aria-valuenow is an interactive splitter in ARIA) -->
<div
  class="handle"
  class:dragging
  role="separator"
  aria-orientation="vertical"
  aria-label={label}
  aria-valuenow={value}
  aria-valuemin={min}
  aria-valuemax={max}
  tabindex="0"
  onpointerdown={start}
  onkeydown={nudge}
></div>

<style>
  .handle {
    position: relative;
    width: 9px;
    margin: 0 -4px;
    z-index: 5;
    cursor: col-resize;
    touch-action: none;
  }

  .handle::after {
    content: '';
    position: absolute;
    inset: 0 4px;
    background: transparent;
    transition: background-color 150ms var(--ease);
  }

  .handle:hover::after,
  .handle:focus-visible::after,
  .handle.dragging::after {
    background: var(--accent);
  }

  .handle:focus-visible {
    outline: none;
  }

  :global(body.resizing) {
    cursor: col-resize;
  }

  :global(body.resizing *) {
    transition: none !important;
  }
</style>
