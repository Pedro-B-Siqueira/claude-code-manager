<script lang="ts">
  import { STATUS_LABELS, type JarvisSession } from '../data/jarvis-session';

  interface Props {
    sessions: readonly JarvisSession[];
    onAdd: () => void;
    onRemove: () => void;
    onCycle: (id: string) => void;
  }

  let { sessions, onAdd, onRemove, onCycle }: Props = $props();
</script>

<aside class="dev-panel" aria-label="Sessões de teste">
  <strong>Mock (dev)</strong>
  <div class="row">
    <button type="button" onclick={onAdd}>+ sessão</button>
    <button type="button" onclick={onRemove} disabled={sessions.length === 0}>− sessão</button>
  </div>
  <ul>
    {#each sessions as session (session.id)}
      <li>
        <button type="button" onclick={() => onCycle(session.id)} title="Trocar status">{session.repo} · {session.name} · {STATUS_LABELS[session.status]}</button>
      </li>
    {/each}
  </ul>
</aside>

<style>
  .dev-panel {
    position: absolute;
    bottom: 64px;
    left: 24px;
    z-index: 6;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 320px;
    padding: 10px;
    border: 1px dashed var(--jarvis-balloon-border);
    border-radius: 10px;
    background: rgb(10 12 16 / 0.86);
    font-size: 11.5px;
  }

  .row {
    display: flex;
    gap: 6px;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  button {
    padding: 3px 8px;
    border: 1px solid var(--jarvis-balloon-border);
    border-radius: 6px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
</style>
