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
  description?: string;
  type: ToastType;
  action?: ToastAction;
}

interface ToastStore {
  toasts: ToastItem[];
  add: (
    message: string,
    type?: ToastType,
    action?: ToastAction,
    description?: string,
  ) => void;
  remove: (id: number) => void;
  clear: () => void;
}

let nextToastId = 0;

export const useToastStore = create<ToastStore>((set, get) => ({
  toasts: [],
  add: (
    message: string,
    type: ToastType = "success",
    action?: ToastAction,
    description?: string,
  ) => {
    // Collapse an identical visible toast: repeated failures (e.g. retrying
    // the same action) must not stack N copies. The existing toast's
    // auto-dismiss timer keeps running.
    const duplicate = get().toasts.find(
      (t) => t.message === message && t.type === type,
    );
    if (duplicate) {
      // Keep existing action/description when a duplicate omits them so
      // recovery affordances and context are never cleared.
      if (action === undefined && description === undefined) return;
      set((s) => ({
        toasts: s.toasts.map((t) =>
          t.id === duplicate.id
            ? {
                ...t,
                ...(action !== undefined ? { action } : {}),
                ...(description !== undefined ? { description } : {}),
              }
            : t,
        ),
      }));
      return;
    }
    const id = ++nextToastId;
    set((s) => ({
      toasts: [
        ...s.toasts,
        {
          id,
          message,
          type,
          action,
          ...(description !== undefined ? { description } : {}),
        },
      ],
    }));
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
  description?: string,
) {
  useToastStore.getState().add(message, type, action, description);
}
