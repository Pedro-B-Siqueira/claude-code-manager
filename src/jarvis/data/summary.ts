import { STATUS_ORDER, type JarvisSession, type JarvisStatus } from './jarvis-session';

export interface StatusTag {
  status: JarvisStatus;
  label: string;
}

export interface SceneSummary {
  headline: string;
  tags: StatusTag[];
}

const TAG_WORDS: Record<JarvisStatus, [singular: string, plural: string]> = {
  working: ['trabalhando', 'trabalhando'],
  waiting: ['esperando você', 'esperando você'],
  done: ['concluída', 'concluídas'],
  idle: ['ociosa', 'ociosas'],
};

function plural(count: number, singular: string, pluralForm: string): string {
  return `${count} ${count === 1 ? singular : pluralForm}`;
}

export function summarizeSessions(sessions: readonly JarvisSession[]): SceneSummary {
  if (sessions.length === 0) return { headline: 'Nenhuma sessão', tags: [] };
  const projectCount = new Set(sessions.map((session) => session.repo)).size;
  const headline = `${plural(sessions.length, 'sessão', 'sessões')} em ${plural(projectCount, 'projeto', 'projetos')}`;
  const tags = STATUS_ORDER.flatMap((status) => {
    const count = sessions.filter((session) => session.status === status).length;
    const [singular, pluralForm] = TAG_WORDS[status];
    return count === 0 ? [] : [{ status, label: plural(count, singular, pluralForm) }];
  });
  return { headline, tags };
}

export function formatElapsed(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}h ${String(minutes).padStart(2, '0')}m`;
  if (minutes > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
}

export interface ProjectLabelText {
  repo: string;
  count: string;
}

/** Text under each project's base: the repo and how many sessions orbit it. */
export function projectLabels(sessions: readonly JarvisSession[]): ProjectLabelText[] {
  const counts = new Map<string, number>();
  for (const session of sessions) counts.set(session.repo, (counts.get(session.repo) ?? 0) + 1);
  return [...counts].map(([repo, count]) => ({ repo, count: plural(count, 'sessão', 'sessões') }));
}
