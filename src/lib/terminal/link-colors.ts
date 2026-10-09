/** Matches the terminal palette's blue, so links stand out from the default text color. */
export const LINK_COLOR = '#82aeff';

const URL_PATTERN = /https?:\/\/[^\s<>"'`]+/g;
const TRAILING_PUNCTUATION = /[.,;:!?'"\]}>]+$/;

export interface CellText {
  chars: string;
  width: number;
}

export interface UrlSpan {
  column: number;
  width: number;
}

/** The http(s) URLs on one terminal row, as cell columns (wide characters take two cells). */
export function urlSpans(cells: readonly CellText[]): UrlSpan[] {
  const { text, columns } = rowText(cells);
  const spans: UrlSpan[] = [];
  for (const match of text.matchAll(URL_PATTERN)) {
    const url = trimUrl(match[0]);
    if (url.length <= 'https://'.length || !/^https?:\/\/[^/]/.test(url)) continue;
    const start = match.index ?? 0;
    const column = columns[start] ?? 0;
    const lastColumn = columns[start + url.length - 1] ?? column;
    spans.push({ column, width: lastColumn - column + 1 });
  }
  return spans;
}

export function spansKey(spans: readonly UrlSpan[]): string {
  return spans.map((span) => `${span.column}:${span.width}`).join(',');
}

function rowText(cells: readonly CellText[]): { text: string; columns: number[] } {
  let text = '';
  const columns: number[] = [];
  cells.forEach((cell, column) => {
    if (cell.width === 0) return;
    const chars = cell.chars === '' ? ' ' : cell.chars;
    text += chars;
    for (let unit = 0; unit < chars.length; unit += 1) columns.push(column);
  });
  return { text, columns };
}

/** Drops punctuation that ends the sentence, keeping a closing parenthesis that has its opening one. */
function trimUrl(url: string): string {
  let trimmed = url.replace(TRAILING_PUNCTUATION, '');
  while (trimmed.endsWith(')') && count(trimmed, '(') < count(trimmed, ')')) {
    trimmed = trimmed.slice(0, -1).replace(TRAILING_PUNCTUATION, '');
  }
  return trimmed;
}

function count(text: string, character: string): number {
  return text.split(character).length - 1;
}
