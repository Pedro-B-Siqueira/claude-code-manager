import { describe, expect, it } from 'vitest';
import { DrawList } from './draw-list';
import { paintFrame, type PaintTarget } from './painter';

class RecordingContext implements PaintTarget {
  globalAlpha = 1;
  strokeStyle: string | CanvasGradient | CanvasPattern = '';
  fillStyle: string | CanvasGradient | CanvasPattern = '';
  lineWidth = 1;
  lineCap: CanvasLineCap = 'butt';
  lineJoin: CanvasLineJoin = 'miter';
  readonly calls: string[] = [];
  private dash: number[] = [];

  setTransform(a: number, _b: number, _c: number, d: number, _e: number, _f: number): void {
    this.calls.push(`scale ${a} ${d}`);
  }

  clearRect(x: number, y: number, width: number, height: number): void {
    this.calls.push(`clear ${x} ${y} ${width} ${height}`);
  }

  setLineDash(segments: number[]): void {
    this.dash = segments;
  }

  beginPath(): void {
    this.calls.push('begin');
  }

  moveTo(x: number, y: number): void {
    this.calls.push(`move ${x} ${y}`);
  }

  lineTo(x: number, y: number): void {
    this.calls.push(`line ${x} ${y}`);
  }

  arc(x: number, y: number, radius: number): void {
    this.calls.push(`arc ${x} ${y} ${radius}`);
  }

  stroke(): void {
    this.calls.push(`stroke ${String(this.strokeStyle)} w${this.lineWidth} a${this.globalAlpha} dash[${this.dash.join(',')}]`);
  }

  fill(): void {
    this.calls.push(`fill ${String(this.fillStyle)} a${this.globalAlpha}`);
  }
}

describe('paintFrame', () => {
  it('clears the canvas at the device pixel ratio before drawing', () => {
    const context = new RecordingContext();
    paintFrame(context, new DrawList(), 1.5, { width: 800, height: 600 });
    expect(context.calls).toEqual(['scale 1.5 1.5', 'clear 0 0 800 600']);
  });

  it('strokes glowing lines twice: 6 px at 14% under the thin opaque line', () => {
    const list = new DrawList();
    list.begin({ kind: 'lines', color: '#3D99E9', width: 1.3, alpha: 1, glow: true }).line(1, 2, 3, 4);
    const context = new RecordingContext();
    paintFrame(context, list, 1, { width: 10, height: 10 });
    expect(context.calls.slice(2)).toEqual(['begin', 'move 1 2', 'line 3 4', 'stroke #3D99E9 w6 a0.14 dash[]', 'stroke #3D99E9 w1.3 a1 dash[]']);
  });

  it('applies the dash pattern of orbits and links', () => {
    const list = new DrawList();
    list.begin({ kind: 'lines', color: '#6699FF', width: 1, alpha: 0.4, dash: 'orbit' }).line(0, 0, 1, 1);
    list.begin({ kind: 'lines', color: '#6699FF', width: 1, alpha: 0.45, dash: 'link' }).line(0, 0, 1, 1);
    const context = new RecordingContext();
    paintFrame(context, list, 1, { width: 10, height: 10 });
    expect(context.calls.filter((call) => call.startsWith('stroke'))).toEqual(['stroke #6699FF w1 a0.4 dash[2,6]', 'stroke #6699FF w1 a0.45 dash[6,8]']);
  });

  it('fills dots and strokes rings, one path per item', () => {
    const list = new DrawList();
    const dots = list.begin({ kind: 'dots', color: '#24B787', width: 0, alpha: 1 });
    dots.circle(5, 5, 2);
    dots.circle(9, 9, 3);
    list.begin({ kind: 'rings', color: '#FF9800', width: 1.5, alpha: 0.5 }).circle(1, 1, 4);
    const context = new RecordingContext();
    paintFrame(context, list, 1, { width: 10, height: 10 });
    expect(context.calls.slice(2)).toEqual([
      'begin',
      'move 7 5',
      'arc 5 5 2',
      'move 12 9',
      'arc 9 9 3',
      'fill #24B787 a1',
      'begin',
      'move 5 1',
      'arc 1 1 4',
      'stroke #FF9800 w1.5 a0.5 dash[]',
    ]);
  });

  it('skips empty items and items faded to nothing', () => {
    const list = new DrawList();
    list.begin({ kind: 'lines', color: '#fff', width: 1, alpha: 1 });
    list.begin({ kind: 'lines', color: '#fff', width: 1, alpha: 0 }).line(0, 0, 1, 1);
    const context = new RecordingContext();
    paintFrame(context, list, 1, { width: 10, height: 10 });
    expect(context.calls).toHaveLength(2);
  });
});
