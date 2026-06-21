import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";
import { useSettingsStore } from "../../stores/settingsStore";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";

export function ModesTab() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const translationOn = (settings.translation_enabled as boolean) ?? false;

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title="Modes"
        description="Control how transcribed text is formatted and translated."
      />

      <SettingsGroup title="Formatting mode">
        <SettingsRow
          label="Active mode"
          description="Dictation is raw, Message is casual, Email is formal."
        >
          <Select
            options={[
              { value: "dictation", label: "Dictation" },
              { value: "message", label: "Message" },
              { value: "email", label: "Email" },
            ]}
            value={(settings.active_mode as string) ?? "dictation"}
            onChange={(e) => void update("active_mode", e.target.value)}
            className="w-44"
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title="Translation">
        <SettingsRow
          label="Translate after transcription"
          description="Convert dictated text to another language before inserting."
        >
          <Switch
            checked={translationOn}
            onChange={(v) => void update("translation_enabled", v)}
          />
        </SettingsRow>
        {translationOn && (
          <SettingsRow label="Target language">
            <Select
              options={[
                { value: "en", label: "English" },
                { value: "id", label: "Bahasa Indonesia" },
              ]}
              value={(settings.translation_target as string) ?? "en"}
              onChange={(e) => void update("translation_target", e.target.value)}
              className="w-44"
            />
          </SettingsRow>
        )}
      </SettingsGroup>
    </div>
  );
}
