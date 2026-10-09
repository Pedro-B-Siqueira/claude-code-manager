import { fetchGridOrder, fetchLayout, saveGridOrder, saveLayout } from '../api/commands';
import { reportWarning, type FailureCause } from '../api/logger';
import type { LiveSessionView, PanelLayout } from '../api/types';

const SAVE_DEBOUNCE_MS = 400;

export const SIDEBAR_LIMITS = { min: 200, max: 380 } as const;
export const FOCUS_PANEL_LIMITS = { min: 240, max: 560 } as const;

/** Cards the user arranged come first, in that order; new sessions follow by start time. */
export function orderSessions(sessions: readonly LiveSessionView[], order: readonly string[]): LiveSessionView[] {
  const position = new Map(order.map((sessionId, index) => [sessionId, index]));
  return [...sessions].sort((first, second) => {
    const firstPosition = position.get(first.sessionId) ?? Number.MAX_SAFE_INTEGER;
    const secondPosition = position.get(second.sessionId) ?? Number.MAX_SAFE_INTEGER;
    return firstPosition - secondPosition || first.startedAt - second.startedAt;
  });
}

class LayoutStore {
  panels = $state<PanelLayout>({ sidebarWidth: 248, focusPanelWidth: 300 });
  gridOrder = $state<string[]>([]);
  private saveTimer: ReturnType<typeof setTimeout> | null = null;

  async load(): Promise<void> {
    const warn = (context: string) => (cause: FailureCause) => {
      reportWarning(context, cause);
      return null;
    };
    const [panels, order] = await Promise.all([fetchLayout().catch(warn('falha ao ler o layout')), fetchGridOrder().catch(warn('falha ao ler a ordem dos cards'))]);
    if (panels) this.panels = panels;
    if (order) this.gridOrder = order;
  }

  resize(patch: Partial<PanelLayout>): void {
    this.panels = { ...this.panels, ...patch };
    if (this.saveTimer) clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => {
      void saveLayout(this.panels).catch((cause: FailureCause) => reportWarning('falha ao salvar o layout', cause));
    }, SAVE_DEBOUNCE_MS);
  }

  reorder(sessionIds: string[]): void {
    const rest = this.gridOrder.filter((sessionId) => !sessionIds.includes(sessionId));
    this.gridOrder = [...sessionIds, ...rest];
    void saveGridOrder(this.gridOrder).catch((cause: FailureCause) => reportWarning('falha ao salvar a ordem dos cards', cause));
  }
}

export const layoutStore = new LayoutStore();
