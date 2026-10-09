<script lang="ts">
  import { Channel } from '@tauri-apps/api/core';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  import { Unicode11Addon } from '@xterm/addon-unicode11';
  import { WebglAddon } from '@xterm/addon-webgl';
  import { Terminal } from '@xterm/xterm';
  import '@xterm/xterm/css/xterm.css';
  import { onMount } from 'svelte';
  import { attachTerminal, detachTerminal, openLink, resizeTerminal, writeTerminal, type TerminalChunk } from '../../api/commands';
  import { reportError, reportWarning, type FailureCause } from '../../api/logger';
  import type { Theme } from '../../api/types';
  import { shortcutFor } from '../../app/shortcuts';
  import { toastStore } from '../../stores/toasts.svelte';
  import { pasteAction, type PastedImage } from '../../terminal/attachments';
  import { shouldOpenLink } from '../../terminal/links';
  import { terminalTheme } from '../../terminal/palette';

  interface Props {
    sessionKey: string;
    theme: Theme;
    scrollback: number;
    /** Receives a supported image pasted with ⌘V; without it, pastes behave as before. */
    onPasteImage?: (image: PastedImage) => void;
    /** Returning true keeps the key from reaching the terminal. */
    beforeEnter?: (event: KeyboardEvent) => boolean;
  }

  let { sessionKey, theme, scrollback, onPasteImage, beforeEnter }: Props = $props();
  let linkHint = $state<{ x: number; y: number } | null>(null);

  const FONT_FAMILY = '"Geist Mono", ui-monospace, Menlo, monospace';
  const FONT_SIZE = 12.5;

  let container: HTMLDivElement;
  let terminal: Terminal | null = null;

  export function pasteText(text: string): void {
    terminal?.paste(text);
  }

  function toBytes(chunk: TerminalChunk): Uint8Array {
    return chunk instanceof ArrayBuffer ? new Uint8Array(chunk) : Uint8Array.from(chunk);
  }

  function loadRenderer(target: Terminal): WebglAddon | null {
    try {
      const webgl = new WebglAddon();
      webgl.onContextLoss(() => webgl.dispose());
      target.loadAddon(webgl);
      return webgl;
    } catch (cause) {
      reportWarning('WebGL indisponível no terminal; usando renderizador padrão', String(cause));
      return null;
    }
  }

  /** ⌘C copies the selection; app shortcuts (⌘K, ⌘N, ⌘1…9) are never typed into the terminal. */
  function handleAppKeys(target: Terminal): void {
    target.attachCustomKeyEventHandler((event) => {
      if (shortcutFor(event)) return false;
      if (beforeEnter?.(event)) return false;
      const isCopy = event.type === 'keydown' && event.metaKey && event.key === 'c' && target.hasSelection();
      if (isCopy) void navigator.clipboard.writeText(target.getSelection());
      return !isCopy;
    });
  }

  function handlePaste(event: ClipboardEvent): void {
    if (!onPasteImage) return;
    const action = pasteAction(event.clipboardData);
    if (action.kind === 'text') return;
    event.preventDefault();
    event.stopPropagation();
    if (action.kind === 'native') {
      terminal?.paste('');
      return;
    }
    const receive = onPasteImage;
    void action.file
      .arrayBuffer()
      .then((buffer) => receive({ bytes: new Uint8Array(buffer), name: action.file.name || 'imagem colada' }))
      .catch(() => toastStore.show('Não foi possível ler a imagem colada.', 'error'));
  }

  function activateLink(event: MouseEvent, uri: string): void {
    if (!shouldOpenLink(event)) return;
    void openLink(uri).catch((cause: FailureCause) => toastStore.failure('Não foi possível abrir o link', cause));
  }

  function showLinkHint(event: MouseEvent): void {
    linkHint = { x: event.clientX, y: event.clientY };
  }

  function hideLinkHint(): void {
    linkHint = null;
  }

  onMount(() => {
    let disposed = false;
    let webgl: WebglAddon | null = null;
    let frame = 0;
    const fit = new FitAddon();
    const target = new Terminal({
      fontFamily: FONT_FAMILY,
      fontSize: FONT_SIZE,
      lineHeight: 1.2,
      scrollback,
      cursorBlink: true,
      allowProposedApi: true,
      macOptionIsMeta: false,
      theme: terminalTheme(theme),
      linkHandler: { activate: activateLink, hover: showLinkHint, leave: hideLinkHint, allowNonHttpProtocols: true },
    });
    terminal = target;
    target.loadAddon(fit);
    target.loadAddon(new WebLinksAddon(activateLink, { hover: showLinkHint, leave: hideLinkHint }));
    target.loadAddon(new Unicode11Addon());
    target.unicode.activeVersion = '11';
    handleAppKeys(target);

    const output = new Channel<TerminalChunk>();
    output.onmessage = (chunk) => target.write(toBytes(chunk));
    const input = target.onData((data) => {
      void writeTerminal(sessionKey, data).catch((cause: FailureCause) => reportError('falha ao enviar ao terminal', cause));
    });

    const syncSize = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        if (disposed || container.clientWidth === 0) return;
        fit.fit();
        void resizeTerminal(sessionKey, target.cols, target.rows).catch((cause: FailureCause) => reportWarning('falha ao redimensionar', cause));
      });
    };
    const observer = new ResizeObserver(syncSize);

    void document.fonts.load(`${FONT_SIZE}px "Geist Mono"`).finally(() => {
      if (disposed) return;
      target.open(container);
      container.addEventListener('paste', handlePaste, { capture: true });
      webgl = loadRenderer(target);
      syncSize();
      observer.observe(container);
      target.focus();
      void attachTerminal(sessionKey, output).catch((cause: FailureCause) => reportError('falha ao conectar ao terminal', cause));
    });

    return () => {
      disposed = true;
      cancelAnimationFrame(frame);
      observer.disconnect();
      container.removeEventListener('paste', handlePaste, { capture: true });
      input.dispose();
      void detachTerminal(sessionKey).catch((cause: FailureCause) => reportWarning('falha ao desconectar do terminal', cause));
      webgl?.dispose();
      target.dispose();
      terminal = null;
    };
  });

  $effect(() => {
    if (terminal) terminal.options.theme = terminalTheme(theme);
  });
</script>

<div class="terminal-host" bind:this={container}></div>
{#if linkHint}
  <div class="link-hint" style:left="{linkHint.x + 12}px" style:top="{linkHint.y + 16}px">⌘+clique para abrir</div>
{/if}

<style>
  .terminal-host {
    width: 100%;
    height: 100%;
    min-height: 0;
    padding: 10px 4px 6px 12px;
    background: var(--terminal-bg);
    border-radius: var(--radius-card);
    overflow: hidden;
  }

  .terminal-host :global(.xterm) {
    height: 100%;
  }

  .link-hint {
    position: fixed;
    z-index: 30;
    padding: 3px 7px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--muted);
    font-size: 11px;
    pointer-events: none;
  }

  .terminal-host :global(.xterm-viewport) {
    background: transparent !important;
  }
</style>
