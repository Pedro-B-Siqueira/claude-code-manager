import { describe, expect, it } from 'vitest';
import type { SessionListItem } from '../api/types';
import { groupLibrary, RECENT_LIMIT } from './grouping';

function item(id: string, overrides: Partial<SessionListItem> = {}): SessionListItem {
  return {
    id,
    title: id,
    customName: null,
    firstPrompt: null,
    project: 'web-dashboard',
    cwd: null,
    branch: null,
    startedAt: 0,
    updatedAt: 0,
    pinned: false,
    pinOrder: null,
    category: null,
    tags: [],
    costUsd: 0,
    filesCount: 0,
    ...overrides,
  };
}

describe('groupLibrary', () => {
  it('orders pinned by pin order and keeps them out of recent', () => {
    const groups = groupLibrary([
      item('a', { pinned: true, pinOrder: 2, updatedAt: 9 }),
      item('b', { pinned: true, pinOrder: 1, updatedAt: 1 }),
      item('c', { updatedAt: 5 }),
    ]);
    expect(groups.pinned.map((session) => session.id)).toEqual(['b', 'a']);
    expect(groups.recent.map((session) => session.id)).toEqual(['c']);
  });

  it('limits recent sessions and sorts newest first', () => {
    const many = Array.from({ length: RECENT_LIMIT + 5 }, (_, index) => item(`s${index}`, { updatedAt: index }));
    const groups = groupLibrary(many);
    expect(groups.recent).toHaveLength(RECENT_LIMIT);
    expect(groups.recent[0]?.id).toBe(`s${RECENT_LIMIT + 4}`);
  });

  it('groups every session by project alphabetically', () => {
    const groups = groupLibrary([
      item('a', { project: 'mobile-app' }),
      item('b', { project: 'api-gateway' }),
      item('c', { project: 'mobile-app', pinned: true, pinOrder: 1 }),
    ]);
    expect(groups.byProject.map((group) => [group.project, group.sessions.length])).toEqual([
      ['api-gateway', 1],
      ['mobile-app', 2],
    ]);
  });
});
