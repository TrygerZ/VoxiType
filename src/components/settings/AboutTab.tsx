import { useEffect, useState } from "react";
import { Mic, ExternalLink, RefreshCw, CheckCircle2 } from "lucide-react";
import { getAppInfo, checkUpdates } from "../../lib/tauri";
import type { AppInfo, UpdateInfo } from "../../types/app";
import { Button } from "../ui/Button";
import { SettingsHeader } from "./SettingsLayout";

export function AboutTab() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [checking, setChecking] = useState(false);
  const [checkError, setCheckError] = useState<string | null>(null);

  useEffect(() => {
    getAppInfo()
      .then(setInfo)
      .catch(() => {});
  }, []);

  const handleCheck = async () => {
    setChecking(true);
    setCheckError(null);
    try {
      setUpdate(await checkUpdates());
    } catch (e: unknown) {
      setCheckError(e instanceof Error ? e.message : String(e));
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="max-w-xl">
      <SettingsHeader title="About" />

      <div className="flex flex-col items-center gap-4 rounded-2xl border border-vx-border bg-vx-bg-secondary/60 p-8 text-center">
        <span className="flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-vx-accent to-vx-accent-hover shadow-[0_8px_28px_rgba(124,108,240,0.4)]">
          <Mic className="h-8 w-8 text-white" />
        </span>
        <div>
          <h3 className="text-xl font-bold tracking-tight">VoxiType</h3>
          <p className="mt-1 text-sm text-vx-text-secondary">
            Free &amp; open-source voice-to-text for every application.
          </p>
        </div>

        <div className="flex items-center gap-3 text-xs text-vx-text-dim">
          {info && <span>v{info.version}</span>}
          <span className="h-1 w-1 rounded-full bg-vx-text-dim" />
          <span>Tauri {info?.tauri ?? "2"}</span>
          <span className="h-1 w-1 rounded-full bg-vx-text-dim" />
          <span>MIT License</span>
        </div>

        <div className="mt-2 flex flex-col items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void handleCheck()}
            disabled={checking}
          >
            <RefreshCw
              className={`h-3.5 w-3.5 ${checking ? "animate-[vx-spin_1s_linear_infinite]" : ""}`}
            />
            {checking ? "Checking..." : "Check for updates"}
          </Button>

          {checkError && (
            <p className="text-xs text-vx-error">Check failed: {checkError}</p>
          )}

          {update && !checkError && (
            <div className="text-xs">
              {update.available ? (
                <a
                  href={update.url}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1 text-vx-accent hover:underline"
                >
                  Update available: v{update.latest_version}
                  <ExternalLink className="h-3 w-3" />
                </a>
              ) : (
                <span className="inline-flex items-center gap-1 text-vx-success">
                  <CheckCircle2 className="h-3.5 w-3.5" />
                  You&apos;re on the latest version
                </span>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
