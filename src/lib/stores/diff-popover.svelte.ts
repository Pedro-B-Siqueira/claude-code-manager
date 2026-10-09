import { fetchEdit, fetchFileEdits } from '../api/commands';
import { reportWarning, type FailureCause } from '../api/logger';
import type { EditDetail } from '../api/types';

export type DiffRequest =
  | { kind: 'file'; sessionId: string; filePath: string; basePath: string | null }
  | { kind: 'edit'; editId: number; basePath: string | null };

export interface AnchorRect {
  top: number;
  left: number;
  right: number;
  bottom: number;
}

const SHOW_DELAY_MS = 260;
const HIDE_DELAY_MS = 160;

function requestKey(request: DiffRequest): string {
  return request.kind === 'file' ? `file:${request.sessionId}:${request.filePath}` : `edit:${request.editId}`;
}

/** One diff popover for the whole app, opened by hovering or focusing a file or edit. */
class DiffPopoverStore {
  open = $state(false);
  anchor = $state<AnchorRect | null>(null);
  basePath = $state<string | null>(null);
  edits = $state<EditDetail[]>([]);
  index = $state(0);
  loading = $state(false);

  private showTimer: ReturnType<typeof setTimeout> | null = null;
  private hideTimer: ReturnType<typeof setTimeout> | null = null;
  private currentKey: string | null = null;

  current = $derived(this.edits[this.index] ?? null);

  request(anchor: AnchorRect, request: DiffRequest): void {
    this.cancelHide();
    if (this.showTimer) clearTimeout(this.showTimer);
    this.showTimer = setTimeout(() => void this.show(anchor, request), this.open ? 0 : SHOW_DELAY_MS);
  }

  private async show(anchor: AnchorRect, request: DiffRequest): Promise<void> {
    const key = requestKey(request);
    this.anchor = anchor;
    this.basePath = request.basePath;
    this.open = true;
    if (key === this.currentKey) return;
    this.currentKey = key;
    this.loading = true;
    const edits = await this.load(request);
    if (this.currentKey !== key) return;
    this.edits = edits;
    this.index = 0;
    this.loading = false;
  }

  private async load(request: DiffRequest): Promise<EditDetail[]> {
    const report = (cause: FailureCause) => {
      reportWarning('falha ao carregar o diff', cause);
      return [];
    };
    if (request.kind === 'file') return fetchFileEdits(request.sessionId, request.filePath).catch(report);
    return fetchEdit(request.editId)
      .then((edit) => (edit ? [edit] : []))
      .catch(report);
  }

  scheduleHide(): void {
    if (this.showTimer) clearTimeout(this.showTimer);
    this.cancelHide();
    this.hideTimer = setTimeout(() => this.hide(), HIDE_DELAY_MS);
  }

  cancelHide(): void {
    if (this.hideTimer) clearTimeout(this.hideTimer);
    this.hideTimer = null;
  }

  hide(): void {
    if (this.showTimer) clearTimeout(this.showTimer);
    this.open = false;
    this.currentKey = null;
    this.edits = [];
    this.loading = false;
  }

  step(delta: number): void {
    const next = this.index + delta;
    if (next >= 0 && next < this.edits.length) this.index = next;
  }
}

export const diffPopoverStore = new DiffPopoverStore();

/** `use:diffHover={request}` — shows the diff popover on hover and on keyboard focus. `null` disables it. */
export function diffHover(element: HTMLElement, initial: DiffRequest | null) {
  let request = initial;
  const anchorOf = (): AnchorRect => {
    const rect = element.getBoundingClientRect();
    return { top: rect.top, left: rect.left, right: rect.right, bottom: rect.bottom };
  };
  const show = () => {
    if (request) diffPopoverStore.request(anchorOf(), request);
  };
  const hide = () => {
    if (request) diffPopoverStore.scheduleHide();
  };
  const escape = (event: KeyboardEvent) => {
    if (event.key === 'Escape') diffPopoverStore.hide();
  };
  element.addEventListener('mouseenter', show);
  element.addEventListener('mouseleave', hide);
  element.addEventListener('focus', show);
  element.addEventListener('blur', hide);
  element.addEventListener('keydown', escape);
  return {
    update(next: DiffRequest | null) {
      request = next;
    },
    destroy() {
      element.removeEventListener('mouseenter', show);
      element.removeEventListener('mouseleave', hide);
      element.removeEventListener('focus', show);
      element.removeEventListener('blur', hide);
      element.removeEventListener('keydown', escape);
    },
  };
}
