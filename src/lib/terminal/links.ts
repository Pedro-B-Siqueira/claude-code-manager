import { PLATFORM, type Platform } from '../platform';

/** Only ⌘+click (Ctrl+click on Linux) opens a link; a plain click keeps selecting text. */
export function shouldOpenLink(event: Pick<MouseEvent, 'metaKey' | 'ctrlKey'>, platform: Platform = PLATFORM): boolean {
  return platform === 'macos' ? event.metaKey : event.ctrlKey;
}

export function linkHint(platform: Platform = PLATFORM): string {
  return platform === 'macos' ? '⌘+clique para abrir' : 'Ctrl+clique para abrir';
}
