import { type SyntheticEvent, useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, Check } from "lucide-react";

import { useT } from "../../lib/i18n";
import { getHotkeySetting } from "../../lib/settingsGuards";
import {
  ackWidgetHide,
  ackWidgetReveal,
  onEvent,
  resetWidgetIdleTimer,
} from "../../lib/tauri";
import { usePlatform } from "../../lib/usePlatform";
import { useAppStore } from "../../stores/appStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type {
  WidgetHideRequestedEvent,
  WidgetRevealRequestedEvent,
} from "../../types/events";
import { formatAccelerator } from "../settings/HotkeyRecorder";
import { Waveform } from "./Waveform";

export type WidgetPhase =
  | "normal"
  | "hiding-shrink"
  | "hiding-fade"
  | "hidden"
  | "revealing-fade"
  | "revealing-expand";

const HIDE_SHRINK_MS = 200;
const HIDE_FADE_MS = 150;
const REVEAL_FADE_MS = 120;
const REVEAL_EXPAND_MS = 200;
const REDUCED_MOTION_FADE_MS = 120;

const prefersReducedMotion = (): boolean =>
  typeof window !== "undefined" &&
  Boolean(window.matchMedia?.("(prefers-reduced-motion: reduce)").matches);

function isWidgetRevealRequestedEvent(
  payload: unknown,
): payload is WidgetRevealRequestedEvent {
  return (
    typeof payload === "object" &&
    payload !== null &&
    "id" in payload &&
    typeof payload.id === "number" &&
    Number.isFinite(payload.id)
  );
}

