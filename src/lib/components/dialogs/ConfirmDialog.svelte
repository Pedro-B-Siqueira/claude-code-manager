<script lang="ts">
  import Dialog from '../common/Dialog.svelte';

  interface Props {
    title: string;
    message: string;
    confirmLabel: string;
    danger?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { title, message, confirmLabel, danger = false, onConfirm, onCancel }: Props = $props();
</script>

<Dialog {title} width={440} onClose={onCancel}>
  <p class="message">{message}</p>
  {#snippet footer()}
    <button type="button" class="button-secondary" onclick={onCancel}>Cancelar</button>
    <button type="button" class={danger ? 'button-danger' : 'button-primary'} onclick={onConfirm}>{confirmLabel}</button>
  {/snippet}
</Dialog>

<style>
  .message {
    margin: 0;
    font-size: 13px;
    line-height: 1.55;
  }
</style>
