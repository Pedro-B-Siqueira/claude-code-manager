import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { IndexProgress } from './types';

export function onLibraryChanged(handler: () => void): Promise<UnlistenFn> {
  return listen<null>('library:changed', () => handler());
}

export function onLibraryProgress(handler: (progress: IndexProgress) => void): Promise<UnlistenFn> {
  return listen<IndexProgress>('library:progress', (event) => handler(event.payload));
}
