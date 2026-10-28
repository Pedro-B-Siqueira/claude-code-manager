import { fetchLiveSessions } from '../lib/api/commands';
import { onLiveChanged } from '../lib/api/events';
import { lockedSummary, type LockedSummary } from '../jarvis/data/lock-text';
import { toJarvisSession } from '../jarvis/data/live-sessions';
import { createLockScreen } from './lock-screen';
import './lock.css';

const REFRESH_DELAY_MS = 1_000;

const screen = createLockScreen(document);
document.body.append(screen.root);

let summary: LockedSummary = { sessions: '', waiting: null };
let refreshTimer: ReturnType<typeof setTimeout> | null = null;

async function refresh(): Promise<void> {
  const views = await fetchLiveSessions().catch(() => null);
  if (views) summary = lockedSummary(views.flatMap((view) => toJarvisSession(view) ?? []));
  screen.update(summary, new Date());
}

/** Redraws when the minute turns, not every second. */
function tickClock(): void {
  screen.update(summary, new Date());
  const now = new Date();
  setTimeout(tickClock, (60 - now.getSeconds()) * 1000 - now.getMilliseconds());
}

void refresh();
tickClock();
void onLiveChanged(() => {
  if (refreshTimer) return;
  refreshTimer = setTimeout(() => {
    refreshTimer = null;
    void refresh();
  }, REFRESH_DELAY_MS);
});

export {};
