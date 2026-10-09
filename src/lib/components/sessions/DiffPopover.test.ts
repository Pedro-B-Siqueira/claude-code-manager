import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { EditDetail } from '../../api/types';
import { diffPopoverStore } from '../../stores/diff-popover.svelte';
import DiffPopover from './DiffPopover.svelte';
import FileChangeList from './FileChangeList.svelte';

const EDIT: EditDetail = {
  id: 7,
  tool: 'Edit',
  filePath: '/repo/src/orders/use-orders.ts',
  timestamp: 0,
  newStart: 20,
  newEnd: 25,
  added: 3,
  removed: 1,
  isNewFile: false,
  why: 'Vou adicionar a paginação no hook de pedidos.',
  lines: [
    { kind: 'context', text: 'export function useOrders() {' },
    { kind: 'removed', text: 'const page = 1;' },
    { kind: 'added', text: 'const page = params.page;' },
  ],
  truncated: false,
};

vi.mock('../../api/commands', () => ({
  fetchFileEdits: vi.fn(async () => [EDIT]),
  fetchEdit: vi.fn(async () => EDIT),
}));

async function flush(): Promise<void> {
  await vi.runAllTimersAsync();
  await tick();
}

describe('diff popover', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => {
    diffPopoverStore.hide();
    vi.useRealTimers();
  });

  it('opens from keyboard focus with path, range, why, diff and the zero-token footer', async () => {
    render(DiffPopover);
    render(FileChangeList, {
      files: [{ path: EDIT.filePath, added: 3, removed: 1 }],
      sessionId: 'session-1',
      basePath: '/repo',
    });
    await fireEvent.focus(screen.getByRole('button', { name: /use-orders\.ts/ }));
    await flush();

    const popover = screen.getByRole('dialog', { name: 'Diff da edição' });
    expect(popover).toHaveTextContent('src/orders/use-orders.ts');
    expect(popover).toHaveTextContent('linhas 20–25');
    expect(popover).toHaveTextContent('Vou adicionar a paginação no hook de pedidos.');
    expect(popover).toHaveTextContent(/−\s*const page = 1;/);
    expect(popover).toHaveTextContent(/\+\s*const page = params\.page;/);
    expect(popover).toHaveTextContent('Lido do transcript local · 0 tokens');
  });

  it('closes on Escape', async () => {
    render(DiffPopover);
    render(FileChangeList, { files: [{ path: EDIT.filePath, added: 3, removed: 1 }], sessionId: 'session-1', basePath: '/repo' });
    const file = screen.getByRole('button', { name: /use-orders\.ts/ });
    await fireEvent.focus(file);
    await flush();
    await fireEvent.keyDown(file, { key: 'Escape' });
    await tick();
    expect(screen.queryByRole('dialog', { name: 'Diff da edição' })).not.toBeInTheDocument();
  });

  it('shows new files without a line range', async () => {
    render(DiffPopover);
    diffPopoverStore.open = true;
    diffPopoverStore.edits = [{ ...EDIT, isNewFile: true, newStart: null, newEnd: null }];
    await tick();
    expect(screen.getByRole('dialog', { name: 'Diff da edição' })).toHaveTextContent('arquivo novo');
  });

  it('does nothing without a session to read from', async () => {
    render(DiffPopover);
    render(FileChangeList, { files: [{ path: EDIT.filePath, added: 3, removed: 1 }] });
    await fireEvent.focus(screen.getByRole('button', { name: /use-orders\.ts/ }));
    await flush();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
