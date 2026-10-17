import { useEffect, useRef } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { onEvent } from "../lib/tauri";
import { useAppStore } from "../stores/appStore";
import { useHistoryStore } from "../stores/historyStore";
import { useStatsStore } from "../stores/statsStore";
import { t } from "../lib/i18n";
import { toast } from "../stores/toastStore";
import { errorCopy } from "../lib/errorCopy";
import { navigate } from "../lib/navigate";
import type {
  StateChangedEvent,
  TranscriptionCompleteEvent,
  TranscriptionErrorEvent,
} from "../types/events";

export interface UseTauriEventsOptions {
  reloadData?: boolean;
}

/**
 * Subscribe to all backend events and reflect them into the app store.
 * Mount once near the app root.
 */
export function useTauriEvents(options: UseTauriEventsOptions = {}) {
  const { reloadData = true } = options;
  const setState = useAppStore((s) => s.setState);
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
      onEvent<TranscriptionCompleteEvent>("transcription_complete", (p) => {
        setResult(p.word_count, {
          injected: p.injected !== false,
          appContext: p.app_context ?? null,
        });
        if (reloadData) {
          void reloadHistory();
          void reloadStats();
          // The text never reached the target app: say so instead of showing
          // a plain success, and offer a one-click copy of the transcript.
          if (p.injected === false && p.text) {
            toast(t("notice.saved_to_history"), "info", {
              label: t("notice.copy_text"),
              onClick: () => void navigator.clipboard.writeText(p.text),
            });
          }
        }
      }),
    );
    unlisteners.push(
      onEvent<TranscriptionErrorEvent>("transcription_error", (p) => {
        setError(p.message);
        if (!reloadData) return;
        // Name the cause in user language and, when it has one, offer the
        // settings screen that fixes it.
        const copy = errorCopy(p.code, p.message);
        toast(copy.message, "error", {
          label: t("notice.open_settings"),
          onClick: () => navigate("settings", copy.settingsTab),
        });
      }),
    );

    return () => {
      clearTimer();
      // Unlisten independently: one rejected registration must not stop the
      // siblings that did register from being torn down (leak on unmount).
      unlisteners.forEach((p) => p.then((fn) => fn()).catch(() => {}));
    };
  }, [
    setState,
    setDuration,
    setResult,
    setError,
    reloadHistory,
    reloadStats,
    reloadData,
  ]);
}
