import type { AppStateEnum } from "../../types/app";

const labelByState: Record<AppStateEnum, string> = {
  idle: "Idle",
  recording: "Recording",
  processing: "Processing",
  error: "Error",
};

const colorByState: Record<AppStateEnum, string> = {
  idle: "bg-vx-text-dim",
  recording: "bg-vx-error",
  processing: "bg-vx-warning",
  error: "bg-vx-error",
};

export function StatusIndicator({ state }: { state: AppStateEnum }) {
  const pulse = state === "recording" || state === "processing";
  return (
    <span className="inline-flex items-center gap-2">
      <span
        className={`h-2 w-2 rounded-full ${colorByState[state]} ${
          pulse ? "animate-[vx-pulse_1.5s_ease-in-out_infinite]" : ""
        }`}
      />
      <span className="text-xs font-medium text-vx-text-secondary">
        {labelByState[state]}
      </span>
    </span>
  );
}
