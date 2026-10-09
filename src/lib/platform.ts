export type Platform = 'macos' | 'linux';

/** WKWebView reports "Macintosh"; WebKitGTK reports "Linux" (X11 or Wayland). */
export function platformFromUserAgent(userAgent: string): Platform {
  return /Macintosh|Mac OS X/.test(userAgent) ? 'macos' : 'linux';
}

export const PLATFORM: Platform = platformFromUserAgent(globalThis.navigator?.userAgent ?? '');
