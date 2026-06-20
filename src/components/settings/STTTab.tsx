import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";

export function STTTab() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);

  return (
    <div className="flex max-w-md flex-col gap-5">
      <h2 className="text-lg font-semibold">Speech-to-Text</h2>

      <Select
        label="Engine"
        options={[
          { value: "whisper_cpp", label: "Whisper.cpp (Local)" },
          { value: "groq", label: "Groq Whisper (Cloud)" },
        ]}
        value={(settings.stt_engine as string) ?? "whisper_cpp"}
        onChange={(e) => void update("stt_engine", e.target.value)}
      />

      <Select
        label="Whisper Model"
        options={[
          { value: "tiny", label: "Tiny (142 MB)" },
          { value: "base", label: "Base (232 MB)" },
          { value: "small", label: "Small (466 MB)" },
          { value: "medium", label: "Medium (1.5 GB)" },
        ]}
        value={(settings.stt_model as string) ?? "small"}
        onChange={(e) => void update("stt_model", e.target.value)}
      />

      <Select
        label="Language"
        options={[
          { value: "auto", label: "Auto detect" },
          { value: "id", label: "Bahasa Indonesia" },
          { value: "en", label: "English" },
        ]}
        value={(settings.language as string) ?? "auto"}
        onChange={(e) => void update("language", e.target.value)}
      />
    </div>
  );
}
