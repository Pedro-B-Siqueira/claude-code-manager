import type { IBufferLine, IDecoration, IDisposable, IMarker, Terminal } from '@xterm/xterm';
import { LINK_COLOR, spansKey, urlSpans, type CellText, type UrlSpan } from './link-colors';

const SCAN_DELAY_MS = 120;

interface ColoredRow {
  marker: IMarker;
  decorations: IDecoration[];
  key: string;
}

/**
 * Paints the http(s) URLs on the visible rows. Claude Code redraws parts of the screen in place, so
 * visible rows are rescanned after every write and a color never outlives the URL it was painted on.
 */
export class LinkColorizer {
  private rows: ColoredRow[] = [];
  private timer: ReturnType<typeof setTimeout> | null = null;
  private readonly subscriptions: IDisposable[];

  constructor(private readonly terminal: Terminal) {
    this.subscriptions = [
      terminal.onWriteParsed(() => this.schedule()),
      terminal.onScroll(() => this.schedule()),
      terminal.onResize(() => this.schedule()),
    ];
  }

  dispose(): void {
    if (this.timer) clearTimeout(this.timer);
    for (const subscription of this.subscriptions) subscription.dispose();
    for (const row of this.rows) disposeRow(row);
    this.rows = [];
  }

  private schedule(): void {
    if (this.timer) return;
    this.timer = setTimeout(() => {
      this.timer = null;
      this.scan();
    }, SCAN_DELAY_MS);
  }

  private scan(): void {
    const buffer = this.terminal.buffer.active;
    this.rows = this.rows.filter((row) => !row.marker.isDisposed);
    const rowsByLine = new Map(this.rows.map((row) => [row.marker.line, row]));
    for (let line = buffer.viewportY; line < buffer.viewportY + this.terminal.rows; line += 1) {
      const bufferLine = buffer.getLine(line);
      if (!bufferLine) continue;
      const spans = urlSpans(cellsOf(bufferLine, this.terminal.cols));
      const key = spansKey(spans);
      const existing = rowsByLine.get(line);
      if (existing?.key === key) continue;
      if (existing) this.remove(existing);
      if (spans.length > 0) this.paint(line, spans, key);
    }
  }

  private paint(line: number, spans: readonly UrlSpan[], key: string): void {
    const buffer = this.terminal.buffer.active;
    const marker = this.terminal.registerMarker(line - (buffer.baseY + buffer.cursorY));
    const decorations = spans.flatMap((span) => this.terminal.registerDecoration({ marker, x: span.column, width: span.width, foregroundColor: LINK_COLOR }) ?? []);
    this.rows.push({ marker, decorations, key });
  }

  private remove(row: ColoredRow): void {
    disposeRow(row);
    this.rows = this.rows.filter((candidate) => candidate !== row);
  }
}

function disposeRow(row: ColoredRow): void {
  for (const decoration of row.decorations) decoration.dispose();
  row.marker.dispose();
}

function cellsOf(line: IBufferLine, columns: number): CellText[] {
  const cells: CellText[] = [];
  for (let column = 0; column < columns; column += 1) {
    const cell = line.getCell(column);
    cells.push({ chars: cell?.getChars() ?? '', width: cell?.getWidth() ?? 1 });
  }
  return cells;
}
