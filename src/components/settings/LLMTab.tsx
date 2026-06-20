import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";

export function LLMTab() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);

  return (
    <div className="flex max-w-md flex-col gap-5">
      <h2 className="text-lg font-semibold">LLM Formatting</h2>

      <Select
        label="Engine"
        options={[
          { value: "ollama", label: "Ollama (Local)" },
          { value: "groq", label: "Groq Llama 3.1 (Cloud)" },
          { value: "rule_based", label: "Rule-based (No LLM)" },
          { value: "off", label: "Off" },
        ]}
        value={(settings.llm_engine as string) ?? "ollama"}
        onChange={(e) => void update("llm_engine", e.target.value)}
      />

      <Input
        label="Ollama Model"
        placeholder="qwen2.5:3b"
        value={(settings.llm_model as string) ?? "qwen2.5:3b"}
        onChange={(e) => void update("llm_model", e.target.value)}
      />

      <Input
        label="Groq API Key"
        type="password"
        placeholder="gsk_..."
        value={(settings.groq_api_key as string) ?? ""}
        onChange={(e) => void update("groq_api_key", e.target.value)}
      />

      <Select
        label="Active Mode"
        options={[
          { value: "dictation", label: "Dictation" },
          { value: "message", label: "Message" },
          { value: "email", label: "Email" },
        ]}
        value={(settings.active_mode as string) ?? "dictation"}
        onChange={(e) => void update("active_mode", e.target.value)}
      />
    </div>
  );
}
