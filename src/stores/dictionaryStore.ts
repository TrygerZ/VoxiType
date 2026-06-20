import { create } from "zustand";
import type { DictionaryEntry } from "../types/app";
import {
  addDictionaryWord,
  deleteDictionaryWord,
  getDictionary,
} from "../lib/tauri";

interface DictionaryStore {
  entries: DictionaryEntry[];
  loading: boolean;
  load: () => Promise<void>;
  add: (entry: DictionaryEntry) => Promise<void>;
  remove: (id: string) => Promise<void>;
}

export const useDictionaryStore = create<DictionaryStore>((set) => ({
  entries: [],
  loading: false,

  load: async () => {
    set({ loading: true });
    try {
      const entries = await getDictionary();
      set({ entries, loading: false });
    } catch {
      set({ loading: false });
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
