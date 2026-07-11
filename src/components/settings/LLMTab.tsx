import { useState } from "react";
import { Check, Loader2, XCircle } from "lucide-react";
import { Input } from "../ui/Input";
import { useDebouncedApiKey } from "../../hooks/useDebouncedApiKey";
import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";
import { useT } from "../../lib/i18n";
import { testGroqApi, formatTauriError } from "../../lib/tauri";
import { toast } from "../ui/Toast";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";

export function LLMTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const engine = (settings.llm_engine as string) ?? "ollama";
  const [testStatus, setTestStatus] = useState<"idle" | "testing" | "ok" | "fail" | "err">("idle");

  const { localKey, onKeyChange } = useDebouncedApiKey(
    (settings.groq_api_key as string) ?? "",
    update,
    t("settings.stt.saved"),
  );

  const handleTestApi = async () => {
    if (!localKey.trim() && !(settings.groq_api_key_set as boolean)) return;
    setTestStatus("testing");
    try {
      await testGroqApi(localKey.trim());
      setTestStatus("ok");
      toast(t("settings.llm.connected"));
      setTimeout(() => setTestStatus("idle"), 3000);
    } catch (e: unknown) {
      const code = (e as { code?: string })?.code;
      if (code === "SttApiKeyInvalid") {
        setTestStatus("fail");
        toast(t("settings.llm.invalid_key"), "error");
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
        title={t("settings.llm.title")}
        description={t("settings.llm.desc")}
      />

      <SettingsGroup title={t("settings.llm.engine_group")}>
        <SettingsRow label={t("settings.llm.formatter")}>
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
              label={t("settings.llm.ollama_model")}
              placeholder="qwen2.5:3b"
              value={(settings.llm_model as string) ?? "qwen2.5:3b"}
              onChange={(e) => void update("llm_model", e.target.value)}
              hint={t("settings.llm.ollama_hint")}
            />
          </div>
        )}
      </SettingsGroup>

      {engine === "groq" && (
        <SettingsGroup title={t("settings.llm.groq_group")}>
          <div className="px-4 py-3.5">
            <Input
              label={t("settings.llm.api_key")}
              type="password"
              showPasswordToggle
              placeholder={
                (settings.groq_api_key_set as boolean)
                  ? t("settings.llm.saved_placeholder")
                  : "gsk_..."
              }
              value={localKey}
              onChange={(e) => onKeyChange(e.target.value)}
              hint={t("settings.llm.api_key_hint")}
            />
            <button
              type="button"
              onClick={handleTestApi}
              disabled={testStatus === "testing" || (!localKey.trim() && !(settings.groq_api_key_set as boolean))}
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
                ? t("settings.llm.testing")
                : testStatus === "ok"
                  ? t("settings.llm.connected")
                  : testStatus === "fail"
                    ? t("settings.llm.invalid_key")
                    : testStatus === "err"
                      ? t("settings.llm.conn_fail")
                      : t("settings.llm.test_conn")}
            </button>
          </div>
        </SettingsGroup>
      )}
    </div>
  );
}
