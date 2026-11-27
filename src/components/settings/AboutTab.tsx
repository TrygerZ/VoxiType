import { useEffect, useState } from "react";
import { CheckCircle2, Copy, ExternalLink, RefreshCw } from "lucide-react";

import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import { getStringSetting } from "../../lib/settingsGuards";
import {
  checkUpdates,
  formatTauriError,
  getAppInfo,
  openUrl,
} from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";
import type { AppInfo, UpdateInfo } from "../../types/app";
import { Button } from "../ui/Button";
import { StatusBadge } from "../ui/StatusBadge";
import { toast } from "../ui/Toast";
import { SettingsHeader } from "./SettingsLayout";

export function AboutTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
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

  const handleCopyDiagnostics = async () => {
    const diagnostics = [
      `VoxiType: ${info?.version ?? "unknown"}`,
      `Tauri: ${info?.tauri ?? "2"}`,
      `Platform: ${info?.platform ?? navigator.platform}`,
      `User-Agent: ${navigator.userAgent}`,
      `STT Engine: ${getStringSetting(settings.stt_engine, "groq")}`,
      `LLM Engine: ${getStringSetting(settings.llm_engine, "ollama")}`,
      `Theme: ${getStringSetting(settings.theme, "dark")}`,
    ].join("\n");

    await invokeAction(async () => {
      await navigator.clipboard.writeText(diagnostics);
      toast(t("history.copied"));
    });
  };

  return (
    <div>
      <SettingsHeader
        title={t("settings.about.title")}
        description={t("settings.about.desc")}
      />

      <div className="flex flex-col items-center justify-center gap-4 rounded-[12px] border border-vx-border-subtle bg-vx-bg-surface p-6 sm:p-10 text-center">
        <span className="flex h-14 w-14 items-center justify-center overflow-hidden rounded-[14px] border border-vx-border-subtle bg-vx-bg-subtle p-1.5">
          <img
            src="/logo.png"
            alt="VoxiType Logo"
            className="h-full w-full object-contain"
          />
        </span>

        <div>
          <h3 className="text-xl font-bold tracking-tight text-vx-text-primary">
            VoxiType
          </h3>
          <p className="mt-1 text-sm text-vx-text-muted">
            {t("settings.about.desc")}
          </p>
        </div>

        <div className="flex flex-wrap items-center justify-center gap-2">
          {info && (
            <StatusBadge tone="neutral">
              {t("settings.about.version", { version: info.version })}
            </StatusBadge>
          )}
          <StatusBadge tone="neutral">
            Tauri {info?.tauri ?? "2"}
          </StatusBadge>
          <StatusBadge tone="neutral">{t("settings.about.license")}</StatusBadge>
        </div>

        <div className="mt-2 flex flex-col items-center gap-3">
          <div className="flex flex-wrap items-center justify-center gap-2.5">
            <Button
              variant="primary"
              size="sm"
              onClick={() => void handleCheck()}
              disabled={checking}
            >
              <RefreshCw
                className={`h-3.5 w-3.5 ${checking ? "animate-spin" : ""}`}
              />
              {checking
                ? t("settings.about.checking")
                : t("settings.about.check_btn")}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => void handleCopyDiagnostics()}
            >
              <Copy className="h-3.5 w-3.5" />
              {t("settings.about.copy_diag")}
            </Button>
          </div>

          {checkError && (
            <StatusBadge tone="error">
              {t("settings.about.check_fail", { error: checkError })}
            </StatusBadge>
          )}

          {update && !checkError && (
            <div className="text-xs">
              {update.available ? (
                <button
                  type="button"
                  onClick={() => void invokeAction(() => openUrl(update.url))}
                  className="inline-flex items-center gap-1.5 rounded-full bg-vx-accent-ember-soft px-3 py-1 text-xs font-medium text-vx-accent-ember hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember"
                >
                  {t("settings.about.update_avail", {
                    version: update.latest_version,
                  })}
                  <ExternalLink className="h-3 w-3" />
                </button>
              ) : (
                <span className="inline-flex items-center gap-1.5 text-vx-semantic-success">
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
