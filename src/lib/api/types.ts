export type Theme = 'dark' | 'light';

export type NotificationPreference = 'auto' | 'enabled' | 'disabled';

export interface HibernationSettings {
  enabled: boolean;
  idleMinutes: number;
}

export interface AppSettings {
  theme: Theme;
  worktreeRoot: string | null;
  branchPrefixes: string[];
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
  pid: number | null;
  hibernated: boolean;
  pinned: boolean;
  exited: boolean;
  startedAt: number;
}

export interface ClaudeGaugeStatus {
  appInstalled: boolean;
  hookInstalled: boolean;
}

export interface AppInfo {
  claudegauge: ClaudeGaugeStatus;
  notificationsEnabled: boolean;
  hooksActive: boolean;
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

export type DiffLineKind = 'context' | 'added' | 'removed' | 'gap';

export interface DiffLine {
  kind: DiffLineKind;
  text: string;
}

export interface EditDetail {
  id: number;
  tool: string;
  filePath: string;
  timestamp: number | null;
  newStart: number | null;
  newEnd: number | null;
  added: number;
  removed: number;
  isNewFile: boolean;
  why: string | null;
  lines: DiffLine[];
  truncated: boolean;
}

export interface ActivityItem {
  id: number;
  timestamp: number | null;
  kind: string;
  tool: string | null;
  target: string | null;
  editId: number | null;
  fromSubagent: boolean;
}

export interface FileStat {
  path: string;
  added: number;
  removed: number;
}

export interface GitStatus {
  branch: string | null;
  diff: { added: number; removed: number; files: FileStat[] };
}

export interface WorktreePlan {
  repoRoot: string;
  repoName: string;
  root: string;
  rootInferred: boolean;
  path: string;
  branch: string;
  base: string;
}

export interface WorktreeInfo {
  path: string;
  branch: string | null;
  isMain: boolean;
  detached: boolean;
  clean: boolean | null;
  inUse: boolean;
}

export interface OpenOutcome {
  warning: string | null;
}
