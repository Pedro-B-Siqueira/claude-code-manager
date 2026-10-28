import type { CameraPose } from './projection';

export const DEFAULT_POSE: Readonly<CameraPose> = { yaw: 0.55, pitch: 0.46, zoom: 1 };

export interface SpinOptions {
  /** False with the toggle off, reduced motion, a selected balloon or no interaction allowed. */
  autoRotate: boolean;
  /** rad/s: 0.12 live, slower in Repouso. */
  speed: number;
}

const YAW_PER_PIXEL = 0.006;
const PITCH_PER_PIXEL = 0.004;
const PITCH_RANGE = [0.12, 1.15] as const;
const ZOOM_RANGE = [0.6, 1.8] as const;
const IDLE_BEFORE_SPIN_MS = 3_000;
const TURN_RATE = 3.2;
const TURN_DONE = 0.002;

function clamp(value: number, [min, max]: readonly [number, number]): number {
  return value < min ? min : value > max ? max : value;
}

function shortestTurn(angle: number): number {
  return Math.atan2(Math.sin(angle), Math.cos(angle));
}

export class CameraRig {
  readonly pose: CameraPose = { ...DEFAULT_POSE };
  private dragStart: CameraPose | null = null;
  private targetYaw: number | null = null;
  private lastInteraction = Number.NEGATIVE_INFINITY;

  beginDrag(now: number): void {
    this.dragStart = { ...this.pose };
    this.targetYaw = null;
    this.lastInteraction = now;
  }

  /** Total pointer movement since `beginDrag`. */
  dragBy(deltaX: number, deltaY: number, now: number): void {
    if (!this.dragStart) return;
    this.pose.yaw = this.dragStart.yaw + deltaX * YAW_PER_PIXEL;
    this.pose.pitch = clamp(this.dragStart.pitch + deltaY * PITCH_PER_PIXEL, PITCH_RANGE);
    this.lastInteraction = now;
  }

  endDrag(now: number): void {
    this.dragStart = null;
    this.lastInteraction = now;
  }

  get dragging(): boolean {
    return this.dragStart !== null;
  }

  zoomBy(factor: number, now: number): void {
    this.pose.zoom = clamp(this.pose.zoom * factor, ZOOM_RANGE);
    this.lastInteraction = now;
  }

  reset(now: number): void {
    Object.assign(this.pose, DEFAULT_POSE);
    this.targetYaw = null;
    this.lastInteraction = now;
  }

  /** Turns the scene until the project at `angle` on the ring faces the camera. */
  faceAngle(angle: number, now: number): void {
    this.targetYaw = -Math.PI / 2 - angle;
    this.lastInteraction = now;
  }

  step(seconds: number, now: number, options: SpinOptions): void {
    if (this.targetYaw !== null) {
      const remaining = shortestTurn(this.targetYaw - this.pose.yaw);
      this.pose.yaw += remaining * Math.min(1, seconds * TURN_RATE);
      if (Math.abs(remaining) < TURN_DONE) this.targetYaw = null;
      return;
    }
    if (this.spinning(now, options)) this.pose.yaw += seconds * options.speed;
  }

  isMoving(now: number, options: SpinOptions): boolean {
    return this.targetYaw !== null || this.spinning(now, options);
  }

  private spinning(now: number, options: SpinOptions): boolean {
    return options.autoRotate && !this.dragging && now - this.lastInteraction > IDLE_BEFORE_SPIN_MS;
  }
}
