import { fetchSettings, saveSettings } from '../api/commands';
import { reportError, type FailureCause } from '../api/logger';
import type { AppSettings, Theme } from '../api/types';
import { applyTheme } from '../theme/apply-theme';

export const DEFAULT_SETTINGS: AppSettings = {
  theme: 'dark',
  worktreeRoot: null,
  branchPrefixes: ['feat-', 'fix-'],
  scrollbackLines: 5000,
  notifications: 'auto',
  hibernation: { enabled: true, idleMinutes: 30 },
  claudeBinary: null,
};

class SettingsStore {
  current = $state<AppSettings>(DEFAULT_SETTINGS);

  async load(): Promise<void> {
    const loaded = await fetchSettings().catch((cause: FailureCause) => {
      reportError('falha ao carregar configurações', cause);
      return DEFAULT_SETTINGS;
    });
    this.current = loaded;
    applyTheme(loaded.theme, false);
  }

  async toggleTheme(): Promise<void> {
    const nextTheme: Theme = this.current.theme === 'dark' ? 'light' : 'dark';
    await this.save({ ...this.current, theme: nextTheme });
  }

  async save(next: AppSettings): Promise<void> {
    if (next.theme !== this.current.theme) applyTheme(next.theme, true);
    await this.update(next);
  }

  private async update(next: AppSettings): Promise<void> {
    this.current = next;
    const saved = await saveSettings(next).catch((cause: FailureCause) => {
      reportError('falha ao salvar configurações', cause);
      return next;
    });
    this.current = saved;
  }
}

export const settingsStore = new SettingsStore();
