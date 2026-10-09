import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/svelte';
import { afterEach, vi } from 'vitest';

// jsdom has no layout engine, so it does not implement scrolling.
// Component tests render macOS labels on any machine; Linux behavior is tested through explicit parameters.
Object.defineProperty(globalThis.navigator, 'userAgent', { value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15', configurable: true });

Element.prototype.scrollIntoView = vi.fn();

// There is no Tauri runtime in jsdom; failures are still logged by the code under test.
vi.mock('@tauri-apps/plugin-log', () => ({
  error: vi.fn(async () => undefined),
  warn: vi.fn(async () => undefined),
  info: vi.fn(async () => undefined),
}));

afterEach(() => {
  cleanup();
});
