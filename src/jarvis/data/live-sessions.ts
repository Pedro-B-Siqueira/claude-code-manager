import type { ActivityItem, LiveSessionView, SessionStatus } from '../../lib/api/types';
import type { JarvisSession, JarvisStatus } from './jarvis-session';

/** The hologram has four colors: asking for permission is waiting for you too. */
const JARVIS_STATUS: Record<SessionStatus, JarvisStatus> = {
  working: 'working',
  permission: 'waiting',
  waiting: 'waiting',
  done: 'done',
  idle: 'idle',
};

/** Ended sessions leave the scene; hibernated ones stay, idle. */
export function toJarvisSession(view: LiveSessionView): JarvisSession | null {
  if (view.exited) return null;
  return {
    id: view.key,
    repo: view.repo,
    name: view.title,
    status: view.hibernated ? 'idle' : JARVIS_STATUS[view.status],
    action: view.action?.text ?? null,
    branch: view.branch,
    lastActivityAt: view.lastActivityAt,
  };
}

const FILE_VERBS: Record<string, string> = { Edit: 'Editou', MultiEdit: 'Editou', Write: 'Escreveu', Read: 'Leu', NotebookEdit: 'Editou' };
const TARGET_VERBS: Record<string, string> = { Bash: 'Rodou', Grep: 'Buscou', Glob: 'Buscou', WebFetch: 'Abriu', WebSearch: 'Pesquisou', Agent: 'Delegou', Task: 'Delegou' };

function fileName(path: string): string {
  return path.split('/').filter(Boolean).at(-1) ?? path;
}

/** One line of the expanded balloon; null for what the user typed. */
export function describeActivity(item: ActivityItem): string | null {
  if (item.kind === 'prompt' || !item.tool) return null;
  const fileVerb = FILE_VERBS[item.tool];
  if (fileVerb) return item.target ? `${fileVerb} ${fileName(item.target)}` : fileVerb;
  const verb = TARGET_VERBS[item.tool] ?? 'Usou';
  const subject = TARGET_VERBS[item.tool] ? item.target : item.tool;
  return subject ? `${verb} ${subject}` : `${verb} ${item.tool}`;
}

const RECENT_EVENTS = 2;

/** `activity` comes newest first; the balloon lists the last two agent events, oldest first. */
export function recentEventLines(newestFirst: readonly ActivityItem[]): string[] {
  return newestFirst
    .flatMap((item) => describeActivity(item) ?? [])
    .slice(0, RECENT_EVENTS)
    .reverse();
}
