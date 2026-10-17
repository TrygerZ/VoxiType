import { create } from "zustand";

export type ToastType = "success" | "error" | "info";

/** Optional recovery affordance rendered inside the toast. */
export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface ToastItem {
  id: number;
  message: string;
  type: ToastType;
  action?: ToastAction;
}

interface ToastStore {
  toasts: ToastItem[];
  add: (message: string, type?: ToastType, action?: ToastAction) => void;
  remove: (id: number) => void;
  clear: () => void;
}

let nextToastId = 0;

export const useToastStore = create<ToastStore>((set, get) => ({
  toasts: [],
  add: (message: string, type: ToastType = "success", action?: ToastAction) => {
    // Collapse an identical visible toast: repeated failures (e.g. retrying
    // the same action) must not stack N copies. The existing toast's
    // auto-dismiss timer keeps running.
    const duplicate = get().toasts.find(
      (t) => t.message === message && t.type === type,
    );
    if (duplicate) {
      // The newer failure carries newer recovery data (e.g. the latest
      // transcript to copy). Keep the old action when the new one has none so
      // the recovery path is never removed.
      if (!action) return;
      set((s) => ({
        toasts: s.toasts.map((t) =>
          t.id === duplicate.id ? { ...t, action } : t,
        ),
      }));
      return;
    }
    const id = ++nextToastId;
    set((s) => ({ toasts: [...s.toasts, { id, message, type, action }] }));
  },
  remove: (id: number) => {
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
  },
  clear: () => set({ toasts: [] }),
}));

export function toast(
  message: string,
  type: ToastType = "success",
  action?: ToastAction,
) {
  useToastStore.getState().add(message, type, action);
}
