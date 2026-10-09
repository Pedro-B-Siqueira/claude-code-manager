<script lang="ts">
  import { untrack } from 'svelte';
  import type { AppSettings, NotificationPreference } from '../../api/types';
  import Dialog from '../common/Dialog.svelte';
  import AttachmentStorage from './AttachmentStorage.svelte';

  interface Props {
    settings: AppSettings;
    claudeGaugeHook: boolean;
    onSave: (settings: AppSettings) => void;
    onClose: () => void;
  }

  let { settings, claudeGaugeHook, onSave, onClose }: Props = $props();

  const NOTIFICATION_OPTIONS: Array<{ value: NotificationPreference; label: string }> = [
    { value: 'auto', label: 'Automático' },
    { value: 'enabled', label: 'Sempre ligadas' },
    { value: 'disabled', label: 'Desligadas' },
  ];

  // The dialog edits a copy taken when it opens; changes apply only on "Salvar".
  const initial = untrack(() => $state.snapshot(settings));
  let draft = $state<AppSettings>(structuredClone(initial));
  let prefixesText = $state(initial.branchPrefixes.join(', '));

  function save(): void {
    const branchPrefixes = prefixesText.split(',').map((prefix) => prefix.trim()).filter(Boolean);
    const worktreeRoot = draft.worktreeRoot?.trim() ? draft.worktreeRoot.trim() : null;
    const claudeBinary = draft.claudeBinary?.trim() ? draft.claudeBinary.trim() : null;
    onSave({ ...draft, branchPrefixes, worktreeRoot, claudeBinary });
  }
</script>

<Dialog title="Configurações" width={560} {onClose}>
  <form class="form" onsubmit={(event) => { event.preventDefault(); save(); }}>
    <fieldset>
      <legend>Aparência</legend>
      <div class="segmented" role="radiogroup" aria-label="Tema">
        <button type="button" role="radio" aria-checked={draft.theme === 'dark'} class:active={draft.theme === 'dark'} onclick={() => (draft.theme = 'dark')}>Escuro</button>
        <button type="button" role="radio" aria-checked={draft.theme === 'light'} class:active={draft.theme === 'light'} onclick={() => (draft.theme = 'light')}>Claro</button>
      </div>
    </fieldset>

    <fieldset>
      <legend>Worktrees</legend>
      <label>
        <span>Pasta dos worktrees</span>
        <input placeholder="Automática (segue os worktrees que você já tem)" bind:value={draft.worktreeRoot} />
      </label>
      <label>
        <span>Prefixos de branch</span>
        <input placeholder="feat-, fix-" bind:value={prefixesText} />
      </label>
    </fieldset>

    <fieldset>
      <legend>Terminal</legend>
      <label>
        <span>Scrollback (linhas)</span>
        <input type="number" min="500" max="100000" step="500" bind:value={draft.scrollbackLines} />
      </label>
      <label>
        <span>Caminho do <code class="mono">claude</code></span>
        <input placeholder="Automático (PATH do shell de login)" bind:value={draft.claudeBinary} />
      </label>
    </fieldset>

    <fieldset>
      <legend>Notificações</legend>
      <select aria-label="Notificações" bind:value={draft.notifications}>
        {#each NOTIFICATION_OPTIONS as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
      </select>
      <p class="hint">
        {claudeGaugeHook
          ? 'O ClaudeGauge já avisa por hook; no automático, o app não repete as notificações.'
          : 'No automático, o app avisa quando uma sessão pede permissão, espera você ou termina.'}
      </p>
    </fieldset>

    <fieldset>
      <legend>Economia de memória</legend>
      <label class="inline">
        <input type="checkbox" bind:checked={draft.hibernation.enabled} />
        <span>Hibernar sessões ociosas</span>
      </label>
      <label>
        <span>Depois de (minutos)</span>
        <input type="number" min="5" max="1440" step="5" disabled={!draft.hibernation.enabled} bind:value={draft.hibernation.idleMinutes} />
      </label>
      <p class="hint">A sessão hibernada encerra o processo e volta com <code class="mono">--resume</code> quando você clica nela.</p>
    </fieldset>

    <AttachmentStorage />
  </form>

  {#snippet footer()}
    <button type="button" class="button-secondary" onclick={onClose}>Cancelar</button>
    <button type="button" class="button-primary" onclick={save}>Salvar</button>
  {/snippet}
</Dialog>

<style>
  .form {
    display: grid;
    gap: 16px;
  }

  fieldset {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-bottom: 4px;
    padding: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  label {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
  }

  label.inline {
    display: flex;
  }

  input:not([type='checkbox']),
  select {
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-button);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    user-select: text;
  }

  input:disabled {
    opacity: 0.5;
  }

  .segmented {
    display: inline-flex;
    width: max-content;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-button);
    background: var(--surface);
  }

  .segmented button {
    height: 28px;
    padding: 0 14px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .segmented button.active {
    background: var(--surface-2);
    color: var(--text);
    font-weight: 550;
  }

  .hint {
    margin: 0;
    font-size: 11.5px;
    color: var(--muted);
  }

  @media (max-width: 560px) {
    label {
      grid-template-columns: 1fr;
    }
  }
</style>
