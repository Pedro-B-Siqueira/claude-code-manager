import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import AttachmentStorage from './AttachmentStorage.svelte';

const usage = { count: 12, bytes: 34 * 1024 * 1024 };
const clearAttachments = vi.fn(async () => usage);

vi.mock('../../api/commands', () => ({
  fetchAttachmentUsage: vi.fn(async () => usage),
  clearAttachments: () => clearAttachments(),
}));

describe('AttachmentStorage', () => {
  beforeEach(() => clearAttachments.mockClear());

  it('shows how much the attachments use', async () => {
    render(AttachmentStorage);
    expect(await screen.findByText('Anexos: 12 imagens · 34 MB')).toBeInTheDocument();
  });

  it('only deletes after confirmation', async () => {
    render(AttachmentStorage);
    await screen.findByText('Anexos: 12 imagens · 34 MB');
    await fireEvent.click(screen.getByRole('button', { name: 'Apagar todos' }));
    expect(clearAttachments).not.toHaveBeenCalled();
    expect(screen.getByText(/As imagens que estão nas bandejas também saem/)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Cancelar' }));
    expect(clearAttachments).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Apagar todos' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Apagar' }));
    await vi.waitFor(() => expect(clearAttachments).toHaveBeenCalledOnce());
  });
});
