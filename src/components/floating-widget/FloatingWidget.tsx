import { Mic, Square, X, Check, AlertCircle, Loader2 } from "lucide-react";
import { useAppStore } from "../../stores/appStore";
import { cancelRecording, stopRecording } from "../../lib/tauri";
import { Waveform } from "./Waveform";
import { StatusIndicator } from "./StatusIndicator";
import { Timer } from "./Timer";
import { ModeLabel } from "./ModeLabel";

export function FloatingWidget() {
  const state = useAppStore((s) => s.state);
  const audioLevel = useAppStore((s) => s.audioLevel);
  const durationSec = useAppStore((s) => s.durationSec);
  const mode = useAppStore((s) => s.mode);
  const wordCount = useAppStore((s) => s.wordCount);
  const errorMessage = useAppStore((s) => s.errorMessage);

  if (state === "idle" && !errorMessage) return null;

  return (
    <div
      data-tauri-drag-region
      className="pointer-events-auto flex w-72 select-none items-center gap-3 rounded-xl border border-vx-border bg-vx-bg-secondary/90 p-3 shadow-lg backdrop-blur-md"
    >
      <div className="flex flex-1 flex-col gap-1.5">
        <div className="flex items-center justify-between">
          <StatusIndicator state={state} />
          <ModeLabel mode={mode} />
        </div>

        {state === "recording" && (
          <div className="flex items-center gap-2">
            <Waveform level={audioLevel} active />
            <Timer seconds={durationSec} />
          </div>
        )}

        {state === "processing" && (
          <div className="flex items-center gap-2 text-sm text-vx-text-secondary">
            <Loader2 className="h-4 w-4 animate-[vx-spin_1s_linear_infinite]" />
            <span>Processing...</span>
          </div>
        )}

        {wordCount !== null && state === "idle" && (
          <div className="flex items-center gap-2 text-sm text-vx-success">
            <Check className="h-4 w-4" />
            <span>{wordCount} words</span>
          </div>
        )}

        {errorMessage && (
          <div className="flex items-center gap-2 text-sm text-vx-error">
            <AlertCircle className="h-4 w-4" />
            <span className="truncate">{errorMessage}</span>
          </div>
        )}
      </div>

      {state === "recording" && (
        <div className="flex flex-col gap-1.5">
          <button
            type="button"
            onClick={() => void stopRecording()}
            className="rounded-md bg-vx-error p-1.5 text-white hover:opacity-90"
            title="Stop"
          >
            <Square className="h-4 w-4" />
          </button>
          <button
            type="button"
            onClick={() => void cancelRecording()}
            className="rounded-md bg-vx-bg-tertiary p-1.5 text-vx-text-secondary hover:bg-vx-border"
            title="Cancel"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      )}

      {state === "idle" && !errorMessage && (
        <Mic className="h-5 w-5 text-vx-text-dim" />
      )}
    </div>
  );
}
