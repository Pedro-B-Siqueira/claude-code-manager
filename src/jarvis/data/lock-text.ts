import type { LockDelay } from '../repouso-types';
import type { JarvisSession } from './jarvis-session';

export interface LockedSummary {
  sessions: string;
  waiting: string | null;
}

/** The top right of the locked screen: how many sessions, and how many wait for the user. */
export function lockedSummary(sessions: readonly JarvisSession[]): LockedSummary {
  const waiting = sessions.filter((session) => session.status === 'waiting').length;
  const count = sessions.length;
  return {
    sessions: count === 0 ? 'Nenhuma sessão' : `${count} ${count === 1 ? 'sessão' : 'sessões'}`,
    waiting: waiting === 0 ? null : `${waiting} esperando você`,
  };
}

export function formatClock(date: Date): string {
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;
}

const SETTINGS_HINT = 'Ajuste em Ajustes do Sistema › Tela Bloqueada.';

/** Shown in the activation dialog when the watchdog could not lock the Mac at once. */
export function lockDelayWarning(delay: LockDelay): string | null {
  switch (delay.kind) {
    case 'immediate':
      return null;
    case 'seconds':
      return `Seu Mac só pede a senha ${delay.seconds} s depois que a tela apaga. Se o app fechar durante o repouso, a área de trabalho fica acessível nesse intervalo. ${SETTINGS_HINT}`;
    case 'off':
      return `Seu Mac não pede a senha quando a tela apaga. Se o app fechar durante o repouso, a área de trabalho fica acessível. ${SETTINGS_HINT}`;
    case 'unknown':
      return `Não consegui ler quando seu Mac pede a senha. Se o app fechar durante o repouso, a área de trabalho pode ficar acessível. ${SETTINGS_HINT}`;
  }
}
