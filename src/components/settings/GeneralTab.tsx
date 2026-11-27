import { useEffect, useState } from "react";
import { Check, FolderOpen, RotateCw } from "lucide-react";

import {
  errorCode,
  formatTauriError,
  getDataDirectory,
  pickDataDirectory,
  restartApp,
  restrictDataDirectory,
  setDataDirectory,
  setFloatingWidgetEnabled,
  type DataDirectoryStatus,
} from "../../lib/tauri";
import {
  formatDirectoryError,
  isAclRejection,
} from "../../lib/dataDirectory";
import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import {
  getBooleanSetting,
  getNumberSetting,
  getStringSetting,
} from "../../lib/settingsGuards";
import { normalizeThemePreference } from "../../lib/theme";
import { useSettingsStore } from "../../stores/settingsStore";
import type { ThemePreference } from "../../types/app";
import { AclRestrictPrompt } from "../ui/AclRestrictPrompt";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { StatusBadge } from "../ui/StatusBadge";
import { Switch } from "../ui/Switch";
import { SettingsGroup, SettingsHeader, SettingsRow } from "./SettingsLayout";

const THEME_OPTIONS: { value: ThemePreference; labelKey: string }[] = [
  { value: "dark", labelKey: "settings.general.theme_dark" },
  { value: "light", labelKey: "settings.general.theme_light" },
  { value: "system", labelKey: "settings.general.theme_system" },
];

const RETENTION_OPTIONS = [
  { value: "30", labelKey: "settings.general.retention_30" },
  { value: "90", labelKey: "settings.general.retention_90" },
  { value: "180", labelKey: "settings.general.retention_180" },
  { value: "0", labelKey: "settings.general.retention_forever" },
] as const;

const MIN_AUTO_HIDE_SECONDS = 3;
const MAX_AUTO_HIDE_SECONDS = 60;

