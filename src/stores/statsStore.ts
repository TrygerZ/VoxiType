import { create } from "zustand";
import type { UsageStats } from "../types/app";
import { formatTauriError, getUsageStats } from "../lib/tauri";

/**
 * Lifetime usage totals sourced from the backend `get_usage_stats` command,
 * which aggregates over the full transcriptions table. Kept separate from the
 * history list (capped at 100 rows) so totals stay accurate and never shrink
 * as older entries scroll out of the list window.
 */
interface StatsStore {
  totals: UsageStats;
  loaded: boolean;
  error: string | null;
  load: () => Promise<void>;
}

const EMPTY: UsageStats = {
  total_words: 0,
  total_duration_ms: 0,
  total_sessions: 0,
};

const normalizeStats = (stats?: Partial<UsageStats> | null): UsageStats => ({
  total_words: typeof stats?.total_words === "number" ? stats.total_words : 0,
  total_duration_ms:
    typeof stats?.total_duration_ms === "number" ? stats.total_duration_ms : 0,
  total_sessions:
    typeof stats?.total_sessions === "number" ? stats.total_sessions : 0,
});

export const useStatsStore = create<StatsStore>((set) => {
  // Monotonic token: every load captures the current value and only commits
  // its result if it is still the latest. Prevents a slow in-flight request
  // from overwriting fresher totals out of order.
  let requestSeq = 0;

  return {
    totals: EMPTY,
    loaded: false,
    error: null,

    load: async () => {
      const seq = ++requestSeq;
      try {
        const totals = await getUsageStats();
        if (seq === requestSeq) {
          set({ totals: normalizeStats(totals), loaded: true, error: null });
        }
      } catch (err: unknown) {
        // Leave the last-known totals in place on failure rather than zeroing
        // the dashboard, but record error for UI retry.
        if (seq === requestSeq) {
          set({ loaded: true, error: formatTauriError(err) });
        }
      }
    },
  };
});
