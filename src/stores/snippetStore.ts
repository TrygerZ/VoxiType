import { create } from "zustand";
import type { Snippet } from "../types/app";
import {
  addSnippet,
  deleteSnippet,
  formatTauriError,
  getSnippets,
} from "../lib/tauri";

interface SnippetStore {
  snippets: Snippet[];
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
  add: (snippet: Snippet) => Promise<void>;
  remove: (id: string) => Promise<void>;
}

export const useSnippetStore = create<SnippetStore>((set) => ({
  snippets: [],
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const snippets = await getSnippets();
      set({ snippets, loading: false, error: null });
    } catch (err: unknown) {
      set({ loading: false, error: formatTauriError(err) });
    }
  },

  add: async (snippet) => {
    await addSnippet(snippet);
    const snippets = await getSnippets();
    set({ snippets });
  },

  remove: async (id) => {
    await deleteSnippet(id);
    set((s) => ({ snippets: s.snippets.filter((x) => x.id !== id) }));
  },
}));
