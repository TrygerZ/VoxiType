import { useState, useEffect } from "react";
import { Check } from "lucide-react";
import { Select } from "../ui/Select";
import { Button } from "../ui/Button";
import { useSettingsStore } from "../../stores/settingsStore";
import { setHotkey } from "../../lib/tauri";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";
import { HotkeyRecorder } from "./HotkeyRecorder";

export function ShortcutsTab() {
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
      setStatus(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title="Shortcuts"
        description="Set the global hotkey used to start dictation in any app."
      />

      <SettingsGroup title="Global hotkey">
        <div className="px-4 py-3.5">
          <HotkeyRecorder value={key} onChange={setKey} />
        </div>
        <SettingsRow
          label="Recording mode"
          description="Push-to-talk holds; toggle starts/stops on press."
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
          Apply hotkey
        </Button>
        {status === "ok" && (
          <span className="flex items-center gap-1 text-xs text-vx-success">
            <Check className="h-3.5 w-3.5" /> Applied
          </span>
        )}
        {status !== "ok" && status !== "idle" && (
          <span className="text-xs text-vx-error">{status}</span>
        )}
      </div>
    </div>
  );
}
