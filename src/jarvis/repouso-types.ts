/** Mirrors `LockSnapshot` in `repouso/lock.rs`, sent with `repouso:changed`. */
export interface LockSnapshot {
  active: boolean;
  since: number | null;
  showDetails: boolean;
  authenticating: boolean;
}

/** When macOS asks for the password after the display goes dark (`sysadminctl -screenLock status`). */
export type LockDelay = { kind: 'immediate' } | { kind: 'seconds'; seconds: number } | { kind: 'off' } | { kind: 'unknown' };

export interface RepousoPreparation {
  lockDelay: LockDelay;
  showDetails: boolean;
}

/** The unlock button: ready, waiting for the system prompt, or after a refused attempt. */
export type UnlockState = 'idle' | 'asking' | 'failed';

export type UnlockResult = 'unlocked' | 'refused' | 'notLocked' | 'alreadyAsking';

export const UNLOCKED: LockSnapshot = { active: false, since: null, showDetails: true, authenticating: false };
