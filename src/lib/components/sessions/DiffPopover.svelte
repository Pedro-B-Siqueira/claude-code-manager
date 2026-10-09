<script lang="ts">
  import { ltrPath, relativePath } from '../../format';
  import { diffPopoverStore } from '../../stores/diff-popover.svelte';
  import Icon from '../common/Icon.svelte';

  const WIDTH = 540;
  const MARGIN = 12;
  const MAX_HEIGHT = 460;

  const edit = $derived(diffPopoverStore.current);

  const position = $derived.by(() => {
    const anchor = diffPopoverStore.anchor;
    if (!anchor) return { top: 0, left: 0 };
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const fitsRight = anchor.right + MARGIN + WIDTH <= viewportWidth - MARGIN;
    const left = fitsRight ? anchor.right + MARGIN : Math.max(MARGIN, anchor.left - MARGIN - WIDTH);
    const top = Math.min(Math.max(MARGIN, anchor.top - 8), Math.max(MARGIN, viewportHeight - MAX_HEIGHT - MARGIN));
    return { top, left: Math.min(left, viewportWidth - WIDTH - MARGIN) };
  });

  const range = $derived.by(() => {
    if (!edit) return '';
    if (edit.isNewFile) return 'arquivo novo';
    if (edit.newStart === null) return '';
    return edit.newEnd !== null && edit.newEnd !== edit.newStart ? `linhas ${edit.newStart}–${edit.newEnd}` : `linha ${edit.newStart}`;
  });
</script>

{#if diffPopoverStore.open}
  <div
    class="popover"
    role="dialog"
    tabindex="-1"
    aria-label="Diff da edição"
    style:top="{position.top}px"
    style:left="{position.left}px"
    style:width="{WIDTH}px"
    onmouseenter={() => diffPopoverStore.cancelHide()}
    onmouseleave={() => diffPopoverStore.scheduleHide()}
  >
    {#if diffPopoverStore.loading}
      <p class="muted">Lendo o transcript…</p>
    {:else if !edit}
      <p class="muted">Nenhuma edição registrada para este arquivo.</p>
    {:else}
      <header>
        <span class="path mono" title={edit.filePath}>{ltrPath(relativePath(edit.filePath, diffPopoverStore.basePath))}</span>
        <span class="range">{range}</span>
        {#if diffPopoverStore.edits.length > 1}
          <span class="nav">
            <button type="button" aria-label="Edição mais recente" disabled={diffPopoverStore.index === 0} onclick={() => diffPopoverStore.step(-1)}>‹</button>
            <span class="mono">{diffPopoverStore.index + 1}/{diffPopoverStore.edits.length}</span>
            <button type="button" aria-label="Edição anterior" disabled={diffPopoverStore.index === diffPopoverStore.edits.length - 1} onclick={() => diffPopoverStore.step(1)}>›</button>
          </span>
        {/if}
      </header>
      <section class="why">
        <h4>Por quê</h4>
        <p>{edit.why ?? 'Sem texto do assistente antes desta edição.'}</p>
      </section>
      <div class="diff mono" aria-label="Antes e depois">
        {#each edit.lines as line, index (index)}
          <div class="line {line.kind}">
            <span class="sign" aria-hidden="true">{line.kind === 'added' ? '+' : line.kind === 'removed' ? '−' : line.kind === 'gap' ? '' : ' '}</span>
            <span class="text">{line.text || ' '}</span>
          </div>
        {:else}
          <div class="line context"><span class="text">Diff indisponível (o transcript não tem mais esta linha).</span></div>
        {/each}
        {#if edit.truncated}
          <div class="line gap"><span class="text">… diff cortado</span></div>
        {/if}
      </div>
    {/if}
    <footer><Icon name="shield" size={12} />Lido do transcript local · 0 tokens</footer>
  </div>
{/if}

<style>
  .popover {
    position: fixed;
    z-index: 80;
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-height: 460px;
    padding: 12px 14px 10px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    box-shadow: 0 18px 48px rgb(0 0 0 / 0.35);
    animation: popover-in var(--duration-popover) var(--ease) both;
    transform-origin: top left;
  }

  header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
    font-size: 12px;
    font-weight: 550;
  }

  .range {
    flex-shrink: 0;
    font-size: 11.5px;
    color: var(--muted);
  }

  .nav {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--muted);
  }

  .nav button {
    width: 22px;
    height: 22px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
  }

  .nav button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .why h4 {
    margin: 0 0 3px;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .why p {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    overflow: hidden;
    user-select: text;
  }

  .diff {
    flex: 1;
    min-height: 0;
    overflow: auto;
    border-radius: 8px;
    background: var(--terminal-bg);
    font-size: 11.5px;
    line-height: 1.5;
    padding: 6px 0;
    user-select: text;
  }

  .line {
    display: flex;
    gap: 8px;
    padding: 0 10px;
    white-space: pre;
  }

  .sign {
    width: 10px;
    flex-shrink: 0;
    text-align: center;
  }

  .line.added {
    background: color-mix(in srgb, var(--green) 14%, transparent);
    color: var(--green);
  }

  .line.removed {
    background: color-mix(in srgb, var(--diff-removed) 14%, transparent);
    color: var(--diff-removed);
  }

  .line.context {
    color: var(--terminal-text);
  }

  .line.gap {
    color: var(--muted);
  }

  footer {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--muted);
  }

  .muted {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  @keyframes popover-in {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.98);
    }
  }
</style>
