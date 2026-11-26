import { useEffect, useRef, useState } from "react";
import { Check, Loader2, XCircle } from "lucide-react";

import { useDebouncedApiKey } from "../../hooks/useDebouncedApiKey";
import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import { getBooleanSetting, getStringSetting } from "../../lib/settingsGuards";
import {
  errorCode,
  formatTauriError,
  testGroqApi,
  testOllama,
} from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { StatusBadge } from "../ui/StatusBadge";
import { toast } from "../ui/Toast";
import { SettingsGroup, SettingsHeader, SettingsRow } from "./SettingsLayout";

type TestStatus = "idle" | "testing" | "ok" | "fail" | "err";

interface OllamaTestState {
  status: "idle" | "testing" | "ok" | "err";
  model?: string;
  error?: string;
}

export function LLMTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const engine = getStringSetting(settings.llm_engine, "ollama");
  const ollamaModel = getStringSetting(settings.llm_model, "qwen2.5:3b");
  const groqKeySet = getBooleanSetting(settings.groq_api_key_set, false);
  const [testStatus, setTestStatus] = useState<TestStatus>("idle");
  const [ollamaTest, setOllamaTest] = useState<OllamaTestState>({
    status: "idle",
  });
  const testTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (testTimerRef.current) clearTimeout(testTimerRef.current);
    };
  }, []);

  const { localKey, onKeyChange } = useDebouncedApiKey(
    getStringSetting(settings.groq_api_key, ""),
    update,
    t("settings.stt.saved"),
  );

  const handleTestApi = async () => {
    if (!localKey.trim() && !groqKeySet) return;
    if (testTimerRef.current) {
      clearTimeout(testTimerRef.current);
      testTimerRef.current = null;
    }
    setTestStatus("testing");
    try {
      await testGroqApi(localKey.trim() || null);
      setTestStatus("ok");
      toast(t("settings.llm.connected"));
    } catch (e: unknown) {
      const code = errorCode(e);
      if (code === "LlmApiKeyInvalid" || code === "SttApiKeyInvalid") {
        setTestStatus("fail");
        toast(t("settings.llm.invalid_key"), "error");
      } else {
        setTestStatus("err");
        toast(formatTauriError(e), "error");
      }
    } finally {
      testTimerRef.current = setTimeout(() => {
        setTestStatus("idle");
        testTimerRef.current = null;
      }, 3000);
    }
  };

  const handleTestOllama = async () => {
    const trimmed = ollamaModel.trim();
    if (!trimmed) return;
    setOllamaTest({ status: "testing" });
    try {
      await testOllama(trimmed);
      setOllamaTest({ status: "ok", model: trimmed });
      toast(t("settings.llm.ollama_ready", { model: trimmed }));
    } catch (e: unknown) {
      const msg = formatTauriError(e);
      setOllamaTest({ status: "err", error: msg });
      toast(msg, "error");
    }
  };

  return (
    <div>
      <SettingsHeader
        title={t("settings.llm.title")}
        description={t("settings.llm.desc")}
      />

      <SettingsGroup title={t("settings.llm.engine_group")}>
        <SettingsRow label={t("settings.llm.formatter")}>
          <Select
            aria-label={t("settings.llm.formatter")}
            options={[
              { value: "ollama", label: "Ollama (Local)" },
              { value: "groq", label: "Groq Llama 3.1 (Cloud)" },
              { value: "rule_based", label: "Rule-based (No LLM)" },
              { value: "off", label: "Off" },
            ]}
            value={engine}
            onChange={(e) =>
              void invokeAction(() => update("llm_engine", e.target.value))
            }
            className="w-52"
          />
        </SettingsRow>
      </SettingsGroup>

      {engine === "ollama" && (
        <SettingsGroup title={t("settings.llm.ollama_group")}>
          <div className="flex flex-col gap-3 px-4 py-4">
            <Input
              label={t("settings.llm.ollama_model")}
              placeholder="qwen2.5:3b"
              value={ollamaModel}
              onChange={(e) => {
                setOllamaTest({ status: "idle" });
                void invokeAction(() => update("llm_model", e.target.value));
              }}
            />
            <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
              <span className="text-xs text-vx-text-dim">
                {t("settings.llm.ollama_hint")}
              </span>
              <div className="flex items-center gap-2.5">
                {ollamaTest.status === "testing" && (
                  <StatusBadge tone="ember">
                    <Loader2 className="h-3 w-3 animate-spin" />
                    {t("settings.llm.testing")}
                  </StatusBadge>
                )}
                {ollamaTest.status === "ok" && ollamaTest.model && (
                  <StatusBadge tone="success">
                    <Check className="h-3 w-3" />
                    {t("settings.llm.ollama_ready", {
                      model: ollamaTest.model,
                    })}
                  </StatusBadge>
                )}
                {ollamaTest.status === "err" && ollamaTest.error && (
                  <StatusBadge tone="error">
                    <XCircle className="h-3 w-3" />
                    {ollamaTest.error}
                  </StatusBadge>
                )}
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => void handleTestOllama()}
                  disabled={
                    ollamaTest.status === "testing" || !ollamaModel.trim()
                  }
                >
                  {t("settings.llm.test_ollama")}
                </Button>
              </div>
            </div>
          </div>
        </SettingsGroup>
      )}

      {engine === "groq" && (
        <SettingsGroup title={t("settings.llm.groq_group")}>
          <div className="flex flex-col gap-3 px-4 py-4">
            <div className="flex items-center justify-between gap-2">
              <span className="text-xs font-medium text-vx-text-secondary">
                {t("settings.llm.api_key")}
              </span>
              <StatusBadge tone="neutral">
                {t("settings.stt.shared_with_llm")}
              </StatusBadge>
            </div>
            <Input
              aria-label={t("settings.llm.api_key")}
              type="password"
              showPasswordToggle
              placeholder={
                groqKeySet ? t("settings.llm.saved_placeholder") : "gsk_..."
              }
              value={localKey}
              onChange={(e) => onKeyChange(e.target.value)}
            />
            <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
              <span className="text-xs text-vx-text-dim">
                {t("settings.llm.api_key_hint")}
              </span>
              <div className="flex items-center gap-2.5">
                <GroqStatusBadge status={testStatus} t={t} />
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => void handleTestApi()}
                  disabled={
                    testStatus === "testing" ||
                    (!localKey.trim() && !groqKeySet)
                  }
                >
                  {t("settings.llm.test_conn")}
                </Button>
              </div>
            </div>
          </div>
        </SettingsGroup>
      )}
    </div>
  );
}

function GroqStatusBadge({
  status,
  t,
}: {
  status: TestStatus;
  t: (key: string) => string;
}) {
  if (status === "idle") return null;
  if (status === "testing") {
    return (
      <StatusBadge tone="ember">
        <Loader2 className="h-3 w-3 animate-spin" />
        {t("settings.llm.testing")}
      </StatusBadge>
    );
  }
  if (status === "ok") {
    return (
      <StatusBadge tone="success">
        <Check className="h-3 w-3" />
        {t("settings.llm.connected")}
      </StatusBadge>
    );
  }
  return (
    <StatusBadge tone={status === "fail" ? "error" : "warning"}>
      <XCircle className="h-3 w-3" />
      {status === "fail"
        ? t("settings.llm.invalid_key")
        : t("settings.llm.conn_fail")}
    </StatusBadge>
  );
}
