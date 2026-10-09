import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { LiveSessionView } from '../../api/types';
import { MOCK_LIVE_SESSIONS } from '../../mock/sessions';
import SessionCard from './SessionCard.svelte';

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

  it('calls onFocus with the session key', async () => {
    const onFocus = vi.fn();
    render(SessionCard, { session: mockSession(0), onFocus });
    await fireEvent.click(screen.getByRole('button', { name: /Focar/ }));
    expect(onFocus).toHaveBeenCalledWith('mock-1');
  });
});
