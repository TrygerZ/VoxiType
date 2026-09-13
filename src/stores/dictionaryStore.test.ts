import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import { useDictionaryStore } from "./dictionaryStore";

describe("dictionaryStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDictionaryStore.setState({
      entries: [],
      loading: false,
      error: null,
    });
  });

  it("sets error to null and populates entries on successful load", async () => {
    const mockEntries = [
      {
        id: "1",
        word: "AI",
        pronunciation: null,
        category: "tech",
        replacement: "Artificial Intelligence",
        language: "en",
        usage_count: 5,
        is_active: true,
      },
    ];
    vi.mocked(tauri.getDictionary).mockResolvedValueOnce(mockEntries);

    await useDictionaryStore.getState().load();

    const state = useDictionaryStore.getState();
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
    expect(state.entries).toEqual(mockEntries);
  });

  it("sets error and preserves empty entries on load failure", async () => {
    vi.mocked(tauri.getDictionary).mockRejectedValueOnce(
      new Error("failed to query dictionary"),
    );

    await useDictionaryStore.getState().load();

    const state = useDictionaryStore.getState();
    expect(state.loading).toBe(false);
    expect(state.error).toBe("failed to query dictionary");
    expect(state.entries).toEqual([]);
  });

  it("clears error on subsequent successful retry", async () => {
    vi.mocked(tauri.getDictionary).mockRejectedValueOnce(
      new Error("transient db error"),
    );

    await useDictionaryStore.getState().load();
    expect(useDictionaryStore.getState().error).toBe("transient db error");

    vi.mocked(tauri.getDictionary).mockResolvedValueOnce([]);

    await useDictionaryStore.getState().load();
    const state = useDictionaryStore.getState();
    expect(state.error).toBeNull();
    expect(state.entries).toEqual([]);
  });
});
