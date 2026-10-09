import { describe, expect, it } from 'vitest';
import type { LiveSessionView } from '../api/types';
import { MOCK_LIVE_SESSIONS } from '../mock/sessions';
import { orderSessions } from './layout.svelte';

function session(sessionId: string, startedAt: number): LiveSessionView {
  const base = MOCK_LIVE_SESSIONS[0];
  if (!base) throw new Error('mock missing');
  return { ...base, key: `key-${sessionId}`, sessionId, startedAt };
}

describe('orderSessions', () => {
  it('follows the saved order and appends new sessions by start time', () => {
    const sessions = [session('a', 1), session('b', 2), session('c', 3), session('d', 0)];
    const ordered = orderSessions(sessions, ['c', 'a']).map((item) => item.sessionId);
    expect(ordered).toEqual(['c', 'a', 'd', 'b']);
  });
});
