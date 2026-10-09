import { PLATFORM, type Platform } from '../platform';

export type ShortcutCommand =
  | { kind: 'palette' }
  | { kind: 'new-session' }
  | { kind: 'settings' }
  | { kind: 'session-index'; index: number };

export type ShortcutId = 'palette' | 'new-session' | 'settings';

type ShortcutKey = Pick<KeyboardEvent, 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'key' | 'code'>;

const LABELS: Record<Platform, Record<ShortcutId, string>> = {
  macos: { palette: '⌘K', 'new-session': '⌘N', settings: '⌘,' },
  linux: { palette: 'Ctrl+Shift+K', 'new-session': 'Ctrl+Shift+N', settings: 'Ctrl+Shift+,' },
};

/**
 * macOS: ⌘K, ⌘N, ⌘, and ⌘1…9. Linux: the same with Ctrl+Shift, matched by physical key because
 * Shift changes the character (Shift+1 is "!", Shift+, depends on the layout). Plain Ctrl stays with
 * the terminal.
 */
export function shortcutFor(event: ShortcutKey, platform: Platform = PLATFORM): ShortcutCommand | null {
  return platform === 'macos' ? macShortcut(event) : linuxShortcut(event);
}

function macShortcut(event: ShortcutKey): ShortcutCommand | null {
  if (!event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return null;
  const key = event.key.toLowerCase();
  if (key === 'k') return { kind: 'palette' };
  if (key === 'n') return { kind: 'new-session' };
  if (key === ',') return { kind: 'settings' };
  if (/^[1-9]$/.test(key)) return { kind: 'session-index', index: Number(key) - 1 };
  return null;
}

function linuxShortcut(event: ShortcutKey): ShortcutCommand | null {
  if (!event.ctrlKey || !event.shiftKey || event.altKey || event.metaKey) return null;
  if (event.code === 'KeyK') return { kind: 'palette' };
  if (event.code === 'KeyN') return { kind: 'new-session' };
  if (event.code === 'Comma') return { kind: 'settings' };
  const digit = /^Digit([1-9])$/.exec(event.code)?.[1];
  return digit ? { kind: 'session-index', index: Number(digit) - 1 } : null;
}

export function shortcutLabel(id: ShortcutId, platform: Platform = PLATFORM): string {
  return LABELS[platform][id];
}
