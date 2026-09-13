import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import { useHistoryStore } from "./historyStore";

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
    const mockItems = [
      {
        id: "h1",
        text_raw: "hello world",
        text_formatted: "Hello world.",
        source_lang: "en",
        mode: "dictation",
        stt_engine: "groq",
        duration_ms: 1200,
        word_count: 2,
        character_count: 12,
        is_pinned: false,
        created_at: "2026-01-01T00:00:00Z",
      },
    ];
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
});
