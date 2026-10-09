import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import { MOCK_LIVE_SESSIONS } from '../../mock/sessions';
import CommandPalette from './CommandPalette.svelte';

vi.mock('../../api/commands', () => ({ searchLibrary: vi.fn(async () => []) }));

const ACTIONS = [
  { id: 'new-session', label: 'Nova sessão', keywords: '', shortcut: '⌘N' },
  { id: 'settings', label: 'Configurações', keywords: '', shortcut: '⌘,' },
];

describe('CommandPalette', () => {
  it('navigates with the arrows and chooses with Enter', async () => {
    const onChoose = vi.fn();
    render(CommandPalette, { actions: ACTIONS, live: MOCK_LIVE_SESSIONS.slice(0, 1), history: [], onChoose, onClose: vi.fn() });
    const input = screen.getByRole('textbox', { name: 'Buscar' });
    expect(screen.getByRole('option', { name: /Nova sessão/ })).toHaveAttribute('aria-selected', 'true');
    await fireEvent.keyDown(input, { key: 'ArrowDown' });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(onChoose).toHaveBeenCalledWith(expect.objectContaining({ label: 'Configurações' }));
  });

  it('filters as you type and closes on Escape', async () => {
    const onClose = vi.fn();
    render(CommandPalette, { actions: ACTIONS, live: MOCK_LIVE_SESSIONS.slice(0, 1), history: [], onChoose: vi.fn(), onClose });
    const input = screen.getByRole('textbox', { name: 'Buscar' });
    await fireEvent.input(input, { target: { value: 'paginacao' } });
    expect(screen.getByRole('option', { name: /Paginação na listagem de pedidos/ })).toBeInTheDocument();
    expect(screen.queryByRole('option', { name: /Configurações/ })).not.toBeInTheDocument();
    await fireEvent.keyDown(input, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledOnce();
  });
});
