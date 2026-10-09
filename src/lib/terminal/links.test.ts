import { describe, expect, it } from 'vitest';
import { shouldOpenLink } from './links';

describe('shouldOpenLink', () => {
  it('opens only with ⌘ held, so a plain click still selects text', () => {
    expect(shouldOpenLink({ metaKey: true })).toBe(true);
    expect(shouldOpenLink({ metaKey: false })).toBe(false);
  });
});
