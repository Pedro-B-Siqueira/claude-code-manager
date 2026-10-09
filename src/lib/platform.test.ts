import { describe, expect, it } from 'vitest';
import { platformFromUserAgent } from './platform';

describe('platformFromUserAgent', () => {
  it('tells the macOS WebView from WebKitGTK', () => {
    expect(platformFromUserAgent('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)')).toBe('macos');
    expect(platformFromUserAgent('Mozilla/5.0 (X11; Ubuntu; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko)')).toBe('linux');
    expect(platformFromUserAgent('Mozilla/5.0 (X11; Linux aarch64) AppleWebKit/605.1.15 (KHTML, like Gecko)')).toBe('linux');
  });
});
