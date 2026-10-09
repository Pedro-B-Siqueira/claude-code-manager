import { fetchAttachmentPreview, importAttachment, removeAttachment, saveAttachment } from '../api/commands';
import { reportWarning, type FailureCause } from '../api/logger';
import type { Attachment, AttachmentKind } from '../api/types';
import { fileName } from '../format';
import { toastStore } from './toasts.svelte';

export const TRAY_LIMIT = 10;
export const MAX_IMAGE_BYTES = 20 * 1024 * 1024;

export interface TrayImage {
  id: string;
  name: string;
  bytes: number;
  previewUrl: string;
}

const MIME_BY_KIND: Record<AttachmentKind, string> = { png: 'image/png', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp' };

/** Images waiting for the next Enter, one tray per terminal session; only lives while the app runs. */
class AttachmentsStore {
  trays = $state<Record<string, TrayImage[]>>({});

  tray(key: string): TrayImage[] {
    return this.trays[key] ?? [];
  }

  async addBytes(key: string, bytes: Uint8Array<ArrayBuffer>, name: string): Promise<void> {
    if (bytes.byteLength > MAX_IMAGE_BYTES) return toastStore.show('A imagem passa de 20 MB.', 'warning');
    if (!this.hasRoom(key)) return;
    const attachment = await saveAttachment(bytes).catch((cause: FailureCause) => {
      toastStore.failure('A imagem não entrou na bandeja', cause);
      return null;
    });
    if (attachment) this.push(key, attachment, name, bytes);
  }

  async addPath(key: string, path: string): Promise<void> {
    if (!this.hasRoom(key)) return;
    const attachment = await importAttachment(path).catch((cause: FailureCause) => {
      toastStore.failure(`${fileName(path)} não entrou na bandeja`, cause);
      return null;
    });
    if (!attachment) return;
    const preview = await fetchAttachmentPreview(attachment.id).catch(() => new ArrayBuffer(0));
    this.push(key, attachment, fileName(path), new Uint8Array(preview));
  }

  async remove(key: string, id: string): Promise<void> {
    const image = this.tray(key).find((candidate) => candidate.id === id);
    this.trays = { ...this.trays, [key]: this.tray(key).filter((candidate) => candidate.id !== id) };
    if (image) URL.revokeObjectURL(image.previewUrl);
    await removeAttachment(id).catch((cause: FailureCause) => reportWarning('falha ao apagar o anexo', cause));
  }

  /** Empties the tray for sending; the previews stay valid until `release` or `restore`. */
  take(key: string): TrayImage[] {
    const images = this.tray(key);
    this.trays = { ...this.trays, [key]: [] };
    return images;
  }

  restore(key: string, images: TrayImage[]): void {
    this.trays = { ...this.trays, [key]: [...images, ...this.tray(key)] };
  }

  release(images: TrayImage[]): void {
    for (const image of images) URL.revokeObjectURL(image.previewUrl);
  }

  clearAll(): void {
    for (const images of Object.values(this.trays)) this.release(images);
    this.trays = {};
  }

  private hasRoom(key: string): boolean {
    if (this.tray(key).length < TRAY_LIMIT) return true;
    toastStore.show(`A bandeja aceita até ${TRAY_LIMIT} imagens.`, 'warning');
    return false;
  }

  private push(key: string, attachment: Attachment, name: string, bytes: Uint8Array<ArrayBuffer>): void {
    if (!this.hasRoom(key)) {
      void removeAttachment(attachment.id).catch((cause: FailureCause) => reportWarning('falha ao apagar o anexo', cause));
      return;
    }
    const previewUrl = URL.createObjectURL(new Blob([bytes], { type: MIME_BY_KIND[attachment.kind] }));
    this.trays = { ...this.trays, [key]: [...this.tray(key), { id: attachment.id, name, bytes: attachment.bytes, previewUrl }] };
  }
}

export const attachmentsStore = new AttachmentsStore();
