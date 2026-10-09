import { closeSession, fetchLiveSessions, newSession, resumeSession, wakeSession } from '../api/commands';
import { onLibraryChanged, onLiveChanged, onSessionExited, onSessionPreview } from '../api/events';
import { describeFailure, reportError, type FailureCause } from '../api/logger';
import type { LiveSessionView } from '../api/types';
import { countByFilter, matchesFilter, needsYou, type SessionFilter } from '../sessions/status';
import { toastStore } from './toasts.svelte';

const RELOAD_DEBOUNCE_MS = 200;
/** Some changes have no event (a finished session turning idle, memory figures), so the list also refreshes on a timer. */
const REFRESH_INTERVAL_MS = 60_000;

/** `inline`: the caller shows `lastError` itself; `toast`: the failure surfaces as a toast. */
type FailureDisplay = 'inline' | 'toast';

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
  private reloadSequence = 0;

  async start(): Promise<void> {
    await onLiveChanged(() => this.scheduleReload());
    await onLibraryChanged(() => this.scheduleReload());
    await onSessionPreview(({ key, lines }) => this.updatePreview(key, lines));
    await onSessionExited(() => this.scheduleReload());
    setInterval(() => {
      if (!document.hidden) this.scheduleReload();
    }, REFRESH_INTERVAL_MS);
    await this.reload();
  }

  /** Only the latest request wins, so a slow older reply never overwrites a newer list. */
  async reload(): Promise<void> {
    const sequence = ++this.reloadSequence;
    const sessions = await fetchLiveSessions().catch((cause: FailureCause) => {
      reportError('falha ao carregar sessões ativas', cause);
      return null;
    });
    if (sessions && sequence === this.reloadSequence) this.sessions = sessions;
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
    return this.track(newSession(cwd), 'Não foi possível abrir a sessão', 'inline');
  }

  async resume(sessionId: string): Promise<LiveSessionView | null> {
    return this.track(resumeSession(sessionId), 'Não foi possível retomar a sessão', 'toast');
  }

  async wake(key: string): Promise<LiveSessionView | null> {
    return this.track(wakeSession(key), 'Não foi possível acordar a sessão', 'toast');
  }

  async close(key: string): Promise<void> {
    await closeSession(key).catch((cause: FailureCause) => {
      reportError('falha ao encerrar a sessão', cause);
      toastStore.failure('Não foi possível encerrar a sessão', cause);
    });
    await this.reload();
  }

  private async track(request: Promise<LiveSessionView>, context: string, display: FailureDisplay): Promise<LiveSessionView | null> {
    this.lastError = null;
    const outcome = await request.then(
      (session): LaunchOutcome => ({ ok: true, session }),
      (cause: FailureCause): LaunchOutcome => ({ ok: false, cause }),
    );
    if (!outcome.ok) {
      reportError(context, outcome.cause);
      this.lastError = `${context}: ${describeFailure(outcome.cause)}`;
      if (display === 'toast') toastStore.failure(context, outcome.cause);
      return null;
    }
    await this.reload();
    return outcome.session;
  }
}

export const liveSessionsStore = new LiveSessionsStore();
