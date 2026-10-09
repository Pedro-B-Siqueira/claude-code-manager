import { describeFailure, type FailureCause } from '../api/logger';

export type ToastTone = 'info' | 'warning' | 'error';

export interface Toast {
  id: number;
  tone: ToastTone;
  message: string;
}

const DISMISS_AFTER_MS = 7_000;

class ToastStore {
  items = $state<Toast[]>([]);
  private nextId = 1;

  show(message: string, tone: ToastTone = 'info'): void {
    const id = this.nextId++;
    this.items = [...this.items, { id, tone, message }];
    setTimeout(() => this.dismiss(id), DISMISS_AFTER_MS);
  }

  failure(context: string, cause: FailureCause): void {
    this.show(`${context}: ${describeFailure(cause)}`, 'error');
  }

  dismiss(id: number): void {
    this.items = this.items.filter((toast) => toast.id !== id);
  }
}

export const toastStore = new ToastStore();
