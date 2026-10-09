import type { LiveSessionView } from '../api/types';
import { MOCK_LIVE_SESSIONS } from '../mock/sessions';
import { countByFilter, matchesFilter, needsYou, type SessionFilter } from '../sessions/status';

export interface ProjectGroup {
  repo: string;
  activeCount: number;
}

function groupByRepo(sessions: readonly LiveSessionView[]): ProjectGroup[] {
  const countsByRepo = new Map<string, number>();
  for (const session of sessions) {
    countsByRepo.set(session.repo, (countsByRepo.get(session.repo) ?? 0) + 1);
  }
  return [...countsByRepo.entries()]
    .map(([repo, activeCount]) => ({ repo, activeCount }))
    .sort((first, second) => first.repo.localeCompare(second.repo));
}

class LiveSessionsStore {
  sessions = $state<LiveSessionView[]>(MOCK_LIVE_SESSIONS);
  counts = $derived(countByFilter(this.sessions));
  needsYouCount = $derived(this.sessions.filter((session) => needsYou(session.status)).length);
  projects = $derived(groupByRepo(this.sessions));

  filtered(filter: SessionFilter): LiveSessionView[] {
    return this.sessions.filter((session) => matchesFilter(session, filter));
  }

  find(key: string | null): LiveSessionView | undefined {
    return this.sessions.find((session) => session.key === key);
  }
}

export const liveSessionsStore = new LiveSessionsStore();
