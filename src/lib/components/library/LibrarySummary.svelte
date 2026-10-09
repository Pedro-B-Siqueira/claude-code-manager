<script lang="ts">
  import type { SessionSummary } from '../../api/types';
  import { formatCost, formatDuration, formatRelativeTime, formatTokens, relativePath } from '../../format';
  import { diffHover } from '../../stores/diff-popover.svelte';
  import Icon from '../common/Icon.svelte';
  import ContextMeter from '../sessions/ContextMeter.svelte';
  import TagEditor from './TagEditor.svelte';

  interface Props {
    summary: SessionSummary;
    knownTags: string[];
    knownCategories: string[];
    onTagsChange: (tags: string[]) => void;
    onCategoryChange: (category: string | null) => void;
  }

  let { summary, knownTags, knownCategories, onTagsChange, onCategoryChange }: Props = $props();

  const VISIBLE_FILES = 8;
  let showAllFiles = $state(false);

  const item = $derived(summary.item);
  const totalTokens = $derived(
    summary.usage.input + summary.usage.output + summary.usage.cacheRead + summary.usage.cacheWrite5m + summary.usage.cacheWrite1h,
  );
  const visibleFiles = $derived(showAllFiles ? summary.files : summary.files.slice(0, VISIBLE_FILES));
  const categoriesListId = 'known-categories';

  function commitCategory(event: Event & { currentTarget: HTMLInputElement }): void {
    const value = event.currentTarget.value.trim();
    if (value !== (item.category ?? '')) onCategoryChange(value === '' ? null : value);
  }
</script>

<article class="summary" aria-label="Resumo da sessão">
  <header>
    <h3>{item.title}</h3>
    <div class="chips">
      <span class="chip">{item.project}</span>
      {#if item.branch}<span class="chip mono">{item.branch}</span>{/if}
      <span class="chip">{formatRelativeTime(item.updatedAt)}</span>
      <span class="chip">{formatDuration(summary.durationMs)}</span>
      {#if summary.model}<span class="chip mono">{summary.model}</span>{/if}
    </div>
  </header>

  {#if !summary.cwdExists}
    <p class="warning" role="note">
      <Icon name="warning" size={14} />
      <span>A pasta original não existe mais: <span class="mono">{item.cwd ?? '—'}</span></span>
    </p>
  {/if}

  <section>
    <h4>Pedido inicial</h4>
    <p class="text clamp">{item.firstPrompt ?? '—'}</p>
  </section>

  <section>
    <h4>Onde parou</h4>
    <p class="text clamp tall">{summary.lastAssistant ?? '—'}</p>
  </section>

  <section>
    <h4>Arquivos tocados <span class="count mono">{summary.files.length}</span></h4>
    {#if summary.files.length === 0}
      <p class="muted">Nenhum arquivo alterado.</p>
    {:else}
      <ul class="files">
        {#each visibleFiles as file (file.path)}
          <li>
            <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so the diff opens from the keyboard) -->
            <span
              class="path mono"
              title={file.path}
              tabindex="0"
              use:diffHover={{ kind: 'file', sessionId: item.id, filePath: file.path, basePath: item.cwd }}
            >{relativePath(file.path, item.cwd)}</span>
            {#if file.isNewFile}<span class="new">novo</span>{/if}
            <span class="delta mono"><span class="added">+{file.added}</span> <span class="removed">−{file.removed}</span></span>
          </li>
        {/each}
      </ul>
      {#if summary.files.length > VISIBLE_FILES}
        <button type="button" class="link" onclick={() => (showAllFiles = !showAllFiles)}>
          {showAllFiles ? 'Mostrar menos' : `Mostrar todos (${summary.files.length})`}
        </button>
      {/if}
    {/if}
  </section>

  <section class="usage">
    <h4>Uso</h4>
    <dl>
      <dt>Tokens</dt>
      <dd class="mono">{formatTokens(totalTokens)}</dd>
      <dt>Custo equivalente</dt>
      <dd class="mono">{formatCost(item.costUsd)}{summary.unknownPricing ? ' + modelo sem preço' : ''}</dd>
    </dl>
    <ContextMeter percent={summary.contextPercent} />
  </section>

  {#if summary.prUrl}
    <section>
      <h4>Pull request</h4>
      <p class="mono pr"><Icon name="pullRequest" size={13} />{summary.prUrl}</p>
    </section>
  {/if}

  <section class="organize">
    <h4>Organização</h4>
    <TagEditor tags={item.tags} suggestions={knownTags} onChange={onTagsChange} />
    <input
      class="category"
      aria-label="Categoria"
      placeholder="Categoria (ex.: feature, bug, revisão)"
      list={categoriesListId}
      value={item.category ?? ''}
      onchange={commitCategory}
    />
    <datalist id={categoriesListId}>
      {#each knownCategories as category (category)}<option value={category}></option>{/each}
    </datalist>
  </section>
</article>

<style>
  .summary {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  h3 {
    margin: 0 0 8px;
    font-size: 16px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }

  h4 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    padding: 1px 8px;
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    font-size: 11.5px;
    color: var(--muted);
  }

  .warning {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin: 0;
    padding: 8px 10px;
    border-radius: 9px;
    background: color-mix(in srgb, var(--amber) var(--status-badge-alpha), transparent);
    color: var(--amber);
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  .text {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.55;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .clamp {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 5;
    line-clamp: 5;
    overflow: hidden;
  }

  .clamp.tall {
    -webkit-line-clamp: 8;
    line-clamp: 8;
  }

  .muted {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  .files {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 3px;
  }

  .files li {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }

  .new {
    padding: 0 6px;
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--green) var(--status-badge-alpha), transparent);
    color: var(--green);
    font-size: 10.5px;
  }

  .delta {
    flex-shrink: 0;
  }

  .added {
    color: var(--green);
  }

  .removed {
    color: var(--diff-removed);
  }

  .count {
    font-weight: 500;
    letter-spacing: 0;
  }

  .link {
    margin-top: 6px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-size: 12px;
    cursor: pointer;
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    margin: 0 0 10px;
    font-size: 12.5px;
  }

  dt {
    color: var(--muted);
  }

  dd {
    margin: 0;
  }

  .pr {
    display: flex;
    gap: 6px;
    align-items: center;
    margin: 0;
    font-size: 12px;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .organize {
    display: grid;
    gap: 8px;
  }

  .category {
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    user-select: text;
  }

  .category:focus {
    outline: none;
    border-color: var(--accent);
  }
</style>
