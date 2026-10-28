import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { activateRepouso, closeJarvis, focusSessionFromJarvis, newSessionFromJarvis, unlockRepouso } from './api';
import JarvisApp from './JarvisApp.svelte';
import type { LockSnapshot } from './repouso-types';
import { JarvisScene } from './scene/jarvis-scene';

const LOCKED: LockSnapshot = { active: true, since: new Date(2026, 9, 27, 16, 13).getTime(), showDetails: false, authenticating: false };
const repousoListeners: Array<(snapshot: LockSnapshot) => void> = [];

vi.mock('./api', () => ({
  closeJarvis: vi.fn(async () => undefined),
  focusSessionFromJarvis: vi.fn(async () => undefined),
  newSessionFromJarvis: vi.fn(async () => undefined),
  fetchShapeRules: vi.fn(async () => []),
  fetchMockData: vi.fn(async () => null),
  onSessionAction: vi.fn(async () => () => undefined),
  prepareRepouso: vi.fn(async () => ({ lockDelay: { kind: 'immediate' }, showDetails: true })),
  activateRepouso: vi.fn(async () => LOCKED),
  unlockRepouso: vi.fn(async () => 'refused'),
  fetchRepousoState: vi.fn(async () => ({ active: false, since: null, showDetails: true, authenticating: false })),
  onRepousoChanged: vi.fn(async (handler: (snapshot: LockSnapshot) => void) => {
    repousoListeners.push(handler);
    return () => undefined;
  }),
}));

describe('JarvisApp', () => {
  beforeAll(() => {
    // jsdom has no canvas (the scene skips painting) and no ResizeObserver (used by bind:clientHeight).
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe(): void {}
        unobserve(): void {}
        disconnect(): void {}
      },
    );
  });

  beforeEach(() => {
    repousoListeners.length = 0;
    vi.mocked(closeJarvis).mockClear();
    vi.mocked(activateRepouso).mockClear();
    vi.mocked(unlockRepouso).mockClear();
    vi.mocked(focusSessionFromJarvis).mockClear();
    vi.mocked(newSessionFromJarvis).mockClear();
  });

  it('shows the mock sessions with a summary and one balloon each', async () => {
    render(JarvisApp, { search: '?source=mock', reducedMotion: true });
    expect(await screen.findByText('4 sessões em 3 projetos')).toBeInTheDocument();
    expect(screen.getByText('2 trabalhando')).toBeInTheDocument();
    expect(screen.getAllByRole('article')).toHaveLength(4);
  });

  it('leaves with Esc and with Sair', async () => {
    render(JarvisApp, { search: '?source=mock', reducedMotion: true });
    await fireEvent.keyDown(window, { key: 'Escape' });
    await fireEvent.click(screen.getByRole('button', { name: 'Sair' }));
    expect(closeJarvis).toHaveBeenCalledTimes(2);
  });

  it('expands a balloon with its recent events and opens the session in the Focus view', async () => {
    render(JarvisApp, { search: '?source=mock', reducedMotion: true });
    await fireEvent.click(await screen.findByRole('button', { name: /timeline-filtros/ }));
    expect(await screen.findByText('Rodou yarn lint')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Abrir sessão' }));
    expect(focusSessionFromJarvis).toHaveBeenCalledWith('design-2');
  });

  it('switches the automatic spin from the footer', async () => {
    render(JarvisApp, { search: '?source=mock', reducedMotion: true });
    const toggle = screen.getByRole('switch', { name: 'Girar sozinho' });
    await fireEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-checked', 'false');
  });

  it('locks into Repouso from the dialog and keeps Esc from leaving', async () => {
    render(JarvisApp, { search: '?source=mock', reducedMotion: true });
    await fireEvent.click(await screen.findByRole('button', { name: 'Repouso' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Ativar repouso' }));
    expect(activateRepouso).toHaveBeenCalledWith(true);
    expect(await screen.findByRole('heading', { name: 'Repouso ativo' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Sair' })).toBeNull();
    expect(screen.queryByRole('switch', { name: 'Girar sozinho' })).toBeNull();
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(closeJarvis).not.toHaveBeenCalled();
  });

  it('shows only names and statuses while locked with the details off', async () => {
    render(JarvisApp, { search: '?source=mock', reducedMotion: true });
    await screen.findByText('4 sessões em 3 projetos');
    repousoListeners.forEach((listener) => listener(LOCKED));
    await screen.findByRole('heading', { name: 'Repouso ativo' });
    expect(screen.queryByText('Edit · timeline-filter.component.ts')).toBeNull();
    expect(screen.queryByRole('button', { name: /timeline-filtros/ })).toBeNull();
  });

  it('reports a refused unlock and returns to the live view once unlocked', async () => {
    render(JarvisApp, { search: '?source=mock', reducedMotion: true });
    await screen.findByText('4 sessões em 3 projetos');
    repousoListeners.forEach((listener) => listener(LOCKED));
    await fireEvent.click(await screen.findByRole('button', { name: 'Desbloquear' }));
    expect(unlockRepouso).toHaveBeenCalledOnce();
    expect(await screen.findByRole('alert')).toHaveTextContent('Não foi possível confirmar sua identidade. Tente novamente.');
    repousoListeners.forEach((listener) => listener({ ...LOCKED, active: false, since: null }));
    expect(await screen.findByRole('button', { name: 'Sair' })).toBeInTheDocument();
  });

  it('pauses when the window loses focus, but keeps turning while locked in Repouso', async () => {
    const pause = vi.spyOn(JarvisScene.prototype, 'pause');
    const resume = vi.spyOn(JarvisScene.prototype, 'resume');
    render(JarvisApp, { search: '?source=mock', reducedMotion: true });
    await screen.findByText('4 sessões em 3 projetos');
    window.dispatchEvent(new Event('blur'));
    expect(pause).toHaveBeenCalledOnce();
    repousoListeners.forEach((listener) => listener(LOCKED));
    await screen.findByRole('heading', { name: 'Repouso ativo' });
    expect(resume).toHaveBeenCalled();
    pause.mockClear();
    window.dispatchEvent(new Event('blur'));
    expect(pause).not.toHaveBeenCalled();
    pause.mockRestore();
    resume.mockRestore();
  });

  it('invites a new session when nothing is running', async () => {
    render(JarvisApp, { search: '', reducedMotion: true });
    await fireEvent.click(await screen.findByRole('button', { name: 'Nova sessão' }));
    expect(newSessionFromJarvis).toHaveBeenCalledOnce();
  });
});
