import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from './types';

export function fetchSettings(): Promise<AppSettings> {
  return invoke<AppSettings>('settings_get');
}

export function saveSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke<AppSettings>('settings_update', { settings });
}
