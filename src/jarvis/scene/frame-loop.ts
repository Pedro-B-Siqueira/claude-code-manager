export interface FrameScheduler {
  request(callback: (time: number) => void): number;
  cancel(handle: number): void;
}

export const ANIMATION_FRAMES: FrameScheduler = {
  request: (callback) => requestAnimationFrame(callback),
  cancel: (handle) => cancelAnimationFrame(handle),
};

export interface FrameLoopOptions {
  fps: number;
  scheduler?: FrameScheduler;
  /** Draws one frame; returns whether anything is still moving. A still scene stops the loop until `wake`. */
  onFrame: (seconds: number, now: number) => boolean;
}

/** A vsync slightly early still counts, so 30 fps does not drift to 20 on a 60 Hz display. */
const VSYNC_TOLERANCE_MS = 1;
const MAX_FRAME_SECONDS = 0.1;

export class FrameLoop {
  private readonly scheduler: FrameScheduler;
  private readonly onFrame: FrameLoopOptions['onFrame'];
  private interval: number;
  private handle: number | null = null;
  private lastFrame: number | null = null;
  private paused = false;
  private disposed = false;

  constructor(options: FrameLoopOptions) {
    this.scheduler = options.scheduler ?? ANIMATION_FRAMES;
    this.onFrame = options.onFrame;
    this.interval = 1000 / options.fps;
  }

  setFps(fps: number): void {
    this.interval = 1000 / fps;
  }

  wake(): void {
    if (this.handle !== null || this.paused || this.disposed) return;
    this.handle = this.scheduler.request(this.tick);
  }

  pause(): void {
    this.paused = true;
    this.stop();
  }

  resume(): void {
    this.paused = false;
    this.wake();
  }

  dispose(): void {
    this.disposed = true;
    this.stop();
  }

  private stop(): void {
    if (this.handle !== null) this.scheduler.cancel(this.handle);
    this.handle = null;
    this.lastFrame = null;
  }

  private readonly tick = (now: number): void => {
    this.handle = null;
    if (this.lastFrame !== null && now - this.lastFrame < this.interval - VSYNC_TOLERANCE_MS) {
      this.handle = this.scheduler.request(this.tick);
      return;
    }
    const seconds = this.lastFrame === null ? 0 : Math.min(MAX_FRAME_SECONDS, (now - this.lastFrame) / 1000);
    this.lastFrame = now;
    const moving = this.onFrame(seconds, now);
    if (this.disposed || this.paused || this.handle !== null) return;
    if (moving) this.handle = this.scheduler.request(this.tick);
    else this.lastFrame = null;
  };
}
