import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import LockPanel from './LockPanel.svelte';

const AT_16_13 = new Date(2026, 9, 27, 16, 13).getTime();

describe('LockPanel', () => {
  it('shows the time and unlocks on request', async () => {
    const onUnlock = vi.fn();
    render(LockPanel, { now: AT_16_13, unlock: 'idle', onUnlock });
    expect(screen.getByText('16:13')).toBeInTheDocument();
    expect(screen.getByText('Bloqueado. As sessões continuam rodando e o Mac não vai dormir.')).toBeInTheDocument();
    expect(screen.getByText('Touch ID ou senha do Mac')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Desbloquear' }));
    expect(onUnlock).toHaveBeenCalledOnce();
  });

  it('waits for the system window without letting a second request through', () => {
    render(LockPanel, { now: AT_16_13, unlock: 'asking', onUnlock: vi.fn() });
    expect(screen.getByRole('button', { name: 'Aguardando confirmação…' })).toBeDisabled();
    expect(screen.getByText('Confirme na janela do sistema.')).toBeInTheDocument();
  });

  it('reports a refused or cancelled attempt and offers another', async () => {
    const onUnlock = vi.fn();
    render(LockPanel, { now: AT_16_13, unlock: 'failed', onUnlock });
    expect(screen.getByRole('alert')).toHaveTextContent('Não foi possível confirmar sua identidade. Tente novamente.');
    await fireEvent.click(screen.getByRole('button', { name: 'Tentar novamente' }));
    expect(onUnlock).toHaveBeenCalledOnce();
  });
});
