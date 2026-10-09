import type { PaletteItem } from '../palette/search';

export interface PaletteHandlers {
  runAction: (actionId: string) => void;
  focusLive: (key: string) => void;
  openHistory: (sessionId: string, query: string | null) => void;
}

/** Live sessions open in Focus; everything else opens the history at the right session. */
export function handlePaletteChoice(item: PaletteItem, handlers: PaletteHandlers): void {
  const target = item.target;
  switch (target.kind) {
    case 'action':
      handlers.runAction(target.actionId);
      return;
    case 'live':
      handlers.focusLive(target.key);
      return;
    case 'history':
      handlers.openHistory(target.sessionId, null);
      return;
    case 'file':
      if (target.liveKey) handlers.focusLive(target.liveKey);
      else handlers.openHistory(target.sessionId, null);
      return;
    case 'branch':
      if (target.liveKey) handlers.focusLive(target.liveKey);
      else handlers.openHistory('', target.branch);
  }
}
