<script lang="ts">
  import { Channel } from '@tauri-apps/api/core';
  import { FitAddon } from '@xterm/addon-fit';
  import { Unicode11Addon } from '@xterm/addon-unicode11';
  import { WebglAddon } from '@xterm/addon-webgl';
  import { Terminal } from '@xterm/xterm';
  import '@xterm/xterm/css/xterm.css';
  import { onMount } from 'svelte';
  import { attachTerminal, detachTerminal, resizeTerminal, writeTerminal, type TerminalChunk } from '../../api/commands';
  import { reportError, reportWarning, type FailureCause } from '../../api/logger';
  import type { Theme } from '../../api/types';
  import { terminalTheme } from '../../terminal/palette';

  interface Props {
    sessionKey: string;
    theme: Theme;
    scrollback: number;
  }

  let { sessionKey, theme, scrollback }: Props = $props();

  const FONT_FAMILY = '"Geist Mono", ui-monospace, Menlo, monospace';
  const FONT_SIZE = 12.5;

  let container: HTMLDivElement;
  let terminal: Terminal | null = null;

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

  function copySelectionOnCommandC(target: Terminal): void {
    target.attachCustomKeyEventHandler((event) => {
      const isCopy = event.type === 'keydown' && event.metaKey && event.key === 'c' && target.hasSelection();
      if (isCopy) void navigator.clipboard.writeText(target.getSelection());
      return !isCopy;
    });
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
    });
    terminal = target;
    target.loadAddon(fit);
    target.loadAddon(new Unicode11Addon());
    target.unicode.activeVersion = '11';
    copySelectionOnCommandC(target);

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

  .terminal-host :global(.xterm-viewport) {
    background: transparent !important;
  }
</style>
