/** `lines`: x1,y1,x2,y2 per segment. `dots` (filled) and `rings` (stroked): x,y,r per circle. */
export type DrawKind = 'lines' | 'dots' | 'rings';

/** Dash patterns from the prototype: orbit rings 2 6, links 6 8. */
export type DashStyle = 'solid' | 'orbit' | 'link';

export interface DrawStyle {
  kind: DrawKind;
  color: string;
  width: number;
  alpha: number;
  /** Drawn twice: a wide faint pass under the thin line. */
  glow?: boolean;
  dash?: DashStyle;
}

const INITIAL_CAPACITY = 64;

export class DrawItem {
  kind: DrawKind = 'lines';
  color = '';
  width = 1;
  alpha = 1;
  glow = false;
  dash: DashStyle = 'solid';
  coords = new Float32Array(INITIAL_CAPACITY);
  length = 0;

  restyle(style: DrawStyle): void {
    this.kind = style.kind;
    this.color = style.color;
    this.width = style.width;
    this.alpha = style.alpha;
    this.glow = style.glow ?? false;
    this.dash = style.dash ?? 'solid';
    this.length = 0;
  }

  line(fromX: number, fromY: number, toX: number, toY: number): void {
    this.reserve(4);
    this.coords[this.length] = fromX;
    this.coords[this.length + 1] = fromY;
    this.coords[this.length + 2] = toX;
    this.coords[this.length + 3] = toY;
    this.length += 4;
  }

  circle(x: number, y: number, radius: number): void {
    this.reserve(3);
    this.coords[this.length] = x;
    this.coords[this.length + 1] = y;
    this.coords[this.length + 2] = radius;
    this.length += 3;
  }

  private reserve(extra: number): void {
    if (this.length + extra <= this.coords.length) return;
    const grown = new Float32Array(Math.max(this.coords.length * 2, this.length + extra));
    grown.set(this.coords.subarray(0, this.length));
    this.coords = grown;
  }
}

/** Everything one frame draws. Items and their buffers are reused frame after frame. */
export class DrawList {
  readonly items: DrawItem[] = [];
  count = 0;

  begin(style: DrawStyle): DrawItem {
    let item = this.items[this.count];
    if (!item) {
      item = new DrawItem();
      this.items.push(item);
    }
    this.count += 1;
    item.restyle(style);
    return item;
  }

  clear(): void {
    this.count = 0;
  }
}
