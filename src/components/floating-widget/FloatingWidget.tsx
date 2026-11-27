import { type SyntheticEvent, useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, Check } from "lucide-react";

import { useT } from "../../lib/i18n";
import {
  ackWidgetHide,
  ackWidgetReveal,
  onEvent,
  resetWidgetIdleTimer,
} from "../../lib/tauri";
import { useAppStore } from "../../stores/appStore";
import type {
  WidgetHideRequestedEvent,
  WidgetRevealRequestedEvent,
} from "../../types/events";
import { Waveform } from "./Waveform";

export type WidgetPhase =
  | "normal"
  | "hiding-content"
  | "hiding-shrink"
  | "hiding-fade"
  | "hidden"
  | "revealing-fade"
  | "revealing-expand";

const HIDE_CONTENT_MS = 100;
const HIDE_SHRINK_MS = 220; // 160ms capsule morph + 60ms micro-hold
const HIDE_FADE_MS = 220;
const REVEAL_FADE_MS = 150;
const REVEAL_EXPAND_MS = 260;
const REDUCED_MOTION_FADE_MS = 120;
const DORMANT_BAR_HEIGHTS = [4, 7, 10, 7, 4] as const;

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

function isHidingPhase(phase: WidgetPhase): boolean {
  return (
    phase === "hiding-content" ||
    phase === "hiding-shrink" ||
    phase === "hiding-fade"
  );
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

      setPhase("hiding-content");
      const t1 = setTimeout(() => {
        setPhase("hiding-shrink");
        const t2 = setTimeout(() => {
          setPhase("hiding-fade");
          const t3 = setTimeout(() => {
            setPhase("hidden");
            void ackWidgetHide(hideId).catch((_err: unknown) => {});
          }, HIDE_FADE_MS);
          timersRef.current.push(t3);
        }, HIDE_SHRINK_MS);
        timersRef.current.push(t2);
      }, HIDE_CONTENT_MS);
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
      } else if (isHidingPhase(phase)) {
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
    if (isHidingPhase(phase)) {
      cancelHide();
    }
    void resetWidgetIdleTimer().catch((_err: unknown) => {});
  };

  const isCapsuleOrHidden =
    phase === "hiding-shrink" ||
    phase === "hiding-fade" ||
    phase === "hidden" ||
    phase === "revealing-fade";

  const activeStateSizeClass = isErrorOrMissed
    ? "vx-widget-pill--error"
    : isInjectedSuccess
      ? "vx-widget-pill--success"
      : state === "processing"
        ? "vx-widget-pill--processing"
        : state === "recording"
          ? "vx-widget-pill--recording"
          : "vx-widget-pill--idle";

  const pillPhaseClass =
    phase === "hiding-shrink" || phase === "revealing-fade"
      ? "vx-widget-pill--capsule"
      : phase === "hiding-fade" || phase === "hidden"
        ? "vx-widget-pill--hidden"
        : activeStateSizeClass;

  const useBloomTiming =
    phase === "revealing-expand" ||
    (!isCapsuleOrHidden && state === "recording");

  const isContentVisible =
    phase === "normal" || phase === "revealing-expand";

  const stateBorderClass = isCapsuleOrHidden
    ? "border-vx-border-strong justify-center px-2"
    : isErrorOrMissed
      ? "border-vx-semantic-error px-3 gap-[7px]"
      : isInjectedSuccess
        ? "border-vx-semantic-success px-3 gap-[7px]"
        : state === "processing"
          ? "border-vx-border-strong px-3 gap-2"
          : state === "recording"
            ? "border-[1.5px] border-vx-accent-ember px-2.5 gap-2"
            : "border-vx-border-default hover:border-vx-border-strong px-2.5 gap-2";

  return (
    <div
      data-tauri-drag-region
      data-testid="floating-widget-pill"
      data-animation-phase={phase}
      onPointerEnter={handlePointerActivity}
      onPointerDown={handlePointerActivity}
      className={`vx-widget-pill ${useBloomTiming ? "vx-widget-pill--bloom" : ""} pointer-events-auto flex items-center justify-center overflow-hidden rounded-full border bg-vx-bg-elevated shadow-vx-widget select-none ${pillPhaseClass} ${stateBorderClass}`}
    >
      {isCapsuleOrHidden ? (
        <div
          data-tauri-drag-region
          className="pointer-events-none flex h-3.5 w-3.5 shrink-0 items-center justify-center"
        >
          <img
            data-tauri-drag-region
            src="/logo.png"
            alt="VoxiType Logo"
            className="h-3.5 w-3.5 object-contain"
            onError={hideBrokenLogo}
          />
        </div>
      ) : (
        <div
          data-tauri-drag-region
          className={`vx-widget-content pointer-events-none flex min-w-0 items-center justify-center gap-2 ${
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
                {t("widget.saved_to_history")}
              </span>
            </>
          ) : isInjectedSuccess ? (
            <>
              <Check
                className="h-3.5 w-3.5 shrink-0 text-vx-semantic-success"
                strokeWidth={2}
              />
              <span className="whitespace-nowrap text-[11px] font-medium text-vx-text-primary">
                {t("widget.words", { count: wordCount ?? 0 })}
              </span>
            </>
          ) : state === "processing" ? (
            <>
              <div className="flex h-3.5 items-center gap-[3px]" aria-hidden>
                <span className="h-[7px] w-[2.5px] rounded-full bg-vx-accent-ember animate-pulse" />
                <span className="h-[13px] w-[2.5px] rounded-full bg-vx-accent-ember animate-pulse [animation-delay:150ms]" />
                <span className="h-[7px] w-[2.5px] rounded-full bg-vx-accent-ember animate-pulse [animation-delay:300ms]" />
              </div>
              <span className="whitespace-nowrap text-[11px] font-medium text-vx-text-primary">
                {t("widget.formatting")}
              </span>
            </>
          ) : state === "recording" ? (
            <>
              <span
                aria-hidden="true"
                className="flex h-[18px] w-[18px] shrink-0 items-center justify-center rounded-full bg-vx-accent-ember-soft"
              >
                <span className="h-2 w-2 rounded-full bg-vx-accent-ember vx-mic-pulse" />
              </span>
              <div className="flex w-[76px] items-center">
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
              <img
                data-tauri-drag-region
                src="/logo.png"
                alt="VoxiType Logo"
                className="h-4 w-4 shrink-0 object-contain"
                onError={hideBrokenLogo}
              />
              <div
                data-testid="widget-dormant-bars"
                aria-hidden="true"
                className="flex h-3 items-center gap-[2.5px]"
              >
                {DORMANT_BAR_HEIGHTS.map((height, idx) => (
                  <span
                    key={idx}
                    className="w-[2px] rounded-full bg-vx-border-strong"
                    style={{ height: `${height}px` }}
                  />
                ))}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
