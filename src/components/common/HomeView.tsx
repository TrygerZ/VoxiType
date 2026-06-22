import { Mic, Loader2 } from "lucide-react";
import { useAppStore } from "../../stores/appStore";
import { startRecording } from "../../lib/tauri";

export function HomeView() {
  const state = useAppStore((s) => s.state);
  const isRecording = state === "recording";
  const isProcessing = state === "processing";

  return (
    <div className="relative flex h-[80vh] flex-col items-center justify-center">
      {/* Zen Mode Mic — absolutely centered, minimal distraction */}
      <button
        type="button"
        onClick={() => void startRecording()}
        disabled={isRecording || isProcessing}
        className="group relative flex h-40 w-40 items-center justify-center disabled:cursor-default outline-none"
      >
        {/* Outer ambient glow */}
        <span
          className={`absolute inset-0 rounded-full transition-all duration-700 ease-out ${
            isRecording
              ? "bg-vx-error/5 scale-110 blur-xl"
              : isProcessing
                ? "bg-vx-warning/5 scale-100 blur-xl"
                : "bg-vx-accent/5 scale-90 blur-xl group-hover:scale-110 group-hover:bg-vx-accent/10"
          }`}
        />
        {/* Core button */}
        <span
          className={`relative flex h-24 w-24 items-center justify-center rounded-full transition-all duration-500 ease-out ${
            isRecording
              ? "bg-vx-error/10 scale-105 shadow-[0_0_40px_rgba(204,139,125,0.2)]"
              : isProcessing
                ? "bg-vx-warning/10"
                : "bg-vx-bg-tertiary shadow-vx-lg group-hover:scale-105"
          }`}
        >
          {isProcessing ? (
            <Loader2 className="h-8 w-8 animate-[vx-spin_1.5s_linear_infinite] text-vx-warning" />
          ) : (
            <Mic
              className={`h-8 w-8 transition-colors duration-300 ${
                isRecording
                  ? "text-vx-error animate-[vx-pulse_1.4s_ease-in-out_infinite]"
                  : "text-vx-text-secondary group-hover:text-vx-accent"
              }`}
            />
          )}
        </span>
      </button>

      {/* Only show text when actively doing something (recording/processing) */}
      <div
        className={`absolute bottom-[20%] flex flex-col items-center transition-all duration-500 ${
          isRecording || isProcessing
            ? "opacity-100 translate-y-0"
            : "opacity-0 translate-y-4 pointer-events-none"
        }`}
      >
        <span
          className={`text-sm font-medium tracking-wide uppercase ${
            isRecording ? "text-vx-error animate-pulse" : "text-vx-warning"
          }`}
        >
          {isRecording ? "Listening..." : "Processing..."}
        </span>
      </div>
    </div>
  );
}
