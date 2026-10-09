import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ConfirmDialog from './ConfirmDialog.svelte';

describe('ConfirmDialog', () => {
  it('confirms, cancels and closes on Escape', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    render(ConfirmDialog, { title: 'Encerrar sessão?', message: 'A conversa continua salva.', confirmLabel: 'Encerrar', danger: true, onConfirm, onCancel });
    expect(screen.getByRole('dialog', { name: 'Encerrar sessão?' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Encerrar' }));
    expect(onConfirm).toHaveBeenCalledOnce();
    await fireEvent.click(screen.getByRole('button', { name: 'Cancelar' }));
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(onCancel).toHaveBeenCalledTimes(2);
  });
});
