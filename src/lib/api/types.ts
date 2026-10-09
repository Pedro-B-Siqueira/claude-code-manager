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

export interface TokenUsage {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite5m: number;
  cacheWrite1h: number;
}

export interface SessionListItem {
  id: string;
  title: string;
  customName: string | null;
  firstPrompt: string | null;
  project: string;
  cwd: string | null;
  branch: string | null;
  startedAt: number | null;
  updatedAt: number | null;
  pinned: boolean;
  pinOrder: number | null;
  category: string | null;
  tags: string[];
  costUsd: number;
  filesCount: number;
}

export interface FileTouched {
  path: string;
  added: number;
  removed: number;
  edits: number;
  isNewFile: boolean;
}

export interface SessionSummary {
  item: SessionListItem;
  aiTitle: string | null;
  lastPrompt: string | null;
  lastAssistant: string | null;
  cwdExists: boolean;
  repoRoot: string | null;
  model: string | null;
  prUrl: string | null;
  durationMs: number | null;
  usage: TokenUsage;
  unknownPricing: boolean;
  contextTokens: number | null;
  contextWindow: number;
  contextPercent: number | null;
  files: FileTouched[];
}

export type MatchSource = 'title' | 'project' | 'branch' | 'file' | 'tag' | 'text';

export interface SearchHit {
  sessionId: string;
  matchedIn: MatchSource;
  snippet: string | null;
}

export interface IndexProgress {
  running: boolean;
  filesDone: number;
  filesTotal: number;
}
