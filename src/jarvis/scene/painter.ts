import type { DashStyle, DrawItem, DrawList } from './draw-list';
import type { Viewport } from './projection';

/** The part of CanvasRenderingContext2D the painter uses; a real 2D context satisfies it. */
export interface PaintTarget {
  globalAlpha: number;
  strokeStyle: string | CanvasGradient | CanvasPattern;
  fillStyle: string | CanvasGradient | CanvasPattern;
  lineWidth: number;
  lineCap: CanvasLineCap;
  lineJoin: CanvasLineJoin;
  setTransform(a: number, b: number, c: number, d: number, e: number, f: number): void;
  clearRect(x: number, y: number, width: number, height: number): void;
  setLineDash(segments: number[]): void;
  beginPath(): void;
  moveTo(x: number, y: number): void;
  lineTo(x: number, y: number): void;
  arc(x: number, y: number, radius: number, startAngle: number, endAngle: number): void;
  stroke(): void;
  fill(): void;
}

const GLOW_WIDTH = 6;
const GLOW_ALPHA = 0.14;
const DASHES: Record<DashStyle, number[]> = { solid: [], orbit: [2, 6], link: [6, 8] };

function tracePath(target: PaintTarget, item: DrawItem): void {
  const { coords, length } = item;
  target.beginPath();
  if (item.kind === 'lines') {
    for (let at = 0; at < length; at += 4) {
      target.moveTo(coords[at] ?? 0, coords[at + 1] ?? 0);
      target.lineTo(coords[at + 2] ?? 0, coords[at + 3] ?? 0);
    }
    return;
  }
  for (let at = 0; at < length; at += 3) {
    const x = coords[at] ?? 0;
    const y = coords[at + 1] ?? 0;
    const radius = coords[at + 2] ?? 0;
    target.moveTo(x + radius, y);
    target.arc(x, y, radius, 0, Math.PI * 2);
  }
}

function paintItem(target: PaintTarget, item: DrawItem): void {
  tracePath(target, item);
  if (item.kind === 'dots') {
    target.fillStyle = item.color;
    target.globalAlpha = item.alpha;
    target.fill();
    return;
  }
  target.strokeStyle = item.color;
  target.setLineDash(DASHES[item.dash]);
  if (item.glow) {
    target.lineWidth = GLOW_WIDTH;
    target.globalAlpha = item.alpha * GLOW_ALPHA;
    target.stroke();
  }
  target.lineWidth = item.width;
  target.globalAlpha = item.alpha;
  target.stroke();
}

/** Draws in CSS pixels; `pixelRatio` is the canvas backing scale (capped below the display's to save memory). */
export function paintFrame(target: PaintTarget, list: DrawList, pixelRatio: number, viewport: Viewport): void {
  target.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
  target.clearRect(0, 0, viewport.width, viewport.height);
  target.lineCap = 'round';
  target.lineJoin = 'round';
  for (let index = 0; index < list.count; index += 1) {
    const item = list.items[index];
    if (item && item.length > 0 && item.alpha > 0) paintItem(target, item);
  }
}
