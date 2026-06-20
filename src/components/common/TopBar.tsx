import { getCurrentWindow } from "@tauri-apps/api/window";

export function TopBar() {
  const win = getCurrentWindow();
  return (
    <div
      data-tauri-drag-region
      className="flex h-8 items-center justify-between border-b border-vx-border bg-vx-bg-secondary px-3"
    >
      <span
        data-tauri-drag-region
        className="text-xs font-medium text-vx-text-dim select-none"
      >
        VoxiType
      </span>
      <div className="flex gap-1">
        <button
          type="button"
          onClick={() => void win.minimize()}
          className="rounded px-2 py-0.5 text-vx-text-dim hover:bg-vx-bg-tertiary hover:text-vx-text-primary"
        >
          &#x2014;
        </button>
        <button
          type="button"
          onClick={() => void win.close()}
          className="rounded px-2 py-0.5 text-vx-text-dim hover:bg-vx-error hover:text-white"
        >
          &#x2715;
        </button>
      </div>
    </div>
  );
}
