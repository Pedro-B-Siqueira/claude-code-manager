import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import LockedHeader from './LockedHeader.svelte';

describe('LockedHeader', () => {
  it('says since when Repouso is on and how many sessions wait for the user', () => {
    render(LockedHeader, { since: new Date(2026, 9, 27, 16, 13).getTime(), summary: { sessions: '4 sessões', waiting: '1 esperando você' } });
    expect(screen.getByRole('heading', { name: 'Repouso ativo' })).toBeInTheDocument();
    expect(screen.getByText('desde 16:13')).toBeInTheDocument();
    expect(screen.getByText('4 sessões')).toBeInTheDocument();
    expect(screen.getByText('1 esperando você')).toBeInTheDocument();
  });

  it('leaves the waiting tag out when nobody waits', () => {
    render(LockedHeader, { since: 0, summary: { sessions: '2 sessões', waiting: null } });
    expect(screen.queryByText(/esperando você/)).toBeNull();
  });
});
