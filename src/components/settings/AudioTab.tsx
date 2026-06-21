import { useEffect, useState } from "react";
import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";
import { getMicrophones } from "../../lib/tauri";
import type { DeviceInfo } from "../../types/app";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";

export function AudioTab() {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);

  useEffect(() => {
    getMicrophones()
      .then(setDevices)
      .catch(() => {});
  }, []);

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title="Audio"
        description="Choose the microphone used for dictation."
      />

      <SettingsGroup title="Input device">
        <SettingsRow label="Microphone">
          <Select
            options={devices.map((d) => ({ value: d.id, label: d.name }))}
            value={(settings.mic_device as string) ?? "default"}
            onChange={(e) => void update("mic_device", e.target.value)}
            className="w-56"
          />
        </SettingsRow>
      </SettingsGroup>
    </div>
  );
}
