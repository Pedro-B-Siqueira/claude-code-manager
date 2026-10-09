<script lang="ts">
  import type { SessionStatus } from '../../api/types';
  import { STATUS_LABELS } from '../../sessions/status';

  interface Props {
    status: SessionStatus;
    hibernated?: boolean;
  }

  let { status, hibernated = false }: Props = $props();

  const label = $derived(hibernated ? `${STATUS_LABELS[status]} · hibernada` : STATUS_LABELS[status]);
</script>

<span class="badge" data-status={status} title={label}>
  <span class="dot" aria-hidden="true"></span>
  <span class="label">{label}</span>
</span>

<style>
  .badge {
    --status-color: var(--muted);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 9px 2px 8px;
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--status-color) var(--status-badge-alpha), transparent);
    color: var(--status-color);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }

  .badge[data-status='working'] {
    --status-color: var(--accent);
  }
  .badge[data-status='permission'] {
    --status-color: var(--amber);
  }
  .badge[data-status='waiting'] {
    --status-color: var(--blue);
  }
  .badge[data-status='done'] {
    --status-color: var(--green);
  }

  .dot {
    position: relative;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--status-color);
  }

  .badge[data-status='working'] .dot::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 1.5px solid var(--status-color);
    animation: pulse-ring 1.6s var(--ease) infinite;
  }
</style>
