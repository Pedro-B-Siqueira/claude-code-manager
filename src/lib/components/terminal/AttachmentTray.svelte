<script lang="ts">
  import { formatBytes } from '../../format';
  import type { TrayImage } from '../../stores/attachments.svelte';
  import Icon from '../common/Icon.svelte';

  interface Props {
    images: TrayImage[];
    onRemove: (id: string) => void;
    onOpen: (id: string) => void;
  }

  let { images, onRemove, onOpen }: Props = $props();
</script>

{#if images.length > 0}
  <div class="tray">
    <ul aria-label="Imagens do próximo prompt">
      {#each images as image (image.id)}
        <li>
          <button type="button" class="preview" aria-label={`Abrir ${image.name}`} title={`${image.name} · ${formatBytes(image.bytes)}`} onclick={() => onOpen(image.id)}>
            <img src={image.previewUrl} alt="" />
          </button>
          <button type="button" class="remove" aria-label={`Remover ${image.name}`} onclick={() => onRemove(image.id)}>
            <Icon name="close" size={10} />
          </button>
        </li>
      {/each}
    </ul>
    <p>{images.length === 1 ? '1 imagem vai junto no próximo Enter' : `${images.length} imagens vão junto no próximo Enter`}</p>
  </div>
{/if}

<style>
  .tray {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 10px 8px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  /* Room for the remove buttons, which sit outside each thumbnail but inside the scroll box. */
  ul {
    display: flex;
    gap: 10px;
    margin: 0;
    padding: 6px 6px 0 0;
    list-style: none;
    overflow-x: auto;
  }

  li {
    position: relative;
    flex: none;
  }

  .preview {
    display: block;
    width: 56px;
    height: 56px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    overflow: hidden;
    cursor: zoom-in;
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .remove {
    position: absolute;
    top: -6px;
    right: -6px;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
  }

  .remove:hover,
  .remove:focus-visible {
    border-color: transparent;
    background: var(--diff-removed);
    color: var(--bg);
  }

  p {
    margin: 0;
    font-size: 11.5px;
    color: var(--muted);
    white-space: nowrap;
  }
</style>
