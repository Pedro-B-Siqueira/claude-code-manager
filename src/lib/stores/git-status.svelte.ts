import { fetchGitStatus } from '../api/commands';
import type { GitStatus } from '../api/types';

const MIN_REFRESH_MS = 5_000;

/** Git status per folder, refreshed at most every few seconds so cards stay cheap to render. */
class GitStatusStore {
  byCwd = $state<Record<string, GitStatus>>({});
  private fetchedAt = new Map<string, number>();

  refresh(cwd: string): void {
    const last = this.fetchedAt.get(cwd) ?? 0;
    if (Date.now() - last < MIN_REFRESH_MS) return;
    this.fetchedAt.set(cwd, Date.now());
    void fetchGitStatus(cwd)
      .then((status) => (this.byCwd = { ...this.byCwd, [cwd]: status }))
      .catch(() => undefined);
  }
}

export const gitStatusStore = new GitStatusStore();
