import { invoke } from '@tauri-apps/api/core';
import { writeTerminal } from '../api/commands';
import { liveSessionsStore } from '../stores/live.svelte';
import { uiStore } from '../stores/ui.svelte';

interface DevScenario {
  name: string;
  cwd: string;
}

const STEP_DELAY_MS = 1_500;

function wait(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

/** Dev builds only: opens a session in the given folder and types into it (see `commands/dev.rs`). */
export async function runDevScenario(): Promise<void> {
  const scenario = await invoke<DevScenario | null>('dev_scenario');
  if (scenario?.name !== 'pty-smoke') return;
  const session = await liveSessionsStore.open(scenario.cwd);
  if (!session) return;
  uiStore.focusSession(session.key);
  await wait(STEP_DELAY_MS);
  await writeTerminal(session.key, 'hello from the smoke test\r');
}
