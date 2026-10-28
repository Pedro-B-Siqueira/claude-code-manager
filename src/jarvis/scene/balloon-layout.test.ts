import { describe, expect, it } from 'vitest';
import { layoutBalloons, type BalloonAnchor } from './balloon-layout';

const BOUNDS = { width: 1200, top: 80 };

function anchor(id: string, x: number, y: number, depth: number, height = 100, selected = false): BalloonAnchor {
  return { id, x, y, depth, height, selected };
}

describe('layoutBalloons', () => {
  it('sits a lone balloon on its anchor', () => {
    expect(layoutBalloons([anchor('a', 500, 400, 0)], BOUNDS)).toEqual([{ id: 'a', x: 500, bottom: 400, opacity: 1, zIndex: 1000 }]);
  });

  it('keeps balloons inside the window horizontally', () => {
    const [left, right] = layoutBalloons([anchor('a', 20, 400, 0), anchor('b', 1190, 600, 0)], BOUNDS);
    expect(left?.x).toBe(148);
    expect(right?.x).toBe(1052);
  });

  it('pushes the farther balloon up when it collides with a nearer one', () => {
    const placed = layoutBalloons([anchor('back', 520, 400, 90), anchor('front', 500, 420, -40)], BOUNDS);
    const front = placed.find((item) => item.id === 'front');
    const back = placed.find((item) => item.id === 'back');
    expect(front?.bottom).toBe(420);
    expect(back?.bottom).toBe(310);
  });

  it('leaves balloons that do not overlap where they are', () => {
    const placed = layoutBalloons([anchor('a', 200, 400, 0), anchor('b', 900, 400, 50)], BOUNDS);
    expect(placed.map((item) => item.bottom)).toEqual([400, 400]);
  });

  it('never lets a balloon climb into the header', () => {
    expect(layoutBalloons([anchor('a', 500, 120, 0, 100)], BOUNDS)[0]?.bottom).toBe(180);
  });

  it('dims balloons at the back to 72% unless selected', () => {
    const placed = layoutBalloons([anchor('back', 200, 400, 90), anchor('picked', 900, 400, 90, 100, true), anchor('front', 600, 400, 10)], BOUNDS);
    expect(placed.map((item) => item.opacity)).toEqual([0.72, 1, 1]);
  });

  it('stacks nearer balloons above farther ones and the selected one above all', () => {
    const placed = layoutBalloons([anchor('back', 200, 400, 90), anchor('picked', 900, 400, 120, 100, true), anchor('front', 600, 400, 10)], BOUNDS);
    const zIndex = Object.fromEntries(placed.map((item) => [item.id, item.zIndex]));
    expect(zIndex.front ?? 0).toBeGreaterThan(zIndex.back ?? 0);
    expect(zIndex.picked).toBe(2000);
  });
});
