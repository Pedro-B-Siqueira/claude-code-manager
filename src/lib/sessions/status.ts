import type { LiveSessionView, SessionStatus } from '../api/types';

export type SessionFilter = 'all' | 'needsYou' | 'working' | 'done' | 'idle';

export type FilterCounts = Record<SessionFilter, number>;

export const STATUS_LABELS: Record<SessionStatus, string> = {
  working: 'Trabalhando',
  permission: 'Pedindo permissão',
  waiting: 'Esperando você',
  done: 'Concluída',
  idle: 'Ociosa',
};

export const FILTER_LABELS: Record<SessionFilter, string> = {
  all: 'Todas',
  needsYou: 'Precisam de você',
  working: 'Trabalhando',
  done: 'Concluídas',
  idle: 'Ociosas',
};

export const FILTER_ORDER: readonly SessionFilter[] = ['all', 'needsYou', 'working', 'done', 'idle'];

export function needsYou(status: SessionStatus): boolean {
  return status === 'permission' || status === 'waiting';
}

export function matchesFilter(session: LiveSessionView, filter: SessionFilter): boolean {
  switch (filter) {
    case 'all':
      return true;
    case 'needsYou':
      return needsYou(session.status);
    case 'working':
      return session.status === 'working';
    case 'done':
      return session.status === 'done';
    case 'idle':
      return session.status === 'idle';
  }
}

export function countByFilter(sessions: readonly LiveSessionView[]): FilterCounts {
  const counts: FilterCounts = { all: 0, needsYou: 0, working: 0, done: 0, idle: 0 };
  for (const filter of FILTER_ORDER) {
    counts[filter] = sessions.filter((session) => matchesFilter(session, filter)).length;
  }
  return counts;
}

export const CONTEXT_WARNING_PERCENT = 75;

export function isContextHigh(percent: number): boolean {
  return percent >= CONTEXT_WARNING_PERCENT;
}
