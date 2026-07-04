import { create } from "zustand";
import type { Snippet } from "../types/app";
import { addSnippet, deleteSnippet, getSnippets } from "../lib/tauri";

interface SnippetStore {
  snippets: Snippet[];
  loading: boolean;
  load: () => Promise<void>;
  add: (snippet: Snippet) => Promise<void>;
  remove: (id: string) => Promise<void>;
}

export const useSnippetStore = create<SnippetStore>((set) => ({
  snippets: [],
  loading: false,

  load: async () => {
    set({ loading: true });
    try {
      const snippets = await getSnippets();
      set({ snippets, loading: false });
    } catch {
      set({ loading: false });
    }
  },

  add: async (snippet) => {
    try {
      await addSnippet(snippet);
      const snippets = await getSnippets();
      set({ snippets });
    } catch {
      // ponytail: surface via caller; keep list unchanged on failure
    }
  },

  remove: async (id) => {
    await deleteSnippet(id);
    set((s) => ({ snippets: s.snippets.filter((x) => x.id !== id) }));
  },
}));
