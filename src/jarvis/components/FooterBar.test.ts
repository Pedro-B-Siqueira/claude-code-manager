import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import FooterBar from './FooterBar.svelte';

describe('FooterBar', () => {
  it('explains the gestures and switches the automatic spin', async () => {
    const onAutoRotateChange = vi.fn();
    render(FooterBar, { autoRotate: true, onAutoRotateChange });
    expect(screen.getByText('Arraste para girar · role para aproximar · clique num balão para focar · duplo clique centraliza')).toBeInTheDocument();
    const toggle = screen.getByRole('switch', { name: 'Girar sozinho' });
    expect(toggle).toHaveAttribute('aria-checked', 'true');
    await fireEvent.click(toggle);
    expect(onAutoRotateChange).toHaveBeenCalledWith(false);
  });
});
