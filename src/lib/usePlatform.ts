import { useEffect, useState } from "react";

import { getAppInfo } from "./tauri";
import type { AppInfo } from "../types/app";

const UNKNOWN: AppInfo = { name: "VoxiType", version: "0.0.0", tauri: "2" };

/** One IPC round trip per session, shared by every caller. */
let pending: Promise<AppInfo> | null = null;

function appInfoOnce(): Promise<AppInfo> {
  pending ??= getAppInfo().catch(() => UNKNOWN);
  return pending;
}

function useAppInfo(): AppInfo {
  const [info, setInfo] = useState<AppInfo>(UNKNOWN);

  useEffect(() => {
    let active = true;
    void appInfoOnce().then((value) => {
      if (active) setInfo(value);
    });
    return () => {
      active = false;
    };
  }, []);

  return info;
}

/** The host OS as reported by the backend, or "unknown" until it loads. */
export function usePlatform(): string {
  return useAppInfo().platform ?? "unknown";
}

/** Whether this Linux session runs Wayland, where dictation is unavailable. */
export function useWayland(): boolean {
  const info = useAppInfo();
  return info.platform === "linux" && info.is_wayland === true;
}

/** Test hook: forget the cached value so a new mock takes effect. */
export function resetPlatformCache() {
  pending = null;
}
