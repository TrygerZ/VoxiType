import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, X } from "lucide-react";

export function TopBar() {
  const win = getCurrentWindow();
  return (
    <div
      data-tauri-drag-region
      className="flex h-9 shrink-0 items-center justify-between border-b border-vx-border bg-vx-bg-secondary/70 px-3 vx-glass"
    >
      <div
        data-tauri-drag-region
        className="flex items-center gap-2 select-none"
      >
        <span className="h-2.5 w-2.5 rounded-full bg-gradient-to-br from-vx-accent to-vx-accent-hover" />
        <span className="text-xs font-semibold tracking-wide text-vx-text-secondary">
          VoxiType
        </span>
      </div>
      <div className="flex gap-1">
        <button
          type="button"
          onClick={() => void win.minimize()}
          className="flex h-6 w-6 items-center justify-center rounded-md text-vx-text-dim transition-colors hover:bg-vx-bg-tertiary hover:text-vx-text-primary"
          aria-label="Minimize"
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          onClick={() => void win.close()}
          className="flex h-6 w-6 items-center justify-center rounded-md text-vx-text-dim transition-colors hover:bg-vx-error hover:text-white"
          aria-label="Close"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  );
}
