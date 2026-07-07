import { useEffect, useState } from "react";
import { ExternalLink, RefreshCw, CheckCircle2 } from "lucide-react";
import { getAppInfo, checkUpdates, formatTauriError } from "../../lib/tauri";
import type { AppInfo, UpdateInfo } from "../../types/app";
import { Button } from "../ui/Button";
import { useT } from "../../lib/i18n";
import { SettingsHeader } from "./SettingsLayout";

export function AboutTab() {
  const t = useT();
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
      setCheckError(formatTauriError(e));
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="mx-auto flex h-full max-w-xl flex-col justify-center pb-20">
      <SettingsHeader title={t("settings.about.title")} />

      <div className="flex flex-col items-center justify-center gap-4 rounded-2xl bg-vx-bg-tertiary p-10 text-center shadow-vx-md">
        <span className="flex h-16 w-16 items-center justify-center overflow-hidden rounded-2xl bg-vx-accent-soft text-vx-accent">
          <img src="/logo.png" alt="VoxiType Logo" className="h-full w-full object-contain" />
        </span>
        <div>
          <h3 className="text-xl font-bold tracking-tight">VoxiType</h3>
          <p className="mt-1 text-sm text-vx-text-secondary">
            {t("settings.about.desc")}
          </p>
        </div>

        <div className="flex items-center gap-3 text-xs text-vx-text-dim">
          {info && <span>{t("settings.about.version", { version: info.version })}</span>}
          <span className="h-1 w-1 rounded-full bg-vx-text-dim" />
          <span>Tauri {info?.tauri ?? "2"}</span>
          <span className="h-1 w-1 rounded-full bg-vx-text-dim" />
          <span>{t("settings.about.license")}</span>
        </div>

        <div className="mt-2 flex flex-col items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void handleCheck()}
            disabled={checking}
          >
            <RefreshCw
              className={`h-3.5 w-3.5 ${checking ? "animate-spin" : ""}`}
            />
            {checking ? t("settings.about.checking") : t("settings.about.check_btn")}
          </Button>

          {checkError && (
            <p className="text-xs text-vx-error">{t("settings.about.check_fail", { error: checkError })}</p>
          )}

          {update && !checkError && (
            <div className="text-xs">
              {update.available ? (
                <a
                  href={update.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1 text-vx-accent hover:underline"
                >
                  {t("settings.about.update_avail", { version: update.latest_version })}
                  <ExternalLink className="h-3 w-3" />
                </a>
              ) : (
                <span className="inline-flex items-center gap-1 text-vx-success">
                  <CheckCircle2 className="h-3.5 w-3.5" />
                  {t("settings.about.latest")}
                </span>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
