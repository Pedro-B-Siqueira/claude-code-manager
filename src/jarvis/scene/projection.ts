export interface Viewport {
  width: number;
  height: number;
}

export interface CameraPose {
  yaw: number;
  pitch: number;
  zoom: number;
}

const FOCAL_LENGTH = 1000;
const CAMERA_DISTANCE = 1350;
const CENTER_Y_RATIO = 0.6;
const REFERENCE_WIDTH = 1200;
const REFERENCE_HEIGHT = 760;
/** The camera looks at this height, so shapes stand around the centre instead of below it. */
const LOOK_AT_Y = 60;

/**
 * Simple perspective from the prototype. `project` writes into the public fields instead of returning
 * an object, so a frame projects thousands of points without allocating.
 */
export class Projector {
  x = 0;
  y = 0;
  /** Distance along the view axis; larger is farther away. */
  depth = 0;
  /** Perspective factor of the last point (focal / (depth + distance)). */
  perspective = 1;
  scale = 1;

  private centerX = 0;
  private centerY = 0;
  private cosYaw = 1;
  private sinYaw = 0;
  private cosPitch = 1;
  private sinPitch = 0;

  configure(viewport: Viewport, pose: CameraPose): void {
    this.scale = Math.min(viewport.width / REFERENCE_WIDTH, viewport.height / REFERENCE_HEIGHT) * pose.zoom;
    this.centerX = viewport.width / 2;
    this.centerY = viewport.height * CENTER_Y_RATIO;
    this.cosYaw = Math.cos(pose.yaw);
    this.sinYaw = Math.sin(pose.yaw);
    this.cosPitch = Math.cos(pose.pitch);
    this.sinPitch = Math.sin(pose.pitch);
  }

  project(x: number, y: number, z: number): void {
    const rotatedX = x * this.cosYaw - z * this.sinYaw;
    const rotatedZ = x * this.sinYaw + z * this.cosYaw;
    const height = y - LOOK_AT_Y;
    const tiltedY = height * this.cosPitch + rotatedZ * this.sinPitch;
    this.depth = -height * this.sinPitch + rotatedZ * this.cosPitch;
    this.perspective = FOCAL_LENGTH / (this.depth + CAMERA_DISTANCE);
    this.x = this.centerX + rotatedX * this.perspective * this.scale;
    this.y = this.centerY - tiltedY * this.perspective * this.scale;
  }

  /** sin(pitch), used to drop labels below a base in proportion to the tilt. */
  get pitchSine(): number {
    return this.sinPitch;
  }
}
