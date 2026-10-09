import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import type { SessionStatus } from '../../api/types';
import { STATUS_LABELS } from '../../sessions/status';
import StatusBadge from './StatusBadge.svelte';

const ALL_STATUSES: SessionStatus[] = ['working', 'permission', 'waiting', 'done', 'idle'];

describe('StatusBadge', () => {
  it.each(ALL_STATUSES)('shows a text label for %s, never color alone', (status) => {
    render(StatusBadge, { status });
    expect(screen.getByText(STATUS_LABELS[status])).toBeInTheDocument();
  });

  it('marks hibernated sessions in the label', () => {
    render(StatusBadge, { status: 'idle', hibernated: true });
    expect(screen.getByText('Ociosa · hibernada')).toBeInTheDocument();
  });
});
