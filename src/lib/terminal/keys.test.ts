import { describe, expect, it } from 'vitest';
import { terminalKeyAction, type TerminalKey } from './keys';

function key(code: string, keyName: string, modifiers: Partial<Pick<TerminalKey, 'metaKey' | 'ctrlKey' | 'shiftKey' | 'altKey'>> = {}, type = 'keydown'): TerminalKey {
  return { type, code, key: keyName, metaKey: false, ctrlKey: false, shiftKey: false, altKey: false, ...modifiers };
}

describe('terminalKeyAction', () => {
  it('keeps macOS as it was: ⌘C copies only with a selection', () => {
    expect(terminalKeyAction(key('KeyC', 'c', { metaKey: true }), 'macos', true)).toBe('copy');
    expect(terminalKeyAction(key('KeyC', 'c', { metaKey: true }), 'macos', false)).toBe('default');
    expect(terminalKeyAction(key('KeyV', 'v', { ctrlKey: true }), 'macos', false)).toBe('default');
  });

  it('copies and pastes text with Ctrl+Shift on Linux', () => {
    expect(terminalKeyAction(key('KeyC', 'C', { ctrlKey: true, shiftKey: true }), 'linux', true)).toBe('copy');
    expect(terminalKeyAction(key('KeyV', 'V', { ctrlKey: true, shiftKey: true }), 'linux', false)).toBe('paste-text');
  });

  it('sends Ctrl+C and Ctrl+V to Claude Code on Linux (Ctrl+V is how it pastes images there)', () => {
    expect(terminalKeyAction(key('KeyV', 'v', { ctrlKey: true }), 'linux', false)).toBe('send-to-terminal');
    expect(terminalKeyAction(key('KeyC', 'c', { ctrlKey: true }), 'linux', true)).toBe('default');
  });

  it('only acts on keydown', () => {
    expect(terminalKeyAction(key('KeyV', 'v', { ctrlKey: true }, 'keyup'), 'linux', false)).toBe('default');
  });
});
