<script lang="ts">
  import { onMount } from 'svelte';
  import { clearAttachments, fetchAttachmentUsage } from '../../api/commands';
  import { reportWarning, type FailureCause } from '../../api/logger';
  import type { AttachmentUsage } from '../../api/types';
  import { formatBytes } from '../../format';
  import { attachmentsStore } from '../../stores/attachments.svelte';
  import { toastStore } from '../../stores/toasts.svelte';

  let usage = $state<AttachmentUsage | null>(null);
  let confirming = $state(false);
  let clearing = $state(false);

  onMount(() => void load());

  async function load(): Promise<void> {
    usage = await fetchAttachmentUsage().catch((cause: FailureCause) => {
      reportWarning('falha ao ler o uso dos anexos', cause);
      return null;
    });
  }

  function describe(current: AttachmentUsage | null): string {
    if (!current) return '—';
    if (current.count === 0) return 'nenhuma imagem';
    return `${current.count} ${current.count === 1 ? 'imagem' : 'imagens'} · ${formatBytes(current.bytes)}`;
  }

  async function clearAll(): Promise<void> {
    confirming = false;
    clearing = true;
    const cleared = await clearAttachments().then(
      () => true,
      (cause: FailureCause) => {
        toastStore.failure('Não foi possível apagar os anexos', cause);
        return false;
      },
    );
    if (cleared) {
      attachmentsStore.clearAll();
      toastStore.show('Anexos apagados.');
    }
    clearing = false;
    await load();
  }
</script>

<fieldset>
  <legend>Armazenamento</legend>
  <div class="row">
    <span>Anexos: {describe(usage)}</span>
    {#if confirming}
      <span class="confirm">
        Apagar todos? As imagens que estão nas bandejas também saem.
        <button type="button" class="danger" onclick={() => void clearAll()}>Apagar</button>
        <button type="button" onclick={() => (confirming = false)}>Cancelar</button>
      </span>
    {:else}
      <button type="button" class="button-secondary" disabled={!usage || usage.count === 0 || clearing} onclick={() => (confirming = true)}>Apagar todos</button>
    {/if}
  </div>
  <p class="hint">Imagens coladas ou arrastadas para o terminal. São apagadas sozinhas depois de 3 dias.</p>
</fieldset>

<style>
  fieldset {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-bottom: 4px;
    padding: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 12.5px;
  }

  .confirm {
    display: inline-flex;
    flex-wrap: wrap;
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

  .hint {
    margin: 0;
    font-size: 11.5px;
    color: var(--muted);
  }
</style>
