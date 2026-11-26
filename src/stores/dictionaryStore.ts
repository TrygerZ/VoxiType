import { create } from "zustand";
import type { DictionaryEntry } from "../types/app";
import {
  addDictionaryWord,
  deleteDictionaryWord,
  formatTauriError,
  getDictionary,
  updateDictionaryWord,
} from "../lib/tauri";

interface DictionaryStore {
  entries: DictionaryEntry[];
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
  add: (entry: DictionaryEntry) => Promise<void>;
  updateWord: (entry: DictionaryEntry) => Promise<void>;
  remove: (id: string) => Promise<void>;
}

export const useDictionaryStore = create<DictionaryStore>((set) => {
  // Monotonic token: every fetch captures the current value and only commits
  // its result if it is still the latest, so a slow earlier load cannot
  // overwrite fresher entries out of order.
  let requestSeq = 0;

  // Bump so a stale in-flight load cannot overwrite the post-mutation list.
  // Clearing `loading` releases the flag the discarded load would leave set;
  // the catch clears it too before rethrowing so `loading` never stays stuck.
  const commitAndReload = async (mutate: () => Promise<void>) => {
    const seq = ++requestSeq;
    try {
      await mutate();
      const entries = await getDictionary();
      if (seq === requestSeq) {
        set({ entries, loading: false });
      }
    } catch (err) {
      set({ loading: false });
      throw err;
    }
  };

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

    add: (entry) => commitAndReload(() => addDictionaryWord(entry)),

    updateWord: (entry) => commitAndReload(() => updateDictionaryWord(entry)),

    // Bump so a stale in-flight load cannot commit afterwards and resurrect
    // the deleted entry. Clearing `loading` releases the flag that discarded
    // load would otherwise leave set; the catch does the same on failure.
    remove: async (id) => {
      ++requestSeq;
      try {
        await deleteDictionaryWord(id);
        set((s) => ({
          entries: s.entries.filter((e) => e.id !== id),
          loading: false,
        }));
      } catch (err) {
        set({ loading: false });
        throw err;
      }
    },
  };
});
