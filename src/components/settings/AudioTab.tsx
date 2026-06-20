import { useEffect, useState } from "react";
import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";
import { getMicrophones } from "../../lib/tauri";
import type { DeviceInfo } from "../../types/app";

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
    <div className="flex max-w-md flex-col gap-5">
      <h2 className="text-lg font-semibold">Audio</h2>

      <Select
        label="Microphone"
        options={devices.map((d) => ({ value: d.id, label: d.name }))}
        value={(settings.mic_device as string) ?? "default"}
        onChange={(e) => void update("mic_device", e.target.value)}
      />
    </div>
  );
}
