import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import RepousoDialog from './RepousoDialog.svelte';

const IMMEDIATE = { lockDelay: { kind: 'immediate' as const }, showDetails: true };

describe('RepousoDialog', () => {
  it('explains the lock and activates it with the remembered choice', async () => {
    const onActivate = vi.fn();
    render(RepousoDialog, { preparation: IMMEDIATE, onCancel: vi.fn(), onActivate });
    expect(screen.getByRole('dialog', { name: 'Ativar repouso?' })).toBeInTheDocument();
    expect(screen.getByText('Todas as telas ficam bloqueadas no Modo Jarvis e o Mac não vai dormir. As sessões continuam rodando. Para sair, use o Touch ID ou a senha do Mac.')).toBeInTheDocument();
    expect(screen.getByText('Desligado, a tela mostra só o nome e o status de cada sessão. Telas extras mostram só o resumo.')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Ativar repouso' }));
    expect(onActivate).toHaveBeenCalledWith(true);
  });

  it('lets the details be turned off', async () => {
    const onActivate = vi.fn();
    render(RepousoDialog, { preparation: IMMEDIATE, onCancel: vi.fn(), onActivate });
    const toggle = screen.getByRole('switch', { name: 'Mostrar o que cada agente está fazendo na tela bloqueada' });
    expect(toggle).toHaveAttribute('aria-checked', 'true');
    await fireEvent.click(toggle);
    await fireEvent.click(screen.getByRole('button', { name: 'Ativar repouso' }));
    expect(onActivate).toHaveBeenCalledWith(false);
  });

  it('cancels with the button and with Esc', async () => {
    const onCancel = vi.fn();
    render(RepousoDialog, { preparation: IMMEDIATE, onCancel, onActivate: vi.fn() });
    await fireEvent.click(screen.getByRole('button', { name: 'Cancelar' }));
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(onCancel).toHaveBeenCalledTimes(2);
  });

  it('warns when the Mac would not ask for the password at once', () => {
    render(RepousoDialog, { preparation: { lockDelay: { kind: 'seconds', seconds: 60 }, showDetails: false }, onCancel: vi.fn(), onActivate: vi.fn() });
    expect(screen.getByRole('note')).toHaveTextContent('Seu Mac só pede a senha 60 s depois que a tela apaga.');
    expect(screen.getByRole('switch', { name: 'Mostrar o que cada agente está fazendo na tela bloqueada' })).toHaveAttribute('aria-checked', 'false');
  });
});
