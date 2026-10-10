import { create } from "zustand";

export type ToastType = "success" | "error" | "info";

export interface ToastItem {
  id: number;
  message: string;
  type: ToastType;
}

interface ToastStore {
  toasts: ToastItem[];
  add: (message: string, type?: ToastType) => void;
  remove: (id: number) => void;
  clear: () => void;
}

let nextToastId = 0;

export const useToastStore = create<ToastStore>((set, get) => ({
  toasts: [],
  add: (message: string, type: ToastType = "success") => {
    // Collapse an identical visible toast: repeated failures (e.g. retrying
    // the same action) must not stack N copies. The existing toast's
    // auto-dismiss timer keeps running.
    const duplicate = get().toasts.some(
      (t) => t.message === message && t.type === type,
    );
    if (duplicate) return;
    const id = ++nextToastId;
    set((s) => ({ toasts: [...s.toasts, { id, message, type }] }));
  },
  remove: (id: number) => {
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
  },
  clear: () => set({ toasts: [] }),
}));

export function toast(message: string, type: ToastType = "success") {
  useToastStore.getState().add(message, type);
}
