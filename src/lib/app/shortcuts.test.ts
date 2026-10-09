import { describe, expect, it } from 'vitest';
import { shortcutFor, shortcutLabel } from './shortcuts';

type Modifiers = Partial<Record<'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey', boolean>>;

function onMac(key: string, modifiers: Modifiers = {}) {
  return shortcutFor({ key, code: '', metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, ...modifiers }, 'macos');
}

function onLinux(key: string, code: string, modifiers: Modifiers = {}) {
  return shortcutFor({ key, code, metaKey: false, ctrlKey: true, altKey: false, shiftKey: true, ...modifiers }, 'linux');
}

describe('shortcuts', () => {
  it('maps the app shortcuts with ⌘ on macOS', () => {
    expect(onMac('k')).toEqual({ kind: 'palette' });
    expect(onMac('N')).toEqual({ kind: 'new-session' });
    expect(onMac(',')).toEqual({ kind: 'settings' });
    expect(onMac('3')).toEqual({ kind: 'session-index', index: 2 });
  });

  it('leaves other macOS keys to the terminal and the system', () => {
    expect(onMac('c')).toBeNull();
    expect(onMac('0')).toBeNull();
    expect(onMac('k', { metaKey: false })).toBeNull();
    expect(onMac('k', { shiftKey: true })).toBeNull();
  });

  it('maps Ctrl+Shift on Linux by physical key, whatever the layout prints', () => {
    expect(onLinux('K', 'KeyK')).toEqual({ kind: 'palette' });
    expect(onLinux('N', 'KeyN')).toEqual({ kind: 'new-session' });
    expect(onLinux('<', 'Comma')).toEqual({ kind: 'settings' });
    expect(onLinux(';', 'Comma')).toEqual({ kind: 'settings' });
    expect(onLinux('!', 'Digit1')).toEqual({ kind: 'session-index', index: 0 });
    expect(onLinux('(', 'Digit9')).toEqual({ kind: 'session-index', index: 8 });
  });

  it('leaves Linux terminal keys alone', () => {
    expect(onLinux('k', 'KeyK', { shiftKey: false })).toBeNull();
    expect(onLinux('C', 'KeyC')).toBeNull();
    expect(onLinux(')', 'Digit0')).toBeNull();
    expect(onLinux('K', 'KeyK', { metaKey: true })).toBeNull();
  });

  it('labels shortcuts for each system', () => {
    expect(shortcutLabel('palette', 'macos')).toBe('⌘K');
    expect(shortcutLabel('settings', 'macos')).toBe('⌘,');
    expect(shortcutLabel('palette', 'linux')).toBe('Ctrl+Shift+K');
    expect(shortcutLabel('new-session', 'linux')).toBe('Ctrl+Shift+N');
  });
});
