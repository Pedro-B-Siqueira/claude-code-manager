import { describe, expect, it } from 'vitest';
import type { ActivityItem, LiveSessionView, SessionStatus } from '../../lib/api/types';
import { describeActivity, recentEventLines, toJarvisSession } from './live-sessions';

function view(overrides: Partial<LiveSessionView> = {}): LiveSessionView {
  return {
    key: 'app:1',
    sessionId: 'sid-1',
    title: 'timeline-filtros',
    repo: 'web',
    cwd: '/repo/web',
    branch: 'feat-timeline-filters',
    isWorktree: false,
    status: 'working',
    statusDetail: null,
    previewLines: [],
    files: [],
    filesTotal: 0,
    contextPercent: null,
    outputTokens: 0,
    memoryMb: null,
    origin: 'app',
    pid: 1,
    hibernated: false,
    pinned: false,
    exited: false,
    startedAt: 0,
    action: { kind: 'tool', text: 'Edit · timeline-filter.component.ts' },
    lastActivityAt: 1_000,
    ...overrides,
  };
}

function activity(kind: string, tool: string | null, target: string | null, id = 1): ActivityItem {
  return { id, timestamp: null, kind, tool, target, editId: null, fromSubagent: false };
}

describe('toJarvisSession', () => {
  it('carries the name, repo, branch, current action and last activity', () => {
    expect(toJarvisSession(view())).toEqual({
      id: 'app:1',
      repo: 'web',
      name: 'timeline-filtros',
      status: 'working',
      action: 'Edit · timeline-filter.component.ts',
      branch: 'feat-timeline-filters',
      lastActivityAt: 1_000,
    });
  });

  it('shows a permission request as waiting, like the design asks', () => {
    const statuses: SessionStatus[] = ['working', 'permission', 'waiting', 'done', 'idle'];
    expect(statuses.map((status) => toJarvisSession(view({ status }))?.status)).toEqual(['working', 'waiting', 'waiting', 'done', 'idle']);
  });

  it('shows a hibernated session as idle and leaves out ended ones', () => {
    expect(toJarvisSession(view({ hibernated: true, status: 'working' }))?.status).toBe('idle');
    expect(toJarvisSession(view({ exited: true }))).toBeNull();
  });

  it('copes with sessions that have no action yet', () => {
    expect(toJarvisSession(view({ action: null, lastActivityAt: null }))).toMatchObject({ action: null, lastActivityAt: null });
  });
});

describe('describeActivity', () => {
  it('says what the agent did, in the words of the design', () => {
    expect(describeActivity(activity('edit', 'Edit', '/repo/src/auth-tab.component.ts'))).toBe('Editou auth-tab.component.ts');
    expect(describeActivity(activity('edit', 'Write', '/repo/notes.md'))).toBe('Escreveu notes.md');
    expect(describeActivity(activity('tool', 'Read', '/repo/timeline.service.ts'))).toBe('Leu timeline.service.ts');
    expect(describeActivity(activity('tool', 'Bash', 'yarn lint'))).toBe('Rodou yarn lint');
    expect(describeActivity(activity('tool', 'Grep', 'TODO'))).toBe('Buscou TODO');
    expect(describeActivity(activity('tool', 'WebFetch', 'https://example.com'))).toBe('Abriu https://example.com');
    expect(describeActivity(activity('tool', 'Agent', 'revisar o diff'))).toBe('Delegou revisar o diff');
    expect(describeActivity(activity('tool', 'mcp__browser__click', null))).toBe('Usou mcp__browser__click');
  });

  it('skips what the user typed', () => {
    expect(describeActivity(activity('prompt', null, 'faz o deploy'))).toBeNull();
  });
});

describe('recentEventLines', () => {
  it('keeps the two latest agent events, oldest first', () => {
    const newestFirst = [activity('tool', 'Bash', 'yarn lint', 4), activity('prompt', null, 'oi', 3), activity('tool', 'Read', '/a/timeline.service.ts', 2), activity('tool', 'Bash', 'ls', 1)];
    expect(recentEventLines(newestFirst)).toEqual(['Leu timeline.service.ts', 'Rodou yarn lint']);
  });
});
