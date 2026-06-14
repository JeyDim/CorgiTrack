import { defineStore } from "pinia";

export type ToastKind = "success" | "error" | "info";

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

let counter = 0;

export const useToastStore = defineStore("toast", {
  state: () => ({ toasts: [] as Toast[] }),
  actions: {
    push(message: string, kind: ToastKind = "info", timeout = 3500) {
      const id = ++counter;
      this.toasts.push({ id, kind, message });
      window.setTimeout(() => this.dismiss(id), timeout);
    },
    success(message: string) {
      this.push(message, "success");
    },
    error(message: string) {
      this.push(message, "error", 5500);
    },
    info(message: string) {
      this.push(message, "info");
    },
    dismiss(id: number) {
      this.toasts = this.toasts.filter((t) => t.id !== id);
    },
  },
});
