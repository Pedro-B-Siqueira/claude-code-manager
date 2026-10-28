import type { JarvisSession } from '../data/jarvis-session';
import { CameraRig, type SpinOptions } from './camera';
import { DrawList } from './draw-list';
import { buildFrame, type FrameOverlay } from './frame-builder';
import { FrameLoop, type FrameScheduler } from './frame-loop';
import { paintFrame, type PaintTarget } from './painter';
import { Projector, type Viewport } from './projection';
import { SceneRoster, type RosterEntry } from './roster';
import type { ShapeRule } from './shape-rules';

export interface DisplayedSession {
  session: JarvisSession;
  /** Closed, fading out: still drawn for a moment. */
  leaving: boolean;
}

export interface JarvisSceneOptions {
  canvas: HTMLCanvasElement;
  reducedMotion: boolean;
  /** Sessions on stage changed: render their balloons (including the ones fading out). */
  onDisplayChange: (sessions: DisplayedSession[]) => void;
  /** Defaults to the canvas 2D context; tests pass null to skip painting. */
  context?: PaintTarget | null;
  scheduler?: FrameScheduler;
  now?: () => number;
  /** Backing-scale cap; measurements compare other values against the default. */
  maxPixelRatio?: number;
}

const LIVE_FPS = 30;
const LOCKED_FPS = 18;
const LIVE_SPIN = 0.12;
const LOCKED_SPIN = 0.07;
/** The backing store grows with the square of the scale; 1× keeps it smallest. In focus the CPU barely changes with it,
 * since compositing each frame costs more than rasterizing the lines. The HTML text stays sharp either way. */
const DEFAULT_MAX_PIXEL_RATIO = 1;
const WHEEL_ZOOM_IN = 1.07;
const WHEEL_ZOOM_OUT = 0.93;
const DEFAULT_HEADER_BOTTOM = 80;

function hide(element: HTMLElement): void {
  element.style.opacity = '0';
  element.style.pointerEvents = 'none';
}

function displayKey(entries: readonly RosterEntry[]): string {
  return entries.map((entry) => `${entry.session.id}${entry.leftAt === null ? '' : '~'}`).join('|');
}

/**
 * The hologram: owns the camera, the roster, the frame loop and the canvas. Balloons and labels are HTML
 * rendered by Svelte; the scene only moves them, writing styles directly so frames never go through
 * Svelte's reactivity.
 */
export class JarvisScene {
  private readonly canvas: HTMLCanvasElement;
  private readonly context: PaintTarget | null;
  private readonly reducedMotion: boolean;
  private readonly onDisplayChange: JarvisSceneOptions['onDisplayChange'];
  private readonly now: () => number;
  private readonly maxPixelRatio: number;
  private readonly loop: FrameLoop;
  private readonly projector = new Projector();
  private readonly camera = new CameraRig();
  private readonly roster = new SceneRoster();
  private readonly list = new DrawList();
  private readonly balloons = new Map<string, HTMLElement>();
  private readonly labels = new Map<string, HTMLElement>();
  private readonly balloonHeights = new Map<string, number>();
  private readonly resizeObserver: ResizeObserver | null;
  private viewport: Viewport = { width: 1, height: 1 };
  private pixelRatio = 1;
  private headerBottom = DEFAULT_HEADER_BOTTOM;
  private rules: readonly ShapeRule[] = [];
  private animationClock = 0;
  private autoRotate = true;
  private locked = false;
  private selectedId: string | null = null;
  private turnAngles = new Map<string, number | null>();
  private shownKey = '';
  private disposed = false;

  constructor(options: JarvisSceneOptions) {
    this.canvas = options.canvas;
    this.context = options.context === undefined ? options.canvas.getContext('2d') : options.context;
    this.reducedMotion = options.reducedMotion;
    this.onDisplayChange = options.onDisplayChange;
    this.now = options.now ?? (() => performance.now());
    this.maxPixelRatio = options.maxPixelRatio ?? DEFAULT_MAX_PIXEL_RATIO;
    this.loop = new FrameLoop({ fps: LIVE_FPS, scheduler: options.scheduler, onFrame: this.drawFrame });
    this.resizeObserver = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(this.measureBalloons);
  }

  setSessions(sessions: readonly JarvisSession[]): void {
    if (this.disposed) return;
    const now = this.now();
    this.roster.update(sessions, now);
    this.announce(this.roster.entries(now), true);
    this.loop.wake();
  }

  setRules(rules: readonly ShapeRule[]): void {
    this.rules = rules;
    this.loop.wake();
  }

  setViewport(width: number, height: number, devicePixelRatio: number): void {
    this.viewport = { width, height };
    this.pixelRatio = Math.min(devicePixelRatio, this.maxPixelRatio);
    this.canvas.width = Math.round(width * this.pixelRatio);
    this.canvas.height = Math.round(height * this.pixelRatio);
    this.loop.wake();
  }

  setHeaderBottom(pixels: number): void {
    this.headerBottom = pixels;
    this.loop.wake();
  }

  setAutoRotate(enabled: boolean): void {
    this.autoRotate = enabled;
    this.loop.wake();
  }

  setLocked(locked: boolean): void {
    this.locked = locked;
    if (locked) this.selectedId = null;
    this.loop.setFps(locked ? LOCKED_FPS : LIVE_FPS);
    this.loop.wake();
  }

