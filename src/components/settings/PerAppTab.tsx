import { useCallback, useEffect, useRef, useState } from "react";
import { Crosshair, Plus, Trash2 } from "lucide-react";

import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import { getBooleanSetting } from "../../lib/settingsGuards";
import {
  deletePerAppMode,
  getActiveApp,
  getPerAppModes,
  listRunningApps,
  setPerAppMode,
} from "../../lib/tauri";
import { usePlatform } from "../../lib/usePlatform";
import { useSettingsStore } from "../../stores/settingsStore";
import type { PerAppMode } from "../../types/app";
import { Button } from "../ui/Button";
import { IconButton } from "../ui/IconButton";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";
import { SettingsGroup, SettingsHeader, SettingsRow } from "./SettingsLayout";

const DETECT_SECONDS = 3;

export function PerAppTab() {
  const t = useT();
  const platform = usePlatform();
  // Per-app routing needs foreground window detection, which is Windows-only
  // until Phase 2. Offering the rules here would silently do nothing.
  // "unknown" keeps them visible so they do not flicker while it loads.
  const supported = platform !== "macos" && platform !== "linux";
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const [modes, setModes] = useState<PerAppMode[]>([]);
  const [runningApps, setRunningApps] = useState<string[]>([]);
  const [activeApp, setActiveApp] = useState<string | null>(null);
  const [countdown, setCountdown] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);

  const [proc, setProc] = useState("");
  const [mode, setMode] = useState("message");

  const mountedRef = useRef(true);
  const detectTimerRef = useRef<number | null>(null);

  const clearDetectTimer = () => {
    if (detectTimerRef.current !== null) {
      window.clearInterval(detectTimerRef.current);
      detectTimerRef.current = null;
    }
  };

  const loadRules = useCallback(async () => {
    await invokeAction(async () => {
      const res = await getPerAppModes();
      if (mountedRef.current) {
        setModes(res);
      }
    });
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    void loadRules();
    if (supported) {
      listRunningApps()
        .then((apps) => {
          if (mountedRef.current && Array.isArray(apps)) {
            setRunningApps(apps);
          }
        })
        .catch(() => {});
    }
    return () => {
      mountedRef.current = false;
      clearDetectTimer();
    };
  }, [loadRules, supported]);

  const captureActiveApp = async () => {
    setBusy(true);
    try {
      await invokeAction(async () => {
        const app = await getActiveApp();
        if (mountedRef.current && app) {
          setActiveApp(app);
          setProc(app);
        }
      });
    } finally {
      if (mountedRef.current) {
        setBusy(false);
      }
    }
  };

  const startDetectCountdown = () => {
    if (busy || countdown !== null) return;
    clearDetectTimer();
    let remaining = DETECT_SECONDS;
    setCountdown(remaining);
    detectTimerRef.current = window.setInterval(() => {
      remaining -= 1;
      if (remaining > 0) {
        if (mountedRef.current) setCountdown(remaining);
        return;
      }
      clearDetectTimer();
      if (mountedRef.current) {
        setCountdown(null);
        void captureActiveApp();
      }
    }, 1000);
  };

  const handleAdd = async () => {
    if (busy || !proc.trim()) return;
    setBusy(true);
    try {
      const ok = await invokeAction(async () => {
        await setPerAppMode({
          id: 0,
          app_process_name: proc.trim().toLowerCase(),
          app_display_name: proc.trim(),
          mode_id: mode,
        });
        await loadRules();
      });
      if (ok && mountedRef.current) {
        setProc("");
      }
    } finally {
      if (mountedRef.current) {
        setBusy(false);
      }
    }
  };

  const handleUpdateRuleMode = async (rule: PerAppMode, nextMode: string) => {
    if (busy || rule.mode_id === nextMode) return;
    const updated: PerAppMode = { ...rule, mode_id: nextMode };
    setBusy(true);
    try {
      const ok = await invokeAction(async () => {
        await setPerAppMode(updated);
      });
      if (ok && mountedRef.current) {
        setModes((prev) =>
          prev.map((item) => (item.id === rule.id ? updated : item)),
        );
      }
    } finally {
      if (mountedRef.current) {
        setBusy(false);
      }
    }
  };

  const handleRemove = async (id: number) => {
    if (busy) return;
    setBusy(true);
    try {
      await invokeAction(async () => {
        await deletePerAppMode(id);
        await loadRules();
      });
    } finally {
      if (mountedRef.current) {
        setBusy(false);
      }
    }
  };

  const perAppOn = getBooleanSetting(settings.per_app_mode, false);
  const modeOptions = [
    { value: "dictation", label: t("settings.modes.dictation") },
    { value: "message", label: t("settings.modes.message") },
    { value: "email", label: t("settings.modes.email") },
  ];

  return (
    <div>
      <SettingsHeader
        title={t("settings.app_rules.title")}
        description={t("settings.app_rules.desc")}
      />

      <SettingsGroup title={t("settings.app_rules.feature_group")}>
        {supported ? (
          <SettingsRow
            label={t("settings.app_rules.enable")}
            description={t("settings.app_rules.enable_desc")}
          >
            <Switch
              checked={perAppOn}
              onChange={(v) =>
                void invokeAction(() => update("per_app_mode", v))
              }
              ariaLabel={t("settings.app_rules.enable")}
            />
          </SettingsRow>
        ) : (
          <SettingsRow
            label={t("settings.app_rules.unsupported")}
            description={t("settings.app_rules.unsupported_desc")}
          >
            <span />
          </SettingsRow>
        )}
      </SettingsGroup>

      <div
        className={`transition-opacity ${
          !perAppOn || !supported ? "pointer-events-none opacity-50" : ""
        }`}
      >
        <SettingsGroup title={t("settings.app_rules.add_group")}>
          <div className="flex flex-col gap-3 px-4 py-4">
            <div className="flex flex-wrap items-end gap-2">
              <div className="min-w-[140px] flex-1">
                <Input
                  aria-label={t("settings.app_rules.proc_name")}
                  placeholder={t("settings.app_rules.proc_name")}
                  value={proc}
                  onChange={(e) => setProc(e.target.value)}
                />
              </div>
              <Button
                variant="ember"
                size="md"
                onClick={startDetectCountdown}
                disabled={busy || countdown !== null}
                title={t("settings.app_rules.detect_btn")}
              >
                <Crosshair className="h-3.5 w-3.5" />
                {t("settings.app_rules.detect_btn")}
              </Button>
              <div className="w-full sm:w-36">
                <Select
                  aria-label={t("settings.app_rules.mode_apply")}
                  options={modeOptions}
                  value={mode}
                  onChange={(e) => setMode(e.target.value)}
                />
              </div>
              <Button
                variant="primary"
                size="md"
                onClick={() => void handleAdd()}
                disabled={busy || !proc.trim()}
              >
                <Plus className="h-3.5 w-3.5" />
                {t("settings.app_rules.add_btn")}
              </Button>
            </div>

            {runningApps.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {runningApps.map((app) => (
                  <button
                    key={app}
                    type="button"
                    onClick={() => setProc(app)}
                    className="rounded-md border border-vx-border-subtle bg-vx-bg-subtle px-2 py-0.5 font-mono text-[11px] text-vx-text-secondary transition-colors hover:border-vx-border-default hover:text-vx-text-primary focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember"
                  >
                    {app}
                  </button>
                ))}
              </div>
            )}

            {countdown !== null && (
              <p className="text-xs text-vx-accent-ember">
                {t("settings.app_rules.detect_countdown", {
                  seconds: countdown,
                })}
              </p>
            )}

            {countdown === null && activeApp && (
              <p className="text-xs text-vx-accent-ember">
                {t("settings.app_rules.detected_app", { app: activeApp })}
              </p>
            )}
          </div>
        </SettingsGroup>

        <SettingsGroup
          title={t("settings.app_rules.active_group", { count: modes.length })}
        >
          {modes.length === 0 ? (
            <div className="px-4 py-6 text-center text-sm text-vx-text-dim">
              {t("settings.app_rules.empty")}
            </div>
          ) : (
            modes.map((m) => (
              <div
                key={m.id}
                className="flex flex-wrap sm:flex-nowrap items-center justify-between gap-3 px-4 py-3"
              >
                <div className="flex min-w-0 flex-col gap-0.5">
                  <span className="truncate font-mono text-sm font-medium text-vx-text-primary">
                    {m.app_process_name}
                  </span>
                  <span className="text-xs text-vx-text-dim">
                    {t("settings.app_rules.auto_override")}
                  </span>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <div className="w-32 sm:w-36">
                    <Select
                      aria-label={t("settings.app_rules.rule_mode_aria", {
                        app: m.app_process_name,
                      })}
                      options={modeOptions}
                      value={m.mode_id}
                      disabled={busy}
                      onChange={(e) =>
                        void handleUpdateRuleMode(m, e.target.value)
                      }
                    />
                  </div>
                  <IconButton
                    icon={<Trash2 className="h-4 w-4" />}
                    label={t("settings.app_rules.delete_rule", {
                      app: m.app_process_name,
                    })}
                    tone="danger"
                    size="sm"
                    disabled={busy}
                    onClick={() => void handleRemove(m.id)}
                  />
                </div>
              </div>
            ))
          )}
        </SettingsGroup>
      </div>
    </div>
  );
}
