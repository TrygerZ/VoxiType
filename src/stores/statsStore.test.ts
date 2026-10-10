import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import { useStatsStore } from "./statsStore";

describe("statsStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStatsStore.setState({
      totals: { total_words: 0, total_duration_ms: 0, total_sessions: 0 },
      loaded: false,
      error: null,
    });
  });

  it("sets error to null and populates totals on successful load", async () => {
    const mockStats = {
      total_words: 100,
      total_duration_ms: 60000,
      total_sessions: 10,
    };
    vi.mocked(tauri.getUsageStats).mockResolvedValueOnce(mockStats);

    await useStatsStore.getState().load();

    const state = useStatsStore.getState();
    expect(state.loaded).toBe(true);
    expect(state.error).toBeNull();
    expect(state.totals).toEqual(mockStats);
  });

  it("normalizes null or undefined stats to zeroed defaults", async () => {
    vi.mocked(tauri.getUsageStats).mockResolvedValueOnce(
      // @ts-expect-error testing null response from backend
      null,
    );

    await useStatsStore.getState().load();

    const state = useStatsStore.getState();
    expect(state.loaded).toBe(true);
    expect(state.error).toBeNull();
    expect(state.totals).toEqual({
      total_words: 0,
      total_duration_ms: 0,
      total_sessions: 0,
    });
  });

  it("sets error and keeps previous totals on load failure", async () => {
    vi.mocked(tauri.getUsageStats).mockRejectedValueOnce(
      new Error("stats query failed"),
    );

    await useStatsStore.getState().load();

    const state = useStatsStore.getState();
    expect(state.loaded).toBe(true);
    expect(state.error).toBe("stats query failed");
  });

  it("resets error on retry after load failure", async () => {
    vi.mocked(tauri.getUsageStats).mockRejectedValueOnce(
      new Error("service unavailable"),
    );

    await useStatsStore.getState().load();
    expect(useStatsStore.getState().error).toBe("service unavailable");

    vi.mocked(tauri.getUsageStats).mockResolvedValueOnce({
      total_words: 50,
      total_duration_ms: 30000,
      total_sessions: 5,
    });

    await useStatsStore.getState().load();
    const state = useStatsStore.getState();
    expect(state.error).toBeNull();
    expect(state.totals.total_words).toBe(50);
  });

  it("ignores a stale load that resolves after a newer one", async () => {
    // First (older) load resolves last with stale totals; the second (newer)
    // load resolves first with fresh totals. The stale result must not
    // overwrite the fresher one.
    let resolveStale: (value: {
      total_words: number;
      total_duration_ms: number;
      total_sessions: number;
    }) => void = () => {};
    const stalePromise = new Promise<{
      total_words: number;
      total_duration_ms: number;
      total_sessions: number;
    }>((resolve) => {
      resolveStale = resolve;
    });

    vi.mocked(tauri.getUsageStats).mockReturnValueOnce(stalePromise);
    const staleLoad = useStatsStore.getState().load();

    vi.mocked(tauri.getUsageStats).mockResolvedValueOnce({
      total_words: 200,
      total_duration_ms: 120000,
      total_sessions: 20,
    });
    await useStatsStore.getState().load();

    // Resolve the older request only after the newer one has committed.
    resolveStale({
      total_words: 1,
      total_duration_ms: 1000,
      total_sessions: 1,
    });
    await staleLoad;

    const state = useStatsStore.getState();
    expect(state.totals.total_words).toBe(200);
    expect(state.totals.total_sessions).toBe(20);
  });
});
