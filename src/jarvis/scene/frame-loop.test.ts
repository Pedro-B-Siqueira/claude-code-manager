import { describe, expect, it } from 'vitest';
import { ManualScheduler } from '../testing/manual-scheduler';
import { FrameLoop } from './frame-loop';

function loopWith(fps: number, keepRunning: () => boolean = () => true) {
  const scheduler = new ManualScheduler();
  const frames: Array<[seconds: number, now: number]> = [];
  const loop = new FrameLoop({
    fps,
    scheduler,
    onFrame: (seconds, now) => {
      frames.push([seconds, now]);
      return keepRunning();
    },
  });
  return { scheduler, frames, loop };
}

describe('FrameLoop', () => {
  it('draws nothing until woken', () => {
    const { scheduler, frames } = loopWith(30);
    scheduler.tick(0);
    expect(frames).toEqual([]);
    expect(scheduler.scheduled).toBe(0);
  });

  it('caps the frame rate by skipping vsyncs that come too soon', () => {
    const { scheduler, frames, loop } = loopWith(30);
    loop.wake();
    for (const time of [1_000, 1_016.7, 1_033.4, 1_050.1, 1_066.8]) scheduler.tick(time);
    expect(frames.map(([, now]) => now)).toEqual([1_000, 1_033.4, 1_066.8]);
    expect(frames[1]?.[0]).toBeCloseTo(0.0334);
  });

  it('runs slower when the rate drops, as in Repouso', () => {
    const { scheduler, frames, loop } = loopWith(30);
    loop.setFps(18);
    loop.wake();
    for (const time of [0, 33.4, 50.1, 55.6, 83.5, 111.2]) scheduler.tick(time);
    expect(frames.map(([, now]) => now)).toEqual([0, 55.6, 111.2]);
  });

  it('stops when the scene is still and starts again when woken', () => {
    let moving = false;
    const { scheduler, frames, loop } = loopWith(30, () => moving);
    loop.wake();
    scheduler.tick(0);
    expect(scheduler.scheduled).toBe(0);
    moving = true;
    loop.wake();
    scheduler.tick(5_000);
    scheduler.tick(5_040);
    expect(frames.map(([seconds, now]) => [seconds, now])).toEqual([
      [0, 0],
      [0, 5_000],
      [0.04, 5_040],
    ]);
  });

  it('pauses without drawing and resumes without a time jump', () => {
    const { scheduler, frames, loop } = loopWith(30);
    loop.wake();
    scheduler.tick(0);
    loop.pause();
    expect(scheduler.scheduled).toBe(0);
    loop.wake();
    expect(scheduler.scheduled).toBe(0);
    loop.resume();
    scheduler.tick(60_000);
    expect(frames.at(-1)).toEqual([0, 60_000]);
  });

  it('limits one frame to 100 ms of animation after a stall', () => {
    const { scheduler, frames, loop } = loopWith(30);
    loop.wake();
    scheduler.tick(0);
    scheduler.tick(2_000);
    expect(frames.at(-1)?.[0]).toBe(0.1);
  });

  it('keeps a single pending frame when a frame wakes the loop itself', () => {
    const scheduler = new ManualScheduler();
    const loop: FrameLoop = new FrameLoop({
      fps: 30,
      scheduler,
      onFrame: () => {
        loop.wake();
        return true;
      },
    });
    loop.wake();
    scheduler.tick(0);
    expect(scheduler.scheduled).toBe(1);
  });

  it('cancels everything on dispose and ignores later wakes', () => {
    const { scheduler, frames, loop } = loopWith(30);
    loop.wake();
    loop.dispose();
    expect(scheduler.scheduled).toBe(0);
    loop.wake();
    loop.resume();
    scheduler.tick(0);
    expect(frames).toEqual([]);
  });
});
