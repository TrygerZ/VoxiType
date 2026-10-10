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

export const useSnippetStore = create<SnippetStore>((set) => {
  // Monotonic token: every fetch captures the current value and only commits
  // its result if it is still the latest, so a slow earlier load cannot
  // overwrite fresher snippets out of order.
  let requestSeq = 0;

  return {
    snippets: [],
    loading: false,
    error: null,

    load: async () => {
      const seq = ++requestSeq;
      set({ loading: true, error: null });
      try {
        const snippets = await getSnippets();
        if (seq === requestSeq) {
          set({ snippets, loading: false, error: null });
        }
      } catch (err: unknown) {
        if (seq === requestSeq) {
          set({ loading: false, error: formatTauriError(err) });
        }
      }
    },

    add: async (snippet) => {
      // Bump so a stale in-flight load cannot overwrite the post-add list.
      // Clearing `loading` releases the flag the discarded load would leave set.
      // On failure the discarded load's clear is skipped, so the catch clears it
      // too before rethrowing; otherwise `loading` would stay stuck true.
      const seq = ++requestSeq;
      try {
        await addSnippet(snippet);
        const snippets = await getSnippets();
        if (seq === requestSeq) {
          set({ snippets, loading: false });
        }
      } catch (err) {
        set({ loading: false });
        throw err;
      }
    },

    // Bump so a stale in-flight load cannot commit afterwards and resurrect
    // the deleted snippet. Clearing `loading` releases the flag that discarded
    // load would otherwise leave set; the catch does the same on failure.
    remove: async (id) => {
      ++requestSeq;
      try {
        await deleteSnippet(id);
        set((s) => ({
          snippets: s.snippets.filter((x) => x.id !== id),
          loading: false,
        }));
      } catch (err) {
        set({ loading: false });
        throw err;
      }
    },
  };
});
