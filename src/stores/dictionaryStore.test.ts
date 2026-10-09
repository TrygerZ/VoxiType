import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import type { DictionaryEntry } from "../types/app";
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

  it("ignores a slow load that resolves after a newer load", async () => {
    let resolveStale!: (entries: DictionaryEntry[]) => void;
    const stalePromise = new Promise<DictionaryEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    const freshEntry: DictionaryEntry = {
      id: "2",
      word: "fresh",
      pronunciation: null,
      category: "tech",
      replacement: "fresh",
      language: "en",
      usage_count: 0,
      is_active: true,
    };
    const staleEntry = { ...freshEntry, id: "1", word: "stale" };

    vi.mocked(tauri.getDictionary).mockImplementationOnce(() => stalePromise);
    const first = useDictionaryStore.getState().load();

    vi.mocked(tauri.getDictionary).mockResolvedValueOnce([freshEntry]);
    await useDictionaryStore.getState().load();

    expect(useDictionaryStore.getState().entries).toEqual([freshEntry]);

    resolveStale([staleEntry]);
    await first;

    expect(useDictionaryStore.getState().entries).toEqual([freshEntry]);
  });

  it("clears loading when add invalidates an in-flight load", async () => {
    let resolveStale!: (entries: DictionaryEntry[]) => void;
    const stalePromise = new Promise<DictionaryEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    vi.mocked(tauri.getDictionary).mockImplementationOnce(() => stalePromise);
    const load = useDictionaryStore.getState().load();
    expect(useDictionaryStore.getState().loading).toBe(true);

    vi.mocked(tauri.addDictionaryWord).mockResolvedValueOnce(undefined);
    vi.mocked(tauri.getDictionary).mockResolvedValueOnce([]);
    await useDictionaryStore.getState().add({
      id: "3",
      word: "new",
      pronunciation: null,
      category: "tech",
      replacement: "new",
      language: "en",
      usage_count: 0,
      is_active: true,
    });
    expect(useDictionaryStore.getState().loading).toBe(false);

    resolveStale([]);
    await load;
    expect(useDictionaryStore.getState().loading).toBe(false);
  });

  it("does not resurrect a removed entry when a stale load resolves late", async () => {
    let resolveStale!: (entries: DictionaryEntry[]) => void;
    const stalePromise = new Promise<DictionaryEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    const removed: DictionaryEntry = {
      id: "1",
      word: "stale",
      pronunciation: null,
      category: "tech",
      replacement: "stale",
      language: "en",
      usage_count: 0,
      is_active: true,
    };
    vi.mocked(tauri.getDictionary).mockImplementationOnce(() => stalePromise);
    const load = useDictionaryStore.getState().load();

    useDictionaryStore.setState({ entries: [removed] });
    vi.mocked(tauri.deleteDictionaryWord).mockResolvedValueOnce(undefined);
    await useDictionaryStore.getState().remove("1");

    expect(useDictionaryStore.getState().entries).toEqual([]);

    resolveStale([removed]);
    await load;

    expect(useDictionaryStore.getState().entries).toEqual([]);
  });

  it("clears loading when remove invalidates an in-flight load", async () => {
    let resolveStale!: (entries: DictionaryEntry[]) => void;
    const stalePromise = new Promise<DictionaryEntry[]>((resolve) => {
      resolveStale = resolve;
    });

    vi.mocked(tauri.getDictionary).mockImplementationOnce(() => stalePromise);
    const load = useDictionaryStore.getState().load();
    expect(useDictionaryStore.getState().loading).toBe(true);

    vi.mocked(tauri.deleteDictionaryWord).mockResolvedValueOnce(undefined);
    await useDictionaryStore.getState().remove("1");
    expect(useDictionaryStore.getState().loading).toBe(false);

    resolveStale([]);
    await load;
    expect(useDictionaryStore.getState().loading).toBe(false);
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
