import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { WorktreeInfo, WorktreePlan } from '../../api/types';
import WorktreeSection from './WorktreeSection.svelte';

const PLAN: WorktreePlan = {
  repoRoot: '/code/web-dashboard',
  repoName: 'web-dashboard',
  root: '/Users/dev/worktrees',
  rootInferred: true,
  path: '/Users/dev/worktrees/web-dashboard-orders',
  branch: 'feat-orders',
  base: 'main',
};

const WORKTREES: WorktreeInfo[] = [
  { path: '/code/web-dashboard', branch: 'main', isMain: true, detached: false, clean: true, inUse: false },
  { path: '/Users/dev/worktrees/web-dashboard-clean', branch: 'fix-a', isMain: false, detached: false, clean: true, inUse: false },
  { path: '/Users/dev/worktrees/web-dashboard-dirty', branch: 'fix-b', isMain: false, detached: false, clean: false, inUse: false },
  { path: '/Users/dev/worktrees/web-dashboard-busy', branch: 'fix-c', isMain: false, detached: false, clean: true, inUse: true },
];

const removeWorktree = vi.fn(async (_cwd: string, _path: string) => undefined);

vi.mock('../../api/commands', () => ({
  planWorktree: vi.fn(async () => PLAN),
  listWorktrees: vi.fn(async () => WORKTREES),
  removeWorktree: (cwd: string, path: string) => removeWorktree(cwd, path),
}));

async function flush(): Promise<void> {
  await vi.runAllTimersAsync();
  await tick();
}

describe('WorktreeSection', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('previews branch, base and folder before anything is created', async () => {
    const onPlan = vi.fn();
    render(WorktreeSection, { cwd: '/code/web-dashboard', prefixes: ['feat-', 'fix-'], enabled: true, prefix: 'feat-', name: 'orders', onPlan, onSelectFolder: vi.fn() });
    await flush();
    expect(screen.getByText('feat-orders')).toBeInTheDocument();
    expect(screen.getByText('/Users/dev/worktrees/web-dashboard-orders')).toBeInTheDocument();
    expect(screen.getByText(/Nada é criado antes da confirmação/)).toBeInTheDocument();
    expect(onPlan).toHaveBeenLastCalledWith(PLAN);
  });

  it('only lets clean, unused worktrees be removed, after a confirmation', async () => {
    render(WorktreeSection, { cwd: '/code/web-dashboard', prefixes: ['feat-'], enabled: false, prefix: 'feat-', name: '', onPlan: vi.fn(), onSelectFolder: vi.fn() });
    await flush();
    expect(screen.getByRole('button', { name: 'Remover worktree web-dashboard-dirty' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Remover worktree web-dashboard-busy' })).toBeDisabled();
    await fireEvent.click(screen.getByRole('button', { name: 'Remover worktree web-dashboard-clean' }));
    expect(removeWorktree).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Sim' }));
    await flush();
    expect(removeWorktree).toHaveBeenCalledWith('/code/web-dashboard', '/Users/dev/worktrees/web-dashboard-clean');
  });
});
