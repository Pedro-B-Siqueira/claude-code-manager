import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { AppSettings } from '../../api/types';
import SettingsDialog from './SettingsDialog.svelte';

const SETTINGS: AppSettings = {
  theme: 'dark',
  worktreeRoot: null,
  branchPrefixes: ['feat-', 'fix-'],
  scrollbackLines: 5000,
  notifications: 'auto',
  hibernation: { enabled: true, idleMinutes: 30 },
  claudeBinary: null,
};

describe('SettingsDialog', () => {
  it('saves a cleaned copy and leaves the original untouched until then', async () => {
    const onSave = vi.fn();
    render(SettingsDialog, { settings: SETTINGS, claudeGaugeHook: true, onSave, onClose: vi.fn() });
    expect(screen.getByText(/O ClaudeGauge já avisa/)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('radio', { name: 'Claro' }));
    await fireEvent.input(screen.getByPlaceholderText('feat-, fix-'), { target: { value: 'feat-, fix-, chore- , ' } });
    await fireEvent.input(screen.getByPlaceholderText(/Automática/), { target: { value: '   ' } });
    expect(SETTINGS.theme).toBe('dark');
    await fireEvent.click(screen.getByRole('button', { name: 'Salvar' }));
    expect(onSave).toHaveBeenCalledWith({ ...SETTINGS, theme: 'light', branchPrefixes: ['feat-', 'fix-', 'chore-'], worktreeRoot: null });
  });
});
