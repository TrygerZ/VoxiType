import { create } from "zustand";
import type { DictionaryEntry } from "../types/app";
import {
  addDictionaryWord,
  deleteDictionaryWord,
  formatTauriError,
  getDictionary,
} from "../lib/tauri";

interface DictionaryStore {
  entries: DictionaryEntry[];
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
  add: (entry: DictionaryEntry) => Promise<void>;
  remove: (id: string) => Promise<void>;
}

export const useDictionaryStore = create<DictionaryStore>((set) => {
  // Monotonic token: every fetch captures the current value and only commits
  // its result if it is still the latest, so a slow earlier load cannot
  // overwrite fresher entries out of order.
  let requestSeq = 0;

  return {
    entries: [],
    loading: false,
    error: null,

    load: async () => {
      const seq = ++requestSeq;
      set({ loading: true, error: null });
      try {
        const entries = await getDictionary();
        if (seq === requestSeq) {
          set({ entries, loading: false, error: null });
        }
      } catch (err: unknown) {
        if (seq === requestSeq) {
          set({ loading: false, error: formatTauriError(err) });
        }
      }
    },

    add: async (entry) => {
      // Bump so a stale in-flight load cannot overwrite the post-add list.
      // Clearing `loading` releases the flag the discarded load would leave set.
      const seq = ++requestSeq;
      await addDictionaryWord(entry);
      const entries = await getDictionary();
      if (seq === requestSeq) {
        set({ entries, loading: false });
      }
    },

    remove: async (id) => {
      await deleteDictionaryWord(id);
      set((s) => ({ entries: s.entries.filter((e) => e.id !== id) }));
    },
  };
});
