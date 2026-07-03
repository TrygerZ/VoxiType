import { create } from "zustand";
import type { AppStateEnum } from "../types/app";

interface AppStore {
  state: AppStateEnum;
  audioLevel: number;
  durationSec: number;
  wordCount: number | null;
  errorMessage: string | null;

  setState: (state: AppStateEnum) => void;
  setAudioLevel: (level: number) => void;
  setDuration: (sec: number) => void;
  setResult: (wordCount: number) => void;
  setError: (message: string | null) => void;
  reset: () => void;
}

export const useAppStore = create<AppStore>((set) => ({
  state: "idle",
  audioLevel: 0,
  durationSec: 0,
  wordCount: null,
  errorMessage: null,

  // Clear any stale error when leaving the error state (e.g. a new recording
  // starts after a failure) so the widget doesn't render error styling on top
  // of a live session.
  setState: (state) =>
    set((s) => ({
      state,
      errorMessage: state === "error" ? s.errorMessage : null,
    })),
  setAudioLevel: (audioLevel) => set({ audioLevel }),
  setDuration: (durationSec) => set({ durationSec }),
  setResult: (wordCount) => set({ wordCount, errorMessage: null }),
  setError: (errorMessage) =>
    set({ errorMessage, state: errorMessage ? "error" : "idle" }),
  reset: () =>
    set({
      state: "idle",
      audioLevel: 0,
      durationSec: 0,
      wordCount: null,
      errorMessage: null,
    }),
}));
