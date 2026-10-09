import type { LiveSessionView, SessionListItem } from '../api/types';

export type PaletteGroup = 'actions' | 'live' | 'history' | 'files' | 'branches';

export const GROUP_LABELS: Record<PaletteGroup, string> = {
  actions: 'Ações',
  live: 'Sessões ativas',
  history: 'Histórico',
  files: 'Arquivos',
  branches: 'Branches',
};

export const GROUP_ORDER: readonly PaletteGroup[] = ['actions', 'live', 'history', 'files', 'branches'];

export type PaletteTarget =
  | { kind: 'action'; actionId: string }
  | { kind: 'live'; key: string }
  | { kind: 'history'; sessionId: string }
  | { kind: 'file'; sessionId: string; liveKey: string | null; path: string }
  | { kind: 'branch'; branch: string; liveKey: string | null };

export interface PaletteItem {
  id: string;
  group: PaletteGroup;
  label: string;
  detail: string | null;
  keywords: string;
  target: PaletteTarget;
}

export interface PaletteAction {
  id: string;
  label: string;
  keywords: string;
  shortcut: string | null;
}

const PER_GROUP = 6;

export function normalize(text: string): string {
  return text.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase();
}

/** Every query word must appear; words that start a token in the text rank higher. */
export function score(text: string, query: string): number | null {
  const haystack = normalize(text);
  const words = normalize(query).split(/\s+/).filter(Boolean);
  let total = 0;
  for (const word of words) {
    const index = haystack.indexOf(word);
    if (index < 0) return null;
    const startsToken = index === 0 || /[\s/._-]/.test(haystack.charAt(index - 1));
    total += (startsToken ? 3 : 1) + 1 / (1 + index / 50);
  }
  return total;
}

function fileName(path: string): string {
  return path.split('/').filter(Boolean).at(-1) ?? path;
}

export function buildItems(actions: PaletteAction[], live: LiveSessionView[], history: SessionListItem[]): PaletteItem[] {
  const items: PaletteItem[] = actions.map((action) => ({
    id: `action:${action.id}`,
    group: 'actions',
    label: action.label,
    detail: action.shortcut,
    keywords: action.keywords,
    target: { kind: 'action', actionId: action.id },
  }));
  const liveIds = new Set(live.map((session) => session.sessionId));
  for (const session of live) {
    items.push({ id: `live:${session.key}`, group: 'live', label: session.title, detail: [session.repo, session.branch].filter(Boolean).join(' · '), keywords: `${session.repo} ${session.branch ?? ''} ${session.cwd}`, target: { kind: 'live', key: session.key } });
    for (const file of session.files) {
      items.push({ id: `file:${session.key}:${file.path}`, group: 'files', label: fileName(file.path), detail: session.title, keywords: file.path, target: { kind: 'file', sessionId: session.sessionId, liveKey: session.key, path: file.path } });
    }
  }
  for (const item of history.filter((candidate) => !liveIds.has(candidate.id))) {
    items.push({ id: `history:${item.id}`, group: 'history', label: item.title, detail: [item.project, item.branch].filter(Boolean).join(' · '), keywords: `${item.project} ${item.branch ?? ''} ${item.tags.join(' ')} ${item.category ?? ''}`, target: { kind: 'history', sessionId: item.id } });
  }
  const branches = new Map<string, string | null>();
  for (const session of live) if (session.branch) branches.set(session.branch, session.key);
  for (const item of history) if (item.branch && !branches.has(item.branch)) branches.set(item.branch, null);
  for (const [branch, liveKey] of branches) {
    items.push({ id: `branch:${branch}`, group: 'branches', label: branch, detail: liveKey ? 'sessão aberta' : 'no histórico', keywords: branch, target: { kind: 'branch', branch, liveKey } });
  }
  return items;
}

/** Without a query: actions and open sessions. With one: the best matches of every group. */
export function rankItems(items: PaletteItem[], query: string): PaletteItem[] {
  if (query.trim() === '') return items.filter((item) => item.group === 'actions' || item.group === 'live');
  const scored = items
    .map((item) => ({ item, value: score(`${item.label} ${item.keywords}`, query) }))
    .filter((entry): entry is { item: PaletteItem; value: number } => entry.value !== null);
  return GROUP_ORDER.flatMap((group) =>
    scored
      .filter((entry) => entry.item.group === group)
      .sort((first, second) => second.value - first.value)
      .slice(0, PER_GROUP)
      .map((entry) => entry.item),
  );
}
