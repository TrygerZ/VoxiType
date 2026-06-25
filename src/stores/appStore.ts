import { create } from "zustand";
import type { AppStateEnum } from "../types/app";

interface AppStore {
  state: AppStateEnum;
  audioLevel: number;
  durationSec: number;
  mode: string;
  wordCount: number | null;
  lastText: string | null;
  errorMessage: string | null;

  setState: (state: AppStateEnum) => void;
  setAudioLevel: (level: number) => void;
  setDuration: (sec: number) => void;
  setMode: (mode: string) => void;
  setResult: (text: string, wordCount: number) => void;
  setError: (message: string | null) => void;
  reset: () => void;
}

export const useAppStore = create<AppStore>((set) => ({
  state: "idle",
  audioLevel: 0,
  durationSec: 0,
  mode: "Dictation",
  wordCount: null,
  lastText: null,
  errorMessage: null,

  setState: (state) => set({ state }),
  setAudioLevel: (audioLevel) => set({ audioLevel }),
  setDuration: (durationSec) => set({ durationSec }),
  setMode: (mode) => set({ mode }),
  setResult: (lastText, wordCount) => set({ lastText, wordCount, errorMessage: null }),
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
