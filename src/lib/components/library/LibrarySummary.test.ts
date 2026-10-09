import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { SessionSummary } from '../../api/types';
import LibrarySummary from './LibrarySummary.svelte';

const SUMMARY: SessionSummary = {
  item: {
    id: 's1',
    title: 'Paginação na listagem de pedidos',
    customName: null,
    firstPrompt: 'Adicione paginação na listagem de pedidos',
    project: 'web-dashboard',
    cwd: '/code/web-dashboard',
    branch: 'feat-orders-pagination',
    startedAt: 1_000,
    updatedAt: 2_000,
    pinned: false,
    pinOrder: null,
    category: null,
    tags: [],
    costUsd: 84.56,
    filesCount: 0,
  },
  aiTitle: null,
  lastPrompt: null,
  lastAssistant: null,
  cwdExists: true,
  repoRoot: '/code/web-dashboard',
  model: 'claude-opus-5-5',
  prUrl: null,
  durationMs: 1_000,
  usage: { input: 1_000, output: 920_000, cacheRead: 263_600_000, cacheWrite5m: 0, cacheWrite1h: 2_110_000 },
  unknownPricing: false,
  contextTokens: 120_000,
  contextWindow: 1_000_000,
  contextPercent: 12,
  files: [],
};

describe('LibrarySummary', () => {
  it('counts only the tokens Claude wrote, without cache re-reads or a dollar value', () => {
    render(LibrarySummary, { summary: SUMMARY, knownTags: [], knownCategories: [], onTagsChange: vi.fn(), onCategoryChange: vi.fn() });
    expect(screen.getByText('Tokens escritos')).toBeInTheDocument();
    expect(screen.getByText('920 mil')).toBeInTheDocument();
    expect(screen.queryByText(/US\$/)).not.toBeInTheDocument();
    expect(screen.queryByText('Custo equivalente')).not.toBeInTheDocument();
  });
});
