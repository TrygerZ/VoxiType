import { useEffect } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { onEvent } from "../lib/tauri";
import { useAppStore } from "../stores/appStore";
import { useHistoryStore } from "../stores/historyStore";
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
  const setResult = useAppStore((s) => s.setResult);
  const setError = useAppStore((s) => s.setError);
  const reloadHistory = useHistoryStore((s) => s.load);

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      onEvent<StateChangedEvent>("state_changed", (p) => setState(p.state)),
    );
    unlisteners.push(
      onEvent<AudioLevelEvent>("audio_level", (p) => setAudioLevel(p.level)),
    );
    unlisteners.push(
      onEvent<TranscriptionCompleteEvent>("transcription_complete", (p) => {
        setResult(p.text, p.word_count);
        void reloadHistory();
      }),
    );
    unlisteners.push(
      onEvent<TranscriptionErrorEvent>("transcription_error", (p) =>
        setError(p.message),
      ),
    );

    return () => {
      unlisteners.forEach((u) => void u.then((fn) => fn()));
    };
  }, [setState, setAudioLevel, setResult, setError, reloadHistory]);
}
