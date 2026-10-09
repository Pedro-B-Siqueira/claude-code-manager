<script lang="ts">
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { onMount } from 'svelte';
  import { openAttachment } from '../../api/commands';
  import { reportWarning, type FailureCause } from '../../api/logger';
  import type { LiveSessionView } from '../../api/types';
  import { attachmentsStore } from '../../stores/attachments.svelte';
  import { toastStore } from '../../stores/toasts.svelte';
  import {
    droppedPathsText,
    dropTargetContains,
    enterGateFor,
    shouldSubmitWithImages,
    splitDroppedPaths,
    type PastedImage,
  } from '../../terminal/attachments';
  import { submitTray } from '../../terminal/submit';
  import AttachmentTray from './AttachmentTray.svelte';
  import XtermView from './XtermView.svelte';

  interface Props {
    session: LiveSessionView;
    scrollback: number;
  }

  let { session, scrollback }: Props = $props();

  let host: HTMLDivElement;
  let terminal = $state<ReturnType<typeof XtermView>>();
  const images = $derived(attachmentsStore.tray(session.key));

  function beforeEnter(event: KeyboardEvent): boolean {
    const decision = enterGateFor(session.key).decide(event, shouldSubmitWithImages(images.length, session.status));
    if (decision === 'submit') void submitTray(session.key);
    return decision !== 'pass';
  }

  function pasteImage(image: PastedImage): void {
    void attachmentsStore.addBytes(session.key, image.bytes, image.name);
  }

  function dropFiles(paths: string[]): void {
    const { images: imagePaths, others } = splitDroppedPaths(paths);
    for (const path of imagePaths) void attachmentsStore.addPath(session.key, path);
    if (others.length > 0) terminal?.pasteText(droppedPathsText(others));
  }

  function openImage(id: string): void {
    void openAttachment(id).catch((cause: FailureCause) => toastStore.failure('Não foi possível abrir a imagem', cause));
  }

  onMount(() => {
    // Tauri reports drop positions in physical pixels.
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type !== 'drop') return;
      const ratio = window.devicePixelRatio || 1;
      const point = { x: event.payload.position.x / ratio, y: event.payload.position.y / ratio };
      if (dropTargetContains(host.getBoundingClientRect(), point)) dropFiles(event.payload.paths);
    });
    return () => {
      void unlisten.then((stop) => stop()).catch((cause: FailureCause) => reportWarning('falha ao parar de ouvir arquivos soltos', cause));
    };
  });
</script>

<div class="session-terminal" bind:this={host}>
  <AttachmentTray {images} onRemove={(id) => void attachmentsStore.remove(session.key, id)} onOpen={openImage} />
  <div class="xterm-slot">
    <XtermView bind:this={terminal} sessionKey={session.key} {scrollback} onPasteImage={pasteImage} {beforeEnter} />
  </div>
</div>

<style>
  .session-terminal {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
  }

  /* The tray renders nothing when empty, so the terminal is pinned to the stretching row. */
  .xterm-slot {
    grid-row: 2;
    min-height: 0;
  }
</style>
