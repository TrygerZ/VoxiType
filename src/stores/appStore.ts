import { create } from "zustand";
import type { AppStateEnum } from "../types/app";

/** Outcome of the last finished session, as reported by the backend. */
export interface InjectionOutcome {
  injected: boolean;
  appContext: string | null;
}

interface AppStore {
  state: AppStateEnum;
  durationSec: number;
  wordCount: number | null;
  errorMessage: string | null;
  outcome: InjectionOutcome | null;

  setState: (state: AppStateEnum) => void;
  setDuration: (sec: number) => void;
  setResult: (wordCount: number, outcome: InjectionOutcome) => void;
  setError: (message: string | null) => void;
  reset: () => void;
}

export const useAppStore = create<AppStore>((set) => ({
  state: "idle",
  durationSec: 0,
  wordCount: null,
  errorMessage: null,
  outcome: null,

  // Clear any stale error when leaving the error state (e.g. a new recording
  // starts after a failure) so the widget doesn't render error styling on top
  // of a live session.
  setState: (state) =>
    set((s) => ({
      state,
      errorMessage: state === "error" ? s.errorMessage : null,
      // Clear any stale result when a new session starts so a canceled
      // recording (Recording→Idle) doesn't flash the previous success pill.
      wordCount: state === "recording" || state === "processing" ? null : s.wordCount,
      outcome: state === "recording" || state === "processing" ? null : s.outcome,
    })),
  setDuration: (durationSec) => set({ durationSec }),
  setResult: (wordCount, outcome) => set({ wordCount, outcome, errorMessage: null }),
  setError: (errorMessage) =>
    set({ errorMessage, state: errorMessage ? "error" : "idle" }),
  reset: () =>
    set({
      state: "idle",
      durationSec: 0,
      wordCount: null,
      errorMessage: null,
      outcome: null,
    }),
}));
