<script lang="ts" module>
  export type IconName =
    | 'plus'
    | 'history'
    | 'search'
    | 'sun'
    | 'moon'
    | 'code'
    | 'folder'
    | 'focus'
    | 'more'
    | 'pin'
    | 'branch'
    | 'worktree'
    | 'grid'
    | 'columns'
    | 'bell'
    | 'gauge'
    | 'shield'
    | 'memory'
    | 'sleep'
    | 'pencil'
    | 'close'
    | 'warning'
    | 'terminal'
    | 'pullRequest';

  const ICON_PATHS: Record<IconName, string> = {
    plus: 'M12 5v14M5 12h14',
    history: 'M3 12a9 9 0 1 0 3-6.7M3 4v4h4M12 7v5l3 2',
    search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14ZM20 20l-4-4',
    sun: 'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8ZM12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4',
    moon: 'M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5Z',
    code: 'M8 7l-5 5 5 5M16 7l5 5-5 5',
    folder: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z',
    focus: 'M4 9V5a1 1 0 0 1 1-1h4M15 4h4a1 1 0 0 1 1 1v4M20 15v4a1 1 0 0 1-1 1h-4M9 20H5a1 1 0 0 1-1-1v-4',
    more: 'M5 12h.01M12 12h.01M19 12h.01',
    pin: 'M9 4h6l-1 6 3 3H7l3-3ZM12 13v7',
    branch: 'M6 4v10M6 14a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5ZM18 5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5ZM18 10c0 4-6 3-12 4',
    worktree: 'M4 5h7v6H4ZM13 13h7v6h-7ZM7.5 11v4.5H13',
    grid: 'M4 4h7v7H4ZM13 4h7v7h-7ZM4 13h7v7H4ZM13 13h7v7h-7Z',
    columns: 'M4 4h16v16H4ZM15 4v16',
    bell: 'M6 16V11a6 6 0 1 1 12 0v5l2 2H4ZM10 21h4',
    gauge: 'M4 16a8 8 0 1 1 16 0M12 16l4-5',
    shield: 'M12 3l7 3v6c0 4-3 7.5-7 9-4-1.5-7-5-7-9V6Z',
    memory: 'M5 7h14v10H5ZM8 7V4M12 7V4M16 7V4M8 20v-3M12 20v-3M16 20v-3',
    sleep: 'M4 6h5l-5 6h5M13 12h4l-4 5h4',
    pencil: 'M4 20h4L19 9l-4-4L4 16ZM13.5 6.5l4 4',
    close: 'M6 6l12 12M18 6L6 18',
    warning: 'M12 4l9 16H3ZM12 10v4M12 17h.01',
    terminal: 'M4 5h16v14H4ZM8 10l3 2-3 2M13 15h3',
    pullRequest: 'M6 3a2 2 0 1 0 0 4a2 2 0 1 0 0-4ZM6 7v14M18 17a2 2 0 1 0 0 4a2 2 0 1 0 0-4ZM18 17V9a3 3 0 0 0-3-3h-4M13 4l-2 2 2 2',
  };
</script>

<script lang="ts">
  interface Props {
    name: IconName;
    size?: number;
  }

  let { name, size = 16 }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.8"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={ICON_PATHS[name]} />
</svg>
