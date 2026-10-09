<script lang="ts">
  import type { FileChangeSummary } from '../../api/types';
  import { fileName } from '../../format';
  import { diffHover, type DiffRequest } from '../../stores/diff-popover.svelte';

  interface Props {
    files: FileChangeSummary[];
    sessionId?: string | null;
    basePath?: string | null;
    visibleCount?: number;
    /** Files the session touched in total, when `files` is only the first few. */
    totalCount?: number;
  }

  let { files, sessionId = null, basePath = null, visibleCount = 3, totalCount }: Props = $props();

  const visibleFiles = $derived(files.slice(0, visibleCount));
  const hiddenCount = $derived(Math.max(0, Math.max(totalCount ?? 0, files.length) - visibleFiles.length));

  function diffRequest(path: string): DiffRequest | null {
    return sessionId ? { kind: 'file', sessionId, filePath: path, basePath } : null;
  }
</script>

{#if files.length === 0}
  <p class="empty">Nenhum arquivo alterado</p>
{:else}
  <ul class="files">
    {#each visibleFiles as file (file.path)}
      <li>
        <button
          type="button"
          class="file"
          title={file.path}
          aria-label={`${fileName(file.path)}: +${file.added} −${file.removed}${sessionId ? '. Passe o mouse ou foque para ver o diff' : ''}`}
          use:diffHover={diffRequest(file.path)}
        >
          <span class="name mono">{fileName(file.path)}</span>
          <span class="delta mono">
            <span class="added">+{file.added}</span>
            <span class="removed">−{file.removed}</span>
          </span>
        </button>
      </li>
    {/each}
    {#if hiddenCount > 0}
      <li class="more">+{hiddenCount} {hiddenCount === 1 ? 'arquivo' : 'arquivos'}</li>
    {/if}
  </ul>
{/if}

<style>
  .files {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  .file {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 3px 6px;
    margin: 0 -6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    font-size: 12px;
    text-align: left;
    cursor: default;
  }

  .file:hover,
  .file:focus-visible {
    background: var(--surface-2);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .delta {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .added {
    color: var(--green);
  }

  .removed {
    color: var(--diff-removed);
  }

  .more,
  .empty {
    margin: 0;
    font-size: 11.5px;
    color: var(--muted);
  }
</style>
