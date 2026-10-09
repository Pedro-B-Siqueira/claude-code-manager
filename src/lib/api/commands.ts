import { invoke, type Channel } from '@tauri-apps/api/core';
import type { AppSettings, IndexProgress, LiveSessionView, SearchHit, SessionListItem, SessionSummary } from './types';

export function fetchSettings(): Promise<AppSettings> {
  return invoke<AppSettings>('settings_get');
}

export function saveSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke<AppSettings>('settings_update', { settings });
}

export function fetchLibrary(): Promise<SessionListItem[]> {
  return invoke<SessionListItem[]>('library_list');
}

export function searchLibrary(query: string): Promise<SearchHit[]> {
  return invoke<SearchHit[]>('library_search', { query });
}

export function fetchSessionSummary(sessionId: string): Promise<SessionSummary | null> {
  return invoke<SessionSummary | null>('library_summary', { sessionId });
}

export function renameSession(sessionId: string, name: string | null): Promise<void> {
  return invoke<void>('library_rename', { sessionId, name });
}

export function pinSession(sessionId: string, pinned: boolean): Promise<void> {
  return invoke<void>('library_pin', { sessionId, pinned });
}

export function setSessionTags(sessionId: string, tags: string[]): Promise<string[]> {
  return invoke<string[]>('library_set_tags', { sessionId, tags });
}

export function setSessionCategory(sessionId: string, category: string | null): Promise<void> {
  return invoke<void>('library_set_category', { sessionId, category });
}

export function fetchTags(): Promise<string[]> {
  return invoke<string[]>('library_tags');
}

export function fetchCategories(): Promise<string[]> {
  return invoke<string[]>('library_categories');
}

export function fetchIndexProgress(): Promise<IndexProgress> {
  return invoke<IndexProgress>('library_status');
}

export function newSession(cwd: string): Promise<LiveSessionView> {
  return invoke<LiveSessionView>('session_new', { cwd });
}

export function resumeSession(sessionId: string): Promise<LiveSessionView> {
  return invoke<LiveSessionView>('session_resume', { sessionId });
}

export function closeSession(key: string): Promise<void> {
  return invoke<void>('session_close', { key });
}

export function fetchLiveSessions(): Promise<LiveSessionView[]> {
  return invoke<LiveSessionView[]>('live_list');
}

export function fetchRecentDirs(): Promise<string[]> {
  return invoke<string[]>('recent_dirs');
}

export type TerminalChunk = ArrayBuffer | number[];

export function attachTerminal(key: string, output: Channel<TerminalChunk>): Promise<void> {
  return invoke<void>('pty_attach', { key, output });
}

export function detachTerminal(key: string): Promise<void> {
  return invoke<void>('pty_detach', { key });
}

export function writeTerminal(key: string, data: string): Promise<void> {
  return invoke<void>('pty_write', { key, data });
}

export function resizeTerminal(key: string, cols: number, rows: number): Promise<void> {
  return invoke<void>('pty_resize', { key, cols, rows });
}

export function quitApp(): Promise<void> {
  return invoke<void>('app_quit');
}
