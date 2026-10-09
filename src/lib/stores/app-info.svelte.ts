import { fetchAppInfo } from '../api/commands';
import { reportWarning, type FailureCause } from '../api/logger';
import type { AppInfo } from '../api/types';

class AppInfoStore {
  current = $state<AppInfo | null>(null);

  async refresh(): Promise<void> {
    this.current = await fetchAppInfo().catch((cause: FailureCause) => {
      reportWarning('falha ao ler informações do app', cause);
      return this.current;
    });
  }
}

export const appInfoStore = new AppInfoStore();
