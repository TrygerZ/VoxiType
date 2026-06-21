import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";

export function STTTab() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const engine = (settings.stt_engine as string) ?? "whisper_cpp";

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title="Speech-to-Text"
        description="Pick the engine that converts your voice to text."
      />

      <SettingsGroup title="Engine">
        <SettingsRow
          label="STT engine"
          description="Groq is recommended unless you build with local Whisper."
        >
          <Select
            options={[
              { value: "whisper_cpp", label: "Whisper.cpp (Local)" },
              { value: "groq", label: "Groq Whisper (Cloud)" },
            ]}
            value={engine}
            onChange={(e) => void update("stt_engine", e.target.value)}
            className="w-48"
          />
        </SettingsRow>
        <SettingsRow
          label="Whisper model"
          description="Larger models are more accurate but slower."
        >
          <Select
            options={[
              { value: "tiny", label: "Tiny (142 MB)" },
              { value: "base", label: "Base (232 MB)" },
              { value: "small", label: "Small (466 MB)" },
              { value: "medium", label: "Medium (1.5 GB)" },
            ]}
            value={(settings.stt_model as string) ?? "small"}
            onChange={(e) => void update("stt_model", e.target.value)}
            className="w-48"
          />
        </SettingsRow>
        <SettingsRow label="Language">
          <Select
            options={[
              { value: "auto", label: "Auto detect" },
              { value: "id", label: "Bahasa Indonesia" },
              { value: "en", label: "English" },
            ]}
            value={(settings.language as string) ?? "auto"}
            onChange={(e) => void update("language", e.target.value)}
            className="w-48"
          />
        </SettingsRow>
      </SettingsGroup>

      {engine === "groq" && (
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
      )}
    </div>
  );
}
