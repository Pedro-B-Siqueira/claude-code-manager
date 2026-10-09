import type { SessionStatus } from '../api/types';

export const SUPPORTED_IMAGE_TYPES: readonly string[] = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'];
const IMAGE_PATH = /\.(png|jpe?g|gif|webp)$/i;
const SHELL_SPECIAL = /[^\p{L}\p{N}_.,:\/@%+=~-]/gu;

export interface PastedImage {
  bytes: Uint8Array<ArrayBuffer>;
  name: string;
}

/** The part of a paste event's `DataTransfer` this module reads. */
export interface ClipboardSnapshot {
  getData(format: string): string;
  files: ArrayLike<File>;
}

export type PasteAction = { kind: 'text' } | { kind: 'image'; file: File } | { kind: 'native' };

/**
 * Text pastes as usual and a supported image goes to the tray. Anything else becomes an empty
 * paste, which makes Claude Code read the clipboard itself, as it does in other terminals.
 */
export function pasteAction(clipboard: ClipboardSnapshot | null): PasteAction {
  if (!clipboard || clipboard.getData('text/plain') !== '') return { kind: 'text' };
  const image = Array.from(clipboard.files).find((file) => SUPPORTED_IMAGE_TYPES.includes(file.type));
  return image ? { kind: 'image', file: image } : { kind: 'native' };
}

export function splitDroppedPaths(paths: readonly string[]): { images: string[]; others: string[] } {
  return {
    images: paths.filter((path) => IMAGE_PATH.test(path)),
    others: paths.filter((path) => !IMAGE_PATH.test(path)),
  };
}

export function escapeShellPath(path: string): string {
  return path.replace(SHELL_SPECIAL, (character) => `\\${character}`);
}

export type KeyLike = Pick<KeyboardEvent, 'type' | 'key' | 'shiftKey' | 'altKey' | 'metaKey' | 'ctrlKey'>;

export function isPlainEnter(event: KeyLike): boolean {
  return event.key === 'Enter' && !event.shiftKey && !event.altKey && !event.metaKey && !event.ctrlKey;
}

/** While Claude Code asks for permission, Enter answers it and the images wait for the next prompt. */
export function shouldSubmitWithImages(trayCount: number, status: SessionStatus): boolean {
  return trayCount > 0 && status !== 'permission';
}

export type EnterDecision = 'submit' | 'swallow' | 'pass';

/**
 * xterm sends Enter on keydown but also looks at its keypress, so the keypress and keyup of an
 * intercepted Enter must be swallowed too. Every Enter is swallowed while images are being sent.
 */
export class EnterGate {
  private swallowing = false;
  private submitting = false;

  decide(event: KeyLike, canSubmit: boolean): EnterDecision {
    if (!isPlainEnter(event)) return 'pass';
    if (event.type !== 'keydown') return this.swallowing ? 'swallow' : 'pass';
    if (this.submitting) {
      this.swallowing = true;
      return 'swallow';
    }
    this.swallowing = canSubmit;
    return canSubmit ? 'submit' : 'pass';
  }

  begin(): void {
    this.submitting = true;
  }

  finish(): void {
    this.submitting = false;
  }
}

const gatesBySession = new Map<string, EnterGate>();

/** One gate per session, so re-creating its terminal view keeps a send in flight guarded. */
export function enterGateFor(key: string): EnterGate {
  const existing = gatesBySession.get(key);
  if (existing) return existing;
  const gate = new EnterGate();
  gatesBySession.set(key, gate);
  return gate;
}

export function droppedPathsText(paths: readonly string[]): string {
  return `${paths.map(escapeShellPath).join(' ')} `;
}

export interface Point {
  x: number;
  y: number;
}

export function dropTargetContains(rect: Pick<DOMRect, 'left' | 'top' | 'right' | 'bottom'>, point: Point): boolean {
  return point.x >= rect.left && point.x <= rect.right && point.y >= rect.top && point.y <= rect.bottom;
}

export function describeUnrecognized(recognized: number, total: number): string {
  if (recognized === 0) return 'O Claude Code não reconheceu as imagens. Confira o prompt e aperte Enter.';
  return `O Claude Code reconheceu ${recognized} de ${total} imagens. Confira o prompt e aperte Enter.`;
}
