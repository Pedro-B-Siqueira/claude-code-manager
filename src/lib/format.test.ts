import { describe, expect, it } from 'vitest';
import { fileName, formatCost, formatMemory } from './format';

describe('format helpers', () => {
  it('formats cost as US dollars in pt-BR', () => {
    expect(formatCost(3.12)).toMatch(/US\$\s?3,12/);
  });

  it('formats memory in MB or GB', () => {
    expect(formatMemory(512)).toBe('512 MB');
    expect(formatMemory(1536)).toBe('1,5 GB');
  });

  it('extracts the file name from a path', () => {
    expect(fileName('src/app/order.component.ts')).toBe('order.component.ts');
    expect(fileName('README.md')).toBe('README.md');
  });
});
