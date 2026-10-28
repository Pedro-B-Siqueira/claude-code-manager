import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    target: 'safari17',
    sourcemap: false,
    // The manifest lets scripts/check-jarvis-isolation.mjs prove the main window never loads Jarvis code.
    manifest: true,
    rollupOptions: {
      input: { main: 'index.html', jarvis: 'jarvis.html', lock: 'lock.html' },
    },
  },
});
