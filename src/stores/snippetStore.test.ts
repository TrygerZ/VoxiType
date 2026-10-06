import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import type { Snippet } from "../types/app";
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

  it("ignores a slow load that resolves after a newer load", async () => {
    let resolveStale!: (snippets: Snippet[]) => void;
    const stalePromise = new Promise<Snippet[]>((resolve) => {
      resolveStale = resolve;
    });

    const freshSnippet: Snippet = {
      id: "s2",
      name: "fresh",
      trigger_phrase: "fresh",
      content: "fresh content",
      category: null,
      mode: null,
      usage_count: 0,
      is_active: true,
    };
    const staleSnippet = { ...freshSnippet, id: "s1", name: "stale" };

    vi.mocked(tauri.getSnippets).mockImplementationOnce(() => stalePromise);
    const first = useSnippetStore.getState().load();

    vi.mocked(tauri.getSnippets).mockResolvedValueOnce([freshSnippet]);
    await useSnippetStore.getState().load();

    expect(useSnippetStore.getState().snippets).toEqual([freshSnippet]);

    resolveStale([staleSnippet]);
    await first;

    expect(useSnippetStore.getState().snippets).toEqual([freshSnippet]);
  });

  it("clears loading when add invalidates an in-flight load", async () => {
    let resolveStale!: (snippets: Snippet[]) => void;
    const stalePromise = new Promise<Snippet[]>((resolve) => {
      resolveStale = resolve;
    });

    vi.mocked(tauri.getSnippets).mockImplementationOnce(() => stalePromise);
    const load = useSnippetStore.getState().load();
    expect(useSnippetStore.getState().loading).toBe(true);

    vi.mocked(tauri.addSnippet).mockResolvedValueOnce(undefined);
    vi.mocked(tauri.getSnippets).mockResolvedValueOnce([]);
    await useSnippetStore.getState().add({
      id: "s3",
      name: "new",
      trigger_phrase: "new",
      content: "new content",
      category: null,
      mode: null,
      usage_count: 0,
      is_active: true,
    });
    expect(useSnippetStore.getState().loading).toBe(false);

    resolveStale([]);
    await load;
    expect(useSnippetStore.getState().loading).toBe(false);
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
