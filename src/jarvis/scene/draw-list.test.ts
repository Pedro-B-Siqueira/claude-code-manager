import { describe, expect, it } from 'vitest';
import { DrawList } from './draw-list';

describe('DrawList', () => {
  it('collects line segments and circles per item with their style', () => {
    const list = new DrawList();
    const floor = list.begin({ kind: 'lines', color: '#6699FF', width: 1, alpha: 0.16 });
    floor.line(0, 0, 10, 10);
    floor.line(10, 10, 20, 0);
    const dots = list.begin({ kind: 'dots', color: '#3D99E9', width: 0, alpha: 1 });
    dots.circle(5, 5, 3);
    expect(list.count).toBe(2);
    expect([...floor.coords.subarray(0, floor.length)]).toEqual([0, 0, 10, 10, 10, 10, 20, 0]);
    expect([...dots.coords.subarray(0, dots.length)]).toEqual([5, 5, 3]);
    expect(list.items[0]?.glow).toBe(false);
    expect(list.items[0]?.dash).toBe('solid');
  });

  it('grows an item past its initial capacity without losing data', () => {
    const list = new DrawList();
    const item = list.begin({ kind: 'lines', color: '#fff', width: 1, alpha: 1 });
    for (let index = 0; index < 500; index += 1) item.line(index, index, index + 1, index + 1);
    expect(item.length).toBe(2_000);
    expect(item.coords[1_996]).toBe(499);
    expect(item.coords[1_999]).toBe(500);
  });

  it('reuses its items on the next frame instead of allocating new ones', () => {
    const list = new DrawList();
    const first = list.begin({ kind: 'lines', color: '#fff', width: 1, alpha: 1, glow: true, dash: 'orbit' });
    first.line(1, 2, 3, 4);
    list.clear();
    expect(list.count).toBe(0);
    const again = list.begin({ kind: 'rings', color: '#FF9800', width: 1.5, alpha: 0.4 });
    expect(again).toBe(first);
    expect(again.length).toBe(0);
    expect(again.kind).toBe('rings');
    expect(again.glow).toBe(false);
    expect(again.dash).toBe('solid');
  });
});
