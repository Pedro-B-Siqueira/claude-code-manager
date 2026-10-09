import { closeSession, fetchLiveSessions, newSession, resumeSession, wakeSession } from '../api/commands';
import { onLibraryChanged, onLiveChanged, onSessionExited, onSessionPreview } from '../api/events';
import { describeFailure, reportError, type FailureCause } from '../api/logger';
import type { LiveSessionView } from '../api/types';
import { countByFilter, matchesFilter, needsYou, type SessionFilter } from '../sessions/status';

const RELOAD_DEBOUNCE_MS = 200;

type LaunchOutcome = { ok: true; session: LiveSessionView } | { ok: false; cause: FailureCause };

export interface ProjectGroup {
  repo: string;
  activeCount: number;
}

function groupByRepo(sessions: readonly LiveSessionView[]): ProjectGroup[] {
  const countsByRepo = new Map<string, number>();
  for (const session of sessions.filter((candidate) => !candidate.exited)) {
    countsByRepo.set(session.repo, (countsByRepo.get(session.repo) ?? 0) + 1);
  }
  return [...countsByRepo.entries()]
    .map(([repo, activeCount]) => ({ repo, activeCount }))
    .sort((first, second) => first.repo.localeCompare(second.repo));
}

class LiveSessionsStore {
  sessions = $state<LiveSessionView[]>([]);
  lastError = $state<string | null>(null);
  counts = $derived(countByFilter(this.sessions));
  needsYouCount = $derived(this.sessions.filter((session) => needsYou(session.status)).length);
  projects = $derived(groupByRepo(this.sessions));

  private reloadTimer: ReturnType<typeof setTimeout> | null = null;

  async start(): Promise<void> {
    await onLiveChanged(() => this.scheduleReload());
    await onLibraryChanged(() => this.scheduleReload());
    await onSessionPreview(({ key, lines }) => this.updatePreview(key, lines));
    await onSessionExited(() => this.scheduleReload());
    await this.reload();
  }

  async reload(): Promise<void> {
    this.sessions = await fetchLiveSessions().catch((cause: FailureCause) => {
      reportError('falha ao carregar sessões ativas', cause);
      return this.sessions;
    });
  }

  private scheduleReload(): void {
    if (this.reloadTimer) clearTimeout(this.reloadTimer);
    this.reloadTimer = setTimeout(() => void this.reload(), RELOAD_DEBOUNCE_MS);
  }

  private updatePreview(key: string, lines: string[]): void {
    const session = this.sessions.find((candidate) => candidate.key === key);
    if (session) session.previewLines = lines;
  }

  filtered(filter: SessionFilter): LiveSessionView[] {
    return this.sessions.filter((session) => matchesFilter(session, filter));
  }

  find(key: string | null): LiveSessionView | undefined {
    return this.sessions.find((session) => session.key === key);
  }

  async open(cwd: string): Promise<LiveSessionView | null> {
    return this.track(newSession(cwd), 'Não foi possível abrir a sessão');
  }

  async resume(sessionId: string): Promise<LiveSessionView | null> {
    return this.track(resumeSession(sessionId), 'Não foi possível retomar a sessão');
  }

  async wake(key: string): Promise<LiveSessionView | null> {
    return this.track(wakeSession(key), 'Não foi possível acordar a sessão');
  }

  async close(key: string): Promise<void> {
    await closeSession(key).catch((cause: FailureCause) => reportError('falha ao encerrar a sessão', cause));
    await this.reload();
  }

  private async track(request: Promise<LiveSessionView>, context: string): Promise<LiveSessionView | null> {
    this.lastError = null;
    const outcome = await request.then(
      (session): LaunchOutcome => ({ ok: true, session }),
      (cause: FailureCause): LaunchOutcome => ({ ok: false, cause }),
    );
    if (!outcome.ok) {
      reportError(context, outcome.cause);
      this.lastError = `${context}: ${describeFailure(outcome.cause)}`;
      return null;
    }
    await this.reload();
    return outcome.session;
  }
}

export const liveSessionsStore = new LiveSessionsStore();
