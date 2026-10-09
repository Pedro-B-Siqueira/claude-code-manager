import type { LiveSessionView } from '../api/types';
import type { SessionFilter } from '../sessions/status';

export type ViewMode = 'grid' | 'focus';

export type PendingConfirmation =
  | { kind: 'end-session'; session: LiveSessionView }
  | { kind: 'quit'; runningSessions: number };

class UiStore {
  viewMode = $state<ViewMode>('grid');
  filter = $state<SessionFilter>('all');
  focusedSessionKey = $state<string | null>(null);
  resumeOpen = $state(false);
  newSessionOpen = $state(false);
  confirmation = $state<PendingConfirmation | null>(null);

  showFilter(filter: SessionFilter): void {
    this.filter = filter;
    this.viewMode = 'grid';
  }

  focusSession(key: string): void {
    this.focusedSessionKey = key;
    this.viewMode = 'focus';
  }
}

export const uiStore = new UiStore();
