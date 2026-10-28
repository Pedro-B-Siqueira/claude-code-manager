import { describe, expect, it } from 'vitest';
import { createLockScreen } from './lock-screen';

describe('createLockScreen', () => {
  it('shows the clock, who locked the screen, the summary and how to unlock', () => {
    const screen = createLockScreen(document);
    document.body.append(screen.root);
    screen.update({ sessions: '4 sessões', waiting: '1 esperando você' }, new Date(2026, 9, 27, 16, 13));
    const text = screen.root.textContent ?? '';
    expect(text).toContain('16:13');
    expect(text).toContain('Bloqueado pelo Claude Code Manager');
    expect(text).toContain('4 sessões');
    expect(text).toContain('1 esperando você');
    expect(text).toContain('Desbloqueie na tela principal com o Touch ID ou a senha do Mac.');
  });

  it('hides the waiting tag when nobody waits', () => {
    const screen = createLockScreen(document);
    screen.update({ sessions: '1 sessão', waiting: '2 esperando você' }, new Date());
    screen.update({ sessions: '1 sessão', waiting: null }, new Date());
    expect(screen.root.textContent).not.toContain('esperando você');
  });
});
