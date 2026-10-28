import { describe, expect, it } from 'vitest';
import { CameraRig, DEFAULT_POSE } from './camera';

const AUTO = { autoRotate: true, speed: 0.12 };
const MANUAL = { autoRotate: false, speed: 0.12 };

function wrapped(angle: number): number {
  return Math.atan2(Math.sin(angle), Math.cos(angle));
}

describe('CameraRig', () => {
  it('starts at the default pose', () => {
    expect(new CameraRig().pose).toEqual({ yaw: 0.55, pitch: 0.46, zoom: 1 });
  });

  it('turns with the drag: free yaw, pitch held between 0.12 and 1.15', () => {
    const rig = new CameraRig();
    rig.beginDrag(0);
    rig.dragBy(100, 50, 10);
    expect(rig.pose.yaw).toBeCloseTo(1.15);
    expect(rig.pose.pitch).toBeCloseTo(0.66);
    rig.dragBy(0, 1_000, 20);
    expect(rig.pose.pitch).toBe(1.15);
    rig.dragBy(0, -1_000, 30);
    expect(rig.pose.pitch).toBe(0.12);
    rig.dragBy(-5_000, 0, 40);
    expect(rig.pose.yaw).toBeCloseTo(0.55 - 30);
  });

  it('zooms between 0.6 and 1.8', () => {
    const rig = new CameraRig();
    rig.zoomBy(1.07, 0);
    expect(rig.pose.zoom).toBeCloseTo(1.07);
    rig.zoomBy(10, 0);
    expect(rig.pose.zoom).toBe(1.8);
    rig.zoomBy(0.01, 0);
    expect(rig.pose.zoom).toBe(0.6);
  });

  it('returns to the default pose', () => {
    const rig = new CameraRig();
    rig.beginDrag(0);
    rig.dragBy(300, 100, 0);
    rig.zoomBy(1.5, 0);
    rig.faceAngle(1, 0);
    rig.reset(0);
    expect(rig.pose).toEqual(DEFAULT_POSE);
    rig.step(0.1, 100, MANUAL);
    expect(rig.pose).toEqual(DEFAULT_POSE);
  });

  it('spins on its own only after 3 s without interaction', () => {
    const rig = new CameraRig();
    rig.zoomBy(1, 1_000);
    rig.step(0.5, 3_900, AUTO);
    expect(rig.pose.yaw).toBe(0.55);
    rig.step(0.5, 4_100, AUTO);
    expect(rig.pose.yaw).toBeCloseTo(0.61);
    rig.step(0.5, 4_600, MANUAL);
    expect(rig.pose.yaw).toBeCloseTo(0.61);
  });

  it('does not spin while dragging', () => {
    const rig = new CameraRig();
    rig.beginDrag(0);
    rig.step(1, 10_000, AUTO);
    expect(rig.pose.yaw).toBe(0.55);
  });

  it('turns a project to the front by the shortest way and stops there', () => {
    const rig = new CameraRig();
    rig.faceAngle(Math.PI / 6, 0);
    const target = -Math.PI / 2 - Math.PI / 6;
    let now = 0;
    for (let frame = 0; frame < 120; frame += 1) {
      now += 33;
      rig.step(0.033, now, MANUAL);
    }
    expect(Math.abs(wrapped(rig.pose.yaw - target))).toBeLessThan(0.002);
    expect(rig.pose.yaw).toBeLessThan(0.55);
    expect(rig.isMoving(now + 10, MANUAL)).toBe(false);
  });

  it('eases toward the project instead of spinning while a turn is pending', () => {
    const rig = new CameraRig();
    rig.faceAngle(Math.PI / 6, -10_000);
    rig.step(0.033, 0, AUTO);
    expect(rig.pose.yaw).toBeLessThan(0.55);
  });

  it('reports motion while easing or spinning', () => {
    const rig = new CameraRig();
    rig.faceAngle(1, 0);
    expect(rig.isMoving(10, MANUAL)).toBe(true);
    const idle = new CameraRig();
    expect(idle.isMoving(5_000, AUTO)).toBe(true);
    expect(idle.isMoving(5_000, MANUAL)).toBe(false);
  });
});
