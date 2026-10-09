<script lang="ts">
  import type { FileChangeSummary } from '../../api/types';
  import { fileName } from '../../format';

  interface Props {
    files: FileChangeSummary[];
    visibleCount?: number;
  }

  let { files, visibleCount = 3 }: Props = $props();

  const visibleFiles = $derived(files.slice(0, visibleCount));
  const hiddenCount = $derived(Math.max(0, files.length - visibleCount));
</script>

{#if files.length === 0}
  <p class="empty">Nenhum arquivo alterado</p>
{:else}
  <ul class="files">
    {#each visibleFiles as file (file.path)}
      <li>
        <button type="button" class="file" title={file.path}>
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

  .file:hover {
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
