import type { ITheme } from '@xterm/xterm';

/**
 * ANSI colors tuned to the app tokens; every foreground keeps ≥4.5:1 contrast on its background.
 * The terminal stays dark in both app themes: Claude Code draws its diffs and dimmed text for a dark
 * terminal, and its own theme setting belongs to the user's configuration, which the app never changes.
 */
export const TERMINAL_THEME: ITheme = {
  background: '#0e0e0d',
  foreground: '#dad7cd',
  cursor: '#d97757',
  cursorAccent: '#0e0e0d',
  selectionBackground: '#d977574d',
  black: '#2a2925',
  red: '#f4a193',
  green: '#72c68c',
  yellow: '#ebbe55',
  blue: '#82aeff',
  magenta: '#d8a2e6',
  cyan: '#7fcfcf',
  white: '#dad7cd',
  brightBlack: '#8f8c84',
  brightRed: '#ffb8ab',
  brightGreen: '#90dca6',
  brightYellow: '#f5d27e',
  brightBlue: '#a6c6ff',
  brightMagenta: '#e7bff0',
  brightCyan: '#a3e2e2',
  brightWhite: '#f5f3ec',
};
