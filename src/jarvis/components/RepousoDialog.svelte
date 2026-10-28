<script lang="ts">
  import Dialog from '../../lib/components/common/Dialog.svelte';
  import Icon from '../../lib/components/common/Icon.svelte';
  import { lockDelayWarning } from '../data/lock-text';
  import type { RepousoPreparation } from '../repouso-types';
  import Switch from './Switch.svelte';

  interface Props {
    preparation: RepousoPreparation;
    onCancel: () => void;
    onActivate: (showDetails: boolean) => void;
  }

  let { preparation, onCancel, onActivate }: Props = $props();

  // The dialog mounts on each opening; the remembered choice is only the starting value.
  // svelte-ignore state_referenced_locally
  let showDetails = $state(preparation.showDetails);
  const warning = $derived(lockDelayWarning(preparation.lockDelay));
  const DETAILS_LABEL = 'Mostrar o que cada agente está fazendo na tela bloqueada';
</script>

<Dialog title="Ativar repouso?" width={600} onClose={onCancel}>
  <p class="message">Todas as telas ficam bloqueadas no Modo Jarvis e o Mac não vai dormir. As sessões continuam rodando. Para sair, use o Touch ID ou a senha do Mac.</p>
  <div class="details">
    <Switch checked={showDetails} label={DETAILS_LABEL} onChange={(checked) => (showDetails = checked)} />
    <span aria-hidden="true">{DETAILS_LABEL}</span>
  </div>
  <p class="note">Desligado, a tela mostra só o nome e o status de cada sessão. Telas extras mostram só o resumo.</p>
  {#if warning}
    <p class="warning" role="note"><Icon name="warning" size={15} />{warning}</p>
  {/if}
  {#snippet footer()}
    <button type="button" class="button-secondary" onclick={onCancel}>Cancelar</button>
    <button type="button" class="button-primary" onclick={() => onActivate(showDetails)}><Icon name="lock" size={15} />Ativar repouso</button>
  {/snippet}
</Dialog>

<style>
  .message {
    margin: 0 0 16px;
    font-size: 13.5px;
    line-height: 1.6;
  }

  .details {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13.5px;
  }

  .note {
    margin: 12px 0 0;
    color: var(--muted);
    font-size: 12px;
  }

  .warning {
    display: flex;
    gap: 8px;
    margin: 14px 0 0;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--jarvis-waiting) 40%, transparent);
    border-radius: var(--radius-button);
    background: color-mix(in srgb, var(--jarvis-waiting) 10%, transparent);
    color: var(--jarvis-waiting);
    font-size: 12.5px;
    line-height: 1.5;
  }

  .warning :global(svg) {
    flex: none;
    margin-top: 2px;
  }
</style>
