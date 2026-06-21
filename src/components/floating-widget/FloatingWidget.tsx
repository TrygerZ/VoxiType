import { Square, X, Check, AlertCircle, Loader2, Mic } from "lucide-react";
import { useAppStore } from "../../stores/appStore";
import { cancelRecording, stopRecording } from "../../lib/tauri";
import { Waveform } from "./Waveform";
import { Timer } from "./Timer";
import { ModeLabel } from "./ModeLabel";

export function FloatingWidget({
  alwaysRender = false,
}: {
  alwaysRender?: boolean;
}) {
  const state = useAppStore((s) => s.state);
  const audioLevel = useAppStore((s) => s.audioLevel);
  const durationSec = useAppStore((s) => s.durationSec);
  const mode = useAppStore((s) => s.mode);
  const wordCount = useAppStore((s) => s.wordCount);
  const errorMessage = useAppStore((s) => s.errorMessage);

  if (!alwaysRender && state === "idle" && !errorMessage) return null;

  const accent =
    state === "recording"
      ? "text-vx-error"
      : state === "processing"
        ? "text-vx-warning"
        : state === "error"
          ? "text-vx-error"
          : "text-vx-success";

  return (
    <div
      data-tauri-drag-region
      className="vx-glass vx-scale-in pointer-events-auto flex w-full items-center gap-3 rounded-2xl border border-vx-border-strong/60 px-3.5 py-2.5 shadow-vx-lg select-none"
    >
      {/* Leading status icon */}
      <div
        className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-vx-bg-tertiary/80 ${accent}`}
      >
        {state === "recording" && (
          <Mic className="h-4.5 w-4.5 animate-[vx-pulse_1.4s_ease-in-out_infinite]" />
        )}
        {state === "processing" && (
          <Loader2 className="h-4.5 w-4.5 animate-[vx-spin_1s_linear_infinite]" />
        )}
        {state === "error" && <AlertCircle className="h-4.5 w-4.5" />}
        {(state === "idle" || (!state && true)) &&
          wordCount !== null &&
          state === "idle" && <Check className="h-4.5 w-4.5" />}
        {state === "idle" && wordCount === null && (
          <Mic className="h-4.5 w-4.5 text-vx-text-dim" />
        )}
      </div>

      {/* Center content */}
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div className="flex items-center justify-between gap-2">
          <span className="text-xs font-semibold text-vx-text-primary">
            {state === "recording"
              ? "Recording"
              : state === "processing"
                ? "Processing"
                : state === "error"
                  ? "Error"
                  : wordCount !== null
                    ? "Done"
                    : "Ready"}
          </span>
          <ModeLabel mode={mode} />
        </div>

        {state === "recording" && (
          <div className="flex items-center gap-2">
            <Waveform level={audioLevel} active />
            <Timer seconds={durationSec} />
          </div>
        )}

        {state === "processing" && (
          <span className="text-xs text-vx-text-secondary">
            Transcribing &amp; formatting...
          </span>
        )}

        {wordCount !== null && state === "idle" && (
          <span className="text-xs text-vx-success">{wordCount} words inserted</span>
        )}

        {errorMessage && state === "error" && (
          <span className="truncate text-xs text-vx-error">{errorMessage}</span>
        )}
      </div>

      {/* Actions */}
      {state === "recording" && (
        <div className="flex shrink-0 gap-1.5">
          <button
            type="button"
            onClick={() => void stopRecording()}
            className="flex h-8 w-8 items-center justify-center rounded-lg bg-vx-error text-white transition-transform hover:scale-105 active:scale-95"
            title="Stop"
          >
            <Square className="h-3.5 w-3.5 fill-current" />
          </button>
          <button
            type="button"
            onClick={() => void cancelRecording()}
            className="flex h-8 w-8 items-center justify-center rounded-lg bg-vx-bg-tertiary text-vx-text-secondary transition-colors hover:bg-vx-bg-elevated hover:text-vx-text-primary"
            title="Cancel"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      )}
    </div>
  );
}
