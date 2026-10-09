import { describe, expect, it } from 'vitest';
import type { LiveSessionView, SessionStatus } from '../api/types';
import { MOCK_LIVE_SESSIONS } from '../mock/sessions';
import { countByFilter, isContextHigh, matchesFilter, needsYou } from './status';

function sessionWith(status: SessionStatus): LiveSessionView {
  const template = MOCK_LIVE_SESSIONS[0];
  if (!template) throw new Error('mock data missing');
  return { ...template, key: status, status };
}

describe('session status helpers', () => {
  it('treats permission and waiting as needing the user', () => {
    expect(needsYou('permission')).toBe(true);
    expect(needsYou('waiting')).toBe(true);
    expect(needsYou('working')).toBe(false);
    expect(needsYou('done')).toBe(false);
    expect(needsYou('idle')).toBe(false);
  });

  it('matches each filter to its statuses', () => {
    expect(matchesFilter(sessionWith('permission'), 'needsYou')).toBe(true);
    expect(matchesFilter(sessionWith('done'), 'working')).toBe(false);
    expect(matchesFilter(sessionWith('idle'), 'all')).toBe(true);
  });

  it('counts sessions per filter', () => {
    const sessions = (['working', 'working', 'permission', 'waiting', 'done', 'idle'] as const).map(sessionWith);
    expect(countByFilter(sessions)).toEqual({ all: 6, needsYou: 2, working: 2, done: 1, idle: 1 });
  });

  it('flags context at 75% or more', () => {
    expect(isContextHigh(74.9)).toBe(false);
    expect(isContextHigh(75)).toBe(true);
  });
});
