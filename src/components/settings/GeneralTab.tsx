import { useSettingsStore } from "../../stores/settingsStore";
import { Switch } from "../ui/Switch";
import { Select } from "../ui/Select";

export function GeneralTab() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);

  return (
    <div className="flex max-w-md flex-col gap-5">
      <h2 className="text-lg font-semibold">General</h2>

      <Select
        label="UI Language"
        options={[
          { value: "id", label: "Bahasa Indonesia" },
          { value: "en", label: "English" },
        ]}
        value={(settings.language as string) ?? "id"}
        onChange={(e) => void update("language", e.target.value)}
      />

      <Switch
        label="Start with Windows"
        checked={(settings.auto_start as boolean) ?? false}
        onChange={(v) => void update("auto_start", v)}
      />

      <Switch
        label="Automatic updates"
        checked={(settings.auto_update as boolean) ?? true}
        onChange={(v) => void update("auto_update", v)}
      />

      <Switch
        label="Sound cues"
        checked={(settings.sound_cues as boolean) ?? false}
        onChange={(v) => void update("sound_cues", v)}
      />
    </div>
  );
}
