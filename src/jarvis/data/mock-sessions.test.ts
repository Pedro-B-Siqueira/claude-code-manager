import { describe, expect, it } from 'vitest';
import { addMockSession, cycleMockStatus, designMocks, removeMockSession } from './mock-sessions';

const NOW = 1_000_000;

describe('designMocks', () => {
  it('mirrors the four sessions of the design with generic names', () => {
    const { sessions } = designMocks(NOW);
    expect(sessions.map((session) => [session.repo, session.status, session.action])).toEqual([
      ['atlas-web', 'working', 'Bash · yarn ngc --noEmit -p control-panel'],
      ['lens-api', 'working', 'Edit · timeline-filter.component.ts'],
      ['atlas-web', 'waiting', 'Quer que eu fique acompanhando e rode de novo?'],
      ['harbor-app', 'done', 'PR 8 enviado (d60d09)'],
    ]);
    expect(NOW - (sessions[0]?.lastActivityAt ?? 0)).toBe(283_000);
    expect(sessions.every((session) => session.events.length === 2)).toBe(true);
  });

  it('ships demo rules that give the mock repos the four special shapes', () => {
    expect(designMocks(NOW).rules.map((rule) => rule.shape).sort()).toEqual(['eye', 'fortress', 'orb', 'tower']);
  });
});

describe('dev panel helpers', () => {
  it('joins the latest project and opens a new one with the first session and every third', () => {
    const one = addMockSession([], NOW);
    const two = addMockSession(one, NOW);
    const three = addMockSession(two, NOW);
    expect(one).toHaveLength(1);
    expect(two[1]?.repo).toBe(one[0]?.repo);
    expect(new Set(three.map((session) => session.repo)).size).toBe(2);
    expect(new Set(three.map((session) => session.id)).size).toBe(3);
  });

  it('removes the newest session', () => {
    const sessions = addMockSession(addMockSession([], NOW), NOW);
    expect(removeMockSession(sessions).map((session) => session.id)).toEqual([sessions[0]?.id]);
    expect(removeMockSession([])).toEqual([]);
  });

  it('cycles a session through the four statuses', () => {
    let sessions = addMockSession([], NOW);
    const id = sessions[0]?.id ?? '';
    const seen = [sessions[0]?.status];
    for (let step = 0; step < 4; step += 1) {
      sessions = cycleMockStatus(sessions, id, NOW);
      seen.push(sessions[0]?.status);
    }
    expect(seen).toEqual(['working', 'waiting', 'done', 'idle', 'working']);
  });
});
