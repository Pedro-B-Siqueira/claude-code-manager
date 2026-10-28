<script lang="ts">
  import { onMount, type Component } from 'svelte';
  import { reportWarning } from '../lib/api/logger';
  import { PLATFORM } from '../lib/platform';
  import { activateRepouso, closeJarvis, fetchRepousoState, focusSessionFromJarvis, newSessionFromJarvis, onRepousoChanged, prepareRepouso, unlockRepouso } from './api';
  import Balloon from './components/Balloon.svelte';
  import EmptyState from './components/EmptyState.svelte';
  import FooterBar from './components/FooterBar.svelte';
  import Hud from './components/Hud.svelte';
  import LockedHeader from './components/LockedHeader.svelte';
  import LockPanel from './components/LockPanel.svelte';
  import RepousoDialog from './components/RepousoDialog.svelte';
  import { lockedSummary } from './data/lock-text';
  import type { JarvisSession } from './data/jarvis-session';
  import { JarvisStore } from './data/jarvis-store.svelte';
  import { launchOptions } from './data/launch';
  import { formatElapsed, projectLabels, summarizeSessions } from './data/summary';
  import { UNLOCKED, type LockSnapshot, type RepousoPreparation, type UnlockState } from './repouso-types';
  import { JarvisScene, type DisplayedSession } from './scene/jarvis-scene';

  interface DevPanelProps {
    sessions: readonly JarvisSession[];
    onAdd: () => void;
    onRemove: () => void;
    onCycle: (id: string) => void;
  }

  interface Props {
    search?: string;
    reducedMotion?: boolean;
  }

  let { search = window.location.search, reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false }: Props = $props();

  const options = $derived(launchOptions(search));
  const store = new JarvisStore();
  const HEADER_GAP = 8;

  let stage: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let scene = $state.raw<JarvisScene | null>(null);
  let displayed = $state.raw<DisplayedSession[]>([]);
  let selectedId = $state<string | null>(null);
  let recentEvents = $state<Record<string, string[]>>({});
  let autoRotate = $state(true);
  let hudHeight = $state(0);
  let now = $state(Date.now());
  let dragging = $state(false);
  let DevPanel = $state.raw<Component<DevPanelProps> | null>(null);
  let lock = $state<LockSnapshot>(UNLOCKED);
  let preparation = $state<RepousoPreparation | null>(null);
  let unlock = $state<UnlockState>('idle');
  let repousoError = $state<string | null>(null);
  let dragStart: { x: number; y: number } | null = null;
  let pinchScale = 1;

  const summary = $derived(summarizeSessions(store.sessions));
  const labels = $derived(projectLabels(displayed.map((item) => item.session)));

  $effect(() => scene?.setSessions(store.sessions));
  $effect(() => scene?.setRules(store.rules));
  $effect(() => scene?.setAutoRotate(autoRotate));
  $effect(() => scene?.setHeaderBottom(hudHeight + HEADER_GAP));
  $effect(() => scene?.setLocked(lock.active));

  onMount(() => {
    const created = new JarvisScene({ canvas, reducedMotion, maxPixelRatio: options.pixelRatio ?? undefined, onDisplayChange: (items) => (displayed = items) });
    scene = created;
    const fitViewport = () => created.setViewport(stage.clientWidth, stage.clientHeight, window.devicePixelRatio || 1);
    fitViewport();
    const resize = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(fitViewport);
    resize?.observe(stage);
    const followVisibility = () => (document.hidden ? created.pause() : created.resume());
    // Repouso shows the hologram to whoever walks by; the system prompt or the kiosk taking focus must not freeze it.
    const pause = () => {
      if (!lock.active) created.pause();
    };
    const resume = () => created.resume();
    document.addEventListener('visibilitychange', followVisibility);
    window.addEventListener('blur', pause);
    window.addEventListener('focus', resume);
    stage.addEventListener('wheel', zoomWithWheel, { passive: false });
    stage.addEventListener('gesturestart', startPinch);
    stage.addEventListener('gesturechange', pinch);
    const clock = window.setInterval(() => (now = Date.now()), 1000);
    void store.start(options.source);
    void fetchRepousoState().then(showLock, () => undefined);
    const stopFollowingRepouso = onRepousoChanged(showLock);
    if (import.meta.env.DEV && options.source === 'mock') void import('./dev/DevMockPanel.svelte').then((module) => (DevPanel = module.default));
    return () => {
      window.clearInterval(clock);
      resize?.disconnect();
      document.removeEventListener('visibilitychange', followVisibility);
      window.removeEventListener('blur', pause);
      window.removeEventListener('focus', resume);
      stage.removeEventListener('wheel', zoomWithWheel);
      stage.removeEventListener('gesturestart', startPinch);
      stage.removeEventListener('gesturechange', pinch);
      store.stop();
      void stopFollowingRepouso.then((unlisten) => unlisten(), () => undefined);
      created.dispose();
      scene = null;
    };
  });

  function isOnControl(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest('button, .balloon, .hud-layer, .footer-layer, .dev-panel, .empty-layer') !== null;
  }

  function startDrag(event: PointerEvent): void {
    if (event.button !== 0 || isOnControl(event.target)) return;
    dragStart = { x: event.clientX, y: event.clientY };
    stage.setPointerCapture?.(event.pointerId);
    dragging = true;
    scene?.pointerDown();
  }

  function drag(event: PointerEvent): void {
    if (dragStart) scene?.pointerMove(event.clientX - dragStart.x, event.clientY - dragStart.y);
  }

  function endDrag(): void {
    if (!dragStart) return;
    dragStart = null;
    dragging = false;
    scene?.pointerUp();
  }

  function zoomWithWheel(event: WheelEvent): void {
    event.preventDefault();
    scene?.wheel(event.deltaY);
  }

  /** WebKit reports trackpad pinches as gesture events with a cumulative `scale`. */
  function gestureScale(event: Event): number | null {
    return 'scale' in event && typeof event.scale === 'number' ? event.scale : null;
  }

  function startPinch(event: Event): void {
    event.preventDefault();
    pinchScale = gestureScale(event) ?? 1;
  }

  function pinch(event: Event): void {
    event.preventDefault();
    const scale = gestureScale(event);
    if (scale === null || scale <= 0) return;
    scene?.zoom(scale / pinchScale);
    pinchScale = scale;
  }

  function recenter(event: MouseEvent): void {
    if (isOnControl(event.target)) return;
    scene?.doubleClick();
    selectedId = null;
  }

  function toggleBalloon(id: string): void {
    selectedId = scene?.select(id) ?? null;
    if (selectedId !== null && !recentEvents[id]) void store.recentEvents(id).then((list) => (recentEvents = { ...recentEvents, [id]: list }));
  }

  function exit(): void {
    void closeJarvis();
  }

  /** Esc closes an open dialog (the dialog handles it) and never leaves Repouso. */
  function handleKey(event: KeyboardEvent): void {
    if (event.key === 'Escape' && !lock.active && preparation === null) exit();
  }

  function showLock(snapshot: LockSnapshot): void {
    lock = snapshot;
    if (!snapshot.active) unlock = 'idle';
    if (snapshot.active) {
      selectedId = null;
      scene?.resume();
    }
  }

  function openRepouso(): void {
    repousoError = null;
    void prepareRepouso().then(
      (ready) => (preparation = ready),
      (cause) => reportWarning('Repouso indisponível', cause),
    );
  }

  function startRepouso(showDetails: boolean): void {
    preparation = null;
    activateRepouso(showDetails).then(showLock, (cause) => {
      repousoError = 'Não foi possível ativar o repouso.';
      reportWarning('falha ao ativar o Repouso', cause);
    });
  }

  function requestUnlock(): void {
    unlock = 'asking';
    unlockRepouso().then(
      (result) => (unlock = result === 'refused' ? 'failed' : result === 'alreadyAsking' ? 'asking' : 'idle'),
      () => (unlock = 'failed'),
    );
  }

  function sceneBalloon(node: HTMLElement, id: string) {
    scene?.attachBalloon(id, node);
    return { destroy: () => scene?.detachBalloon(id) };
  }

  function sceneLabel(node: HTMLElement, repo: string) {
    scene?.attachLabel(repo, node);
    return { destroy: () => scene?.detachLabel(repo) };
  }
