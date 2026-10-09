import { describe, expect, it } from 'vitest';
import { shortcutFor } from './shortcuts';

function press(key: string, modifiers: Partial<Record<'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey', boolean>> = {}) {
  return shortcutFor({ key, metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, ...modifiers });
}

describe('shortcuts', () => {
  it('maps the app shortcuts', () => {
    expect(press('k')).toEqual({ kind: 'palette' });
    expect(press('N')).toEqual({ kind: 'new-session' });
    expect(press(',')).toEqual({ kind: 'settings' });
    expect(press('3')).toEqual({ kind: 'session-index', index: 2 });
  });

  it('leaves other keys to the terminal and the system', () => {
    expect(press('c')).toBeNull();
    expect(press('0')).toBeNull();
    expect(press('k', { metaKey: false })).toBeNull();
    expect(press('k', { shiftKey: true })).toBeNull();
  });
});
