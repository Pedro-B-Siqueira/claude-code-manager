import type { ShapeRule } from '../scene/shape-rules';
import { STATUS_ORDER, type JarvisSession, type JarvisStatus } from './jarvis-session';

/** Mock sessions carry their recent events; real ones fetch them when a balloon expands. */
export interface MockSession extends JarvisSession {
  events: string[];
}

export interface MockData {
  sessions: MockSession[];
  rules: ShapeRule[];
}

const DEMO_RULES: ShapeRule[] = [
  { match: 'atlas-*', shape: 'tower' },
  { match: '*lens*', shape: 'eye' },
  { match: 'harbor-*', shape: 'fortress' },
  { match: 'orbit-*', shape: 'orb' },
];

/** The four sessions of the design, with invented repository names. */
export function designMocks(now: number): MockData {
  const sessions: MockSession[] = [
    {
      id: 'design-1',
      repo: 'atlas-web',
      name: 'remove-legacy-default-template',
      status: 'working',
      action: 'Bash · yarn ngc --noEmit -p control-panel',
      branch: 'feat-remove-legacy-template',
      lastActivityAt: now - 283_000,
      events: ['Editou auth-link-tab.component.ts', 'Removeu legacy-default-template.json'],
    },
    {
      id: 'design-2',
      repo: 'lens-api',
      name: 'timeline-filtros',
      status: 'working',
      action: 'Edit · timeline-filter.component.ts',
      branch: 'feat-timeline-filters',
      lastActivityAt: now - 72_000,
      events: ['Leu timeline.service.ts', 'Rodou yarn lint'],
    },
    {
      id: 'design-3',
      repo: 'atlas-web',
      name: 'Planejamento (meta outubro)',
      status: 'waiting',
      action: 'Quer que eu fique acompanhando e rode de novo?',
      branch: 'feat-timeline-redesign',
      lastActivityAt: now - 99_000,
      events: ['Atualizou andamento.md', 'Rodou seed-schedule-fixes.mjs'],
    },
    {
      id: 'design-4',
      repo: 'harbor-app',
      name: 'vehicle-redesign-prs-7-8',
      status: 'done',
      action: 'PR 8 enviado (d60d09)',
      branch: 'feat-redesign-vehicle-detail',
      lastActivityAt: now - 109_000,
      events: ['Escreveu commit-message.txt', 'Enviou o PR 8'],
    },
  ];
  return { sessions, rules: DEMO_RULES };
}

const FIRST_REPO = 'atlas-web';
const EXTRA_REPOS = [FIRST_REPO, 'lens-api', 'harbor-app', 'orbit-ui', 'notes', 'dotfiles', 'shop'];

const MOCK_ACTIONS: Record<JarvisStatus, string | null> = {
  working: 'Bash · npm test -- --watch=false',
  waiting: 'Posso seguir com a migração do banco?',
  done: 'Testes passando (42 de 42)',
  idle: null,
};

function nextMockNumber(sessions: readonly MockSession[]): number {
  const numbers = sessions.map((session) => Number(/^mock-(\d+)$/.exec(session.id)?.[1] ?? 0));
  return Math.max(0, ...numbers) + 1;
}

/** Joins the latest project, opening a new one with the first session and then every third. */
export function addMockSession(sessions: readonly MockSession[], now: number): MockSession[] {
  const repos = [...new Set(sessions.map((session) => session.repo))];
  const opensProject = sessions.length === 0 || sessions.length % 3 === 2;
  const repo = opensProject ? (EXTRA_REPOS.find((candidate) => !repos.includes(candidate)) ?? `repo-${repos.length + 1}`) : (sessions.at(-1)?.repo ?? FIRST_REPO);
  const number = nextMockNumber(sessions);
  const added: MockSession = {
    id: `mock-${number}`,
    repo,
    name: `sessao-${number}`,
    status: 'working',
    action: MOCK_ACTIONS.working,
    branch: `feat-mock-${number}`,
    lastActivityAt: now,
    events: ['Leu README.md', 'Rodou npm test'],
  };
  return [...sessions, added];
}

export function removeMockSession(sessions: readonly MockSession[]): MockSession[] {
  return sessions.slice(0, -1);
}

export function cycleMockStatus(sessions: readonly MockSession[], id: string, now: number): MockSession[] {
  return sessions.map((session) => {
    if (session.id !== id) return session;
    const status = STATUS_ORDER[(STATUS_ORDER.indexOf(session.status) + 1) % STATUS_ORDER.length] ?? 'working';
    return { ...session, status, action: MOCK_ACTIONS[status], lastActivityAt: now };
  });
}
