import type { UnlistenFn } from '@tauri-apps/api/event';
import { fetchActivity, fetchLiveSessions } from '../../lib/api/commands';
import { onLiveChanged } from '../../lib/api/events';
import { reportWarning } from '../../lib/api/logger';
import type { ActionUpdate } from '../../lib/api/types';
import { fetchMockData, fetchShapeRules, onSessionAction } from '../api';
import type { ShapeRule } from '../scene/shape-rules';
import type { JarvisSession } from './jarvis-session';
import type { SessionSourceKind } from './launch';
import { recentEventLines, toJarvisSession } from './live-sessions';
import { addMockSession, cycleMockStatus, designMocks, removeMockSession, type MockSession } from './mock-sessions';

/** Text in the balloons changes at most four times a second, however fast the hooks fire. */
const UPDATE_INTERVAL_MS = 250;
/** Enough rows to find two agent events between the user's prompts. */
const ACTIVITY_ROWS = 12;

export class JarvisStore {
  sessions = $state.raw<JarvisSession[]>([]);
  rules = $state.raw<ShapeRule[]>([]);
  /** False until the first data arrives, so the empty state does not flash on open. */
  ready = $state(false);
  private mocks: MockSession[] = [];
  private sessionIds = new Map<string, string>();
  private unlisteners: UnlistenFn[] = [];
  private reloadTimer: ReturnType<typeof setTimeout> | null = null;
  private actionTimer: ReturnType<typeof setTimeout> | null = null;
  private pendingActions = new Map<string, ActionUpdate>();
  private readonly now: () => number;

  constructor(now: () => number = Date.now) {
    this.now = now;
  }

  async start(source: SessionSourceKind): Promise<void> {
    const userRules = await fetchShapeRules().catch(() => []);
    if (source === 'mock') {
      const data = (await fetchMockData().catch(() => null)) ?? designMocks(this.now());
      this.rules = [...userRules, ...data.rules];
      this.showMocks(data.sessions);
    } else {
      this.rules = userRules;
      await this.reload();
      await this.follow();
    }
    this.ready = true;
  }

  stop(): void {
    for (const unlisten of this.unlisteners) unlisten();
    this.unlisteners = [];
    if (this.reloadTimer) clearTimeout(this.reloadTimer);
    if (this.actionTimer) clearTimeout(this.actionTimer);
  }

  /** The two most recent events of a session, for its expanded balloon. */
  async recentEvents(id: string): Promise<string[]> {
    const mock = this.mocks.find((session) => session.id === id);
    if (mock) return mock.events.slice(-2);
    const sessionId = this.sessionIds.get(id);
    if (!sessionId) return [];
    return recentEventLines(await fetchActivity(sessionId, ACTIVITY_ROWS).catch(() => []));
  }

  addMock(): void {
    this.showMocks(addMockSession(this.mocks, this.now()));
  }

  removeMock(): void {
    this.showMocks(removeMockSession(this.mocks));
  }

  cycleMock(id: string): void {
    this.showMocks(cycleMockStatus(this.mocks, id, this.now()));
  }

  private showMocks(sessions: MockSession[]): void {
    this.mocks = sessions;
    this.sessions = sessions;
  }

  private async follow(): Promise<void> {
    const unlisteners = await Promise.all([onLiveChanged(() => this.scheduleReload()), onSessionAction((update) => this.queueAction(update))]).catch((cause) => {
      reportWarning('Modo Jarvis: sem atualizações das sessões', cause);
      return [];
    });
    this.unlisteners = unlisteners;
  }

  private async reload(): Promise<void> {
    // On failure the next change retries; meanwhile the scene keeps what it showed.
    const views = await fetchLiveSessions().catch((cause) => {
      reportWarning('Modo Jarvis: falha ao ler as sessões', cause);
      return null;
    });
    if (!views) return;
    this.sessionIds = new Map(views.map((view) => [view.key, view.sessionId]));
    this.sessions = views.flatMap((view) => toJarvisSession(view) ?? []);
  }

  private scheduleReload(): void {
    if (this.reloadTimer) return;
    this.reloadTimer = setTimeout(() => {
      this.reloadTimer = null;
      void this.reload();
    }, UPDATE_INTERVAL_MS);
  }

  private queueAction(update: ActionUpdate): void {
    this.pendingActions.set(update.key, update);
    if (this.actionTimer) return;
    this.actionTimer = setTimeout(() => {
      this.actionTimer = null;
      this.applyActions();
    }, UPDATE_INTERVAL_MS);
  }

  private applyActions(): void {
    const pending = this.pendingActions;
    this.pendingActions = new Map();
    if (!this.sessions.some((session) => pending.has(session.id))) return;
    this.sessions = this.sessions.map((session) => {
      const update = pending.get(session.id);
      return update ? { ...session, action: update.action?.text ?? null, lastActivityAt: update.lastActivityAt } : session;
    });
  }
}
