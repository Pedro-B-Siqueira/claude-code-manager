<script lang="ts">
  import Icon from '../common/Icon.svelte';

  interface Props {
    tags: string[];
    suggestions: string[];
    onChange: (tags: string[]) => void;
  }

  let { tags, suggestions, onChange }: Props = $props();

  let draft = $state('');
  const listId = `tag-suggestions-${Math.random().toString(36).slice(2, 8)}`;

  function addDraft(): void {
    const tag = draft.trim().replace(/,$/, '');
    draft = '';
    if (!tag || tags.some((existing) => existing.toLowerCase() === tag.toLowerCase())) return;
    onChange([...tags, tag]);
  }

  function remove(tag: string): void {
    onChange(tags.filter((existing) => existing !== tag));
  }

  function handleKey(event: KeyboardEvent): void {
    if (event.key === 'Enter' || event.key === ',') {
      event.preventDefault();
      addDraft();
    }
    if (event.key === 'Backspace' && draft === '' && tags.length > 0) {
      remove(tags[tags.length - 1] ?? '');
    }
  }
</script>

<div class="tags">
  {#each tags as tag (tag)}
    <span class="tag">
      {tag}
      <button type="button" aria-label={`Remover tag ${tag}`} onclick={() => remove(tag)}>
        <Icon name="close" size={11} />
      </button>
    </span>
  {/each}
  <input
    aria-label="Adicionar tag"
    placeholder={tags.length === 0 ? 'Adicionar tag…' : ''}
    list={listId}
    bind:value={draft}
    onkeydown={handleKey}
    onblur={addDraft}
  />
  <datalist id={listId}>
    {#each suggestions.filter((suggestion) => !tags.includes(suggestion)) as suggestion (suggestion)}
      <option value={suggestion}></option>
    {/each}
  </datalist>
</div>

<style>
  .tags {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 5px 6px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface);
  }

  .tags:focus-within {
    border-color: var(--accent);
  }

  .tag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 4px 1px 9px;
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    font-size: 12px;
  }

  .tag button {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .tag button:hover {
    color: var(--text);
    background: var(--border);
  }

  input {
    flex: 1;
    min-width: 90px;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    user-select: text;
  }
</style>
