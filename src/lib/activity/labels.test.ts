import { describe, expect, it } from 'vitest';
import type { ActivityItem } from '../api/types';
import { activityLabel } from './labels';

function item(kind: string, tool: string | null): ActivityItem {
  return { id: 1, timestamp: null, kind, tool, target: null, editId: null, fromSubagent: false };
}

describe('activity labels', () => {
  it('describes prompts, edits and tools in Portuguese', () => {
    expect(activityLabel(item('prompt', null))).toBe('Prompt');
    expect(activityLabel(item('edit', 'Edit'))).toBe('Editou');
    expect(activityLabel(item('edit', 'Write'))).toBe('Escreveu');
    expect(activityLabel(item('tool', 'Bash'))).toBe('Rodou');
    expect(activityLabel(item('tool', 'Grep'))).toBe('Buscou');
    expect(activityLabel(item('tool', 'mcp__custom'))).toBe('mcp__custom');
  });
});
