import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { LiveSessionView } from '../../api/types';
import { MOCK_LIVE_SESSIONS } from '../../mock/sessions';
import SessionCard from './SessionCard.svelte';

function session(overrides: Partial<LiveSessionView>): LiveSessionView {
  const base = MOCK_LIVE_SESSIONS[0];
  if (!base) throw new Error('mock data missing');
  return { ...base, ...overrides };
}

describe('session card menu', () => {
  it('offers ending a running session', async () => {
    const onEnd = vi.fn();
    render(SessionCard, { session: session({}), onFocus: vi.fn(), onEnd, onResume: vi.fn(), onRemove: vi.fn() });
    await fireEvent.click(screen.getByRole('button', { name: 'Mais ações' }));
    expect(screen.queryByRole('menuitem', { name: 'Retomar sessão' })).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Encerrar sessão' }));
    expect(onEnd).toHaveBeenCalledOnce();
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('offers resuming or removing an exited session', async () => {
    const onResume = vi.fn();
    const onRemove = vi.fn();
    render(SessionCard, { session: session({ exited: true }), onFocus: vi.fn(), onEnd: vi.fn(), onResume, onRemove });
    expect(screen.getByText('encerrada')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Mais ações' }));
    expect(screen.queryByRole('menuitem', { name: 'Encerrar sessão' })).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Retomar sessão' }));
    expect(onResume).toHaveBeenCalledOnce();
    await fireEvent.click(screen.getByRole('button', { name: 'Mais ações' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Remover da grade' }));
    expect(onRemove).toHaveBeenCalledOnce();
  });
});
