export type Theme = 'dark' | 'light';

export type NotificationPreference = 'auto' | 'enabled' | 'disabled';

export interface HibernationSettings {
  enabled: boolean;
  idleMinutes: number;
}

export interface AppSettings {
  theme: Theme;
  worktreeRoot: string;
  scrollbackLines: number;
  notifications: NotificationPreference;
  hibernation: HibernationSettings;
  claudeBinary: string | null;
}

export interface AppErrorPayload {
  kind: string;
  message: string;
}

export type SessionStatus = 'working' | 'permission' | 'waiting' | 'done' | 'idle';

export type SessionOrigin = 'app' | 'external';

export interface FileChangeSummary {
  path: string;
  added: number;
  removed: number;
}

export interface LiveSessionView {
  key: string;
  sessionId: string;
  title: string;
  repo: string;
  cwd: string;
  branch: string | null;
  isWorktree: boolean;
  status: SessionStatus;
  statusDetail: string | null;
  previewLines: string[];
  files: FileChangeSummary[];
  contextPercent: number | null;
  totalTokens: number;
  costUsd: number;
  memoryMb: number | null;
  origin: SessionOrigin;
  hibernated: boolean;
  pinned: boolean;
}
