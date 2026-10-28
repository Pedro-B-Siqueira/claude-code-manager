<script lang="ts">
  import Switch from './Switch.svelte';

  interface Props {
    autoRotate: boolean;
    onAutoRotateChange: (enabled: boolean) => void;
  }

  let { autoRotate, onAutoRotateChange }: Props = $props();
</script>

<footer class="footer">
  <p class="hint">Arraste para girar · role para aproximar · clique num balão para focar · duplo clique centraliza</p>
  <div class="spin">
    <span aria-hidden="true">Girar sozinho</span>
    <Switch checked={autoRotate} label="Girar sozinho" onChange={onAutoRotateChange} />
  </div>
</footer>

<style>
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 24px 20px;
    pointer-events: none;
  }

  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }

  .spin {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    pointer-events: auto;
  }
</style>