  /** Toggles the selection; a newly selected session's project turns to the front. Returns the selection. */
  select(id: string): string | null {
    if (this.locked) return null;
    const now = this.now();
    this.selectedId = this.selectedId === id ? null : id;
    const angle = this.selectedId === null ? null : this.turnAngles.get(id);
    if (angle !== null && angle !== undefined) this.camera.faceAngle(angle, now);
    this.loop.wake();
    return this.selectedId;
  }

  attachBalloon(id: string, element: HTMLElement): void {
    hide(element);
    element.dataset.balloonId = id;
    this.balloons.set(id, element);
    this.resizeObserver?.observe(element);
    this.loop.wake();
  }

  detachBalloon(id: string): void {
    const element = this.balloons.get(id);
    if (element) this.resizeObserver?.unobserve(element);
    this.balloons.delete(id);
    this.balloonHeights.delete(id);
  }

  attachLabel(repo: string, element: HTMLElement): void {
    hide(element);
    this.labels.set(repo, element);
    this.loop.wake();
  }

  detachLabel(repo: string): void {
    this.labels.delete(repo);
  }

  pointerDown(): void {
    if (this.locked) return;
    this.camera.beginDrag(this.now());
    this.loop.wake();
  }

  /** Total movement since `pointerDown`. */
  pointerMove(deltaX: number, deltaY: number): void {
    if (this.locked) return;
    this.camera.dragBy(deltaX, deltaY, this.now());
    this.loop.wake();
  }

  pointerUp(): void {
    if (!this.camera.dragging) return;
    this.camera.endDrag(this.now());
    this.loop.wake();
  }

  wheel(deltaY: number): void {
    this.zoom(deltaY > 0 ? WHEEL_ZOOM_OUT : WHEEL_ZOOM_IN);
  }

  /** Trackpad pinch: ratio between this gesture step and the previous one. */
  zoom(factor: number): void {
    if (this.locked) return;
    this.camera.zoomBy(factor, this.now());
    this.loop.wake();
  }

  doubleClick(): void {
    if (this.locked) return;
    this.camera.reset(this.now());
    this.selectedId = null;
    this.loop.wake();
  }

  pause(): void {
    this.loop.pause();
  }

  resume(): void {
    this.loop.resume();
  }

  dispose(): void {
    this.disposed = true;
    this.loop.dispose();
    this.resizeObserver?.disconnect();
    this.balloons.clear();
    this.labels.clear();
    this.canvas.width = 0;
    this.canvas.height = 0;
  }

  private spinOptions(): SpinOptions {
    const allowed = this.locked || (this.autoRotate && this.selectedId === null);
    return { autoRotate: allowed && !this.reducedMotion, speed: this.locked ? LOCKED_SPIN : LIVE_SPIN };
  }

  private readonly drawFrame = (seconds: number, now: number): boolean => {
    const spin = this.spinOptions();
    this.camera.step(seconds, now, spin);
    if (!this.reducedMotion) this.animationClock += seconds;
    this.projector.configure(this.viewport, this.camera.pose);
    const entries = this.roster.entries(now);
    this.announce(entries, false);
    const overlay = buildFrame(
      {
        projector: this.projector,
        viewport: this.viewport,
        entries,
        roster: this.roster,
        rules: this.rules,
        clock: this.animationClock,
        now,
        yaw: this.camera.pose.yaw,
        reducedMotion: this.reducedMotion,
        selectedId: this.selectedId,
        balloonHeights: this.balloonHeights,
        headerBottom: this.headerBottom,
      },
      this.list,
    );
    this.turnAngles = overlay.turnAngles;
    if (this.context) paintFrame(this.context, this.list, this.pixelRatio, this.viewport);
    this.placeOverlay(overlay);
    return !this.reducedMotion || this.camera.dragging || this.roster.isAnimating(now) || this.camera.isMoving(now, spin);
  };

  private announce(entries: readonly RosterEntry[], force: boolean): void {
    const key = displayKey(entries);
    if (!force && key === this.shownKey) return;
    this.shownKey = key;
    this.onDisplayChange(entries.map((entry) => ({ session: entry.session, leaving: entry.leftAt !== null })));
  }

  private placeOverlay(overlay: FrameOverlay): void {
    const placedBalloons = new Set<string>();
    for (const balloon of overlay.balloons) {
      const element = this.balloons.get(balloon.id);
      if (!element) continue;
      placedBalloons.add(balloon.id);
      element.style.transform = `translate(${balloon.x.toFixed(1)}px, ${balloon.bottom.toFixed(1)}px) translate(-50%, -100%)`;
      element.style.opacity = balloon.opacity.toFixed(2);
      element.style.zIndex = String(balloon.zIndex);
      element.style.pointerEvents = this.locked ? 'none' : '';
    }
    for (const [id, element] of this.balloons) if (!placedBalloons.has(id)) hide(element);
    const placedLabels = new Set<string>();
    for (const label of overlay.labels) {
      const element = this.labels.get(label.repo);
      if (!element) continue;
      placedLabels.add(label.repo);
      element.style.transform = `translate(${label.x.toFixed(1)}px, ${label.y.toFixed(1)}px) translate(-50%, 0)`;
      element.style.opacity = label.opacity.toFixed(2);
    }
    for (const [repo, element] of this.labels) if (!placedLabels.has(repo)) hide(element);
  }

  private readonly measureBalloons = (resized: ResizeObserverEntry[]): void => {
    for (const { target } of resized) {
      const id = target instanceof HTMLElement ? target.dataset.balloonId : undefined;
      if (id && target instanceof HTMLElement) this.balloonHeights.set(id, target.offsetHeight);
    }
    this.loop.wake();
  };
}
