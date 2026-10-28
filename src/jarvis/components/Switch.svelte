<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    onChange: (checked: boolean) => void;
  }

  let { checked, label, onChange }: Props = $props();
</script>

<button type="button" class="switch" role="switch" aria-checked={checked} aria-label={label} onclick={() => onChange(!checked)}>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative;
    flex: none;
    width: 36px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--border);
    cursor: pointer;
    transition: background-color 150ms var(--ease);
  }

  .switch[aria-checked='true'] {
    background: var(--jarvis-selected);
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: white;
    transition: transform 150ms var(--ease);
  }

  .switch[aria-checked='true'] .knob {
    transform: translateX(16px);
  }
</style>
