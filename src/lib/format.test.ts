import { describe, expect, it } from 'vitest';
import { fileName, formatBytes, formatDuration, formatMemory, formatRelativeTime, relativePath } from './format';

describe('format helpers', () => {
  it('formats memory in MB or GB', () => {
    expect(formatMemory(512)).toBe('512 MB');
    expect(formatMemory(1536)).toBe('1,5 GB');
  });

  it('extracts the file name from a path', () => {
    expect(fileName('src/app/order.component.ts')).toBe('order.component.ts');
    expect(fileName('README.md')).toBe('README.md');
  });
});


describe('time and path helpers', () => {
  const now = Date.UTC(2026, 9, 8, 12, 0, 0);

  it('formats relative times in Portuguese', () => {
    expect(formatRelativeTime(now - 30_000, now)).toBe('agora');
    expect(formatRelativeTime(now - 5 * 60_000, now)).toBe('há 5 minutos');
    expect(formatRelativeTime(now - 24 * 60 * 60_000, now)).toBe('ontem');
    expect(formatRelativeTime(null, now)).toBe('—');
  });

  it('formats durations', () => {
    expect(formatDuration(20_000)).toBe('menos de 1 min');
    expect(formatDuration(45 * 60_000)).toBe('45 min');
    expect(formatDuration(125 * 60_000)).toBe('2 h 5 min');
    expect(formatDuration(null)).toBe('—');
  });

  it('shows paths relative to the session folder', () => {
    expect(relativePath('/repo/src/a.ts', '/repo')).toBe('src/a.ts');
    expect(relativePath('/other/a.ts', '/repo')).toBe('/other/a.ts');
  });
});

describe('formatBytes', () => {
  it('uses KB below one megabyte and the memory format above', () => {
    expect(formatBytes(0)).toBe('0 KB');
    expect(formatBytes(300)).toBe('1 KB');
    expect(formatBytes(512 * 1024)).toBe('512 KB');
    expect(formatBytes(34 * 1024 * 1024)).toBe('34 MB');
  });
});
