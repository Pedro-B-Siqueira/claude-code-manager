import type { ActivityItem } from '../api/types';

const TOOL_VERBS: Record<string, string> = {
  Bash: 'Rodou',
  Read: 'Leu',
  Grep: 'Buscou',
  Glob: 'Buscou',
  WebFetch: 'Pesquisou',
  WebSearch: 'Pesquisou',
  Task: 'Delegou',
  Agent: 'Delegou',
  Skill: 'Skill',
  TodoWrite: 'Planejou',
  NotebookEdit: 'Editou',
};

export function activityLabel(item: ActivityItem): string {
  if (item.kind === 'prompt') return 'Prompt';
  if (item.kind === 'edit') return item.tool === 'Write' ? 'Escreveu' : 'Editou';
  return TOOL_VERBS[item.tool ?? ''] ?? item.tool ?? 'Ação';
}

const timeFormat = new Intl.DateTimeFormat('pt-BR', { hour: '2-digit', minute: '2-digit' });

export function activityTime(timestamp: number | null): string {
  return timestamp === null ? '--:--' : timeFormat.format(timestamp);
}
