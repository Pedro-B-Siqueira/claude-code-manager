import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { SessionListItem } from '../api/types';
import { libraryStore } from './library.svelte';
import { toastStore } from './toasts.svelte';

function item(id: string): SessionListItem {
  return { id, title: id, customName: null, firstPrompt: null, project: 'app', cwd: '/app', branch: null, startedAt: null, updatedAt: null, pinned: false, pinOrder: null, category: null, tags: [], costUsd: 0, filesCount: 0 };
}

let library: SessionListItem[] = [];
const deleteSession = vi.fn(async (sessionId: string) => {
  library = library.filter((candidate) => candidate.id !== sessionId);
});

vi.mock('../api/commands', () => ({
  fetchLibrary: vi.fn(async () => library),
  fetchTags: vi.fn(async () => []),
  fetchCategories: vi.fn(async () => []),
  fetchSessionSummary: vi.fn(async () => null),
  deleteSession: (sessionId: string) => deleteSession(sessionId),
}));

describe('libraryStore.deleteConversation', () => {
  beforeEach(async () => {
    library = [item('a'), item('b')];
    toastStore.items = [];
    deleteSession.mockClear();
    await libraryStore.reload();
    libraryStore.selectedId = 'a';
  });

  it('removes the conversation, clears its selection and says where it went', async () => {
    await libraryStore.deleteConversation('a');
    expect(deleteSession).toHaveBeenCalledWith('a');
    expect(libraryStore.items.map((candidate) => candidate.id)).toEqual(['b']);
    expect(libraryStore.selectedId).toBeNull();
    expect(toastStore.items.at(-1)?.message).toBe('Conversa movida para a Lixeira.');
  });

  it('keeps everything and shows the reason when the app refuses', async () => {
    deleteSession.mockRejectedValueOnce({ kind: 'invalid', message: 'feche a sessão antes de apagar a conversa' });
    await libraryStore.deleteConversation('a');
    expect(libraryStore.items).toHaveLength(2);
    expect(libraryStore.selectedId).toBe('a');
    expect(toastStore.items.at(-1)).toMatchObject({ tone: 'error' });
    expect(toastStore.items.at(-1)?.message).toContain('feche a sessão antes');
  });
});
