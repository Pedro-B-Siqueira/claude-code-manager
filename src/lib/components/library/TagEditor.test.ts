import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import TagEditor from './TagEditor.svelte';

describe('TagEditor', () => {
  it('adds a tag on Enter, ignoring case-insensitive duplicates', async () => {
    const onChange = vi.fn();
    render(TagEditor, { tags: ['bug'], suggestions: [], onChange });
    const input = screen.getByRole('combobox', { name: 'Adicionar tag' });
    await fireEvent.input(input, { target: { value: 'review' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(onChange).toHaveBeenCalledWith(['bug', 'review']);

    await fireEvent.input(input, { target: { value: 'BUG' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it('removes a tag with its button', async () => {
    const onChange = vi.fn();
    render(TagEditor, { tags: ['bug', 'review'], suggestions: [], onChange });
    await fireEvent.click(screen.getByRole('button', { name: 'Remover tag bug' }));
    expect(onChange).toHaveBeenCalledWith(['review']);
  });
});
