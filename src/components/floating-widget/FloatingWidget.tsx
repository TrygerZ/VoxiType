import { Square, X, Check, AlertCircle, Loader2, Mic, Settings } from "lucide-react";
import { useAppStore } from "../../stores/appStore";
import { cancelRecording, stopRecording } from "../../lib/tauri";
import { Waveform } from "./Waveform";
import { Timer } from "./Timer";

export function FloatingWidget({
  alwaysRender = false,
}: {
  alwaysRender?: boolean;
}) {
  const state = useAppStore((s) => s.state);
  const mode = useAppStore((s) => s.mode);
  const audioLevel = useAppStore((s) => s.audioLevel);
  const durationSec = useAppStore((s) => s.durationSec);
  const wordCount = useAppStore((s) => s.wordCount);
  const errorMessage = useAppStore((s) => s.errorMessage);

  if (!alwaysRender && state === "idle" && !errorMessage) return null;

  // Premium dynamic borders and ambient shadows matching HomeView.tsx
  const stateStyles = {
    idle: {
      accent: "text-vx-accent bg-vx-accent-soft",
      container: "border-vx-border/50 bg-vx-bg-secondary/70 shadow-vx-lg hover:border-vx-accent/30",
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

  return (
    <div
      data-tauri-drag-region
      className={`vx-scale-in pointer-events-auto flex h-12 w-auto min-w-[240px] max-w-[400px] items-center justify-between gap-4 rounded-full border px-3.5 py-1.5 backdrop-blur-xl select-none transition-all duration-300 ${currentStyle.container}`}
    >
      <div className="flex flex-1 items-center gap-3">
        {/* Leading Status Icon */}
        <div
          className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-full transition-colors duration-300 ${currentStyle.accent}`}
        >
          {state === "recording" && (
            <Mic className="h-4 w-4 animate-[vx-pulse_1.4s_ease-in-out_infinite]" />
          )}
          {state === "processing" && (
            <Loader2 className="h-4.5 w-4.5 animate-[vx-spin_1s_linear_infinite]" />
          )}
          {state === "error" && <AlertCircle className="h-4.5 w-4.5" />}
          {isSuccess && <Check className="h-4.5 w-4.5" />}
          {state === "idle" && wordCount === null && (
            <Mic className="h-4 w-4" />
          )}
        </div>

        {/* Center content (Dynamic) */}
        <div className="flex flex-1 flex-col justify-center min-w-0">
          {state === "recording" && (
            <div className="flex items-center gap-3">
              <div className="w-16">
                <Waveform level={audioLevel} active />
              </div>
              <Timer seconds={durationSec} />
            </div>
          )}

          {state === "processing" && (
            <span className="text-xs font-semibold tracking-wider uppercase text-vx-warning">
              Processing...
            </span>
          )}

          {isSuccess && (
            <span className="truncate text-xs font-medium text-vx-success">
              {wordCount} words inserted
            </span>
          )}

          {errorMessage && state === "error" && (
            <span className="truncate text-xs font-medium text-vx-error">
              {errorMessage}
            </span>
          )}

          {state === "idle" && wordCount === null && (
            <div className="flex items-center gap-1.5">
              <span className="text-[10px] font-semibold uppercase tracking-wider text-vx-text-dim">
                Mode:
              </span>
              <span className="rounded-full border border-vx-accent/30 bg-vx-accent-soft px-2 py-0.5 text-[9px] font-bold uppercase tracking-wider text-vx-accent">
                {mode}
              </span>
            </div>
          )}
        </div>
      </div>

      {/* Actions */}
      {state === "recording" ? (
        <div className="flex shrink-0 items-center gap-1.5 pl-2 border-l border-vx-border/40">
          <button
            type="button"
            onClick={() => void stopRecording()}
            className="flex h-8 w-8 items-center justify-center rounded-full bg-vx-error text-white shadow-sm transition-all hover:scale-105 hover:bg-vx-error/90 active:scale-95"
            title="Stop"
          >
            <Square className="h-3 w-3 fill-current" />
          </button>
          <button
            type="button"
            onClick={() => void cancelRecording()}
            className="flex h-8 w-8 items-center justify-center rounded-full bg-vx-bg-tertiary text-vx-text-secondary border border-vx-border/40 transition-all hover:bg-vx-bg-elevated hover:text-vx-text-primary hover:scale-105 active:scale-95"
            title="Cancel"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      ) : (
        state === "idle" && wordCount === null && (
          <div className="flex shrink-0 items-center pl-2 border-l border-vx-border/40 text-vx-text-dim">
            <Settings className="h-3.5 w-3.5" />
          </div>
        )
      )}
    </div>
  );
}
