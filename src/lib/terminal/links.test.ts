import { describe, expect, it } from 'vitest';
import { linkHint, shouldOpenLink } from './links';

describe('terminal links', () => {
  it('open only with the system modifier, so a plain click still selects text', () => {
    expect(shouldOpenLink({ metaKey: true, ctrlKey: false }, 'macos')).toBe(true);
    expect(shouldOpenLink({ metaKey: false, ctrlKey: true }, 'macos')).toBe(false);
    expect(shouldOpenLink({ metaKey: false, ctrlKey: true }, 'linux')).toBe(true);
    expect(shouldOpenLink({ metaKey: true, ctrlKey: false }, 'linux')).toBe(false);
    expect(shouldOpenLink({ metaKey: false, ctrlKey: false }, 'linux')).toBe(false);
  });

  it('names the modifier in the hint', () => {
    expect(linkHint('macos')).toBe('⌘+clique para abrir');
    expect(linkHint('linux')).toBe('Ctrl+clique para abrir');
  });
});
