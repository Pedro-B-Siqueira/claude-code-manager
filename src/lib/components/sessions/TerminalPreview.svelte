<script lang="ts">
  interface Props {
    lines: string[];
  }

  let { lines }: Props = $props();
</script>

<div class="preview mono" role="log" aria-label="Últimas linhas do terminal">
  {#each lines as line, index (index)}
    <div class="line">{line || ' '}</div>
  {/each}
</div>

<style>
  .preview {
    height: 74px;
    padding: 9px 11px;
    border-radius: 9px;
    background: var(--terminal-bg);
    color: var(--terminal-text);
    font-size: 11.5px;
    line-height: 1.5;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }

  .line {
    flex-shrink: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: pre;
  }
</style>
