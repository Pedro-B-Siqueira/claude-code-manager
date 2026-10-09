import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Attachment } from '../api/types';
import { attachmentsStore, MAX_IMAGE_BYTES, TRAY_LIMIT } from './attachments.svelte';
import { toastStore } from './toasts.svelte';

let saved = 0;
const saveAttachment = vi.fn(async (bytes: Uint8Array<ArrayBuffer>): Promise<Attachment> => {
  saved += 1;
  return { id: `id-${saved}.png`, path: `/support/attachments/id-${saved}.png`, kind: 'png', bytes: bytes.byteLength };
});
const removeAttachment = vi.fn(async (_id: string) => undefined);

vi.mock('../api/commands', () => ({
  saveAttachment: (bytes: Uint8Array<ArrayBuffer>) => saveAttachment(bytes),
  removeAttachment: (id: string) => removeAttachment(id),
  importAttachment: vi.fn(async (path: string): Promise<Attachment> => ({ id: 'dropped.png', path, kind: 'png', bytes: 4 })),
  fetchAttachmentPreview: vi.fn(async () => new ArrayBuffer(4)),
}));

const revoked: string[] = [];
URL.createObjectURL = vi.fn(() => `blob:preview-${saved}`);
URL.revokeObjectURL = vi.fn((url: string) => void revoked.push(url));

const IMAGE = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);

describe('attachmentsStore', () => {
  beforeEach(() => {
    attachmentsStore.clearAll();
    toastStore.items = [];
    saved = 0;
    revoked.length = 0;
    saveAttachment.mockClear();
    removeAttachment.mockClear();
  });

  it('keeps one tray per session', async () => {
    await attachmentsStore.addBytes('a', IMAGE, 'shot.png');
    await attachmentsStore.addPath('b', '/Users/dev/Desktop/photo.png');
    expect(attachmentsStore.tray('a').map((image) => image.name)).toEqual(['shot.png']);
    expect(attachmentsStore.tray('b').map((image) => image.name)).toEqual(['photo.png']);
  });

  it('refuses images over the limit before they reach the backend', async () => {
    await attachmentsStore.addBytes('a', new Uint8Array(MAX_IMAGE_BYTES + 1), 'huge.png');
    expect(saveAttachment).not.toHaveBeenCalled();
    expect(toastStore.items.at(-1)?.message).toBe('A imagem passa de 20 MB.');
  });

  it('holds at most ten images per tray', async () => {
    for (let index = 0; index < TRAY_LIMIT + 1; index += 1) await attachmentsStore.addBytes('a', IMAGE, `shot-${index}.png`);
    expect(attachmentsStore.tray('a')).toHaveLength(TRAY_LIMIT);
    expect(saveAttachment).toHaveBeenCalledTimes(TRAY_LIMIT);
    expect(toastStore.items.at(-1)?.message).toBe('A bandeja aceita até 10 imagens.');
  });

  it('deletes the file and frees the preview when an image is removed', async () => {
    await attachmentsStore.addBytes('a', IMAGE, 'shot.png');
    const [image] = attachmentsStore.tray('a');
    await attachmentsStore.remove('a', image?.id ?? '');
    expect(removeAttachment).toHaveBeenCalledWith('id-1.png');
    expect(attachmentsStore.tray('a')).toEqual([]);
    expect(revoked).toEqual([image?.previewUrl]);
  });

  it('takes one session’s images for sending without touching another session', async () => {
    await attachmentsStore.addBytes('a', IMAGE, 'one.png');
    await attachmentsStore.addBytes('b', IMAGE, 'two.png');
    const taken = attachmentsStore.take('a');
    expect(taken.map((image) => image.name)).toEqual(['one.png']);
    expect(attachmentsStore.tray('a')).toEqual([]);
    expect(attachmentsStore.tray('b')).toHaveLength(1);
    attachmentsStore.restore('a', taken);
    expect(attachmentsStore.tray('a')).toHaveLength(1);
    attachmentsStore.release(attachmentsStore.take('a'));
    expect(revoked).toHaveLength(1);
  });
});
