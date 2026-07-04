import { useState } from "react";
import { Check, Loader2, XCircle } from "lucide-react";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";
import { testGroqApi, formatTauriError } from "../../lib/tauri";
import { toast } from "../ui/Toast";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";

export function STTTab() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const [testStatus, setTestStatus] = useState<"idle" | "testing" | "ok" | "fail" | "err">("idle");

  const handleApiKeyChange = async (value: string) => {
    try {
      await update("groq_api_key", value);
      if (value.trim()) {
        toast("API key saved");
      }
    } catch (e: unknown) {
      toast(formatTauriError(e), "error");
    }
  };

  const handleTestApi = async () => {
    const key = (settings.groq_api_key as string) ?? "";
    if (!key.trim() && !(settings.groq_api_key_set as boolean)) return;
    setTestStatus("testing");
    try {
      await testGroqApi(key.trim());
      setTestStatus("ok");
      toast("Connection successful!");
      setTimeout(() => setTestStatus("idle"), 3000);
    } catch (e: unknown) {
      const code = (e as { code?: string })?.code;
      if (code === "SttApiKeyInvalid") {
        setTestStatus("fail");
        toast("Invalid API key", "error");
      } else {
        setTestStatus("err");
        toast(formatTauriError(e), "error");
      }
      setTimeout(() => setTestStatus("idle"), 3000);
    }
  };

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
            onChange={(e) => void handleApiKeyChange(e.target.value)}
            hint="Get a free key at console.groq.com — stored encrypted."
          />
          <button
            type="button"
            onClick={handleTestApi}
            disabled={testStatus === "testing" || (!(settings.groq_api_key as string)?.trim() && !(settings.groq_api_key_set as boolean))}
            className={`mt-3 w-full flex items-center justify-center gap-2 rounded-lg border px-4 py-2.5 text-sm font-medium transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed ${
              testStatus === "ok"
                ? "border-green-500/40 bg-green-500/10 text-green-600"
                : testStatus === "fail"
                  ? "border-red-500/40 bg-red-500/10 text-red-600"
                  : testStatus === "err"
                    ? "border-amber-500/40 bg-amber-500/10 text-amber-600"
                    : testStatus === "testing"
                      ? "border-vx-accent/40 bg-vx-accent/10 text-vx-accent"
                      : "border-vx-border bg-vx-bg-tertiary/60 text-vx-text-secondary hover:border-vx-border-strong hover:text-vx-text-primary"
            }`}
          >
            {testStatus === "testing" ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : testStatus === "ok" ? (
              <Check className="h-4 w-4" />
            ) : testStatus === "fail" || testStatus === "err" ? (
              <XCircle className="h-4 w-4" />
            ) : null}
            {testStatus === "testing"
              ? "Testing..."
              : testStatus === "ok"
                ? "Connected!"
                : testStatus === "fail"
                  ? "Invalid key"
                  : testStatus === "err"
                    ? "Connection failed"
                    : "Test Connection"}
          </button>
        </div>
      </SettingsGroup>
    </div>
  );
}
