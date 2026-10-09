import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { SessionListItem } from '../../api/types';
import HistoryRow from './HistoryRow.svelte';

const ITEM: SessionListItem = {
  id: 'session-1',
  title: 'Paginação na listagem de pedidos',
  customName: null,
  firstPrompt: 'Adicione paginação',
  project: 'web-dashboard',
  cwd: '/code/web-dashboard',
  branch: 'feat-orders-pagination',
  startedAt: null,
  updatedAt: null,
  pinned: false,
  pinOrder: null,
  category: null,
  tags: [],
  costUsd: 0.5,
  filesCount: 2,
};

function renderRow(overrides: Partial<SessionListItem> = {}) {
  const handlers = { onSelect: vi.fn(), onRename: vi.fn(), onTogglePin: vi.fn() };
  render(HistoryRow, { item: { ...ITEM, ...overrides }, selected: false, ...handlers });
  return handlers;
}

describe('HistoryRow', () => {
  it('selects the session when clicked', async () => {
    const handlers = renderRow();
    await fireEvent.click(screen.getByText('Paginação na listagem de pedidos'));
    expect(handlers.onSelect).toHaveBeenCalledWith('session-1');
  });

  it('renames on Enter and clears the custom name when emptied', async () => {
    const handlers = renderRow();
    await fireEvent.click(screen.getByRole('button', { name: 'Renomear' }));
    const input = screen.getByRole('textbox', { name: 'Novo nome da sessão' });
    await fireEvent.input(input, { target: { value: 'Pedidos paginados' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(handlers.onRename).toHaveBeenCalledWith('session-1', 'Pedidos paginados');

    await fireEvent.click(screen.getByRole('button', { name: 'Renomear' }));
    const again = screen.getByRole('textbox', { name: 'Novo nome da sessão' });
    await fireEvent.input(again, { target: { value: '   ' } });
    await fireEvent.keyDown(again, { key: 'Enter' });
    expect(handlers.onRename).toHaveBeenLastCalledWith('session-1', null);
  });

  it('cancels renaming on Escape', async () => {
    const handlers = renderRow();
    await fireEvent.click(screen.getByRole('button', { name: 'Renomear' }));
    await fireEvent.keyDown(screen.getByRole('textbox', { name: 'Novo nome da sessão' }), { key: 'Escape' });
    expect(handlers.onRename).not.toHaveBeenCalled();
    expect(screen.getByText('Paginação na listagem de pedidos')).toBeInTheDocument();
  });

  it('toggles pin with an accessible pressed state', async () => {
    const handlers = renderRow({ pinned: true });
    const pin = screen.getByRole('button', { name: 'Desafixar' });
    expect(pin).toHaveAttribute('aria-pressed', 'true');
    await fireEvent.click(pin);
    expect(handlers.onTogglePin).toHaveBeenCalledWith('session-1');
  });

  it('highlights search matches without rendering HTML', () => {
    render(HistoryRow, {
      item: ITEM,
      selected: false,
      matchedIn: 'text',
      snippet: '…os «testes» <b>passando</b>',
      onSelect: vi.fn(),
      onRename: vi.fn(),
      onTogglePin: vi.fn(),
    });
    expect(screen.getByText('testes').tagName).toBe('MARK');
    expect(screen.getByText(/<b>passando<\/b>/)).toBeInTheDocument();
    expect(screen.getByText('conversa')).toBeInTheDocument();
  });
});
