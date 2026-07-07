import { useState, useEffect } from "react";
import { Check } from "lucide-react";
import { Select } from "../ui/Select";
import { Button } from "../ui/Button";
import { useSettingsStore } from "../../stores/settingsStore";
import { useT } from "../../lib/i18n";
import { setHotkey, formatTauriError } from "../../lib/tauri";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";
import { HotkeyRecorder } from "./HotkeyRecorder";

export function ShortcutsTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const loadSettings = useSettingsStore((s) => s.load);

  const hotkeyRaw = settings.hotkey as
    | { key: string; mode: string }
    | undefined;
  const [key, setKey] = useState(hotkeyRaw?.key ?? "Ctrl+Space");
  const [mode, setMode] = useState(hotkeyRaw?.mode ?? "ptt");
  const [status, setStatus] = useState<"idle" | "ok" | string>("idle");

  // Keep the editor in sync with the stored hotkey. Settings may load after
  // this tab mounts (or change elsewhere), so mirror the store whenever the
  // persisted hotkey changes to avoid showing a stale combination.
  useEffect(() => {
    if (hotkeyRaw?.key) setKey(hotkeyRaw.key);
    if (hotkeyRaw?.mode) setMode(hotkeyRaw.mode);
  }, [hotkeyRaw?.key, hotkeyRaw?.mode]);

  const handleApply = async () => {
    try {
      await setHotkey(key, mode);
      await loadSettings();
      setStatus("ok");
      setTimeout(() => setStatus("idle"), 2000);
    } catch (e: unknown) {
      setStatus(formatTauriError(e));
    }
  };

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title={t("settings.shortcuts.title")}
        description={t("settings.shortcuts.desc")}
      />

      <SettingsGroup title={t("settings.shortcuts.group")}>
        <div className="px-4 py-3.5">
          <HotkeyRecorder value={key} onChange={setKey} />
        </div>
        <SettingsRow
          label={t("settings.shortcuts.mode")}
          description={t("settings.shortcuts.mode_desc")}
        >
          <Select
            options={[
              { value: "ptt", label: "Push-to-Talk" },
              { value: "toggle", label: "Toggle" },
            ]}
            value={mode}
            onChange={(e) => setMode(e.target.value)}
            className="w-44"
          />
        </SettingsRow>
      </SettingsGroup>

      <div className="flex items-center gap-3">
        <Button variant="primary" size="sm" onClick={() => void handleApply()}>
          {t("settings.shortcuts.apply_btn")}
        </Button>
        {status === "ok" && (
          <span className="flex items-center gap-1 text-xs text-vx-success">
            <Check className="h-3.5 w-3.5" /> {t("settings.shortcuts.applied")}
          </span>
        )}
        {status !== "ok" && status !== "idle" && (
          <span className="text-xs text-vx-error">{status}</span>
        )}
      </div>
    </div>
  );
}
