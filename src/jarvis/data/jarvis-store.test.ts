import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ShapeRule } from '../scene/shape-rules';
import { JarvisStore } from './jarvis-store.svelte';
import type { MockData } from './mock-sessions';

let backendRules: ShapeRule[] = [];
let mockFile: MockData | null = null;

vi.mock('../api', () => ({
  fetchShapeRules: vi.fn(async () => backendRules),
  fetchMockData: vi.fn(async () => mockFile),
}));

const NOW = 5_000_000;

describe('JarvisStore', () => {
  beforeEach(() => {
    backendRules = [{ match: 'mine*', shape: 'orb' }];
    mockFile = null;
  });

  it('shows the design mocks in mock mode, with the user rules ahead of the demo rules', async () => {
    const store = new JarvisStore(() => NOW);
    await store.start('mock');
    expect(store.ready).toBe(true);
    expect(store.sessions.map((session) => session.id)).toEqual(['design-1', 'design-2', 'design-3', 'design-4']);
    expect(store.rules[0]).toEqual({ match: 'mine*', shape: 'orb' });
    expect(store.rules.length).toBeGreaterThan(1);
  });

  it('prefers the mock file of a development run', async () => {
    mockFile = { sessions: [{ id: 'x', repo: 'r', name: 'x', status: 'done', action: null, branch: null, lastActivityAt: null, events: ['Enviou o PR'] }], rules: [{ match: 'r', shape: 'eye' }] };
    const store = new JarvisStore(() => NOW);
    await store.start('mock');
    expect(store.sessions.map((session) => session.id)).toEqual(['x']);
    expect(store.rules).toEqual([{ match: 'mine*', shape: 'orb' }, { match: 'r', shape: 'eye' }]);
    expect(await store.recentEvents('x')).toEqual(['Enviou o PR']);
  });

  it('still opens when the rules cannot be read', async () => {
    backendRules = [];
    const { fetchShapeRules } = await import('../api');
    vi.mocked(fetchShapeRules).mockRejectedValueOnce(new Error('sem arquivo'));
    const store = new JarvisStore(() => NOW);
    await store.start('mock');
    expect(store.ready).toBe(true);
    expect(store.sessions).toHaveLength(4);
  });

  it('lets the dev panel add, remove and cycle mock sessions', async () => {
    const store = new JarvisStore(() => NOW);
    await store.start('mock');
    store.addMock();
    expect(store.sessions).toHaveLength(5);
    store.cycleMock('design-1');
    expect(store.sessions[0]?.status).toBe('waiting');
    store.removeMock();
    expect(store.sessions).toHaveLength(4);
  });
});
