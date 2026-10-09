import type { SessionFilter } from '../sessions/status';

export type ViewMode = 'grid' | 'focus';

class UiStore {
  viewMode = $state<ViewMode>('grid');
  filter = $state<SessionFilter>('all');
  focusedSessionKey = $state<string | null>(null);

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
