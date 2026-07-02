import { create } from "zustand";
import type { UsageStats } from "../types/app";
import { getUsageStats } from "../lib/tauri";

/**
 * Lifetime usage totals sourced from the backend `get_usage_stats` command,
 * which aggregates over the full transcriptions table. Kept separate from the
 * history list (capped at 100 rows) so totals stay accurate and never shrink
 * as older entries scroll out of the list window.
 */
interface StatsStore {
  totals: UsageStats;
  loaded: boolean;
  load: () => Promise<void>;
}

const EMPTY: UsageStats = {
  total_words: 0,
  total_duration_ms: 0,
  total_sessions: 0,
};

export const useStatsStore = create<StatsStore>((set) => ({
  totals: EMPTY,
  loaded: false,

  load: async () => {
    try {
      const totals = await getUsageStats();
      set({ totals, loaded: true });
    } catch {
      // Leave the last-known totals in place on failure rather than zeroing
      // the dashboard.
      set({ loaded: true });
    }
  },
}));
