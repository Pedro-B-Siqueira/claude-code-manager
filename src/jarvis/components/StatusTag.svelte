<script lang="ts">
  import type { JarvisStatus } from '../data/jarvis-session';

  interface Props {
    status: JarvisStatus;
    label: string;
  }

  let { status, label }: Props = $props();
</script>

<span class="tag" style:--tag-color="var(--jarvis-{status})">{label}</span>

<style>
  .tag {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 8px;
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--tag-color) 16%, transparent);
    color: var(--tag-color);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
</style>
