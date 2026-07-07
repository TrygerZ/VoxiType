import { useEffect, useState } from "react";
import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";
import { getMicrophones } from "../../lib/tauri";
import type { DeviceInfo } from "../../types/app";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";
import { useT } from "../../lib/i18n";

export function AudioTab() {
  const t = useT();
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
        title={t("settings.audio.title")}
        description={t("settings.audio.desc")}
      />

      <SettingsGroup title={t("settings.audio.device_group")}>
        <SettingsRow label={t("settings.audio.mic")}>
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
