import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import type { TranscriptionEntry } from "../types/app";
import { useHistoryStore } from "./historyStore";

const createMockItem = (id: string, text: string): TranscriptionEntry => ({
  id,
  text_raw: text,
  text_formatted: `${text}.`,
  source_lang: "en",
  mode: "dictation",
  stt_engine: "groq",
  duration_ms: 1200,
  word_count: 2,
  character_count: 12,
  is_pinned: false,
  created_at: "2026-01-01T00:00:00Z",
});

describe("historyStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useHistoryStore.setState({
      items: [],
      loading: false,
      error: null,
      query: "",
      hasMore: false,
    });
  });

  it("sets error to null and populates items on successful load", async () => {
    const mockItems = [createMockItem("h1", "hello world")];
    vi.mocked(tauri.getHistory).mockResolvedValueOnce(mockItems);

    await useHistoryStore.getState().load();

    const state = useHistoryStore.getState();
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
    expect(state.items).toEqual(mockItems);
  });

  it("sets error and keeps empty items on load failure", async () => {
    vi.mocked(tauri.getHistory).mockRejectedValueOnce(
      new Error("database locked"),
    );

    await useHistoryStore.getState().load();

    const state = useHistoryStore.getState();
    expect(state.loading).toBe(false);
    expect(state.error).toBe("database locked");
    expect(state.items).toEqual([]);
  });

  it("resets error on retry after load failure", async () => {
    vi.mocked(tauri.getHistory).mockRejectedValueOnce(
      new Error("connection timeout"),
    );

    await useHistoryStore.getState().load();
    expect(useHistoryStore.getState().error).toBe("connection timeout");

    vi.mocked(tauri.getHistory).mockResolvedValueOnce([]);

    await useHistoryStore.getState().load();
    const state = useHistoryStore.getState();
    expect(state.error).toBeNull();
    expect(state.items).toEqual([]);
  });

  it("preserves active query during load()", async () => {
    const mockItems = [createMockItem("h1", "filtered item")];
    useHistoryStore.setState({ query: "active filter" });
    vi.mocked(tauri.searchHistory).mockResolvedValueOnce(mockItems);

    await useHistoryStore.getState().load();

    const state = useHistoryStore.getState();
    expect(state.query).toBe("active filter");
    expect(state.items).toEqual(mockItems);
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
  });

  it("re-runs searchHistory on load() when query is active", async () => {
    const searchItems = [createMockItem("h2", "meeting notes")];
    useHistoryStore.setState({ query: "meeting" });
    vi.mocked(tauri.searchHistory).mockResolvedValueOnce(searchItems);

    await useHistoryStore.getState().load();

    expect(tauri.searchHistory).toHaveBeenCalledWith("meeting");
    expect(tauri.getHistory).not.toHaveBeenCalled();
    const state = useHistoryStore.getState();
    expect(state.items).toEqual(searchItems);
    expect(state.query).toBe("meeting");
  });

  it("falls back to getHistory on load() when query is whitespace only", async () => {
    const allItems = [createMockItem("h3", "general note")];
    useHistoryStore.setState({ query: "   " });
    vi.mocked(tauri.getHistory).mockResolvedValueOnce(allItems);

    await useHistoryStore.getState().load();

    expect(tauri.getHistory).toHaveBeenCalled();
    expect(tauri.searchHistory).not.toHaveBeenCalled();
    expect(useHistoryStore.getState().items).toEqual(allItems);
  });

  it("sets error and leaves loading false on failed debounced search", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(tauri.searchHistory).mockRejectedValueOnce(
        new Error("search syntax error"),
      );

      await useHistoryStore.getState().search("bad query");
      expect(useHistoryStore.getState().query).toBe("bad query");

      await vi.advanceTimersByTimeAsync(300);

      const state = useHistoryStore.getState();
      expect(state.loading).toBe(false);
      expect(state.error).toBe("search syntax error");
    } finally {
      vi.useRealTimers();
    }
  });

  it("sets error and leaves loading false on failed empty search", async () => {
    vi.mocked(tauri.getHistory).mockRejectedValueOnce(
      new Error("db read error"),
    );

    await useHistoryStore.getState().search("");

    const state = useHistoryStore.getState();
    expect(state.loading).toBe(false);
    expect(state.error).toBe("db read error");
  });

  it("ignores stale response when a newer request completes first", async () => {
    let resolveStale!: (items: TranscriptionEntry[]) => void;
    const stalePromise = new Promise<TranscriptionEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    const freshItem = createMockItem("h-fresh", "fresh result");
    const staleItem = createMockItem("h-stale", "stale result");

    vi.mocked(tauri.getHistory).mockImplementationOnce(() => stalePromise);
    const firstCall = useHistoryStore.getState().load();

    vi.mocked(tauri.getHistory).mockResolvedValueOnce([freshItem]);
    await useHistoryStore.getState().load();

    expect(useHistoryStore.getState().items).toEqual([freshItem]);

    resolveStale([staleItem]);
    await firstCall;

    const state = useHistoryStore.getState();
    expect(state.items).toEqual([freshItem]);
    expect(state.loading).toBe(false);
  });

  it("does not resurrect a removed item when a stale fetch resolves late", async () => {
    let resolveStale!: (items: TranscriptionEntry[]) => void;
    const stalePromise = new Promise<TranscriptionEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    const removed = createMockItem("h-remove", "to be removed");
    vi.mocked(tauri.getHistory).mockImplementationOnce(() => stalePromise);
    const load = useHistoryStore.getState().load();

    useHistoryStore.setState({ items: [removed] });
    vi.mocked(tauri.deleteHistory).mockResolvedValueOnce(undefined);
    await useHistoryStore.getState().remove("h-remove");

    expect(useHistoryStore.getState().items).toEqual([]);

    // The stale fetch still carries the deleted row; it must be discarded.
    resolveStale([removed]);
    await load;

    expect(useHistoryStore.getState().items).toEqual([]);
  });

  it("clears loading when remove invalidates an in-flight load", async () => {
    let resolveStale!: (items: TranscriptionEntry[]) => void;
    const stalePromise = new Promise<TranscriptionEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    vi.mocked(tauri.getHistory).mockImplementationOnce(() => stalePromise);
    const load = useHistoryStore.getState().load();
    expect(useHistoryStore.getState().loading).toBe(true);

    vi.mocked(tauri.deleteHistory).mockResolvedValueOnce(undefined);
    await useHistoryStore.getState().remove("h1");
    expect(useHistoryStore.getState().loading).toBe(false);

    resolveStale([]);
    await load;
    expect(useHistoryStore.getState().loading).toBe(false);
  });

  it("clears loading when remove rejects after invalidating an in-flight load", async () => {
    let resolveStale!: (items: TranscriptionEntry[]) => void;
    const stalePromise = new Promise<TranscriptionEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    vi.mocked(tauri.getHistory).mockImplementationOnce(() => stalePromise);
    const load = useHistoryStore.getState().load();
    expect(useHistoryStore.getState().loading).toBe(true);

    vi.mocked(tauri.deleteHistory).mockRejectedValueOnce(
      new Error("delete failed"),
    );
    await expect(useHistoryStore.getState().remove("h1")).rejects.toThrow(
      "delete failed",
    );
    // The discarded load's clear is skipped, so the mutation must clear it.
    expect(useHistoryStore.getState().loading).toBe(false);

    resolveStale([]);
    await load;
    expect(useHistoryStore.getState().loading).toBe(false);
  });

  it("clears loading when clear invalidates an in-flight load", async () => {
    let resolveStale!: (items: TranscriptionEntry[]) => void;
    const stalePromise = new Promise<TranscriptionEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    vi.mocked(tauri.getHistory).mockImplementationOnce(() => stalePromise);
    const load = useHistoryStore.getState().load();
    expect(useHistoryStore.getState().loading).toBe(true);

    vi.mocked(tauri.clearHistory).mockResolvedValueOnce(0);
    await useHistoryStore.getState().clear();
    expect(useHistoryStore.getState().loading).toBe(false);

    resolveStale([]);
    await load;
    expect(useHistoryStore.getState().loading).toBe(false);
  });

  it("clears loading when clear rejects after invalidating an in-flight load", async () => {
    let resolveStale!: (items: TranscriptionEntry[]) => void;
    const stalePromise = new Promise<TranscriptionEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    vi.mocked(tauri.getHistory).mockImplementationOnce(() => stalePromise);
    const load = useHistoryStore.getState().load();
    expect(useHistoryStore.getState().loading).toBe(true);

    vi.mocked(tauri.clearHistory).mockRejectedValueOnce(
      new Error("clear failed"),
    );
    await expect(useHistoryStore.getState().clear()).rejects.toThrow(
      "clear failed",
    );
    expect(useHistoryStore.getState().loading).toBe(false);

    resolveStale([]);
    await load;
    expect(useHistoryStore.getState().loading).toBe(false);
  });

  it("clears loading when togglePin invalidates an in-flight load", async () => {
    let resolveStale!: (items: TranscriptionEntry[]) => void;
    const stalePromise = new Promise<TranscriptionEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    vi.mocked(tauri.getHistory).mockImplementationOnce(() => stalePromise);
    const load = useHistoryStore.getState().load();
    expect(useHistoryStore.getState().loading).toBe(true);

    vi.mocked(tauri.pinHistory).mockResolvedValueOnce(undefined);
    await useHistoryStore.getState().togglePin("h1", true);
    expect(useHistoryStore.getState().loading).toBe(false);

    resolveStale([]);
    await load;
    expect(useHistoryStore.getState().loading).toBe(false);
  });

  it("clears loading when togglePin rejects after invalidating an in-flight load", async () => {
    let resolveStale!: (items: TranscriptionEntry[]) => void;
    const stalePromise = new Promise<TranscriptionEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    vi.mocked(tauri.getHistory).mockImplementationOnce(() => stalePromise);
    const load = useHistoryStore.getState().load();
    expect(useHistoryStore.getState().loading).toBe(true);

    vi.mocked(tauri.pinHistory).mockRejectedValueOnce(
      new Error("pin failed"),
    );
    await expect(
      useHistoryStore.getState().togglePin("h1", true),
    ).rejects.toThrow("pin failed");
    expect(useHistoryStore.getState().loading).toBe(false);

    resolveStale([]);
    await load;
    expect(useHistoryStore.getState().loading).toBe(false);
  });

  it("load() requests the first page and sets hasMore when the page is full", async () => {
    const fullPage = Array.from({ length: 100 }, (_, i) =>
      createMockItem(`h${i}`, `item ${i}`),
    );
    vi.mocked(tauri.getHistory).mockResolvedValueOnce(fullPage);

    await useHistoryStore.getState().load();

    expect(tauri.getHistory).toHaveBeenCalledWith({ limit: 100, offset: 0 });
    const state = useHistoryStore.getState();
    expect(state.items).toEqual(fullPage);
    expect(state.hasMore).toBe(true);
  });

  it("load() clears hasMore when a short page is returned", async () => {
    vi.mocked(tauri.getHistory).mockResolvedValueOnce([
      createMockItem("h1", "only one"),
    ]);

    await useHistoryStore.getState().load();

    expect(useHistoryStore.getState().hasMore).toBe(false);
  });

  it("loadMore() requests the next page and appends results", async () => {
    const firstPage = Array.from({ length: 100 }, (_, i) =>
      createMockItem(`h${i}`, `item ${i}`),
    );
    const secondPage = [createMockItem("h100", "next page item")];
    vi.mocked(tauri.getHistory)
      .mockResolvedValueOnce(firstPage)
      .mockResolvedValueOnce(secondPage);

    await useHistoryStore.getState().load();
    await useHistoryStore.getState().loadMore();

    expect(tauri.getHistory).toHaveBeenLastCalledWith({
      limit: 100,
      offset: 100,
    });
    const state = useHistoryStore.getState();
    expect(state.items).toEqual([...firstPage, ...secondPage]);
    expect(state.hasMore).toBe(false);
  });

  it("loadMore() does nothing when hasMore is false", async () => {
    useHistoryStore.setState({ items: [createMockItem("h1", "x")], hasMore: false });

    await useHistoryStore.getState().loadMore();

    expect(tauri.getHistory).not.toHaveBeenCalled();
  });

  it("loadMore() does nothing while a load is already in flight", async () => {
    useHistoryStore.setState({ hasMore: true, loading: true });

    await useHistoryStore.getState().loadMore();

    expect(tauri.getHistory).not.toHaveBeenCalled();
  });

  it("ignores stale rejection when a newer request has already completed", async () => {
    let rejectStale!: (err: Error) => void;
    const stalePromise = new Promise<TranscriptionEntry[]>((_, reject) => {
      rejectStale = reject;
    });

    const freshItem = createMockItem("h-fresh", "fresh result");

    vi.mocked(tauri.getHistory).mockImplementationOnce(() => stalePromise);
    const firstCall = useHistoryStore.getState().load();

    vi.mocked(tauri.getHistory).mockResolvedValueOnce([freshItem]);
    await useHistoryStore.getState().load();

    expect(useHistoryStore.getState().items).toEqual([freshItem]);
    expect(useHistoryStore.getState().error).toBeNull();

    rejectStale(new Error("late failure"));
    await firstCall;

    const state = useHistoryStore.getState();
    expect(state.items).toEqual([freshItem]);
    expect(state.error).toBeNull();
    expect(state.loading).toBe(false);
  });
});
