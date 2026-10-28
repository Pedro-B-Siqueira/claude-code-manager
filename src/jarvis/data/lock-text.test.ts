import { describe, expect, it } from 'vitest';
import type { JarvisSession, JarvisStatus } from './jarvis-session';
import { formatClock, lockDelayWarning, lockedSummary } from './lock-text';

function session(id: string, status: JarvisStatus): JarvisSession {
  return { id, repo: 'web', name: id, status, action: null, branch: null, lastActivityAt: null };
}

describe('lockedSummary', () => {
  it('counts the sessions and only mentions the waiting ones when there are any', () => {
    expect(lockedSummary([session('a', 'working'), session('b', 'waiting'), session('c', 'done'), session('d', 'waiting')])).toEqual({ sessions: '4 sessões', waiting: '2 esperando você' });
    expect(lockedSummary([session('a', 'working')])).toEqual({ sessions: '1 sessão', waiting: null });
    expect(lockedSummary([])).toEqual({ sessions: 'Nenhuma sessão', waiting: null });
  });
});

describe('formatClock', () => {
  it('shows hours and minutes on a 24-hour clock', () => {
    expect(formatClock(new Date(2026, 9, 27, 16, 13, 59))).toBe('16:13');
    expect(formatClock(new Date(2026, 9, 27, 7, 5))).toBe('07:05');
  });
});

describe('lockDelayWarning', () => {
  it('says nothing when the Mac asks for the password at once', () => {
    expect(lockDelayWarning({ kind: 'immediate' })).toBeNull();
  });

  it('explains the gap the watchdog leaves otherwise', () => {
    expect(lockDelayWarning({ kind: 'seconds', seconds: 60 })).toBe(
      'Seu Mac só pede a senha 60 s depois que a tela apaga. Se o app fechar durante o repouso, a área de trabalho fica acessível nesse intervalo. Ajuste em Ajustes do Sistema › Tela Bloqueada.',
    );
    expect(lockDelayWarning({ kind: 'off' })).toContain('não pede a senha');
    expect(lockDelayWarning({ kind: 'unknown' })).toContain('Não consegui ler');
  });
});
