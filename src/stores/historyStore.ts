import { create } from "zustand";
import { debounce } from "../lib/debounce";
import type { TranscriptionEntry } from "../types/app";
import {
  clearHistory,
  deleteHistory,
  getHistory,
  pinHistory,
  searchHistory,
} from "../lib/tauri";

interface HistoryStore {
  items: TranscriptionEntry[];
  loading: boolean;
  query: string;
  load: () => Promise<void>;
  search: (query: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  clear: (keepPinned?: boolean) => Promise<void>;
  togglePin: (id: string, pinned: boolean) => Promise<void>;
}

export const useHistoryStore = create<HistoryStore>((set) => {
  const doSearch = debounce(async (q: string) => {
    try {
      const items = q.trim()
        ? await searchHistory(q)
        : await getHistory();
      set({ items });
    } catch {
      // search failed silently — keep existing items
    }
  }, 300);

  return {
    items: [],
    loading: false,
    query: "",

    load: async () => {
      set({ loading: true });
      try {
        const items = await getHistory();
        set({ items, loading: false, query: "" });
      } catch {
        set({ loading: false });
      }
    },

    search: async (query) => {
      set({ query });
      if (!query.trim()) {
        doSearch.cancel();
        try {
          const items = await getHistory();
          set({ items });
        } catch {
          // silent
        }
        return;
      }
      doSearch(query);
    },

    remove: async (id) => {
      await deleteHistory(id);
      set((s) => ({ items: s.items.filter((i) => i.id !== id) }));
    },

    clear: async (keepPinned = true) => {
      await clearHistory(keepPinned);
      set((s) => ({
        items: keepPinned ? s.items.filter((i) => i.is_pinned) : [],
      }));
    },

    togglePin: async (id, pinned) => {
      await pinHistory(id, pinned);
      set((s) => ({
        items: s.items.map((i) =>
          i.id === id ? { ...i, is_pinned: pinned } : i,
        ),
      }));
    },
  };
});
