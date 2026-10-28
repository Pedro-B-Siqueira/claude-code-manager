import type { FrameScheduler } from '../scene/frame-loop';

/** Test double for requestAnimationFrame: frames fire only when the test calls `tick`. */
export class ManualScheduler implements FrameScheduler {
  private pending = new Map<number, (time: number) => void>();
  private nextHandle = 1;

  request(callback: (time: number) => void): number {
    const handle = this.nextHandle;
    this.nextHandle += 1;
    this.pending.set(handle, callback);
    return handle;
  }

  cancel(handle: number): void {
    this.pending.delete(handle);
  }

  get scheduled(): number {
    return this.pending.size;
  }

  /** Fires every pending callback at `time`, like one vsync. */
  tick(time: number): void {
    const due = [...this.pending.values()];
    this.pending.clear();
    for (const callback of due) callback(time);
  }
}
