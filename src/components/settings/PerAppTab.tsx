import { useEffect, useState, useCallback } from "react";
import { Plus, Trash2, Crosshair } from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import { useT } from "../../lib/i18n";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { invoke } from "@tauri-apps/api/core";
import { Switch } from "../ui/Switch";

interface PerAppMode {
  id: number;
  app_process_name: string;
  app_display_name?: string | null;
  mode_id: string;
}

export function PerAppTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const [modes, setModes] = useState<PerAppMode[]>([]);
  const [activeApp, setActiveApp] = useState<string | null>(null);

  const [proc, setProc] = useState("");
  const [mode, setMode] = useState("message");

  const load = useCallback(async () => {
    try {
      const res = await invoke<PerAppMode[]>("get_per_app_modes");
      setModes(res);
    } catch {}
  }, []);

  const getActive = async () => {
    try {
      const a = await invoke<string | null>("get_active_app");
      if (a) {
        setActiveApp(a);
        setProc(a);
      }
    } catch {}
  };

  useEffect(() => {
    void load();
  }, [load]);

  const handleAdd = async () => {
    if (!proc.trim()) return;
    try {
      await invoke("set_per_app_mode", {
        mapping: {
          id: 0,
          app_process_name: proc.trim().toLowerCase(),
          app_display_name: proc.trim(),
          mode_id: mode,
        },
      });
      setProc("");
      void load();
    } catch {}
  };

  const handleRemove = async (id: number) => {
    try {
      await invoke("delete_per_app_mode", { id });
      void load();
    } catch {}
  };

  const perAppOn = (settings.per_app_mode as boolean) ?? false;

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title={t("settings.app_rules.title")}
        description={t("settings.app_rules.desc")}
      />

      <SettingsGroup title={t("settings.app_rules.feature_group")}>
        <SettingsRow
          label={t("settings.app_rules.enable")}
          description={t("settings.app_rules.enable_desc")}
        >
          <Switch
            checked={perAppOn}
            onChange={(v) => void update("per_app_mode", v)}
          />
        </SettingsRow>
      </SettingsGroup>

      <div className={`transition-opacity ${!perAppOn ? "pointer-events-none opacity-50" : ""}`}>
        <SettingsGroup title={t("settings.app_rules.add_group")}>
          <div className="flex flex-col gap-3 px-4 py-3.5">
            <div className="flex items-end gap-2">
              <div className="flex-1">
                <Input
                  label={t("settings.app_rules.proc_name")}
                  placeholder="slack"
                  value={proc}
                  onChange={(e) => setProc(e.target.value)}
                />
              </div>
              <Button variant="ghost" onClick={() => void getActive()} title={t("settings.app_rules.detect_btn")}>
                <Crosshair className="h-4 w-4" /> {t("settings.app_rules.detect_btn")}
              </Button>
            </div>

            <div className="flex items-end gap-2">
              <div className="flex-1">
                <Select
                  label={t("settings.app_rules.mode_apply")}
                  options={[
                    { value: "dictation", label: t("settings.modes.dictation") },
                    { value: "message", label: t("settings.modes.message") },
                    { value: "email", label: t("settings.modes.email") },
                  ]}
                  value={mode}
                  onChange={(e) => setMode(e.target.value)}
                />
              </div>
              <Button variant="primary" onClick={() => void handleAdd()}>
                <Plus className="h-4 w-4" /> {t("settings.app_rules.add_btn")}
              </Button>
            </div>
            {activeApp && (
              <p className="text-xs text-vx-accent">
                {t("settings.app_rules.detected_app", { app: activeApp })}
              </p>
            )}
          </div>
        </SettingsGroup>

        <SettingsGroup title={t("settings.app_rules.active_group")}>
          {modes.length === 0 ? (
            <div className="px-4 py-6 text-center text-sm text-vx-text-dim">
              {t("settings.app_rules.empty")}
            </div>
          ) : (
            modes.map((m) => (
              <div key={m.id} className="flex items-center justify-between px-4 py-3">
                <div className="flex flex-col">
                  <span className="text-sm font-semibold text-vx-text-primary">
                    {m.app_process_name}
                  </span>
                  <span className="text-xs text-vx-text-dim capitalize">
                    {t("settings.app_rules.applies_mode", { mode: t(`settings.modes.${m.mode_id}`) })}
                  </span>
                </div>
                <button
                  type="button"
                  onClick={() => void handleRemove(m.id)}
                  className="rounded-lg p-1.5 text-vx-text-dim transition-colors hover:bg-vx-error/15 hover:text-vx-error"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            ))
          )}
        </SettingsGroup>
      </div>
    </div>
  );
}