</script>

<svelte:window onkeydown={handleKey} />

<div
  class="stage"
  class:dragging
  class:locked={lock.active}
  bind:this={stage}
  role="application"
  aria-label="Holograma das sessões"
  onpointerdown={startDrag}
  onpointermove={drag}
  onpointerup={endDrag}
  onpointercancel={endDrag}
  ondblclick={recenter}
>
  <canvas bind:this={canvas} aria-hidden="true"></canvas>

  <div class="labels" aria-hidden="true">
    {#each labels as label (label.repo)}
      <div class="label" use:sceneLabel={label.repo}>
        <span class="repo mono">{label.repo}</span>
        <span class="count">{label.count}</span>
      </div>
    {/each}
  </div>

  <div class="balloons">
    {#each displayed as item (item.session.id)}
      <div class="slot" use:sceneBalloon={item.session.id}>
        <Balloon
          session={item.session}
          elapsed={item.session.lastActivityAt === null ? '' : formatElapsed(now - item.session.lastActivityAt)}
          selected={selectedId === item.session.id}
          interactive={!item.leaving && !lock.active}
          showAction={!lock.active || lock.showDetails}
          recentEvents={recentEvents[item.session.id] ?? null}
          onToggle={() => toggleBalloon(item.session.id)}
          onOpen={() => void focusSessionFromJarvis(item.session.id)}
        />
      </div>
    {/each}
  </div>

  <div class="hud-layer" bind:clientHeight={hudHeight}>
    {#if lock.active}
      <LockedHeader since={lock.since ?? now} summary={lockedSummary(store.sessions)} />
    {:else}
      <Hud {summary} canLock={PLATFORM === 'macos'} onLock={openRepouso} onExit={exit} />
      {#if repousoError}<p class="repouso-error" role="alert">{repousoError}</p>{/if}
    {/if}
  </div>

  {#if lock.active}
    <div class="lock-layer">
      <LockPanel {now} {unlock} onUnlock={requestUnlock} />
    </div>
  {:else}
    {#if store.ready && store.sessions.length === 0}
      <div class="empty-layer">
        <EmptyState onNewSession={() => void newSessionFromJarvis()} />
      </div>
    {/if}

    <div class="footer-layer">
      <FooterBar {autoRotate} onAutoRotateChange={(enabled) => (autoRotate = enabled)} />
    </div>
  {/if}

  {#if preparation}
    <RepousoDialog {preparation} onCancel={() => (preparation = null)} onActivate={startRepouso} />
  {/if}

  {#if DevPanel && !lock.active}
    <DevPanel sessions={store.sessions} onAdd={() => store.addMock()} onRemove={() => store.removeMock()} onCycle={(id) => store.cycleMock(id)} />
  {/if}
</div>

<style>
  .stage {
    position: fixed;
    inset: 0;
    overflow: hidden;
    background: var(--jarvis-bg);
    color: var(--jarvis-text);
    cursor: grab;
    touch-action: none;
  }

  .stage.dragging {
    cursor: grabbing;
  }

  .stage.locked {
    cursor: default;
  }

  .lock-layer {
    position: absolute;
    bottom: 48px;
    left: 50%;
    z-index: 5;
    transform: translateX(-50%);
  }

  .repouso-error {
    margin: 0 24px;
    color: var(--diff-removed);
    font-size: 12.5px;
    pointer-events: auto;
  }

  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  /* Layers, back to front: labels, balloons, empty state, HUD and footer, lock panel; dialogs (z-index 60) above all.
     The balloon layer isolates the z-indexes the scene gives each balloon. */
  .labels,
  .balloons {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .labels {
    z-index: 1;
  }

  .balloons {
    z-index: 2;
    isolation: isolate;
  }

  .label,
  .slot {
    position: absolute;
    top: 0;
    left: 0;
    will-change: transform;
  }

  .label {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    white-space: nowrap;
  }

  .label .repo {
    color: var(--jarvis-structure);
    font-size: 12px;
    font-weight: 500;
  }

  .label .count {
    color: var(--jarvis-muted);
    font-size: 11.5px;
  }

  .slot {
    pointer-events: auto;
    cursor: default;
  }

  .hud-layer,
  .footer-layer {
    position: absolute;
    right: 0;
    left: 0;
    z-index: 4;
    pointer-events: none;
  }

  .hud-layer {
    top: 0;
  }

  .footer-layer {
    bottom: 0;
  }

  .empty-layer {
    position: absolute;
    top: 30%;
    left: 50%;
    z-index: 3;
    transform: translate(-50%, -50%);
    cursor: default;
  }
</style>
