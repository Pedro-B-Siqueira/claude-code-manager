import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import JarvisButton from './JarvisButton.svelte';

describe('JarvisButton', () => {
  it('opens the Jarvis mode and shows its shortcut', async () => {
    const onOpen = vi.fn();
    render(JarvisButton, { onOpen });
    await fireEvent.click(screen.getByRole('button', { name: 'Modo Jarvis' }));
    expect(onOpen).toHaveBeenCalledOnce();
    expect(screen.getByText('⌘J')).toBeInTheDocument();
  });
});
