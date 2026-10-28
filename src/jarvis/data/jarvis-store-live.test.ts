import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActionUpdate, ActivityItem, LiveSessionView } from '../../lib/api/types';
import { JarvisStore } from './jarvis-store.svelte';

const listeners = { live: [] as Array<() => void>, action: [] as Array<(update: ActionUpdate) => void> };
const unlisten = vi.fn();
let liveViews: LiveSessionView[] = [];
let activity: ActivityItem[] = [];

vi.mock('../api', () => ({
  fetchShapeRules: vi.fn(async () => []),
  fetchMockData: vi.fn(async () => null),
  onSessionAction: vi.fn(async (handler: (update: ActionUpdate) => void) => {
    listeners.action.push(handler);
    return unlisten;
  }),
}));

vi.mock('../../lib/api/commands', () => ({
  fetchLiveSessions: vi.fn(async () => liveViews),
  fetchActivity: vi.fn(async () => activity),
}));

vi.mock('../../lib/api/events', () => ({
  onLiveChanged: vi.fn(async (handler: () => void) => {
    listeners.live.push(handler);
    return unlisten;
  }),
}));

function view(key: string, overrides: Partial<LiveSessionView> = {}): LiveSessionView {
  return {
    key,
    sessionId: `sid-${key}`,
    title: key,
    repo: 'web',
    cwd: '/web',
    branch: null,
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
    action: null,
    lastActivityAt: null,
    ...overrides,
  };
}

async function started(): Promise<JarvisStore> {
  const store = new JarvisStore(() => 0);
  await store.start('live');
  return store;
}

describe('JarvisStore with live sessions', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    listeners.live = [];
    listeners.action = [];
    unlisten.mockClear();
    liveViews = [view('a'), view('b', { exited: true })];
    activity = [];
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows the running sessions of the app and leaves out ended ones', async () => {
    const store = await started();
    expect(store.ready).toBe(true);
    expect(store.sessions.map((session) => session.id)).toEqual(['a']);
  });

  it('reloads once for a burst of changes, at most four times a second', async () => {
    const { fetchLiveSessions } = await import('../../lib/api/commands');
    const store = await started();
    vi.mocked(fetchLiveSessions).mockClear();
    liveViews = [view('a'), view('c')];
    for (let change = 0; change < 5; change += 1) listeners.live.forEach((handler) => handler());
    expect(fetchLiveSessions).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(250);
    expect(fetchLiveSessions).toHaveBeenCalledOnce();
    expect(store.sessions.map((session) => session.id)).toEqual(['a', 'c']);
  });

  it('applies the latest action of a burst at most four times a second', async () => {
    const store = await started();
    listeners.action.forEach((handler) => handler({ key: 'a', action: { kind: 'tool', text: 'Read · a.ts' }, lastActivityAt: 10 }));
    listeners.action.forEach((handler) => handler({ key: 'a', action: { kind: 'tool', text: 'Edit · a.ts' }, lastActivityAt: 20 }));
    expect(store.sessions[0]?.action).toBeNull();
    await vi.advanceTimersByTimeAsync(250);
    expect(store.sessions[0]).toMatchObject({ action: 'Edit · a.ts', lastActivityAt: 20 });
  });

  it('ignores actions of sessions that are not on stage', async () => {
    const store = await started();
    listeners.action.forEach((handler) => handler({ key: 'gone', action: { kind: 'result', text: 'x' }, lastActivityAt: 5 }));
    await vi.advanceTimersByTimeAsync(250);
    expect(store.sessions.map((session) => session.action)).toEqual([null]);
  });

  it('reads the two latest events of a session from its transcript', async () => {
    const { fetchActivity } = await import('../../lib/api/commands');
    activity = [
      { id: 2, timestamp: null, kind: 'tool', tool: 'Bash', target: 'yarn lint', editId: null, fromSubagent: false },
      { id: 1, timestamp: null, kind: 'tool', tool: 'Read', target: '/w/timeline.service.ts', editId: null, fromSubagent: false },
    ];
    const store = await started();
    expect(await store.recentEvents('a')).toEqual(['Leu timeline.service.ts', 'Rodou yarn lint']);
    expect(fetchActivity).toHaveBeenCalledWith('sid-a', expect.any(Number));
  });

  it('stops listening when the window goes away', async () => {
    const store = await started();
    store.stop();
    expect(unlisten).toHaveBeenCalledTimes(2);
  });
});
