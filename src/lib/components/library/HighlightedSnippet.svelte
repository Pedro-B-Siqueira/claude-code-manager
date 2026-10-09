<script lang="ts">
  interface Props {
    text: string;
  }

  let { text }: Props = $props();

  interface SnippetPart {
    value: string;
    highlighted: boolean;
  }

  /** The search index marks matches with « and »; split on them instead of rendering HTML. */
  function splitMarks(snippet: string): SnippetPart[] {
    return snippet.split(/(«[^»]*»)/).filter(Boolean).map((part) => {
      const highlighted = part.startsWith('«') && part.endsWith('»');
      return { value: highlighted ? part.slice(1, -1) : part, highlighted };
    });
  }

  const parts = $derived(splitMarks(text));
</script>

<span class="snippet">
  {#each parts as part, index (index)}
    {#if part.highlighted}<mark>{part.value}</mark>{:else}{part.value}{/if}
  {/each}
</span>

<style>
  .snippet {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  mark {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--text);
    border-radius: 3px;
    padding: 0 1px;
  }
</style>
