#!/usr/bin/env node
// Proves the main window never loads Jarvis or Repouso code: walks every file the built index.html can reach
// (static and dynamic imports, CSS) and fails if any of them carries text that only Jarvis code has.
// Usage: node scripts/check-jarvis-isolation.mjs [distDir]
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const JARVIS_MARKERS = ['Arraste para girar', 'jarvis_close', 'Nenhuma sessão ativa', 'Girar sozinho', 'Ativar repouso?', 'Bloqueado pelo Claude Code Manager'];

const distDir = process.argv[2] ?? 'dist';
const manifest = JSON.parse(readFileSync(join(distDir, '.vite', 'manifest.json'), 'utf8'));

function reachableFiles(entryKey) {
  const files = new Set();
  const pending = [entryKey];
  const seen = new Set();
  while (pending.length > 0) {
    const key = pending.pop();
    if (seen.has(key)) continue;
    seen.add(key);
    const chunk = manifest[key];
    if (!chunk) throw new Error(`manifest has no entry "${key}"`);
    files.add(chunk.file);
    for (const css of chunk.css ?? []) files.add(css);
    pending.push(...(chunk.imports ?? []), ...(chunk.dynamicImports ?? []));
  }
  return [...files];
}

function markersIn(files) {
  return files.flatMap((file) => {
    const text = readFileSync(join(distDir, file), 'utf8');
    return JARVIS_MARKERS.filter((marker) => text.includes(marker)).map((marker) => `${file}: "${marker}"`);
  });
}

function totalBytes(files) {
  return files.reduce((sum, file) => sum + readFileSync(join(distDir, file)).byteLength, 0);
}

const mainFiles = reachableFiles('index.html');
const jarvisFiles = [...new Set([...reachableFiles('jarvis.html'), ...reachableFiles('lock.html')])];
const leaks = markersIn(mainFiles);
const jarvisOnly = jarvisFiles.filter((file) => !mainFiles.includes(file));

if (markersIn(jarvisFiles).length === 0) {
  console.error('check-jarvis-isolation: the Jarvis bundle has none of the markers; update JARVIS_MARKERS.');
  process.exit(2);
}
if (leaks.length > 0) {
  console.error(`check-jarvis-isolation: Jarvis code reachable from the main window:\n  ${leaks.join('\n  ')}`);
  process.exit(1);
}
console.log(`check-jarvis-isolation: ok. Main window loads ${mainFiles.length} files (${(totalBytes(mainFiles) / 1024).toFixed(1)} KiB), none from Jarvis.`);
console.log(`Jarvis-only files: ${jarvisOnly.join(', ')} (${(totalBytes(jarvisOnly) / 1024).toFixed(1)} KiB), loaded only by its own window.`);
