import {
  fetchCategories,
  fetchIndexProgress,
  fetchLibrary,
  fetchSessionSummary,
  fetchTags,
  pinSession,
  renameSession,
  searchLibrary,
  setSessionCategory,
  setSessionTags,
} from '../api/commands';
import { onLibraryChanged, onLibraryProgress } from '../api/events';
import { reportError, type FailureCause } from '../api/logger';
import type { IndexProgress, SearchHit, SessionListItem, SessionSummary } from '../api/types';
import { groupLibrary } from '../library/grouping';

const SEARCH_DEBOUNCE_MS = 150;
const RELOAD_DEBOUNCE_MS = 250;

function logged<T>(context: string, fallback: T): (cause: FailureCause) => T {
  return (cause) => {
    reportError(context, cause);
    return fallback;
  };
}

class LibraryStore {
  items = $state<SessionListItem[]>([]);
  progress = $state<IndexProgress>({ running: false, filesDone: 0, filesTotal: 0 });
  query = $state('');
  hits = $state<SearchHit[]>([]);
  selectedId = $state<string | null>(null);
  summary = $state<SessionSummary | null>(null);
  knownTags = $state<string[]>([]);
  knownCategories = $state<string[]>([]);

  groups = $derived(groupLibrary(this.items));
  pinned = $derived(this.groups.pinned);
  itemsById = $derived(new Map(this.items.map((item) => [item.id, item])));

  private searchTimer: ReturnType<typeof setTimeout> | null = null;
  private reloadTimer: ReturnType<typeof setTimeout> | null = null;

  async start(): Promise<void> {
    await onLibraryChanged(() => this.scheduleReload());
    await onLibraryProgress((progress) => (this.progress = progress));
    this.progress = await fetchIndexProgress().catch(logged('falha ao ler progresso da indexação', this.progress));
    await this.reload();
  }

  async reload(): Promise<void> {
    this.items = await fetchLibrary().catch(logged('falha ao carregar a biblioteca', this.items));
    this.knownTags = await fetchTags().catch(logged('falha ao carregar tags', this.knownTags));
    this.knownCategories = await fetchCategories().catch(logged('falha ao carregar categorias', this.knownCategories));
    if (this.query.trim()) await this.runSearch();
    if (this.selectedId) await this.loadSummary(this.selectedId);
  }

  private scheduleReload(): void {
    if (this.reloadTimer) clearTimeout(this.reloadTimer);
    this.reloadTimer = setTimeout(() => void this.reload(), RELOAD_DEBOUNCE_MS);
  }

  setQuery(query: string): void {
    this.query = query;
    if (this.searchTimer) clearTimeout(this.searchTimer);
    if (!query.trim()) {
      this.hits = [];
      return;
    }
    this.searchTimer = setTimeout(() => void this.runSearch(), SEARCH_DEBOUNCE_MS);
  }

  private async runSearch(): Promise<void> {
    const query = this.query;
    const hits = await searchLibrary(query).catch(logged('falha na busca', []));
    if (query === this.query) this.hits = hits;
  }

  async select(sessionId: string): Promise<void> {
    this.selectedId = sessionId;
    await this.loadSummary(sessionId);
  }

  private async loadSummary(sessionId: string): Promise<void> {
    const summary = await fetchSessionSummary(sessionId).catch(logged('falha ao carregar o resumo', null));
    if (this.selectedId === sessionId) this.summary = summary;
  }

  async rename(sessionId: string, name: string | null): Promise<void> {
    await renameSession(sessionId, name).catch(logged('falha ao renomear a sessão', undefined));
    await this.reload();
  }

  async togglePin(sessionId: string): Promise<void> {
    const pinned = this.itemsById.get(sessionId)?.pinned ?? false;
    await pinSession(sessionId, !pinned).catch(logged('falha ao fixar a sessão', undefined));
    await this.reload();
  }

  async setTags(sessionId: string, tags: string[]): Promise<void> {
    await setSessionTags(sessionId, tags).catch(logged('falha ao salvar tags', tags));
    await this.reload();
  }

  async setCategory(sessionId: string, category: string | null): Promise<void> {
    await setSessionCategory(sessionId, category).catch(logged('falha ao salvar categoria', undefined));
    await this.reload();
  }
}

export const libraryStore = new LibraryStore();
