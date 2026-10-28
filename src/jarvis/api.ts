import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ActionUpdate } from '../lib/api/types';
import type { MockData } from './data/mock-sessions';
import type { LockSnapshot, RepousoPreparation, UnlockResult } from './repouso-types';
import type { ShapeRule } from './scene/shape-rules';

export function closeJarvis(): Promise<void> {
  return invoke<void>('jarvis_close');
}

/** Leaves the Jarvis mode and opens the session in the Focus view of the main window. */
export function focusSessionFromJarvis(key: string): Promise<void> {
  return invoke<void>('jarvis_focus_session', { key });
}

/** Leaves the Jarvis mode and opens the new-session dialog in the main window. */
export function newSessionFromJarvis(): Promise<void> {
  return invoke<void>('jarvis_new_session');
}

/** Rules from `jarvis-shapes.json` in the app data folder; empty when the file does not exist. */
export function fetchShapeRules(): Promise<ShapeRule[]> {
  return invoke<ShapeRule[]>('jarvis_shape_rules');
}

/** Development builds only: sessions from the file named by `CCM_JARVIS_MOCK`, if any. */
export function fetchMockData(): Promise<MockData | null> {
  return invoke<MockData | null>('jarvis_mock_data');
}

/** Emitted on every change of a session's current action; only this window listens. */
export function onSessionAction(handler: (update: ActionUpdate) => void): Promise<UnlistenFn> {
  return listen<ActionUpdate>('session:action', (event) => handler(event.payload));
}

export function prepareRepouso(): Promise<RepousoPreparation> {
  return invoke<RepousoPreparation>('repouso_prepare');
}

/** Locks every display in the Jarvis mode until Touch ID or the user's password unlocks it. */
export function activateRepouso(showDetails: boolean): Promise<LockSnapshot> {
  return invoke<LockSnapshot>('repouso_activate', { showDetails });
}

/** Resolves once the user answers the system prompt. */
export function unlockRepouso(): Promise<UnlockResult> {
  return invoke<UnlockResult>('repouso_unlock');
}

export function fetchRepousoState(): Promise<LockSnapshot> {
  return invoke<LockSnapshot>('repouso_state');
}

export function onRepousoChanged(handler: (snapshot: LockSnapshot) => void): Promise<UnlistenFn> {
  return listen<LockSnapshot>('repouso:changed', (event) => handler(event.payload));
}
