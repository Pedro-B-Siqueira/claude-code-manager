import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, IndexProgress, SearchHit, SessionListItem, SessionSummary } from './types';

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
