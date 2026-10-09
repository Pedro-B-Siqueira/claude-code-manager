<script lang="ts">
  import { listWorktrees, planWorktree, removeWorktree } from '../../api/commands';
  import { describeFailure, type FailureCause } from '../../api/logger';
  import type { WorktreeInfo, WorktreePlan } from '../../api/types';
  import { toastStore } from '../../stores/toasts.svelte';
  import Icon from '../common/Icon.svelte';

  interface Props {
    cwd: string;
    prefixes: string[];
    enabled: boolean;
    prefix: string;
    name: string;
    onPlan: (plan: WorktreePlan | null) => void;
    onSelectFolder: (path: string) => void;
  }

  let { cwd, prefixes, enabled = $bindable(), prefix = $bindable(), name = $bindable(), onPlan, onSelectFolder }: Props = $props();

  const PLAN_DEBOUNCE_MS = 250;
  let plan = $state<WorktreePlan | null>(null);
  let planError = $state<string | null>(null);
  let worktrees = $state<WorktreeInfo[]>([]);
  let pendingRemoval = $state<string | null>(null);

  async function loadWorktrees(folder: string): Promise<void> {
    worktrees = await listWorktrees(folder).catch(() => []);
  }

  $effect(() => {
    void loadWorktrees(cwd);
  });

  $effect(() => {
    const request = { cwd, prefix, name: name.trim(), enabled };
    plan = null;
    planError = null;
    onPlan(null);
    if (!request.enabled || request.name === '') return;
    const timer = setTimeout(() => {
      void planWorktree(request.cwd, request.prefix, request.name).then(
        (result) => {
          plan = result;
          onPlan(result);
        },
        (cause: FailureCause) => (planError = describeFailure(cause)),
      );
    }, PLAN_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  });

  async function confirmRemoval(path: string): Promise<void> {
    pendingRemoval = null;
    const removed = await removeWorktree(cwd, path).then(
      () => true,
      (cause: FailureCause) => {
        toastStore.failure('O worktree não foi removido', cause);
        return false;
      },
    );
    if (removed) toastStore.show('Worktree removido. A branch continua no repositório.');
    await loadWorktrees(cwd);
  }

  function folderName(path: string): string {
    return path.split('/').filter(Boolean).at(-1) ?? path;
  }
</script>

<section class="worktree">
  <label class="toggle">
    <input type="checkbox" bind:checked={enabled} />
    <span>Isolar em um worktree novo</span>
  </label>

  {#if enabled}
    <div class="fields">
      <select aria-label="Tipo de branch" bind:value={prefix}>
        {#each prefixes as option (option)}<option value={option}>{option}</option>{/each}
      </select>
      <input aria-label="Nome da branch" placeholder="ex.: paginacao-pedidos" bind:value={name} />
    </div>
    {#if planError}
      <p class="error">{planError}</p>
    {:else if plan}
      <dl class="plan">
        <dt>Branch</dt>
        <dd class="mono">{plan.branch}</dd>
        <dt>A partir de</dt>
        <dd class="mono">{plan.base}</dd>
        <dt>Pasta</dt>
        <dd class="mono">{plan.path}</dd>
      </dl>
      <p class="hint">
        {plan.rootInferred ? 'Pasta escolhida a partir dos worktrees que você já tem.' : 'Ajuste a pasta de worktrees nas configurações.'}
        Nada é criado antes da confirmação.
      </p>
    {/if}
  {/if}

  {#if worktrees.length > 1}
    <h4>Worktrees deste repositório</h4>
    <ul class="list">
      {#each worktrees.filter((worktree) => !worktree.isMain) as worktree (worktree.path)}
        <li>
          <button type="button" class="open" onclick={() => onSelectFolder(worktree.path)} title="Abrir sessão neste worktree">
            <Icon name="worktree" size={13} />
            <span class="name">{folderName(worktree.path)}</span>
            <span class="branch mono">{worktree.branch ?? (worktree.detached ? 'HEAD destacado' : '—')}</span>
            {#if worktree.clean === false}<span class="dirty">alterações pendentes</span>{/if}
          </button>
          {#if pendingRemoval === worktree.path}
            <span class="confirm">
              Remover?
              <button type="button" class="danger" onclick={() => void confirmRemoval(worktree.path)}>Sim</button>
              <button type="button" onclick={() => (pendingRemoval = null)}>Não</button>
            </span>
          {:else}
            <button
              type="button"
              class="remove"
              aria-label={`Remover worktree ${folderName(worktree.path)}`}
              disabled={worktree.clean !== true || worktree.inUse}
              title={worktree.inUse ? 'Há uma sessão aberta neste worktree' : worktree.clean !== true ? 'Só worktrees sem alterações pendentes podem ser removidos' : 'Remover worktree'}
              onclick={() => (pendingRemoval = worktree.path)}
            >
              <Icon name="close" size={12} />
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .worktree {
    display: grid;
    gap: 10px;
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
  }

  .fields {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 8px;
  }

  select,
  input:not([type='checkbox']) {
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-button);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    user-select: text;
  }

  .plan {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 3px 12px;
    margin: 0;
    font-size: 12px;
  }

  dt {
    color: var(--muted);
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .hint,
  .error {
    margin: 0;
    font-size: 11.5px;
    color: var(--muted);
  }

  .error {
    color: var(--diff-removed);
  }

  h4 {
    margin: 4px 0 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  .list li {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .open {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }

  .open:hover {
    background: var(--surface-2);
  }

  .name {
    font-weight: 550;
  }

  .branch {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
    color: var(--muted);
  }

  .dirty {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--amber);
  }

  .remove {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .remove:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--diff-removed);
  }

  .remove:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .confirm {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
  }

  .confirm button {
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    font-size: 12px;
    cursor: pointer;
  }

  .confirm .danger {
    border-color: transparent;
    background: var(--diff-removed);
    color: var(--bg);
  }
</style>
