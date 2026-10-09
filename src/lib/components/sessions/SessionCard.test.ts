import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { LiveSessionView } from '../../api/types';
import { MOCK_LIVE_SESSIONS } from '../../mock/sessions';
import SessionCard from './SessionCard.svelte';

vi.mock('../../api/commands', () => ({
  fetchGitStatus: vi.fn(async () => {
    throw new Error('git is not available in component tests');
  }),
  openVsCode: vi.fn(async () => ({ warning: null })),
  openFinder: vi.fn(async () => undefined),
  openPullRequest: vi.fn(async () => 'https://example.com/pull/1'),
}));

function mockSession(index: number): LiveSessionView {
  const session = MOCK_LIVE_SESSIONS[index];
  if (!session) throw new Error(`mock session ${index} missing`);
  return session;
}

describe('SessionCard', () => {
  it('shows title, repo, branch, worktree seal and file deltas', () => {
    render(SessionCard, { session: mockSession(0), onFocus: vi.fn() });
    expect(screen.getByRole('heading', { name: 'Paginação na listagem de pedidos' })).toBeInTheDocument();
    expect(screen.getByText('feat-orders-pagination')).toBeInTheDocument();
    expect(screen.getByText('worktree')).toBeInTheDocument();
    expect(screen.getByText('+12')).toBeInTheDocument();
    expect(screen.getByText('−3')).toBeInTheDocument();
  });

  it('shows the requested command when asking for permission', () => {
    render(SessionCard, { session: mockSession(1), onFocus: vi.fn() });
    expect(screen.getByText('$ npm run db:migrate')).toBeInTheDocument();
  });

  it('turns the context meter amber from 75%', () => {
    const { container } = render(SessionCard, { session: mockSession(1), onFocus: vi.fn() });
    expect(container.querySelector('.meter')).toHaveClass('high');
  });

  it('opens the session when the card is clicked anywhere but its controls', async () => {
    const onFocus = vi.fn();
    render(SessionCard, { session: mockSession(0), onFocus });
    expect(screen.queryByRole('button', { name: /Focar/ })).not.toBeInTheDocument();
    await fireEvent.click(screen.getByText('feat-orders-pagination'));
    expect(onFocus).toHaveBeenCalledOnce();
    expect(onFocus).toHaveBeenCalledWith('mock-1');
    await fireEvent.click(screen.getByRole('button', { name: 'Abrir no VS Code' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Mais ações' }));
    expect(onFocus).toHaveBeenCalledOnce();
  });

  it('opens the session from the keyboard through the title, once', async () => {
    const onFocus = vi.fn();
    render(SessionCard, { session: mockSession(0), onFocus });
    await fireEvent.click(screen.getByRole('button', { name: 'Paginação na listagem de pedidos' }));
    expect(onFocus).toHaveBeenCalledOnce();
  });

  it('does not open the session after the card was dragged', async () => {
    const onFocus = vi.fn();
    render(SessionCard, { session: mockSession(0), onFocus });
    const repo = screen.getByText('feat-orders-pagination');
    await fireEvent.mouseDown(repo, { clientX: 10, clientY: 10 });
    await fireEvent.click(repo, { clientX: 60, clientY: 14 });
    expect(onFocus).not.toHaveBeenCalled();
  });

  it('counts every touched file, not only the ones listed on the card', () => {
    render(SessionCard, { session: { ...mockSession(0), filesTotal: 9 }, onFocus: vi.fn() });
    expect(screen.getByText('+6 arquivos')).toBeInTheDocument();
  });

  it('shows the tokens Claude wrote and no dollar value', () => {
    render(SessionCard, { session: { ...mockSession(0), outputTokens: 1_284_000 }, onFocus: vi.fn() });
    expect(screen.getByText('1,3 mi tok escritos')).toBeInTheDocument();
    expect(screen.queryByText(/US\$/)).not.toBeInTheDocument();
  });
});
