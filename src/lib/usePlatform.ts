import { useEffect, useState } from "react";

import { getPlatform } from "./tauri";

/** Shared fetch so every caller in a session costs one IPC round trip. */
let pending: Promise<string> | null = null;

function platformOnce(): Promise<string> {
  pending ??= getPlatform();
  return pending;
}

/** The host OS as reported by the backend, or "unknown" until it loads. */
export function usePlatform(): string {
  const [platform, setPlatform] = useState("unknown");

  useEffect(() => {
    let active = true;
    void platformOnce().then((value) => {
      if (active) setPlatform(value);
    });
    return () => {
      active = false;
    };
  }, []);

  return platform;
}

/** Test hook: forget the cached value so a new mock takes effect. */
export function resetPlatformCache() {
  pending = null;
}
