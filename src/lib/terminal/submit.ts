import { submitWithImages } from '../api/commands';
import type { FailureCause } from '../api/logger';
import { attachmentsStore } from '../stores/attachments.svelte';
import { toastStore } from '../stores/toasts.svelte';
import { describeUnrecognized, enterGateFor } from './attachments';

/**
 * Sends one session's tray with its Enter. The session's gate swallows further Enters meanwhile;
 * when nothing could be pasted, the images go back to the tray.
 */
export async function submitTray(key: string): Promise<void> {
  const gate = enterGateFor(key);
  gate.begin();
  const taken = attachmentsStore.take(key);
  const outcome = await submitWithImages(
    key,
    taken.map((image) => image.id),
  ).catch((cause: FailureCause) => {
    attachmentsStore.restore(key, taken);
    toastStore.failure('As imagens não foram enviadas', cause);
    return null;
  });
  gate.finish();
  if (!outcome) return;
  attachmentsStore.release(taken);
  if (outcome.kind === 'notRecognized') toastStore.show(describeUnrecognized(outcome.recognized, taken.length), 'warning');
}
