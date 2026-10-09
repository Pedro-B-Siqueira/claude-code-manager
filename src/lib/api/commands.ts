import { invoke, type Channel } from '@tauri-apps/api/core';
import type {
  ActivityItem,
  AppInfo,
  AppSettings,
  Attachment,
  AttachmentUsage,
  EditDetail,
  GitStatus,
  IndexProgress,
  LiveSessionView,
  SearchHit,
  SessionListItem,
  SessionSummary,
  SubmitOutcome,
  OpenOutcome,
  PanelLayout,
  WorktreeInfo,
  WorktreePlan,
} from './types';

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

export function newSession(cwd: string): Promise<LiveSessionView> {
  return invoke<LiveSessionView>('session_new', { cwd });
}

export function resumeSession(sessionId: string): Promise<LiveSessionView> {
  return invoke<LiveSessionView>('session_resume', { sessionId });
}

export function closeSession(key: string): Promise<void> {
  return invoke<void>('session_close', { key });
}

export function fetchLiveSessions(): Promise<LiveSessionView[]> {
  return invoke<LiveSessionView[]>('live_list');
}

export function fetchRecentDirs(): Promise<string[]> {
  return invoke<string[]>('recent_dirs');
}

export type TerminalChunk = ArrayBuffer | number[];

export function attachTerminal(key: string, output: Channel<TerminalChunk>): Promise<void> {
  return invoke<void>('pty_attach', { key, output });
}

export function detachTerminal(key: string): Promise<void> {
  return invoke<void>('pty_detach', { key });
}

export function writeTerminal(key: string, data: string): Promise<void> {
  return invoke<void>('pty_write', { key, data });
}

export function resizeTerminal(key: string, cols: number, rows: number): Promise<void> {
  return invoke<void>('pty_resize', { key, cols, rows });
}

export function quitApp(): Promise<void> {
  return invoke<void>('app_quit');
}

export function fetchAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>('app_info');
}

export function takeNotifiedSession(): Promise<string | null> {
  return invoke<string | null>('take_notified_session');
}

export function fetchFileEdits(sessionId: string, filePath: string): Promise<EditDetail[]> {
  return invoke<EditDetail[]>('library_file_edits', { sessionId, filePath });
}

export function fetchEdit(editId: number): Promise<EditDetail | null> {
  return invoke<EditDetail | null>('library_edit', { editId });
}

export function fetchActivity(sessionId: string, limit: number): Promise<ActivityItem[]> {
  return invoke<ActivityItem[]>('library_activity', { sessionId, limit });
}

export function fetchGitStatus(cwd: string): Promise<GitStatus> {
  return invoke<GitStatus>('git_status', { cwd });
}

export function planWorktree(cwd: string, prefix: string, name: string): Promise<WorktreePlan> {
  return invoke<WorktreePlan>('worktree_plan', { cwd, prefix, name });
}

export function createWorktree(cwd: string, prefix: string, name: string): Promise<LiveSessionView> {
  return invoke<LiveSessionView>('worktree_create', { cwd, prefix, name });
}

export function listWorktrees(cwd: string): Promise<WorktreeInfo[]> {
  return invoke<WorktreeInfo[]>('worktree_list', { cwd });
}

export function removeWorktree(cwd: string, path: string): Promise<void> {
  return invoke<void>('worktree_remove', { cwd, path });
}

export function openVsCode(path: string, expectedBranch: string | null): Promise<OpenOutcome> {
  return invoke<OpenOutcome>('open_vscode', { path, expectedBranch });
}

export function openFinder(path: string): Promise<void> {
  return invoke<void>('open_finder', { path });
}

export function openPullRequest(sessionId: string): Promise<string> {
  return invoke<string>('open_pr', { sessionId });
}

export function fetchLayout(): Promise<PanelLayout> {
  return invoke<PanelLayout>('layout_get');
}

export function saveLayout(layout: PanelLayout): Promise<PanelLayout> {
  return invoke<PanelLayout>('layout_set', { layout });
}

export function fetchGridOrder(): Promise<string[]> {
  return invoke<string[]>('grid_order_get');
}

export function saveGridOrder(sessionIds: string[]): Promise<void> {
  return invoke<void>('grid_order_set', { sessionIds });
}

export function wakeSession(key: string): Promise<LiveSessionView> {
  return invoke<LiveSessionView>('session_wake', { key });
}

export function setUiVisible(visible: boolean): Promise<void> {
  return invoke<void>('ui_set_visible', { visible });
}

/** The image goes as the raw request body. */
export function saveAttachment(bytes: Uint8Array<ArrayBuffer>): Promise<Attachment> {
  return invoke<Attachment>('attachment_save', bytes);
}

export function importAttachment(path: string): Promise<Attachment> {
  return invoke<Attachment>('attachment_import', { path });
}

export function fetchAttachmentPreview(id: string): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>('attachment_preview', { id });
}

export function openAttachment(id: string): Promise<void> {
  return invoke<void>('attachment_open', { id });
}

export function removeAttachment(id: string): Promise<void> {
  return invoke<void>('attachment_remove', { id });
}

export function fetchAttachmentUsage(): Promise<AttachmentUsage> {
  return invoke<AttachmentUsage>('attachments_usage');
}

export function clearAttachments(): Promise<AttachmentUsage> {
  return invoke<AttachmentUsage>('attachments_clear');
}

export function submitWithImages(key: string, attachmentIds: string[]): Promise<SubmitOutcome> {
  return invoke<SubmitOutcome>('session_submit_with_images', { key, attachmentIds });
}

export function openLink(url: string): Promise<void> {
  return invoke<void>('open_link', { url });
}
