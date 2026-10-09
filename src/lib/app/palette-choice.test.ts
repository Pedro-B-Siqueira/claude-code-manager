import { describe, expect, it, vi } from 'vitest';
import type { PaletteItem, PaletteTarget } from '../palette/search';
import { handlePaletteChoice } from './palette-choice';

function item(target: PaletteTarget): PaletteItem {
  return { id: 'x', group: 'actions', label: 'x', detail: null, keywords: '', target };
}

describe('handlePaletteChoice', () => {
  it('routes each kind of result', () => {
    const handlers = { runAction: vi.fn(), focusLive: vi.fn(), openHistory: vi.fn() };
    handlePaletteChoice(item({ kind: 'action', actionId: 'theme' }), handlers);
    handlePaletteChoice(item({ kind: 'live', key: 'k1' }), handlers);
    handlePaletteChoice(item({ kind: 'history', sessionId: 's1' }), handlers);
    handlePaletteChoice(item({ kind: 'file', sessionId: 's2', liveKey: null, path: '/a.ts' }), handlers);
    handlePaletteChoice(item({ kind: 'branch', branch: 'feat-x', liveKey: null }), handlers);
    handlePaletteChoice(item({ kind: 'branch', branch: 'feat-y', liveKey: 'k2' }), handlers);
    expect(handlers.runAction).toHaveBeenCalledWith('theme');
    expect(handlers.focusLive.mock.calls).toEqual([['k1'], ['k2']]);
    expect(handlers.openHistory.mock.calls).toEqual([['s1', null], ['s2', null], ['', 'feat-x']]);
  });
});
