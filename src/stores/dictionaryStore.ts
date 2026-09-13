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

export const useDictionaryStore = create<DictionaryStore>((set) => ({
  entries: [],
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const entries = await getDictionary();
      set({ entries, loading: false, error: null });
    } catch (err: unknown) {
      set({ loading: false, error: formatTauriError(err) });
    }
  },

  add: async (entry) => {
    await addDictionaryWord(entry);
    const entries = await getDictionary();
    set({ entries });
  },

  remove: async (id) => {
    await deleteDictionaryWord(id);
    set((s) => ({ entries: s.entries.filter((e) => e.id !== id) }));
  },
}));
