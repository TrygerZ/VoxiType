import { useEffect } from "react";
import { useAppStore } from "../../stores/appStore";
import { Waveform } from "./Waveform";

export function FloatingWidget({
  alwaysRender = false,
}: {
  alwaysRender?: boolean;
}) {
  const state = useAppStore((s) => s.state);
  const audioLevel = useAppStore((s) => s.audioLevel);
  const wordCount = useAppStore((s) => s.wordCount);
  const errorMessage = useAppStore((s) => s.errorMessage);
  const reset = useAppStore((s) => s.reset);

  // When persistently visible, the window is no longer hidden after a result,
  // so revert the success/error flash back to the idle pill ourselves.
  const showingResult =
    (state === "idle" && wordCount !== null) || state === "error";
  useEffect(() => {
    if (!alwaysRender || !showingResult) return;
    const t = setTimeout(() => reset(), 2200);
    return () => clearTimeout(t);
  }, [alwaysRender, showingResult, reset]);

  if (!alwaysRender && state === "idle" && !errorMessage) return null;

  // Premium dynamic borders and ambient shadows matching HomeView.tsx
  const stateStyles = {
    idle: {
      accent: "text-vx-accent bg-vx-accent-soft",
      container: "border-vx-border/50 bg-vx-bg-secondary/70 shadow-vx-md hover:border-vx-accent/30",
    },
    recording: {
      accent: "text-vx-error bg-vx-error/15 shadow-[0_0_12px_rgba(204,139,125,0.2)]",
      container: "border-vx-error/30 bg-vx-bg-secondary/80 shadow-[0_0_24px_rgba(204,139,125,0.25)]",
    },
    processing: {
      accent: "text-vx-warning bg-vx-warning/15",
      container: "border-vx-warning/30 bg-vx-bg-secondary/80 shadow-[0_0_24px_rgba(212,182,133,0.25)]",
    },
    success: {
      accent: "text-vx-success bg-vx-success/15",
      container: "border-vx-success/30 bg-vx-bg-secondary/80 shadow-[0_0_24px_rgba(138,174,147,0.25)]",
    },
    error: {
      accent: "text-vx-error bg-vx-error/15",
      container: "border-vx-error/30 bg-vx-bg-secondary/80 shadow-[0_0_24px_rgba(204,139,125,0.25)]",
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
  let displayLevel = audioLevel;

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
    displayLevel = 0.0;
  }

  return (
    <div
      data-tauri-drag-region
      className={`vx-scale-in pointer-events-auto flex h-10 w-28 items-center gap-2.5 rounded-full border px-2.5 select-none transition-all duration-300 ${currentStyle.container}`}
    >
      {/* VoxiType Logo */}
      <div
        data-tauri-drag-region
        className="pointer-events-none flex h-6 w-6 shrink-0 items-center justify-center overflow-hidden rounded-full border border-vx-border/40 bg-vx-bg-tertiary"
      >
        <img
          data-tauri-drag-region
          src="/logo.png"
          alt="VoxiType Logo"
          className="h-full w-full object-contain"
          onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
        />
      </div>

      {/* Audio strength indicator (Waveform or Bouncing Dots) */}
      <div
        data-tauri-drag-region
        className="pointer-events-none flex flex-1 items-center justify-center min-w-0"
      >
        {state === "processing" ? (
          <div className="flex h-5 items-center justify-center gap-1.5" aria-hidden>
            <span className="h-1.5 w-1.5 rounded-full bg-vx-warning animate-bounce [animation-delay:-0.3s]" />
            <span className="h-1.5 w-1.5 rounded-full bg-vx-warning animate-bounce [animation-delay:-0.15s]" />
            <span className="h-1.5 w-1.5 rounded-full bg-vx-warning animate-bounce" />
          </div>
        ) : (
          <Waveform
            level={displayLevel}
            active={waveformActive}
            barClassName={barClassName}
          />
        )}
      </div>
    </div>
  );
}
