import { useEffect, useRef, useState } from "react";
import { Check } from "lucide-react";

import { useT } from "../../lib/i18n";
import { isHotkeyConfig } from "../../lib/settingsGuards";
import { formatTauriError, setHotkey } from "../../lib/tauri";
import { usePlatform } from "../../lib/usePlatform";
import { useSettingsStore } from "../../stores/settingsStore";
import { Button } from "../ui/Button";
import { Select } from "../ui/Select";
import { StatusBadge } from "../ui/StatusBadge";
import { HotkeyRecorder } from "./HotkeyRecorder";
import { SettingsGroup, SettingsHeader, SettingsRow } from "./SettingsLayout";

export function ShortcutsTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const loadSettings = useSettingsStore((s) => s.load);

  const hotkey = isHotkeyConfig(settings.hotkey) ? settings.hotkey : undefined;
  const [key, setKey] = useState(hotkey?.key ?? "Ctrl+Space");
  const [mode, setMode] = useState(hotkey?.mode ?? "ptt");
  const [status, setStatus] = useState<"idle" | "ok" | string>("idle");
  const [busy, setBusy] = useState(false);
  const platform = usePlatform();
  const statusTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Keep the editor in sync with the stored hotkey. Settings may load after
  // this tab mounts (or change elsewhere), so mirror the store whenever the
  // persisted hotkey changes to avoid showing a stale combination.
  useEffect(() => {
    if (hotkey?.key) setKey(hotkey.key);
    if (hotkey?.mode) setMode(hotkey.mode);
  }, [hotkey?.key, hotkey?.mode]);

  // Cancel the pending "applied" reset timer on unmount.
  useEffect(() => {
    return () => {
      if (statusTimerRef.current) clearTimeout(statusTimerRef.current);
    };
  }, []);

  const handleApply = async () => {
    if (busy) return;
    // Clear any pending reset timer so a prior success can't erase this
    // attempt's error (or stale "ok") a moment later.
    if (statusTimerRef.current) {
      clearTimeout(statusTimerRef.current);
      statusTimerRef.current = null;
    }
    setBusy(true);
    try {
      await setHotkey(key, mode);
      await loadSettings();
      setStatus("ok");
      if (statusTimerRef.current) clearTimeout(statusTimerRef.current);
      statusTimerRef.current = setTimeout(() => {
        setStatus("idle");
        statusTimerRef.current = null;
      }, 2000);
    } catch (e: unknown) {
      setStatus(formatTauriError(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <SettingsHeader
        title={t("settings.shortcuts.title")}
        description={t("settings.shortcuts.desc")}
      />

      <SettingsGroup title={t("settings.shortcuts.group")}>
        <div className="px-4 py-4">
          <HotkeyRecorder value={key} onChange={setKey} platform={platform} />
        </div>
        <SettingsRow
          label={t("settings.shortcuts.mode")}
          description={t("settings.shortcuts.mode_desc")}
        >
          <Select
            aria-label={t("settings.shortcuts.mode")}
            options={[
              { value: "ptt", label: "Push-to-Talk" },
              { value: "toggle", label: "Toggle" },
            ]}
            value={mode}
            onChange={(e) => setMode(e.target.value)}
            className="w-full sm:w-44"
          />
        </SettingsRow>
      </SettingsGroup>

      <div className="flex items-center gap-3">
        <Button
          variant="primary"
          size="sm"
          onClick={() => void handleApply()}
          disabled={busy}
        >
          {t("settings.shortcuts.apply_btn")}
        </Button>
        {status === "ok" && (
          <StatusBadge tone="success">
            <Check className="h-3.5 w-3.5" />
            {t("settings.shortcuts.applied")}
          </StatusBadge>
        )}
        {status !== "ok" && status !== "idle" && (
          <StatusBadge tone="error">{status}</StatusBadge>
        )}
      </div>
    </div>
  );
}
