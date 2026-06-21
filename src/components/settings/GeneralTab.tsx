import { useSettingsStore } from "../../stores/settingsStore";
import { Switch } from "../ui/Switch";
import { Select } from "../ui/Select";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";

export function GeneralTab() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title="General"
        description="Language, startup behavior, and privacy preferences."
      />

      <SettingsGroup title="Appearance">
        <SettingsRow label="Interface language">
          <Select
            options={[
              { value: "id", label: "Bahasa Indonesia" },
              { value: "en", label: "English" },
            ]}
            value={(settings.language as string) ?? "id"}
            onChange={(e) => void update("language", e.target.value)}
            className="w-48"
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title="System">
        <SettingsRow
          label="Start with Windows"
          description="Launch VoxiType automatically on login."
        >
          <Switch
            checked={(settings.auto_start as boolean) ?? false}
            onChange={(v) => void update("auto_start", v)}
          />
        </SettingsRow>
        <SettingsRow
          label="Automatic updates"
          description="Check for new versions on launch."
        >
          <Switch
            checked={(settings.auto_update as boolean) ?? true}
            onChange={(v) => void update("auto_update", v)}
          />
        </SettingsRow>
        <SettingsRow
          label="Sound cues"
          description="Play a tone when recording starts and stops."
        >
          <Switch
            checked={(settings.sound_cues as boolean) ?? false}
            onChange={(v) => void update("sound_cues", v)}
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title="Input &amp; privacy">
        <SettingsRow
          label="Command mode"
          description="Spoken commands like 'new line' trigger keystrokes."
        >
          <Switch
            checked={(settings.command_mode as boolean) ?? false}
            onChange={(v) => void update("command_mode", v)}
          />
        </SettingsRow>
        <SettingsRow
          label="Anonymous usage stats"
          description="Stored locally only — never transmitted."
        >
          <Switch
            checked={(settings.telemetry as boolean) ?? false}
            onChange={(v) => void update("telemetry", v)}
          />
        </SettingsRow>
      </SettingsGroup>
    </div>
  );
}
