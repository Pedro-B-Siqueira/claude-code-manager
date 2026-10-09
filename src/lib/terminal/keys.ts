import type { Platform } from '../platform';

export type TerminalKey = Pick<KeyboardEvent, 'type' | 'code' | 'key' | 'metaKey' | 'ctrlKey' | 'shiftKey' | 'altKey'>;

export type TerminalKeyAction = 'copy' | 'paste-text' | 'send-to-terminal' | 'default';

/**
 * macOS: ⌘C copies a selection; everything else is xterm's. Linux follows terminal conventions:
 * Ctrl+Shift+C/V copy and paste text, and Ctrl+V must reach Claude Code, which reads images from the
 * clipboard on it, instead of triggering the WebView's own paste.
 */
export function terminalKeyAction(event: TerminalKey, platform: Platform, hasSelection: boolean): TerminalKeyAction {
  if (event.type !== 'keydown') return 'default';
  if (platform === 'macos') {
    const copies = event.metaKey && !event.ctrlKey && !event.altKey && event.key.toLowerCase() === 'c' && hasSelection;
    return copies ? 'copy' : 'default';
  }
  if (!event.ctrlKey || event.altKey || event.metaKey) return 'default';
  if (event.shiftKey && event.code === 'KeyC') return 'copy';
  if (event.shiftKey && event.code === 'KeyV') return 'paste-text';
  if (!event.shiftKey && event.code === 'KeyV') return 'send-to-terminal';
  return 'default';
}
