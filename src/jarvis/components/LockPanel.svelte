<script lang="ts">
  import Icon from '../../lib/components/common/Icon.svelte';
  import { formatClock } from '../data/lock-text';
  import type { UnlockState } from '../repouso-types';

  interface Props {
    now: number;
    unlock: UnlockState;
    onUnlock: () => void;
  }

  let { now, unlock, onUnlock }: Props = $props();
</script>

<section class="panel" aria-label="Tela bloqueada">
  <p class="clock">{formatClock(new Date(now))}</p>
  <p class="status">Bloqueado. As sessões continuam rodando e o Mac não vai dormir.</p>
  {#if unlock === 'failed'}
    <p class="failure" role="alert"><Icon name="warning" size={16} />Não foi possível confirmar sua identidade. Tente novamente.</p>
  {/if}
  {#if unlock === 'asking'}
    <button type="button" class="button-primary" disabled><Icon name="lock" size={15} />Aguardando confirmação…</button>
    <p class="caption">Confirme na janela do sistema.</p>
  {:else}
    <button type="button" class="button-primary" onclick={onUnlock}><Icon name="lock" size={15} />{unlock === 'failed' ? 'Tentar novamente' : 'Desbloquear'}</button>
    {#if unlock === 'idle'}<p class="caption">Touch ID ou senha do Mac</p>{/if}
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    width: min(500px, calc(100vw - 32px));
    padding: 24px 24px 22px;
    border: 1px solid var(--jarvis-balloon-border);
    border-radius: 14px;
    background: rgb(13 16 21 / 0.94);
    box-shadow: 0 24px 64px rgb(0 0 0 / 0.45);
    text-align: center;
  }

  .clock {
    margin: 0;
    font-size: 48px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.01em;
  }

  .status {
    margin: 0 0 4px;
    color: var(--jarvis-text);
    font-size: 14px;
  }

  .failure {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    margin: 0;
    padding: 12px;
    border-radius: var(--radius-button);
    background: color-mix(in srgb, var(--diff-removed) 14%, transparent);
    color: var(--diff-removed);
    font-size: 13px;
  }

  .caption {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
</style>
