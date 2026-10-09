export type ShortcutCommand =
  | { kind: 'palette' }
  | { kind: 'new-session' }
  | { kind: 'settings' }
  | { kind: 'session-index'; index: number };

/** ⌘K, ⌘N, ⌘, and ⌘1…9. Anything else (including ⌘C/⌘V in the terminal) is left alone. */
export function shortcutFor(event: Pick<KeyboardEvent, 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'key'>): ShortcutCommand | null {
  if (!event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return null;
  const key = event.key.toLowerCase();
  if (key === 'k') return { kind: 'palette' };
  if (key === 'n') return { kind: 'new-session' };
  if (key === ',') return { kind: 'settings' };
  if (/^[1-9]$/.test(key)) return { kind: 'session-index', index: Number(key) - 1 };
  return null;
}
