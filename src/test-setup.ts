import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/svelte';
import { afterEach, vi } from 'vitest';

// jsdom has no layout engine, so it does not implement scrolling.
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
