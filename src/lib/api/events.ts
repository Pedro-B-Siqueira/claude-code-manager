import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { IndexProgress } from './types';

export function onLibraryChanged(handler: () => void): Promise<UnlistenFn> {
  return listen<null>('library:changed', () => handler());
}

export function onLibraryProgress(handler: (progress: IndexProgress) => void): Promise<UnlistenFn> {
  return listen<IndexProgress>('library:progress', (event) => handler(event.payload));
}

export interface PreviewPayload {
  key: string;
  lines: string[];
}

export interface ExitPayload {
  key: string;
  code: number | null;
}

export function onLiveChanged(handler: () => void): Promise<UnlistenFn> {
  return listen<null>('live:changed', () => handler());
}

export function onSessionPreview(handler: (payload: PreviewPayload) => void): Promise<UnlistenFn> {
  return listen<PreviewPayload>('session:preview', (event) => handler(event.payload));
}

export function onSessionExited(handler: (payload: ExitPayload) => void): Promise<UnlistenFn> {
  return listen<ExitPayload>('session:exited', (event) => handler(event.payload));
}

export function onCloseRequested(handler: (runningSessions: number) => void): Promise<UnlistenFn> {
  return listen<number>('app:close-requested', (event) => handler(event.payload));
}
