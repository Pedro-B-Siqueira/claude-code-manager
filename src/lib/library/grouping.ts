import type { SessionListItem } from '../api/types';

export const RECENT_LIMIT = 15;

export interface ProjectSessions {
  project: string;
  sessions: SessionListItem[];
}

export interface LibraryGroups {
  pinned: SessionListItem[];
  recent: SessionListItem[];
  byProject: ProjectSessions[];
}

function byUpdatedDesc(first: SessionListItem, second: SessionListItem): number {
  return (second.updatedAt ?? 0) - (first.updatedAt ?? 0);
}

function byPinOrder(first: SessionListItem, second: SessionListItem): number {
  return (first.pinOrder ?? 0) - (second.pinOrder ?? 0);
}

export function groupLibrary(items: readonly SessionListItem[]): LibraryGroups {
  const pinned = items.filter((item) => item.pinned).sort(byPinOrder);
  const unpinned = items.filter((item) => !item.pinned).sort(byUpdatedDesc);
  const sessionsByProject = new Map<string, SessionListItem[]>();
  for (const item of [...items].sort(byUpdatedDesc)) {
    const sessions = sessionsByProject.get(item.project) ?? [];
    sessions.push(item);
    sessionsByProject.set(item.project, sessions);
  }
  const byProject = [...sessionsByProject.entries()]
    .map(([project, sessions]) => ({ project, sessions }))
    .sort((first, second) => first.project.localeCompare(second.project, 'pt-BR'));
  return { pinned, recent: unpinned.slice(0, RECENT_LIMIT), byProject };
}
