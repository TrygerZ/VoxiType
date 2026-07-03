import { useEffect, useRef } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { onEvent } from "../lib/tauri";
import { useAppStore } from "../stores/appStore";
import { useHistoryStore } from "../stores/historyStore";
import { useStatsStore } from "../stores/statsStore";
import type {
  AudioLevelEvent,
  StateChangedEvent,
  TranscriptionCompleteEvent,
  TranscriptionErrorEvent,
} from "../types/events";

/**
 * Subscribe to all backend events and reflect them into the app store.
 * Mount once near the app root.
 */
export function useTauriEvents() {
  const setState = useAppStore((s) => s.setState);
  const setAudioLevel = useAppStore((s) => s.setAudioLevel);
  const setDuration = useAppStore((s) => s.setDuration);
  const setResult = useAppStore((s) => s.setResult);
  const setError = useAppStore((s) => s.setError);
  const reloadHistory = useHistoryStore((s) => s.load);
  const reloadStats = useStatsStore((s) => s.load);

  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    const clearTimer = () => {
      if (timerRef.current !== null) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };

    unlisteners.push(
      onEvent<StateChangedEvent>("state_changed", (p) => {
        setState(p.state);
        if (p.state === "recording") {
          const start = Date.now();
          setDuration(0);
          clearTimer();
          timerRef.current = setInterval(() => {
            setDuration(Math.floor((Date.now() - start) / 1000));
          }, 250);
        } else {
          clearTimer();
        }
      }),
    );
    unlisteners.push(
      onEvent<AudioLevelEvent>("audio_level", (p) => setAudioLevel(p.level)),
    );
    unlisteners.push(
      onEvent<TranscriptionCompleteEvent>("transcription_complete", (p) => {
        setResult(p.word_count);
        void reloadHistory();
        void reloadStats();
      }),
    );
    unlisteners.push(
      onEvent<TranscriptionErrorEvent>("transcription_error", (p) =>
        setError(p.message),
      ),
    );

    return () => {
      clearTimer();
      unlisteners.forEach((u) => void u.then((fn) => fn()));
    };
  }, [setState, setAudioLevel, setDuration, setResult, setError, reloadHistory, reloadStats]);
}
