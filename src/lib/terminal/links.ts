/** Only ⌘+click opens a link; a plain click keeps selecting text. */
export function shouldOpenLink(event: Pick<MouseEvent, 'metaKey'>): boolean {
  return event.metaKey;
}
