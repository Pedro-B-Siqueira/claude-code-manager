import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Attachment, SubmitOutcome } from '../api/types';
import { attachmentsStore } from '../stores/attachments.svelte';
import { toastStore } from '../stores/toasts.svelte';
import { enterGateFor, type KeyLike } from './attachments';
import { submitTray } from './submit';

let pendingSubmit: { resolve: (outcome: SubmitOutcome) => void; reject: (cause: Error) => void } | null = null;
const submitWithImages = vi.fn(
  (_key: string, _ids: string[]) =>
    new Promise<SubmitOutcome>((resolve, reject) => {
      pendingSubmit = { resolve, reject };
    }),
);
let saved = 0;

vi.mock('../api/commands', () => ({
  submitWithImages: (key: string, ids: string[]) => submitWithImages(key, ids),
  saveAttachment: vi.fn(async (bytes: Uint8Array<ArrayBuffer>): Promise<Attachment> => {
    saved += 1;
    return { id: `img-${saved}.png`, path: `/support/attachments/img-${saved}.png`, kind: 'png', bytes: bytes.byteLength };
  }),
  removeAttachment: vi.fn(async () => undefined),
  importAttachment: vi.fn(),
  fetchAttachmentPreview: vi.fn(),
}));

URL.createObjectURL = vi.fn(() => 'blob:preview');
URL.revokeObjectURL = vi.fn();

const IMAGE = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
const ENTER: KeyLike = { type: 'keydown', key: 'Enter', shiftKey: false, altKey: false, metaKey: false, ctrlKey: false };

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

describe('submitTray', () => {
  beforeEach(async () => {
    attachmentsStore.clearAll();
    toastStore.items = [];
    submitWithImages.mockClear();
    pendingSubmit = null;
    await attachmentsStore.addBytes('a', IMAGE, 'one.png');
    await attachmentsStore.addBytes('b', IMAGE, 'two.png');
  });

  it('sends only the session captured at Enter and empties its tray', async () => {
    const [image] = attachmentsStore.tray('a');
    const sending = submitTray('a');
    expect(submitWithImages).toHaveBeenCalledWith('a', [image?.id]);
    expect(attachmentsStore.tray('a')).toEqual([]);
    expect(attachmentsStore.tray('b')).toHaveLength(1);
    pendingSubmit?.resolve({ kind: 'submitted' });
    await sending;
    expect(attachmentsStore.tray('a')).toEqual([]);
    expect(toastStore.items).toEqual([]);
  });

  it('swallows any Enter for that session while the images are being sent', async () => {
    const sending = submitTray('a');
    expect(enterGateFor('a').decide(ENTER, true)).toBe('swallow');
    expect(enterGateFor('b').decide(ENTER, false)).toBe('pass');
    pendingSubmit?.resolve({ kind: 'submitted' });
    await sending;
    expect(enterGateFor('a').decide(ENTER, false)).toBe('pass');
  });

  it('warns when Claude Code did not recognize the images', async () => {
    const sending = submitTray('a');
    pendingSubmit?.resolve({ kind: 'notRecognized', recognized: 0 });
    await sending;
    expect(toastStore.items.at(-1)).toMatchObject({ tone: 'warning', message: 'O Claude Code não reconheceu as imagens. Confira o prompt e aperte Enter.' });
  });

  it('puts the images back when nothing could be pasted', async () => {
    const sending = submitTray('a');
    pendingSubmit?.reject(new Error('o Claude Code não está aceitando colagem agora'));
    await sending;
    await settle();
    expect(attachmentsStore.tray('a').map((image) => image.name)).toEqual(['one.png']);
    expect(toastStore.items.at(-1)).toMatchObject({ tone: 'error' });
    expect(enterGateFor('a').decide(ENTER, true)).toBe('submit');
  });
});
