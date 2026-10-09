import { error as logError, warn as logWarn } from '@tauri-apps/plugin-log';
import type { AppErrorPayload } from './types';

/** What a rejected `invoke` can carry: the backend's serialized `AppError`, a JS `Error` or a plain message. */
export type FailureCause = AppErrorPayload | Error | string;

export function describeFailure(cause: FailureCause): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === 'string') return cause;
  return `${cause.kind}: ${cause.message}`;
}

export function reportError(context: string, cause: FailureCause): void {
  void logError(`${context}: ${describeFailure(cause)}`);
}

export function reportWarning(context: string, cause: FailureCause): void {
  void logWarn(`${context}: ${describeFailure(cause)}`);
}