function formatDuration(seconds: number): string {
  const safe = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
  const mins = Math.floor(safe / 60);
  const secs = safe % 60;
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

function hideBrokenLogo(event: SyntheticEvent<HTMLImageElement>) {
  event.currentTarget.style.display = "none";
}

export function FloatingWidget({
  alwaysRender = false,
}: {
  alwaysRender?: boolean;
}) {
  const t = useT();
  const state = useAppStore((s) => s.state);
  const durationSec = useAppStore((s) => s.durationSec);
  const wordCount = useAppStore((s) => s.wordCount);
  const errorMessage = useAppStore((s) => s.errorMessage);
  const outcome = useAppStore((s) => s.outcome);
  const reset = useAppStore((s) => s.reset);
  const rawHotkey = useSettingsStore((s) => s.settings.hotkey);
  const platform = usePlatform();

  const [phase, setPhase] = useState<WidgetPhase>("normal");
  const timersRef = useRef<ReturnType<typeof setTimeout>[]>([]);

  const clearAnimationTimers = useCallback(() => {
    for (const timer of timersRef.current) {
      clearTimeout(timer);
    }
    timersRef.current = [];
  }, []);

  const startReveal = useCallback(() => {
    clearAnimationTimers();
    if (prefersReducedMotion()) {
      setPhase("revealing-fade");
      const timer = setTimeout(() => {
        setPhase("normal");
      }, REDUCED_MOTION_FADE_MS);
      timersRef.current.push(timer);
      return;
    }

    setPhase("revealing-fade");
    const t1 = setTimeout(() => {
      setPhase("revealing-expand");
      const t2 = setTimeout(() => {
        setPhase("normal");
      }, REVEAL_EXPAND_MS);
      timersRef.current.push(t2);
    }, REVEAL_FADE_MS);
    timersRef.current.push(t1);
  }, [clearAnimationTimers]);

  const startHide = useCallback(
    (hideId: number) => {
      clearAnimationTimers();

      if (prefersReducedMotion()) {
        setPhase("hiding-fade");
        const timer = setTimeout(() => {
          setPhase("hidden");
          void ackWidgetHide(hideId).catch((_err: unknown) => {});
        }, REDUCED_MOTION_FADE_MS);
        timersRef.current.push(timer);
        return;
      }

      setPhase("hiding-shrink");
      const t1 = setTimeout(() => {
        setPhase("hiding-fade");
        const t2 = setTimeout(() => {
          setPhase("hidden");
          void ackWidgetHide(hideId).catch((_err: unknown) => {});
        }, HIDE_FADE_MS);
        timersRef.current.push(t2);
      }, HIDE_SHRINK_MS);
      timersRef.current.push(t1);
    },
    [clearAnimationTimers],
  );

  const cancelHide = useCallback(() => {
    clearAnimationTimers();
    setPhase("normal");
  }, [clearAnimationTimers]);

  // Subscribe to backend hide/reveal events.
  useEffect(() => {
    let disposed = false;
    const unlisteners: (() => void)[] = [];

    onEvent<WidgetHideRequestedEvent>(
      "floating_widget_hide_requested",
      (e) => {
        startHide(e.id);
      },
    )
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlisteners.push(unlisten);
        }
      })
      .catch((_err: unknown) => {});

    onEvent<unknown>("floating_widget_reveal_requested", (payload) => {
      if (!isWidgetRevealRequestedEvent(payload)) return;
      // ACK immediately: the backend reloads the webview when no ACK arrives
      // within 1500 ms, so this must not wait for the reveal animation.
      void ackWidgetReveal(payload.id).catch((err: unknown) => {
        console.error("Failed to acknowledge widget reveal:", err);
      });
      startReveal();
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlisteners.push(unlisten);
        }
      })
      .catch((_err: unknown) => {});

    return () => {
      disposed = true;
      clearAnimationTimers();
      for (const fn of unlisteners) {
        fn();
      }
      unlisteners.length = 0;
    };
  }, [startHide, startReveal, clearAnimationTimers]);

  // Recording start acts as an immediate reveal/cancel safeguard.
  useEffect(() => {
    if (state === "recording") {
      if (phase === "hidden") {
        startReveal();
      } else if (phase === "hiding-shrink" || phase === "hiding-fade") {
        cancelHide();
      }
    }
  }, [state, phase, startReveal, cancelHide]);

  // When persistently visible, the window is no longer hidden after a result,
  // so revert the success/error flash back to the idle pill ourselves.
  const showingResult =
    (state === "idle" && wordCount !== null) || state === "error";
  useEffect(() => {
    if (!alwaysRender || !showingResult) return;
    const timer = setTimeout(() => reset(), 2200);
    return () => clearTimeout(timer);
  }, [alwaysRender, showingResult, wordCount, reset]);

  if (!alwaysRender && state === "idle" && !errorMessage) return null;

  const isSuccess = state === "idle" && wordCount !== null;
  const reachedTarget = outcome?.injected !== false;
  const isErrorOrMissed =
    Boolean(errorMessage) || state === "error" || (isSuccess && !reachedTarget);
  const isInjectedSuccess = isSuccess && reachedTarget;

  const handlePointerActivity = () => {
    if (phase === "hiding-shrink" || phase === "hiding-fade") {
      cancelHide();
    }
    void resetWidgetIdleTimer().catch((_err: unknown) => {});
  };

  const isCapsuleOrHidden =
    phase === "hiding-shrink" ||
    phase === "hiding-fade" ||
    phase === "hidden" ||
    phase === "revealing-fade";

  const pillPhaseClass =
    phase === "normal" || phase === "revealing-expand"
      ? "vx-widget-pill--normal"
      : phase === "hiding-shrink" || phase === "revealing-fade"
        ? "vx-widget-pill--capsule"
        : "vx-widget-pill--hidden";

  const isContentVisible =
    phase === "normal" || phase === "revealing-expand";

  const stateBorderClass = isCapsuleOrHidden
    ? "border-vx-border-strong justify-center px-2"
    : isErrorOrMissed
      ? "border-vx-semantic-error px-3 gap-2"
      : isInjectedSuccess
        ? "border-vx-semantic-success px-3 gap-2"
        : state === "recording"
          ? "border-vx-accent-ember px-2.5 gap-2"
          : "border-vx-border-default hover:border-vx-border-strong px-2.5 gap-2";

  const hotkey = getHotkeySetting(rawHotkey, {
    key: "Ctrl+Space",
    mode: "ptt",
  });
  const hotkeyLabel = formatAccelerator(hotkey.key, platform)
    .split("+")
    .join(" + ");

  return (
    <div
      data-tauri-drag-region
      data-testid="floating-widget-pill"
      data-animation-phase={phase}
      onPointerEnter={handlePointerActivity}
      onPointerDown={handlePointerActivity}
      className={`vx-widget-pill pointer-events-auto flex items-center rounded-full border bg-vx-bg-elevated shadow-vx-widget select-none ${pillPhaseClass} ${stateBorderClass}`}
    >
      {isCapsuleOrHidden ? (
        <div
          data-tauri-drag-region
          className="pointer-events-none flex h-[18px] w-[18px] shrink-0 items-center justify-center"
        >
          <img
            data-tauri-drag-region
            src="/logo.png"
            alt="VoxiType Logo"
            className="h-[18px] w-[18px] object-contain"
            onError={hideBrokenLogo}
          />
        </div>
      ) : (
        <div
          data-tauri-drag-region
          className={`vx-widget-content pointer-events-none flex min-w-0 items-center gap-2 ${
            isContentVisible
              ? "vx-widget-content--visible"
              : "vx-widget-content--hidden"
          }`}
        >
          {isErrorOrMissed ? (
            <>
              <AlertTriangle
                className="h-3.5 w-3.5 shrink-0 text-vx-semantic-error"
                strokeWidth={1.75}
              />
              <span className="truncate whitespace-nowrap text-[11px] font-medium text-vx-text-primary">
                {t("widget.copied_fallback")}
              </span>
            </>
          ) : isInjectedSuccess ? (
            <>
              <Check
                className="h-3.5 w-3.5 shrink-0 text-vx-semantic-success"
                strokeWidth={2}
              />
              <span className="whitespace-nowrap text-[11px] font-medium text-vx-text-primary">
                {t("widget.injected_words", { count: wordCount ?? 0 })}
              </span>
            </>
          ) : state === "processing" ? (
            <>
              <div className="flex h-4 items-center gap-[3px]" aria-hidden>
                <span className="h-2 w-[2.5px] rounded-full bg-vx-accent-ember animate-pulse" />
                <span className="h-3.5 w-[2.5px] rounded-full bg-vx-accent-ember animate-pulse [animation-delay:150ms]" />
                <span className="h-2 w-[2.5px] rounded-full bg-vx-accent-ember animate-pulse [animation-delay:300ms]" />
              </div>
              <span className="whitespace-nowrap text-[11px] font-medium text-vx-text-primary">
                {t("widget.formatting")}
              </span>
            </>
          ) : state === "recording" ? (
            <>
              <div
                data-tauri-drag-region
                className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-vx-bg-subtle"
              >
                <img
                  data-tauri-drag-region
                  src="/logo.png"
                  alt="VoxiType Logo"
                  className="h-5 w-5 object-contain"
                  onError={hideBrokenLogo}
                />
              </div>
              <div className="flex w-11 items-center">
                <Waveform
                  active
                  barCount={12}
                  barClassName="bg-vx-accent-ember"
                />
              </div>
              <span className="font-mono text-[11px] font-medium tabular-nums text-vx-accent-ember">
                {formatDuration(durationSec)}
              </span>
            </>
          ) : (
            <>
              <div
                data-tauri-drag-region
                className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-vx-bg-subtle"
              >
                <img
                  data-tauri-drag-region
                  src="/logo.png"
                  alt="VoxiType Logo"
                  className="h-5 w-5 object-contain"
                  onError={hideBrokenLogo}
                />
              </div>
              <span className="whitespace-nowrap font-mono text-[11px] font-medium text-vx-text-secondary">
                {hotkeyLabel}
              </span>
            </>
          )}
        </div>
      )}
    </div>
  );
}