export function GeneralTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);

  const widgetEnabled = getBooleanSetting(settings.floating_widget, true);
  const autoHideRaw = getNumberSetting(
    settings.floating_widget_auto_hide_seconds,
    0,
  );
  const autoHideEnabled =
    autoHideRaw >= MIN_AUTO_HIDE_SECONDS &&
    autoHideRaw <= MAX_AUTO_HIDE_SECONDS;
  const [autoHideSeconds, setAutoHideSeconds] = useState<number | string>(
    autoHideEnabled ? autoHideRaw : MIN_AUTO_HIDE_SECONDS,
  );

  useEffect(() => {
    if (
      autoHideRaw >= MIN_AUTO_HIDE_SECONDS &&
      autoHideRaw <= MAX_AUTO_HIDE_SECONDS
    ) {
      setAutoHideSeconds(autoHideRaw);
    }
  }, [autoHideRaw]);

  const toggleAutoHide = (enabled: boolean) => {
    if (!enabled) {
      void invokeAction(() => update("floating_widget_auto_hide_seconds", 0));
      return;
    }
    const parsed = parseInt(String(autoHideSeconds), 10);
    const valid =
      !Number.isNaN(parsed) &&
      parsed >= MIN_AUTO_HIDE_SECONDS &&
      parsed <= MAX_AUTO_HIDE_SECONDS
        ? parsed
        : MIN_AUTO_HIDE_SECONDS;
    setAutoHideSeconds(valid);
    void invokeAction(() => update("floating_widget_auto_hide_seconds", valid));
  };

  const handleSecondsChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const raw = e.target.value;
    setAutoHideSeconds(raw);
    const parsed = parseInt(raw, 10);
    if (
      !Number.isNaN(parsed) &&
      parsed >= MIN_AUTO_HIDE_SECONDS &&
      parsed <= MAX_AUTO_HIDE_SECONDS
    ) {
      void invokeAction(() =>
        update("floating_widget_auto_hide_seconds", parsed),
      );
    }
  };

  const handleSecondsBlur = () => {
    const num = parseInt(String(autoHideSeconds), 10);
    if (Number.isNaN(num) || num < MIN_AUTO_HIDE_SECONDS) {
      setAutoHideSeconds(MIN_AUTO_HIDE_SECONDS);
      if (autoHideEnabled) {
        void invokeAction(() =>
          update("floating_widget_auto_hide_seconds", MIN_AUTO_HIDE_SECONDS),
        );
      }
    } else if (num > MAX_AUTO_HIDE_SECONDS) {
      setAutoHideSeconds(MAX_AUTO_HIDE_SECONDS);
      if (autoHideEnabled) {
        void invokeAction(() =>
          update("floating_widget_auto_hide_seconds", MAX_AUTO_HIDE_SECONDS),
        );
      }
    }
  };

  // Toggle the floating widget: update the local store optimistically and let
  // the backend persist the setting and show/hide the overlay window.
  const toggleFloatingWidget = async (v: boolean) => {
    const prev = useSettingsStore.getState().settings.floating_widget;
    useSettingsStore.setState((s) => ({
      settings: { ...s.settings, floating_widget: v },
    }));
    await invokeAction(
      () => setFloatingWidgetEnabled(v),
      () => {
        if (useSettingsStore.getState().settings.floating_widget === v) {
          useSettingsStore.setState((s) => ({
            settings: { ...s.settings, floating_widget: prev },
          }));
        }
      },
    );
  };

  const themePref = normalizeThemePreference(settings.theme);
  const retentionValue = String(
    getNumberSetting(settings.history_retention_days, 90),
  );

  return (
    <div>
      <SettingsHeader
        title={t("settings.general.title")}
        description={t("settings.general.desc")}
      />

      <SettingsGroup title={t("settings.general.appearance")}>
        <SettingsRow
          label={t("settings.general.theme")}
          description={t("settings.general.theme_desc")}
        >
          <div
            role="group"
            aria-label={t("settings.general.theme")}
            className="inline-flex rounded-[10px] border border-vx-border-subtle bg-vx-bg-subtle p-0.5"
          >
            {THEME_OPTIONS.map((opt) => {
              const active = themePref === opt.value;
              return (
                <button
                  key={opt.value}
                  type="button"
                  aria-pressed={active}
                  onClick={() =>
                    void invokeAction(() => update("theme", opt.value))
                  }
                  className={`rounded-[8px] px-3 py-1 text-xs font-medium transition-colors duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember ${
                    active
                      ? "bg-vx-bg-elevated text-vx-text-primary"
                      : "text-vx-text-muted hover:text-vx-text-primary"
                  }`}
                >
                  {t(opt.labelKey)}
                </button>
              );
            })}
          </div>
        </SettingsRow>
        <SettingsRow label={t("settings.general.lang")}>
          <Select
            aria-label={t("settings.general.lang")}
            options={[
              { value: "id", label: "Bahasa Indonesia" },
              { value: "en", label: "English" },
            ]}
            value={getStringSetting(settings.language, "en")}
            onChange={(e) =>
              void invokeAction(() => update("language", e.target.value))
            }
            className="w-full sm:w-48"
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.widget")}
          description={t("settings.general.widget.desc")}
        >
          <Switch
            checked={widgetEnabled}
            onChange={toggleFloatingWidget}
            ariaLabel={t("settings.general.widget")}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.widget_auto_hide")}
          description={t("settings.general.widget_auto_hide.desc")}
        >
          <div
            className={`flex items-center gap-3 transition-opacity ${
              !widgetEnabled ? "opacity-40" : ""
            }`}
          >
            <Switch
              checked={autoHideEnabled}
              disabled={!widgetEnabled}
              onChange={toggleAutoHide}
              ariaLabel={t("settings.general.widget_auto_hide")}
              data-testid="widget-auto-hide-switch"
            />
            <div className="flex items-center gap-1.5">
              <div className="w-16">
                <Input
                  type="number"
                  min={MIN_AUTO_HIDE_SECONDS}
                  max={MAX_AUTO_HIDE_SECONDS}
                  value={autoHideSeconds}
                  disabled={!autoHideEnabled || !widgetEnabled}
                  onChange={handleSecondsChange}
                  onBlur={handleSecondsBlur}
                  aria-label={t(
                    "settings.general.widget_auto_hide.seconds_label",
                  )}
                  data-testid="widget-auto-hide-input"
                  className="h-8 !py-1 px-1.5 text-center text-sm disabled:cursor-not-allowed disabled:opacity-40"
                />
              </div>
              <span className="text-xs text-vx-text-dim">
                {t("settings.general.widget_auto_hide.seconds")}
              </span>
            </div>
          </div>
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("settings.general.system")}>
        <DataDirectoryRow t={t} />
        <SettingsRow
          label={t("settings.general.startup")}
          description={t("settings.general.startup.desc")}
        >
          <Switch
            checked={getBooleanSetting(settings.auto_start, false)}
            onChange={(v) => void invokeAction(() => update("auto_start", v))}
            ariaLabel={t("settings.general.startup")}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.history_retention")}
          description={t("settings.general.history_retention_desc")}
        >
          <Select
            aria-label={t("settings.general.history_retention")}
            options={RETENTION_OPTIONS.map((opt) => ({
              value: opt.value,
              label: t(opt.labelKey),
            }))}
            value={retentionValue}
            onChange={(e) =>
              void invokeAction(() =>
                update("history_retention_days", Number(e.target.value)),
              )
            }
            className="w-full sm:w-40"
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.updates")}
          description={t("settings.general.updates.desc")}
        >
          <Switch
            checked={getBooleanSetting(settings.auto_update, true)}
            onChange={(v) => void invokeAction(() => update("auto_update", v))}
            ariaLabel={t("settings.general.updates")}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.onboarding")}
          description={t("settings.general.onboarding.desc")}
        >
          <Button
            size="sm"
            variant="secondary"
            onClick={() =>
              void invokeAction(() => update("onboarding_completed", false))
            }
          >
            {t("settings.general.onboarding.button")}
          </Button>
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("settings.general.privacy")}>
        <SettingsRow
          label={t("settings.general.stats")}
          description={t("settings.general.stats.desc")}
        >
          <Switch
            checked={getBooleanSetting(settings.telemetry, false)}
            onChange={(v) => void invokeAction(() => update("telemetry", v))}
            ariaLabel={t("settings.general.stats")}
          />
        </SettingsRow>
      </SettingsGroup>
    </div>
  );
}

