import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";

export function STTTab() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title="Speech-to-Text"
        description="Groq Whisper cloud transcription."
      />

      <SettingsGroup title="Engine">
        <SettingsRow label="Language">
          <Select
            options={[
              { value: "auto", label: "Auto detect" },
              { value: "id", label: "Bahasa Indonesia" },
              { value: "en", label: "English" },
            ]}
            value={(settings.stt_language as string) ?? "auto"}
            onChange={(e) => void update("stt_language", e.target.value)}
            className="w-48"
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title="Groq credentials">
        <div className="px-4 py-3.5">
          <Input
            label="Groq API Key"
            type="password"
            placeholder={
              (settings.groq_api_key_set as boolean)
                ? "•••••••• (saved)"
                : "gsk_..."
            }
            value={(settings.groq_api_key as string) ?? ""}
            onChange={(e) => void update("groq_api_key", e.target.value)}
            hint="Get a free key at console.groq.com — stored encrypted."
          />
        </div>
      </SettingsGroup>
    </div>
  );
}
