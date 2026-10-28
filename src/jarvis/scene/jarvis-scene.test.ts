import { describe, expect, it } from 'vitest';
import type { JarvisSession, JarvisStatus } from '../data/jarvis-session';
import { ManualScheduler } from '../testing/manual-scheduler';
import { JarvisScene, type DisplayedSession } from './jarvis-scene';

function session(id: string, repo: string, status: JarvisStatus = 'idle'): JarvisSession {
  return { id, repo, name: id, status, action: null, branch: null, lastActivityAt: null };
}

function stage(reducedMotion = true) {
  const scheduler = new ScheduledClock();
  const canvas = document.createElement('canvas');
  const displays: DisplayedSession[][] = [];
  const scene = new JarvisScene({
    canvas,
    reducedMotion,
    context: null,
    scheduler: scheduler.frames,
    now: () => scheduler.now,
    onDisplayChange: (sessions) => displays.push(sessions),
  });
  scene.setViewport(1200, 760, 1);
  return { scene, scheduler, canvas, displays };
}

/** Frames and the scene clock move together, like a real display. */
class ScheduledClock {
  readonly frames = new ManualScheduler();
  now = 0;

  advance(milliseconds: number, frameMs = 33.4): void {
    const end = this.now + milliseconds;
    while (this.now < end) {
      this.now = Math.min(end, this.now + frameMs);
      this.frames.tick(this.now);
    }
  }
}

function attached(scene: JarvisScene, kind: 'balloon' | 'label', key: string): HTMLElement {
  const element = document.createElement('div');
  if (kind === 'balloon') scene.attachBalloon(key, element);
  else scene.attachLabel(key, element);
  return element;
}

function translation(element: HTMLElement): { x: number; y: number } {
  const match = /translate\(([-\d.]+)px, ([-\d.]+)px\)/.exec(element.style.transform);
  return { x: Number(match?.[1]), y: Number(match?.[2]) };
}

describe('JarvisScene', () => {
  it('reports the sessions on stage and keeps a closed one until its fade ends', () => {
    const { scene, scheduler, displays } = stage();
    scene.setSessions([session('a', 'web'), session('b', 'web')]);
    expect(displays.at(-1)?.map((item) => [item.session.id, item.leaving])).toEqual([
      ['a', false],
      ['b', false],
    ]);
    scene.setSessions([session('a', 'web')]);
    expect(displays.at(-1)?.map((item) => [item.session.id, item.leaving])).toEqual([
      ['a', false],
      ['b', true],
    ]);
    scheduler.advance(400);
    expect(displays.at(-1)?.map((item) => item.session.id)).toEqual(['a']);
  });

  it('moves attached balloons and labels to their projected place every frame', () => {
    const { scene, scheduler } = stage();
    const balloon = attached(scene, 'balloon', 'a');
    const label = attached(scene, 'label', 'web');
    scene.setSessions([session('a', 'web')]);
    scheduler.advance(2_000);
    expect(balloon.style.transform).toMatch(/translate\([\d.]+px, [\d.]+px\)/);
    expect(Number(balloon.style.opacity)).toBe(1);
    expect(translation(label).x).toBeCloseTo(600, 0);
  });

  it('keeps balloons hidden until their session forms', () => {
    const { scene, scheduler } = stage();
    const late = attached(scene, 'balloon', 'b');
    scene.setSessions([session('a', 'web'), session('b', 'api')]);
    scheduler.advance(100);
    expect(late.style.opacity).toBe('0');
    expect(late.style.pointerEvents).toBe('none');
  });

  it('brings the selected session to the front and lets go when selected again', () => {
    const { scene, scheduler } = stage();
    const label = attached(scene, 'label', 'api');
    scene.setSessions([session('a', 'web'), session('b', 'api')]);
    scheduler.advance(2_000);
    scene.select('b');
    scheduler.advance(3_000);
    expect(translation(label).x).toBeCloseTo(600, 0);
    expect(translation(label).y).toBeGreaterThan(456);
    expect(scene.select('b')).toBeNull();
  });

  it('turns and zooms with gestures, and the double click restores the camera', () => {
    const { scene, scheduler } = stage();
    const label = attached(scene, 'label', 'web');
    scene.setSessions([session('a', 'web')]);
    scheduler.advance(2_000);
    const start = translation(label);
    scene.wheel(-100);
    scene.wheel(-100);
    scheduler.advance(100);
    expect(translation(label).y).toBeGreaterThan(start.y);
    scene.doubleClick();
    scheduler.advance(100);
    expect(translation(label).y).toBeCloseTo(start.y);
  });

  it('ignores gestures and selection while locked in Repouso', () => {
    const { scene, scheduler } = stage();
    const label = attached(scene, 'label', 'web');
    scene.setSessions([session('a', 'web'), session('b', 'api')]);
    scheduler.advance(2_000);
    scene.setLocked(true);
    scheduler.advance(100);
    const start = translation(label);
    scene.pointerDown();
    scene.pointerMove(300, 120);
    scene.pointerUp();
    scene.wheel(-100);
    expect(scene.select('b')).toBeNull();
    scheduler.advance(500);
    expect(translation(label)).toEqual(start);
  });

  it('stops drawing a still scene when motion is reduced and draws again on a change', () => {
    const { scene, scheduler } = stage(true);
    scene.setSessions([session('a', 'web')]);
    scheduler.advance(2_000);
    expect(scheduler.frames.scheduled).toBe(0);
    scene.setSessions([session('a', 'web'), session('b', 'web')]);
    expect(scheduler.frames.scheduled).toBe(1);
  });

  it('keeps drawing while shapes spin with motion allowed', () => {
    const { scene, scheduler } = stage(false);
    scene.setSessions([session('a', 'web')]);
    scheduler.advance(5_000);
    expect(scheduler.frames.scheduled).toBe(1);
  });

  it('pauses and resumes the frames', () => {
    const { scene, scheduler } = stage(false);
    scene.setSessions([session('a', 'web')]);
    scene.pause();
    expect(scheduler.frames.scheduled).toBe(0);
    scene.resume();
    expect(scheduler.frames.scheduled).toBe(1);
  });

  it('backs the canvas at the CSS size even on Retina, to keep its memory small', () => {
    const { scene, canvas } = stage();
    scene.setViewport(1000, 500, 2);
    expect([canvas.width, canvas.height]).toEqual([1000, 500]);
    scene.setViewport(1000, 500, 0.5);
    expect([canvas.width, canvas.height]).toEqual([500, 250]);
  });

  it('takes another backing-scale cap when measuring', () => {
    const canvas = document.createElement('canvas');
    const scene = new JarvisScene({ canvas, reducedMotion: true, context: null, scheduler: new ManualScheduler(), maxPixelRatio: 2, onDisplayChange: () => undefined });
    scene.setViewport(1000, 500, 2);
    expect([canvas.width, canvas.height]).toEqual([2000, 1000]);
  });

  it('releases the canvas and the frames on dispose', () => {
    const { scene, scheduler, canvas } = stage(false);
    scene.setSessions([session('a', 'web')]);
    scene.dispose();
    expect([canvas.width, canvas.height]).toEqual([0, 0]);
    expect(scheduler.frames.scheduled).toBe(0);
    scene.setSessions([session('b', 'web')]);
    expect(scheduler.frames.scheduled).toBe(0);
  });
});