function DataDirectoryRow({
  t,
}: {
  t: (key: string, params?: Record<string, string | number>) => string;
}) {
  const [dataDirectory, setDataDirectoryStatus] =
    useState<DataDirectoryStatus | null>(null);
  const [selectedDirectory, setSelectedDirectory] = useState("");
  const [pendingRestart, setPendingRestart] = useState<string | null>(null);
  const [lastError, setLastError] = useState<string | null>(null);
  const [dataStatus, setDataStatus] = useState("");
  const [dataError, setDataError] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirmRestart, setConfirmRestart] = useState(false);
  const [aclRejected, setAclRejected] = useState(false);

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
      setAclRejected(false);
      await setDataDirectory(selectedDirectory);
      setPendingRestart(selectedDirectory);
      setSelectedDirectory("");
      setLastError(null);
      setDataStatus(t("data_directory.success"));
    } catch (e: unknown) {
      // A folder on another drive inherits write access for every local
      // account. Offer the fix instead of dead-ending the user.
      if (isAclRejection(e)) {
        setAclRejected(true);
        setDataError("");
      } else {
        setDataError(formatDirectoryError(e, t));
      }
    } finally {
      setBusy(false);
    }
  };

  const restrictAndApply = async () => {
    if (!selectedDirectory || busy) return;
    setBusy(true);
    try {
      setDataError("");
      await restrictDataDirectory(selectedDirectory);
      setAclRejected(false);
      setPendingRestart(selectedDirectory);
      setSelectedDirectory("");
      setLastError(null);
      setDataStatus(t("data_directory.success"));
    } catch (e: unknown) {
      setAclRejected(false);
      setDataError(
        t("data_directory.acl_failed", { reason: formatTauriError(e) }),
      );
    } finally {
      setBusy(false);
    }
  };

  const handleRestart = async (force = false) => {
    if (busy) return;
    setBusy(true);
    try {
      setDataError("");
      await restartApp(force);
      setConfirmRestart(false);
    } catch (e: unknown) {
      // The backend refuses a restart while work is active; ask first.
      if (errorCode(e) === "WorkInProgress") {
        setConfirmRestart(true);
      } else {
        setDataError(formatTauriError(e));
      }
      setBusy(false);
    }
  };

  const aclSafe =
    Boolean(dataDirectory?.active) &&
    !aclRejected &&
    !dataError &&
    !lastError;

  return (
    <div className="flex flex-col gap-3 px-4 py-3.5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="text-sm font-medium text-vx-text-primary">
            {t("settings.general.storage")}
          </span>
          <span className="text-xs text-vx-text-dim">
            {t("data_directory.restart")}
          </span>
        </div>
        <div className="flex shrink-0 flex-wrap items-center gap-2">
          <Button
            size="sm"
            variant="secondary"
            disabled={busy}
            onClick={() => void chooseDataDirectory()}
          >
            <FolderOpen className="h-3.5 w-3.5" />
            {t("data_directory.choose")}
          </Button>
          <Button
            size="sm"
            variant="secondary"
            disabled={!selectedDirectory || busy}
            onClick={() => void applyDataDirectory()}
          >
            {t("data_directory.apply")}
          </Button>
          {pendingRestart && !confirmRestart && (
            <Button
              size="sm"
              variant="primary"
              disabled={busy}
              onClick={() => void handleRestart()}
              data-testid="restart-app-button"
            >
              <RotateCw className="h-3.5 w-3.5" />
              {t("data_directory.restart_now")}
            </Button>
          )}
          {pendingRestart && confirmRestart && (
            <>
              <Button
                size="sm"
                variant="danger"
                disabled={busy}
                onClick={() => void handleRestart(true)}
                data-testid="restart-app-confirm-button"
              >
                {t("data_directory.restart_confirm_btn")}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={busy}
                onClick={() => setConfirmRestart(false)}
                data-testid="restart-app-cancel-button"
              >
                {t("data_directory.restart_cancel")}
              </Button>
            </>
          )}
        </div>
      </div>

      <div className="flex flex-wrap sm:flex-nowrap items-center justify-between gap-2 sm:gap-3 rounded-[10px] border border-vx-border-subtle bg-vx-bg-subtle px-3 py-2">
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="text-[10px] uppercase tracking-wider text-vx-text-dim">
            {t("data_directory.active")}
          </span>
          <span
            className="break-all font-mono text-xs text-vx-text-primary"
            data-testid="active-data-directory"
          >
            {dataDirectory?.active ??
              (dataError ? "" : t("data_directory.loading"))}
          </span>
        </div>
        {aclSafe && (
          <StatusBadge tone="success" className="shrink-0">
            <Check className="h-3 w-3" />
            {t("settings.general.acl_verified")}
          </StatusBadge>
        )}
      </div>

      {confirmRestart && (
        <span
          role="alert"
          className="text-xs text-vx-semantic-error"
          data-testid="restart-app-confirm-message"
        >
          {t("data_directory.restart_confirm")}
        </span>
      )}

      {pendingRestart && (
        <div
          className="rounded-lg border border-vx-accent-ember/30 bg-vx-accent-ember-soft px-3 py-1.5"
          data-testid="pending-data-directory"
        >
          <span className="break-all font-mono text-[11px] text-vx-accent-ember">
            {t("data_directory.pending_restart", { path: pendingRestart })}
          </span>
        </div>
      )}

      {selectedDirectory && (
        <span
          className="break-all font-mono text-[11px] text-vx-text-primary"
          data-testid="selected-data-directory"
        >
          {selectedDirectory}
        </span>
      )}

      {dataStatus && (
        <span
          role="status"
          className="text-xs text-vx-semantic-success"
          data-testid="data-directory-status"
        >
          {dataStatus}
        </span>
      )}

      {aclRejected && (
        <AclRestrictPrompt
          message={t("data_directory.acl_confirm")}
          onConfirm={() => void restrictAndApply()}
          onCancel={() => {
            setAclRejected(false);
            setSelectedDirectory("");
          }}
          busy={busy}
          t={t}
        />
      )}

      {dataError && (
        <span
          role="alert"
          className="text-xs text-vx-semantic-error"
          data-testid="data-directory-error"
        >
          {dataError}
        </span>
      )}

      {lastError && (
        <span
          role="alert"
          className="break-all text-xs text-vx-semantic-error"
          data-testid="data-directory-last-error"
        >
          {t("data_directory.fallback_error", { reason: lastError })}
        </span>
      )}
    </div>
  );
}
