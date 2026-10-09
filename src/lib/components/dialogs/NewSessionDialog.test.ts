import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { LiveSessionView, WorktreePlan } from '../../api/types';
import { MOCK_LIVE_SESSIONS } from '../../mock/sessions';
import NewSessionDialog from './NewSessionDialog.svelte';

const REPO = '/code/web-dashboard';

const PLAN: WorktreePlan = {
  repoRoot: REPO,
  repoName: 'web-dashboard',
  root: '/Users/dev/worktrees',
  rootInferred: true,
  path: '/Users/dev/worktrees/web-dashboard-orders',
  branch: 'feat-orders',
  base: 'main',
};

function openedSession(): LiveSessionView {
  const session = MOCK_LIVE_SESSIONS[0];
  if (!session) throw new Error('mock data missing');
  return session;
}

const newSession = vi.fn(async (_cwd: string) => openedSession());
const createWorktree = vi.fn(async (_cwd: string, _prefix: string, _name: string) => openedSession());

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(async () => null) }));
vi.mock('../../api/commands', () => ({
  fetchRecentDirs: vi.fn(async () => [REPO]),
  fetchLiveSessions: vi.fn(async () => []),
  listWorktrees: vi.fn(async () => []),
  planWorktree: vi.fn(async () => PLAN),
  newSession: (cwd: string) => newSession(cwd),
  createWorktree: (cwd: string, prefix: string, name: string) => createWorktree(cwd, prefix, name),
}));

async function flush(): Promise<void> {
  await vi.runAllTimersAsync();
  await tick();
}

describe('NewSessionDialog', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    newSession.mockClear();
    createWorktree.mockClear();
  });
  afterEach(() => vi.useRealTimers());

  it('opens a plain session in the selected folder', async () => {
    const onOpened = vi.fn();
    render(NewSessionDialog, { onClose: vi.fn(), onOpened });
    await flush();
    expect(screen.getByRole('radio', { name: /web-dashboard/ })).toHaveAttribute('aria-checked', 'true');
    await fireEvent.click(screen.getByRole('button', { name: 'Abrir sessão' }));
    await flush();
    expect(newSession).toHaveBeenCalledWith(REPO);
    expect(createWorktree).not.toHaveBeenCalled();
    expect(onOpened).toHaveBeenCalledWith(openedSession().key);
  });

  it('creates a worktree only after the exact command is confirmed', async () => {
    const onOpened = vi.fn();
    render(NewSessionDialog, { onClose: vi.fn(), onOpened });
    await flush();
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Isolar em um worktree novo' }));
    await fireEvent.input(screen.getByRole('textbox', { name: 'Nome da branch' }), { target: { value: 'orders' } });
    await flush();

    await fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
    expect(screen.getByText(`git -C ${REPO} worktree add -b feat-orders ${PLAN.path} main`)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Voltar' }));
    await flush();
    expect(createWorktree).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
    const confirm = screen.getByRole('button', { name: 'Criar worktree e abrir' });
    await fireEvent.click(confirm);
    await fireEvent.click(confirm);
    await flush();
    expect(createWorktree).toHaveBeenCalledOnce();
    expect(createWorktree).toHaveBeenCalledWith(REPO, 'feat-', 'orders');
    expect(newSession).not.toHaveBeenCalled();
    expect(onOpened).toHaveBeenCalledWith(openedSession().key);
  });

  it('keeps the dialog open with the reason when the session fails to start', async () => {
    newSession.mockRejectedValueOnce({ kind: 'claudeNotFound', message: 'claude não encontrado' });
    const onOpened = vi.fn();
    render(NewSessionDialog, { onClose: vi.fn(), onOpened });
    await flush();
    await fireEvent.click(screen.getByRole('button', { name: 'Abrir sessão' }));
    await flush();
    expect(screen.getByRole('alert')).toHaveTextContent('Não foi possível abrir a sessão');
    expect(onOpened).not.toHaveBeenCalled();
  });
});
