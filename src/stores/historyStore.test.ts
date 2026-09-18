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
