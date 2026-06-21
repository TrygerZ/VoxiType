import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";

export function LLMTab() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const engine = (settings.llm_engine as string) ?? "ollama";

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title="AI Formatting"
        description="Clean up and format transcribed text. Falls back to rule-based if unavailable."
      />

      <SettingsGroup title="Engine">
        <SettingsRow label="Formatter">
          <Select
            options={[
              { value: "ollama", label: "Ollama (Local)" },
              { value: "groq", label: "Groq Llama 3.1 (Cloud)" },
              { value: "rule_based", label: "Rule-based (No LLM)" },
              { value: "off", label: "Off" },
            ]}
            value={engine}
            onChange={(e) => void update("llm_engine", e.target.value)}
            className="w-52"
          />
        </SettingsRow>
        {engine === "ollama" && (
          <div className="px-4 py-3.5">
            <Input
              label="Ollama Model"
              placeholder="qwen2.5:3b"
              value={(settings.llm_model as string) ?? "qwen2.5:3b"}
              onChange={(e) => void update("llm_model", e.target.value)}
              hint="Make sure Ollama is running locally on port 11434."
            />
          </div>
        )}
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
              hint="Shared with the STT setting — stored encrypted."
            />
          </div>
        </SettingsGroup>
      )}
    </div>
  );
}
