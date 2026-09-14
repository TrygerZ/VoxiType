import { useCallback, useEffect, useRef, useState } from "react";
import { ackWidgetHide, onEvent, resetWidgetIdleTimer } from "../../lib/tauri";
import { useAppStore } from "../../stores/appStore";
import type {
  WidgetHideRequestedEvent,
  WidgetRevealRequestedEvent,
} from "../../types/events";
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

export function FloatingWidget({
  alwaysRender = false,
}: {
  alwaysRender?: boolean;
}) {
  const state = useAppStore((s) => s.state);
  const wordCount = useAppStore((s) => s.wordCount);
  const errorMessage = useAppStore((s) => s.errorMessage);
  const reset = useAppStore((s) => s.reset);

  const [phase, setPhase] = useState<WidgetPhase>("normal");
  const timersRef = useRef<ReturnType<typeof setTimeout>[]>([]);

  const clearAnimationTimers = useCallback(() => {
    for (const t of timersRef.current) {
      clearTimeout(t);
    }
    timersRef.current = [];
  }, []);

  const startReveal = useCallback(() => {
    clearAnimationTimers();
    if (prefersReducedMotion()) {
      setPhase("revealing-fade");
      const t = setTimeout(() => {
        setPhase("normal");
      }, REDUCED_MOTION_FADE_MS);
      timersRef.current.push(t);
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
        const t = setTimeout(() => {
          setPhase("hidden");
          void ackWidgetHide(hideId).catch((_err: unknown) => {});
        }, REDUCED_MOTION_FADE_MS);
        timersRef.current.push(t);
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

    onEvent<WidgetRevealRequestedEvent>(
      "floating_widget_reveal_requested",
      () => {
        startReveal();
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
    const t = setTimeout(() => reset(), 2200);
    return () => clearTimeout(t);
  }, [alwaysRender, showingResult, wordCount, reset]);

  if (!alwaysRender && state === "idle" && !errorMessage) return null;

  // Premium dynamic borders and ambient shadows matching HomeView.tsx
  const stateStyles = {
    idle: {
      accent: "text-vx-accent bg-vx-accent-soft",
      container:
        "border-vx-border/50 bg-vx-bg-secondary/70 shadow-vx-md hover:border-vx-accent/30",
    },
    recording: {
      accent:
        "text-vx-error bg-vx-error/15 shadow-[0_0_12px_rgba(204,139,125,0.2)]",
      container:
        "border-vx-error/30 bg-vx-bg-secondary/80 shadow-[0_0_24px_rgba(204,139,125,0.25)]",
    },
    processing: {
      accent: "text-vx-warning bg-vx-warning/15",
      container:
        "border-vx-warning/30 bg-vx-bg-secondary/80 shadow-[0_0_24px_rgba(212,182,133,0.25)]",
    },
    success: {
      accent: "text-vx-success bg-vx-success/15",
      container:
        "border-vx-success/30 bg-vx-bg-secondary/80 shadow-[0_0_24px_rgba(138,174,147,0.25)]",
    },
    error: {
      accent: "text-vx-error bg-vx-error/15",
      container:
        "border-vx-error/30 bg-vx-bg-secondary/80 shadow-[0_0_24px_rgba(204,139,125,0.25)]",
    },
  };

  const isSuccess = state === "idle" && wordCount !== null;
  const currentStyle = errorMessage
    ? stateStyles.error
    : isSuccess
      ? stateStyles.success
      : state === "recording"
        ? stateStyles.recording
        : state === "processing"
          ? stateStyles.processing
          : stateStyles.idle;

  // Select color and state configuration for the waveform bars
  let barClassName = "bg-vx-accent";
  let waveformActive = false;

  if (errorMessage || state === "error") {
    barClassName = "bg-vx-error";
  } else if (isSuccess) {
    barClassName = "bg-vx-success";
  } else if (state === "recording") {
    barClassName = "bg-vx-error";
    waveformActive = true;
  } else if (state === "processing") {
    barClassName = "bg-vx-warning animate-pulse";
    waveformActive = false;
  }

  const handlePointerActivity = () => {
    if (phase === "hiding-shrink" || phase === "hiding-fade") {
      cancelHide();
    }
    void resetWidgetIdleTimer().catch((_err: unknown) => {});
  };

  const isContentVisible =
    phase === "normal" || phase === "revealing-expand";

  const pillPhaseClass =
    phase === "normal" || phase === "revealing-expand"
      ? "vx-widget-pill--normal"
      : phase === "hiding-shrink" || phase === "revealing-fade"
        ? "vx-widget-pill--capsule"
        : "vx-widget-pill--hidden";

  return (
    <div
      data-tauri-drag-region
      data-testid="floating-widget-pill"
      data-animation-phase={phase}
      onPointerEnter={handlePointerActivity}
      onPointerDown={handlePointerActivity}
      className={`vx-widget-pill pointer-events-auto flex h-10 w-28 items-center gap-2.5 rounded-full border px-2.5 select-none ${pillPhaseClass} ${currentStyle.container}`}
    >
      {/* VoxiType Logo */}
      <div
        data-tauri-drag-region
        className={`vx-widget-content pointer-events-none flex h-6 w-6 shrink-0 items-center justify-center overflow-hidden rounded-full border border-vx-border/40 bg-vx-bg-tertiary ${
          isContentVisible
            ? "vx-widget-content--visible"
            : "vx-widget-content--hidden"
        }`}
      >
        <img
          data-tauri-drag-region
          src="/logo.png"
          alt="VoxiType Logo"
          className="h-full w-full object-contain"
          onError={(e) => {
            (e.target as HTMLImageElement).style.display = "none";
          }}
        />
      </div>

      {/* Audio strength indicator (Waveform or Bouncing Dots) */}
      <div
        data-tauri-drag-region
        className={`vx-widget-content pointer-events-none flex flex-1 items-center justify-center min-w-0 ${
          isContentVisible
            ? "vx-widget-content--visible"
            : "vx-widget-content--hidden"
        }`}
      >
        {state === "processing" ? (
          <div
            className="flex h-5 items-center justify-center gap-1.5"
            aria-hidden
          >
            <span className="h-1.5 w-1.5 rounded-full bg-vx-warning animate-bounce [animation-delay:-0.3s]" />
            <span className="h-1.5 w-1.5 rounded-full bg-vx-warning animate-bounce [animation-delay:-0.15s]" />
            <span className="h-1.5 w-1.5 rounded-full bg-vx-warning animate-bounce" />
          </div>
        ) : (
          <Waveform active={waveformActive} barClassName={barClassName} />
        )}
      </div>
    </div>
  );
}
