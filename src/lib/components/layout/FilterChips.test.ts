import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import FilterChips from './FilterChips.svelte';

const COUNTS = { all: 5, needsYou: 2, working: 1, done: 1, idle: 1 };

describe('FilterChips', () => {
  it('renders every filter with its count', () => {
    render(FilterChips, { active: 'all', counts: COUNTS, onSelect: vi.fn() });
    expect(screen.getByRole('radio', { name: /Precisam de você\s*2/ })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /Todas\s*5/ })).toHaveAttribute('aria-checked', 'true');
  });

  it('reports the selected filter', async () => {
    const onSelect = vi.fn();
    render(FilterChips, { active: 'all', counts: COUNTS, onSelect });
    await fireEvent.click(screen.getByRole('radio', { name: /Concluídas/ }));
    expect(onSelect).toHaveBeenCalledWith('done');
  });
});
