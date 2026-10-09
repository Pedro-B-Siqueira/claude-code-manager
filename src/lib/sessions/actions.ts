import { openFinder, openPullRequest, openVsCode } from '../api/commands';
import type { FailureCause } from '../api/logger';
import { toastStore } from '../stores/toasts.svelte';

export async function openInVsCode(path: string, expectedBranch: string | null): Promise<void> {
  const outcome = await openVsCode(path, expectedBranch).catch((cause: FailureCause) => {
    toastStore.failure('Não foi possível abrir o VS Code', cause);
    return null;
  });
  if (outcome?.warning) toastStore.show(outcome.warning, 'warning');
}

export async function openInFinder(path: string): Promise<void> {
  await openFinder(path).catch((cause: FailureCause) => toastStore.failure('Não foi possível abrir no Finder', cause));
}

export async function openPr(sessionId: string): Promise<void> {
  const url = await openPullRequest(sessionId).catch((cause: FailureCause) => {
    toastStore.failure('Não foi possível abrir o PR', cause);
    return null;
  });
  if (url) toastStore.show(`Abrindo ${url}`);
}
