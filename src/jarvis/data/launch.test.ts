import { describe, expect, it } from 'vitest';
import { clipAction, launchOptions } from './launch';

describe('launchOptions', () => {
  it('shows the live sessions unless the window was opened for mock data', () => {
    expect(launchOptions('').source).toBe('live');
    expect(launchOptions('?source=mock').source).toBe('mock');
    expect(launchOptions('?source=anything').source).toBe('live');
  });

  it('takes a canvas pixel ratio for measurements, within 0.5 to 3', () => {
    expect(launchOptions('').pixelRatio).toBeNull();
    expect(launchOptions('?source=mock&pixelRatio=2').pixelRatio).toBe(2);
    expect(launchOptions('?pixelRatio=9').pixelRatio).toBe(3);
    expect(launchOptions('?pixelRatio=0.1').pixelRatio).toBe(0.5);
    expect(launchOptions('?pixelRatio=abc').pixelRatio).toBeNull();
    expect(launchOptions('?pixelRatio=').pixelRatio).toBeNull();
  });
});

describe('clipAction', () => {
  it('keeps short actions as they are', () => {
    expect(clipAction('Edit · timeline-filter.component.ts')).toBe('Edit · timeline-filter.component.ts');
  });

  it('cuts long actions to 96 characters with an ellipsis', () => {
    const clipped = clipAction(`Bash · ${'x'.repeat(200)}`);
    expect(clipped).toHaveLength(96);
    expect(clipped.endsWith('…')).toBe(true);
  });

  it('flattens line breaks so a multi-line question fits the balloon', () => {
    expect(clipAction('Quer que eu rode\n  de novo?')).toBe('Quer que eu rode de novo?');
  });
});
