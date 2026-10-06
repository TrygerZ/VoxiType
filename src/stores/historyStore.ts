import { create } from "zustand";
import { debounce } from "../lib/debounce";
import type { TranscriptionEntry } from "../types/app";
import {
  clearHistory,
  deleteHistory,
  formatTauriError,
  getHistory,
  pinHistory,
  searchHistory,
} from "../lib/tauri";

interface HistoryStore {
  items: TranscriptionEntry[];
  loading: boolean;
  error: string | null;
  query: string;
  load: () => Promise<void>;
  search: (query: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  clear: (keepPinned?: boolean) => Promise<void>;
  togglePin: (id: string, pinned: boolean) => Promise<void>;
}

export const useHistoryStore = create<HistoryStore>((set, get) => {
  // Monotonic token: every fetch captures the current value and only commits
  // its result if it is still the latest. Prevents a slow in-flight request
  // (e.g. an earlier search) from overwriting fresher results out of order.
  let requestSeq = 0;

  const fetchItems = async (q: string, seq: number) => {
    const trimmed = q.trim();
    set({ loading: true, error: null });
    try {
      const items = trimmed
        ? await searchHistory(trimmed)
        : await getHistory();
      if (seq === requestSeq) {
        set({ items, loading: false, error: null });
      }
    } catch (err: unknown) {
      if (seq === requestSeq) {
        set({ loading: false, error: formatTauriError(err) });
      }
    }
  };

  const doSearch = debounce(async (q: string) => {
    await fetchItems(q, ++requestSeq);
  }, 300);

  return {
    items: [],
    loading: false,
    error: null,
    query: "",

    load: async () => {
      doSearch.cancel();
      await fetchItems(get().query, ++requestSeq);
    },

    search: async (query) => {
      set({ query, error: null });
      if (!query.trim()) {
        doSearch.cancel();
        await fetchItems("", ++requestSeq);
        return;
      }
      doSearch(query);
    },

    // Each mutation bumps the token so an in-flight fetch cannot commit stale
    // items afterwards and resurrect the row just deleted. Clearing `loading`
    // here releases the flag the discarded fetch would otherwise leave set.
    remove: async (id) => {
      ++requestSeq;
      await deleteHistory(id);
      set((s) => ({
        items: s.items.filter((i) => i.id !== id),
        loading: false,
      }));
    },

    clear: async (keepPinned = true) => {
      ++requestSeq;
      await clearHistory(keepPinned);
      set((s) => ({
        items: keepPinned ? s.items.filter((i) => i.is_pinned) : [],
        loading: false,
      }));
    },

    togglePin: async (id, pinned) => {
      ++requestSeq;
      await pinHistory(id, pinned);
      set((s) => ({
        items: s.items.map((i) =>
          i.id === id ? { ...i, is_pinned: pinned } : i,
        ),
        loading: false,
      }));
    },
  };
});
