import { describe, expect, it } from 'vitest';
import type { JarvisSession, JarvisStatus } from './jarvis-session';
import { formatElapsed, projectLabels, summarizeSessions } from './summary';

function session(id: string, repo: string, status: JarvisStatus): JarvisSession {
  return { id, repo, name: id, status, action: null, branch: null, lastActivityAt: null };
}

describe('summarizeSessions', () => {
  it('reports an empty scene', () => {
    expect(summarizeSessions([])).toEqual({ headline: 'Nenhuma sessão', tags: [] });
  });

  it('counts sessions, projects and only the statuses that are present', () => {
    const summary = summarizeSessions([
      session('a', 'web', 'working'),
      session('b', 'api', 'working'),
      session('c', 'web', 'waiting'),
      session('d', 'mobile', 'done'),
    ]);
    expect(summary.headline).toBe('4 sessões em 3 projetos');
    expect(summary.tags).toEqual([
      { status: 'working', label: '2 trabalhando' },
      { status: 'waiting', label: '1 esperando você' },
      { status: 'done', label: '1 concluída' },
    ]);
  });

  it('uses singular and plural forms', () => {
    expect(summarizeSessions([session('a', 'web', 'idle')])).toEqual({
      headline: '1 sessão em 1 projeto',
      tags: [{ status: 'idle', label: '1 ociosa' }],
    });
    expect(summarizeSessions([session('a', 'web', 'done'), session('b', 'web', 'done'), session('c', 'web', 'idle'), session('d', 'web', 'idle')]).tags).toEqual([
      { status: 'done', label: '2 concluídas' },
      { status: 'idle', label: '2 ociosas' },
    ]);
  });
});

describe('formatElapsed', () => {
  it('formats the time since the last activity like the balloons in the design', () => {
    expect(formatElapsed(283_000)).toBe('4m 43s');
    expect(formatElapsed(72_000)).toBe('1m 12s');
    expect(formatElapsed(45_400)).toBe('45s');
    expect(formatElapsed(0)).toBe('0s');
    expect(formatElapsed(3_900_000)).toBe('1h 05m');
  });

  it('never shows negative times from clock skew', () => {
    expect(formatElapsed(-5_000)).toBe('0s');
  });
});

describe('projectLabels', () => {
  it('names each project once, in order of arrival, with its session count', () => {
    expect(projectLabels([session('a', 'web', 'idle'), session('b', 'api', 'idle'), session('c', 'web', 'done')])).toEqual([
      { repo: 'web', count: '2 sessões' },
      { repo: 'api', count: '1 sessão' },
    ]);
  });
});
