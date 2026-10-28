<script lang="ts">
  import Icon from '../../lib/components/common/Icon.svelte';

  interface Props {
    onNewSession: () => void;
  }

  let { onNewSession }: Props = $props();
</script>

<section class="empty" aria-labelledby="jarvis-empty-title">
  <h2 id="jarvis-empty-title">Nenhuma sessão ativa</h2>
  <p>Quando você abrir uma sessão do Claude Code, ela aparece aqui e o holograma vai se formando em volta do projeto.</p>
  <button type="button" class="button-primary" onclick={onNewSession}><Icon name="plus" size={15} />Nova sessão</button>
</section>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    max-width: 460px;
    text-align: center;
  }

  h2 {
    margin: 0;
    font-size: 19px;
    font-weight: 650;
  }

  p {
    margin: 0 0 6px;
    color: var(--muted);
    font-size: 13.5px;
    line-height: 1.6;
  }
</style>
