import { useEffect, useState } from "react";
import { FolderOpen, RotateCw } from "lucide-react";
import {
  formatTauriError,
  getDataDirectory,
  pickDataDirectory,
  restartApp,
  setDataDirectory,
  setFloatingWidgetEnabled,
  type DataDirectoryStatus,
} from "../../lib/tauri";
import { formatDirectoryError } from "../../lib/dataDirectory";
import { useSettingsStore } from "../../stores/settingsStore";
import { Switch } from "../ui/Switch";
import { Select } from "../ui/Select";
import { Button } from "../ui/Button";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";
import { useT } from "../../lib/i18n";

export function GeneralTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const [dataDirectory, setDataDirectoryStatus] =
    useState<DataDirectoryStatus | null>(null);
  const [selectedDirectory, setSelectedDirectory] = useState("");
  const [pendingRestart, setPendingRestart] = useState<string | null>(null);
  const [lastError, setLastError] = useState<string | null>(null);
  const [dataStatus, setDataStatus] = useState("");
  const [dataError, setDataError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void getDataDirectory()
      .then((status) => {
        setDataDirectoryStatus(status);
        setPendingRestart(status.pending);
        setLastError(status.lastError);
      })
      .catch((e: unknown) => setDataError(formatTauriError(e)));
  }, []);

  const chooseDataDirectory = async () => {
    setBusy(true);
    try {
      setDataError("");
      setDataStatus("");
      const path = await pickDataDirectory();
      if (path) setSelectedDirectory(path);
    } catch (e: unknown) {
      setDataError(formatDirectoryError(e, t));
    } finally {
      setBusy(false);
    }
  };

  const applyDataDirectory = async () => {
    if (!selectedDirectory || busy) return;
    setBusy(true);
    try {
      setDataError("");
      await setDataDirectory(selectedDirectory);
      setPendingRestart(selectedDirectory);
      setSelectedDirectory("");
      setLastError(null);
      setDataStatus(t("data_directory.success"));
    } catch (e: unknown) {
      setDataError(formatDirectoryError(e, t));
    } finally {
      setBusy(false);
    }
  };

  const handleRestart = async () => {
    if (busy) return;
    setBusy(true);
    try {
      setDataError("");
      await restartApp();
    } catch (e: unknown) {
      setDataError(formatTauriError(e));
      setBusy(false);
    }
  };

  // Toggle the floating widget: update the local store optimistically and let
  // the backend persist the setting and show/hide the overlay window.
  const toggleFloatingWidget = async (v: boolean) => {
    const prev = useSettingsStore.getState().settings.floating_widget;
    useSettingsStore.setState((s) => ({
      settings: { ...s.settings, floating_widget: v },
    }));
    try {
      await setFloatingWidgetEnabled(v);
    } catch {
      if (useSettingsStore.getState().settings.floating_widget === v) {
        useSettingsStore.setState((s) => ({
          settings: { ...s.settings, floating_widget: prev },
        }));
      }
    }
  };

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title={t("settings.general.title")}
        description={t("settings.general.desc")}
      />

      <SettingsGroup title={t("settings.general.appearance")}>
        <SettingsRow label={t("settings.general.lang")}>
          <Select
            options={[
              { value: "id", label: "Bahasa Indonesia" },
              { value: "en", label: "English" },
            ]}
            value={(settings.language as string) ?? "en"}
            onChange={(e) => void update("language", e.target.value)}
            className="w-48"
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.widget")}
          description={t("settings.general.widget.desc")}
        >
          <Switch
            checked={(settings.floating_widget as boolean) ?? true}
            onChange={toggleFloatingWidget}
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("settings.general.system")}>
        <SettingsRow
          label={t("settings.general.storage")}
          description={t("data_directory.restart")}
        >
          <div className="flex max-w-xs flex-col items-end gap-2">
            <div className="flex flex-col items-end">
              <span className="text-[10px] uppercase tracking-wider text-vx-text-dim">
                {t("data_directory.active")}
              </span>
              <span
                className="max-w-xs break-all text-right font-mono text-[11px] text-vx-text-dim"
                data-testid="active-data-directory"
              >
                {dataDirectory?.active ?? (dataError ? "" : t("data_directory.loading"))}
              </span>
            </div>
            {pendingRestart && (
              <div
                className="flex flex-col items-end rounded bg-vx-accent-soft/30 px-2 py-1"
                data-testid="pending-data-directory"
              >
                <span className="max-w-xs break-all text-right font-mono text-[11px] text-vx-accent">
                  {t("data_directory.pending_restart", {
                    path: pendingRestart,
                  })}
                </span>
              </div>
            )}
            <div className="flex gap-2">
              <Button
                size="sm"
                disabled={busy}
                onClick={() => void chooseDataDirectory()}
              >
                <FolderOpen className="h-3.5 w-3.5" />
                {t("data_directory.choose")}
              </Button>
              <Button
                size="sm"
                variant="primary"
                disabled={!selectedDirectory || busy}
                onClick={() => void applyDataDirectory()}
              >
                {t("data_directory.apply")}
              </Button>
              {pendingRestart && (
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={busy}
                  onClick={() => void handleRestart()}
                  data-testid="restart-app-button"
                >
                  <RotateCw className="h-3.5 w-3.5" />
                  {t("data_directory.restart_now")}
                </Button>
              )}
            </div>
            {selectedDirectory && (
              <span
                className="max-w-xs break-all text-right font-mono text-[11px] text-vx-text-primary"
                data-testid="selected-data-directory"
              >
                {selectedDirectory}
              </span>
            )}
            {dataStatus && (
              <span
                role="status"
                className="text-xs text-green-600"
                data-testid="data-directory-status"
              >
                {dataStatus}
              </span>
            )}
            {dataError && (
              <span
                role="alert"
                className="text-xs text-vx-error"
                data-testid="data-directory-error"
              >
                {dataError}
              </span>
            )}
            {lastError && (
              <span
                role="alert"
                className="max-w-xs break-all text-right text-xs text-vx-error"
                data-testid="data-directory-last-error"
              >
                {t("data_directory.fallback_error", { reason: lastError })}
              </span>
            )}
          </div>
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.onboarding")}
          description={t("settings.general.onboarding.desc")}
        >
          <Button
            size="sm"
            onClick={() => void update("onboarding_completed", false)}
          >
            {t("settings.general.onboarding.button")}
          </Button>
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.startup")}
          description={t("settings.general.startup.desc")}
        >
          <Switch
            checked={(settings.auto_start as boolean) ?? false}
            onChange={(v) => void update("auto_start", v)}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.updates")}
          description={t("settings.general.updates.desc")}
        >
          <Switch
            checked={(settings.auto_update as boolean) ?? true}
            onChange={(v) => void update("auto_update", v)}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.sound")}
          description={t("settings.general.sound.desc")}
        >
          <Switch
            checked={(settings.sound_cues as boolean) ?? false}
            onChange={(v) => void update("sound_cues", v)}
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("settings.general.privacy")}>
        <SettingsRow
          label={t("settings.general.command")}
          description={t("settings.general.command.desc")}
        >
          <Switch
            checked={(settings.command_mode as boolean) ?? false}
            onChange={(v) => void update("command_mode", v)}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.stats")}
          description={t("settings.general.stats.desc")}
        >
          <Switch
            checked={(settings.telemetry as boolean) ?? false}
            onChange={(v) => void update("telemetry", v)}
          />
        </SettingsRow>
      </SettingsGroup>
    </div>
  );
}
