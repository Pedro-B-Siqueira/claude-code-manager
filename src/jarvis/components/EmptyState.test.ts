import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import EmptyState from './EmptyState.svelte';

describe('EmptyState', () => {
  it('explains the empty hologram and opens a new session', async () => {
    const onNewSession = vi.fn();
    render(EmptyState, { onNewSession });
    expect(screen.getByRole('heading', { name: 'Nenhuma sessão ativa' })).toBeInTheDocument();
    expect(screen.getByText('Quando você abrir uma sessão do Claude Code, ela aparece aqui e o holograma vai se formando em volta do projeto.')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Nova sessão' }));
    expect(onNewSession).toHaveBeenCalledOnce();
  });
});
