import { Mic, Loader2, Keyboard } from "lucide-react";
import { useAppStore } from "../../stores/appStore";
import { startRecording } from "../../lib/tauri";
import { useT } from "../../lib/i18n";

export function HomeView() {
  const t = useT();
  const state = useAppStore((s) => s.state);
  const isRecording = state === "recording";
  const isProcessing = state === "processing";

  return (
    <div className="relative flex h-full flex-col items-center justify-center gap-8 p-8">
      {/* Mic button with status-aware halo */}
      <div className="relative flex items-center justify-center">
        {isRecording && (
          <>
            <span className="absolute h-44 w-44 animate-ping rounded-full bg-vx-error/10" />
            <span className="absolute h-32 w-32 rounded-full bg-vx-error/20 blur-xl" />
          </>
        )}
        {state === "idle" && (
          <span className="absolute h-40 w-40 rounded-full bg-vx-accent/10 blur-2xl" />
        )}
        <button
          type="button"
          onClick={() => void startRecording()}
          disabled={isRecording || isProcessing}
          className={`group relative flex h-28 w-28 items-center justify-center rounded-full border transition-all duration-300 disabled:cursor-default ${
            isRecording
              ? "border-vx-error/40 bg-vx-error/15"
              : isProcessing
                ? "border-vx-warning/40 bg-vx-warning/10"
                : "border-vx-border-strong bg-vx-bg-secondary hover:scale-105 hover:border-vx-accent/50 hover:bg-vx-accent/10"
          }`}
        >
          {isProcessing ? (
            <Loader2 className="h-11 w-11 animate-[vx-spin_1s_linear_infinite] text-vx-warning" />
          ) : (
            <Mic
              className={`h-11 w-11 transition-transform duration-300 ${
                isRecording
                  ? "text-vx-error animate-[vx-pulse_1.4s_ease-in-out_infinite]"
                  : "text-vx-accent group-hover:scale-110"
              }`}
            />
          )}
        </button>
      </div>

      {/* Status text */}
      <div className="flex flex-col items-center gap-2 text-center">
        {state === "idle" && (
          <>
            <h2 className="text-xl font-semibold tracking-tight">
              Ready to dictate
            </h2>
            <p className="flex items-center gap-1.5 text-sm text-vx-text-secondary">
              <Keyboard className="h-4 w-4" />
              {t("idle.hint")}
            </p>
          </>
        )}
        {isRecording && (
          <h2 className="text-xl font-semibold tracking-tight text-vx-error">
            {t("recording")}
          </h2>
        )}
        {isProcessing && (
          <h2 className="text-xl font-semibold tracking-tight text-vx-warning">
            {t("processing")}
          </h2>
        )}
      </div>

      {/* Hint chips */}
      {state === "idle" && (
        <div className="flex flex-wrap items-center justify-center gap-2">
          {["Dictation", "Message", "Email"].map((m) => (
            <span
              key={m}
              className="rounded-full border border-vx-border bg-vx-bg-secondary/70 px-3 py-1 text-xs text-vx-text-secondary"
            >
              {m}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
