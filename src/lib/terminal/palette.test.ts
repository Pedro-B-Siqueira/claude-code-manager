import { describe, expect, it } from 'vitest';
import { TERMINAL_THEME } from './palette';

function luminance(hex: string): number {
  const channels = [1, 3, 5].map((start) => Number.parseInt(hex.slice(start, start + 2), 16) / 255);
  const [red = 0, green = 0, blue = 0] = channels.map((channel) => (channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4));
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function contrast(first: string, second: string): number {
  const [lighter, darker] = [luminance(first), luminance(second)].sort((a, b) => b - a);
  return ((lighter ?? 0) + 0.05) / ((darker ?? 0) + 0.05);
}

describe('terminal palette', () => {
  it('is dark, because Claude Code draws its diffs and dimmed text for a dark terminal', () => {
    expect(luminance(TERMINAL_THEME.background ?? '#ffffff')).toBeLessThan(0.05);
  });

  it('keeps every text color readable on the background', () => {
    const background = TERMINAL_THEME.background ?? '#000000';
    const textColors = ['foreground', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white', 'brightBlack', 'brightRed', 'brightGreen', 'brightYellow', 'brightBlue', 'brightMagenta', 'brightCyan', 'brightWhite'] as const;
    for (const name of textColors) {
      expect(contrast(TERMINAL_THEME[name] ?? background, background), name).toBeGreaterThanOrEqual(4.5);
    }
  });
});
