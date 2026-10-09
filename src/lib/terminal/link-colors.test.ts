import { describe, expect, it } from 'vitest';
import { spansKey, urlSpans, type CellText } from './link-colors';

function cells(text: string): CellText[] {
  return Array.from(text, (chars) => ({ chars: chars === ' ' ? '' : chars, width: 1 }));
}

describe('urlSpans', () => {
  it('finds each http(s) URL and the cells it covers', () => {
    const row = cells('- Link: https://github.com/acme/repo/pull/1425 e http://localhost:5173/');
    expect(urlSpans(row)).toEqual([
      { column: 8, width: 'https://github.com/acme/repo/pull/1425'.length },
      { column: 49, width: 'http://localhost:5173/'.length },
    ]);
  });

  it('leaves out punctuation that ends the sentence, keeping balanced parentheses', () => {
    expect(urlSpans(cells('veja https://example.com/a).'))).toEqual([{ column: 5, width: 'https://example.com/a'.length }]);
    expect(urlSpans(cells('(https://en.wikipedia.org/wiki/Foo_(bar))'))).toEqual([{ column: 1, width: 'https://en.wikipedia.org/wiki/Foo_(bar)'.length }]);
  });

  it('counts wide characters as two cells', () => {
    const row: CellText[] = [{ chars: '漢', width: 2 }, { chars: '', width: 0 }, ...cells(' https://x.dev')];
    expect(urlSpans(row)).toEqual([{ column: 3, width: 'https://x.dev'.length }]);
  });

  it('ignores rows without links and text that only looks like one', () => {
    expect(urlSpans(cells('nenhum link aqui'))).toEqual([]);
    expect(urlSpans(cells('https:// sem host'))).toEqual([]);
  });
});

describe('spansKey', () => {
  it('changes when the links on a row move or change', () => {
    expect(spansKey([{ column: 1, width: 5 }])).not.toBe(spansKey([{ column: 2, width: 5 }]));
    expect(spansKey([])).toBe('');
  });
});
