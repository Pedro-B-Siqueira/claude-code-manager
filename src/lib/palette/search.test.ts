import { describe, expect, it } from 'vitest';
import type { LiveSessionView, SessionListItem } from '../api/types';
import { MOCK_LIVE_SESSIONS } from '../mock/sessions';
import { buildItems, normalize, rankItems, score, type PaletteAction } from './search';

const ACTIONS: PaletteAction[] = [
  { id: 'new', label: 'Nova sessão', keywords: 'abrir terminal', shortcut: '⌘N' },
  { id: 'theme', label: 'Alternar tema', keywords: 'claro escuro', shortcut: null },
];

function historyItem(id: string, title: string, branch: string | null): SessionListItem {
  return { id, title, customName: null, firstPrompt: null, project: 'docs-site', cwd: null, branch, startedAt: null, updatedAt: null, pinned: false, pinOrder: null, category: null, tags: ['docs'], costUsd: 0, filesCount: 0 };
}

const LIVE: LiveSessionView[] = MOCK_LIVE_SESSIONS.slice(0, 2);
const HISTORY = [historyItem('old', 'Revisão do guia de contribuição', 'docs-guide'), historyItem(LIVE[0]?.sessionId ?? '', 'duplicate of live', null)];

describe('palette search', () => {
  it('ignores accents and case', () => {
    expect(normalize('Paginação')).toBe('paginacao');
    expect(score('Paginação na listagem', 'PAGINACAO')).not.toBeNull();
    expect(score('Paginação na listagem', 'inexistente')).toBeNull();
  });

  it('prefers matches at the start of a word', () => {
    const atStart = score('orders-table.tsx', 'table') ?? 0;
    const inside = score('timetable.tsx', 'table') ?? 0;
    expect(atStart).toBeGreaterThan(inside);
  });

  it('shows actions and open sessions before typing', () => {
    const ranked = rankItems(buildItems(ACTIONS, LIVE, HISTORY), '');
    expect(new Set(ranked.map((item) => item.group))).toEqual(new Set(['actions', 'live']));
  });

  it('finds sessions, files, branches and history, without duplicating open sessions', () => {
    const items = buildItems(ACTIONS, LIVE, HISTORY);
    expect(items.filter((item) => item.group === 'history').map((item) => item.label)).toEqual(['Revisão do guia de contribuição']);
    const files = rankItems(items, 'orders-table');
    expect(files[0]).toMatchObject({ group: 'files', label: 'orders-table.tsx' });
    const branches = rankItems(items, 'docs-guide');
    expect(branches.some((item) => item.group === 'branches' && item.label === 'docs-guide')).toBe(true);
    expect(rankItems(items, 'tema')[0]).toMatchObject({ group: 'actions', label: 'Alternar tema' });
  });
});
