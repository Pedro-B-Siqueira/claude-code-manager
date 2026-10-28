/** The four colors of the hologram. "Pedindo permissão" from the app joins `waiting`. */
export type JarvisStatus = 'working' | 'waiting' | 'done' | 'idle';

export interface JarvisSession {
  /** Live session key. */
  id: string;
  repo: string;
  name: string;
  status: JarvisStatus;
  /** What the agent is doing now: tool · target, the question it asked or the last line of its result. */
  action: string | null;
  branch: string | null;
  /** Epoch ms of the last activity, for the elapsed time in the balloon. */
  lastActivityAt: number | null;
}

/** Most urgent first: a project takes the color of its most urgent session. */
export const STATUS_PRIORITY: Record<JarvisStatus, number> = { waiting: 3, working: 2, done: 1, idle: 0 };

export const STATUS_ORDER: readonly JarvisStatus[] = ['working', 'waiting', 'done', 'idle'];

export const STATUS_LABELS: Record<JarvisStatus, string> = {
  working: 'Trabalhando',
  waiting: 'Esperando você',
  done: 'Concluída',
  idle: 'Ociosa',
};
