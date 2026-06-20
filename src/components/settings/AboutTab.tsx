import { useEffect, useState } from "react";
import { getAppInfo } from "../../lib/tauri";
import type { AppInfo } from "../../types/app";

export function AboutTab() {
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    getAppInfo()
      .then(setInfo)
      .catch(() => {});
  }, []);

  return (
    <div className="flex max-w-md flex-col gap-4">
      <h2 className="text-lg font-semibold">About</h2>
      <div className="flex flex-col gap-2 text-sm text-vx-text-secondary">
        <p>
          <span className="text-vx-text-primary font-medium">VoxiType</span>{" "}
          &mdash; Free & open-source voice-to-text for all applications.
        </p>
        {info && (
          <>
            <p>Version: {info.version}</p>
            <p>Tauri: {info.tauri}</p>
          </>
        )}
        <p>License: MIT</p>
      </div>
    </div>
  );
}
