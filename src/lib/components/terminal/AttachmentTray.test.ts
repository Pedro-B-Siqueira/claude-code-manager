import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import AttachmentTray from './AttachmentTray.svelte';

const IMAGES = [
  { id: 'a.png', name: 'shot.png', bytes: 2048, previewUrl: 'blob:a' },
  { id: 'b.png', name: 'photo.png', bytes: 4096, previewUrl: 'blob:b' },
];

describe('AttachmentTray', () => {
  it('shows each image with a remove button and says they go with the next Enter', async () => {
    const onRemove = vi.fn();
    const onOpen = vi.fn();
    render(AttachmentTray, { images: IMAGES, onRemove, onOpen });
    expect(screen.getByText('2 imagens vão junto no próximo Enter')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Remover shot.png' }));
    expect(onRemove).toHaveBeenCalledWith('a.png');
    await fireEvent.click(screen.getByRole('button', { name: 'Abrir photo.png' }));
    expect(onOpen).toHaveBeenCalledWith('b.png');
  });

  it('renders nothing without images', () => {
    const { container } = render(AttachmentTray, { images: [], onRemove: vi.fn(), onOpen: vi.fn() });
    expect(container.querySelector('.tray')).toBeNull();
  });
});
