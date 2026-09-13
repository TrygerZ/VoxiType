import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import { useSnippetStore } from "./snippetStore";

describe("snippetStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSnippetStore.setState({
      snippets: [],
      loading: false,
      error: null,
    });
  });

  it("sets error to null and populates snippets on successful load", async () => {
    const mockSnippets = [
      {
        id: "s1",
        name: "addr",
        trigger_phrase: "addr",
        content: "123 Main St",
        category: null,
        mode: null,
        usage_count: 0,
        is_active: true,
      },
    ];
    vi.mocked(tauri.getSnippets).mockResolvedValueOnce(mockSnippets);

    await useSnippetStore.getState().load();

    const state = useSnippetStore.getState();
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
    expect(state.snippets).toEqual(mockSnippets);
  });

  it("sets error and keeps empty snippets on load failure", async () => {
    vi.mocked(tauri.getSnippets).mockRejectedValueOnce(
      new Error("failed to fetch snippets"),
    );

    await useSnippetStore.getState().load();

    const state = useSnippetStore.getState();
    expect(state.loading).toBe(false);
    expect(state.error).toBe("failed to fetch snippets");
    expect(state.snippets).toEqual([]);
  });

  it("resets error on subsequent successful retry", async () => {
    vi.mocked(tauri.getSnippets).mockRejectedValueOnce(
      new Error("io error"),
    );

    await useSnippetStore.getState().load();
    expect(useSnippetStore.getState().error).toBe("io error");

    vi.mocked(tauri.getSnippets).mockResolvedValueOnce([]);

    await useSnippetStore.getState().load();
    const state = useSnippetStore.getState();
    expect(state.error).toBeNull();
    expect(state.snippets).toEqual([]);
  });
});
