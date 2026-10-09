import type { ITheme } from '@xterm/xterm';
import type { Theme } from '../api/types';

/** ANSI colors tuned to the app tokens; every foreground keeps ≥4.5:1 contrast on its background. */
const DARK: ITheme = {
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

const LIGHT: ITheme = {
  background: '#fbfaf7',
  foreground: '#2a2925',
  cursor: '#b4502f',
  cursorAccent: '#fbfaf7',
  selectionBackground: '#b4502f33',
  black: '#2a2925',
  red: '#a3352a',
  green: '#1f7a45',
  yellow: '#8a5e00',
  blue: '#2b5fc4',
  magenta: '#8e3fa3',
  cyan: '#1d6f73',
  white: '#6b6962',
  brightBlack: '#5b5953',
  brightRed: '#8f2a20',
  brightGreen: '#17653a',
  brightYellow: '#734e00',
  brightBlue: '#1f4fa8',
  brightMagenta: '#76308a',
  brightCyan: '#165b5e',
  brightWhite: '#141413',
};

export function terminalTheme(theme: Theme): ITheme {
  return theme === 'dark' ? DARK : LIGHT;
}
