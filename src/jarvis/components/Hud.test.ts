import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Hud from './Hud.svelte';

const SUMMARY = {
  headline: '4 sessões em 3 projetos',
  tags: [
    { status: 'working' as const, label: '2 trabalhando' },
    { status: 'waiting' as const, label: '1 esperando você' },
  ],
};

describe('Hud', () => {
  it('shows the title, the summary and only the status tags it gets', () => {
    render(Hud, { summary: SUMMARY, canLock: true, onLock: vi.fn(), onExit: vi.fn() });
    expect(screen.getByRole('heading', { name: 'Modo Jarvis' })).toBeInTheDocument();
    expect(screen.getByText('4 sessões em 3 projetos')).toBeInTheDocument();
    expect(screen.getAllByRole('listitem').map((item) => item.textContent?.trim())).toEqual(['2 trabalhando', '1 esperando você']);
  });

  it('offers Repouso, Sair and the Esc hint', async () => {
    const onLock = vi.fn();
    const onExit = vi.fn();
    render(Hud, { summary: SUMMARY, canLock: true, onLock, onExit });
    await fireEvent.click(screen.getByRole('button', { name: 'Repouso' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Sair' }));
    expect(onLock).toHaveBeenCalledOnce();
    expect(onExit).toHaveBeenCalledOnce();
    expect(screen.getByText('Esc')).toBeInTheDocument();
  });

  it('hides Repouso where the system cannot lock the screen', () => {
    render(Hud, { summary: { headline: 'Nenhuma sessão', tags: [] }, canLock: false, onLock: vi.fn(), onExit: vi.fn() });
    expect(screen.queryByRole('button', { name: 'Repouso' })).toBeNull();
    expect(screen.queryByRole('list')).toBeNull();
  });
});
