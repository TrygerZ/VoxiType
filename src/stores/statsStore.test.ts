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
});
