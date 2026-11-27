import { useEffect, useRef, useState } from "react";
import { Check, FolderOpen, Loader2, XCircle } from "lucide-react";

import { useDebouncedApiKey } from "../../hooks/useDebouncedApiKey";
import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import {
  clampWhisperThreads,
  getNumberSetting,
  getStringSetting,
} from "../../lib/settingsGuards";
import {
  errorCode,
  formatTauriError,
  pickSetupFile,
  testGroqApi,
  testWhisperCpp,
} from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { StatusBadge } from "../ui/StatusBadge";
import { toast } from "../ui/Toast";
import { SettingsGroup, SettingsHeader, SettingsRow } from "./SettingsLayout";

type TestStatus = "idle" | "testing" | "ok" | "fail" | "err";

export function STTTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const updateWhisperPaths = useSettingsStore((s) => s.updateWhisperPaths);
  const [groqStatus, setGroqStatus] = useState<TestStatus>("idle");
  const [whisperStatus, setWhisperStatus] = useState<TestStatus>("idle");
  const groqTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const whisperTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (groqTimerRef.current) clearTimeout(groqTimerRef.current);
      if (whisperTimerRef.current) clearTimeout(whisperTimerRef.current);
    };
  }, []);

  const engine = getStringSetting(settings.stt_engine, "groq");
  const language = getStringSetting(settings.stt_language, "auto");
  const groqKey = getStringSetting(settings.groq_api_key, "");
  const groqKeySet = settings.groq_api_key_set === true;
  const whisperBinary = getStringSetting(settings.whisper_cpp_binary_path, "");
  const whisperModel = getStringSetting(settings.whisper_cpp_model_path, "");
  const whisperThreads = getNumberSetting(settings.whisper_cpp_threads, 4);

  const { localKey, onKeyChange } = useDebouncedApiKey(
    groqKey,
    update,
    t("settings.stt.saved"),
  );

  const handleTestApi = async () => {
    if (!localKey.trim() && !groqKeySet) return;
    await runStatus(setGroqStatus, groqTimerRef, async () => {
      await testGroqApi(localKey.trim() || null);
      toast(t("settings.stt.connected"));
    });
  };

  const handleTestWhisper = async () => {
    if (!whisperBinary.trim() || !whisperModel.trim()) return;
    await runStatus(setWhisperStatus, whisperTimerRef, async () => {
      await testWhisperCpp(
        whisperBinary.trim(),
        whisperModel.trim(),
        language,
        whisperThreads,
      );
      toast(t("settings.stt.ready"));
    });
  };

  // Whisper paths are only writable via the picker-gated backend command;
  // the backend refuses any path not returned by the native dialog.
  const handlePickBinary = async () => {
    await invokeAction(async () => {
      const file = await pickSetupFile("whisper_binary");
      if (file) await updateWhisperPaths(file, null);
    });
  };

  const handlePickModel = async () => {
    await invokeAction(async () => {
      const file = await pickSetupFile("whisper_model");
      if (file) await updateWhisperPaths(null, file);
    });
  };

  return (
    <div>
      <SettingsHeader
        title={t("settings.stt.title")}
        description={t("settings.stt.desc")}
      />

      <SettingsGroup title={t("settings.stt.engine_group")}>
        <SettingsRow label={t("settings.stt.engine")}>
          <Select
            aria-label={t("settings.stt.engine")}
            options={[
              { value: "groq", label: "Groq Whisper" },
              { value: "whisper_cpp", label: "Offline whisper.cpp" },
            ]}
            value={engine}
            onChange={(e) =>
              void invokeAction(() => update("stt_engine", e.target.value))
            }
            className="w-full sm:w-48"
          />
        </SettingsRow>
        <SettingsRow label={t("settings.stt.lang")}>
          <Select
            aria-label={t("settings.stt.lang")}
            options={[
              { value: "auto", label: t("settings.stt.lang_auto") },
              { value: "id", label: "Bahasa Indonesia" },
              { value: "en", label: "English" },
            ]}
            value={language}
            onChange={(e) =>
              void invokeAction(() => update("stt_language", e.target.value))
            }
            className="w-full sm:w-48"
          />
        </SettingsRow>
      </SettingsGroup>

      {engine === "whisper_cpp" ? (
        <SettingsGroup title={t("settings.stt.offline_group")}>
          <div className="flex flex-col gap-4 px-4 py-4">
            <PathPickerField
              label={t("settings.stt.binary_path")}
              placeholder="whisper-cli"
              value={whisperBinary}
              onBrowse={handlePickBinary}
              browseLabel={t("settings.stt.browse")}
              hint={t("settings.stt.binary_hint")}
            />
            <PathPickerField
              label={t("settings.stt.model_path")}
              placeholder="C:\\models\\ggml-base.bin"
              value={whisperModel}
              onBrowse={handlePickModel}
              browseLabel={t("settings.stt.browse")}
              hint={t("settings.stt.model_hint")}
            />
            <Input
              label={t("settings.stt.threads")}
              type="number"
              min={1}
              max={32}
              value={whisperThreads}
              onChange={(e) =>
                void invokeAction(() =>
                  update(
                    "whisper_cpp_threads",
                    clampWhisperThreads(e.target.value),
                  ),
                )
              }
            />
            <div className="flex items-center justify-end gap-2.5 pt-1">
              <TestStatusBadge
                status={whisperStatus}
                okLabel={t("settings.stt.ready")}
                failLabel={t("settings.stt.check_setup")}
                errLabel={t("settings.stt.test_fail")}
                testingLabel={t("settings.stt.testing")}
              />
              <Button
                variant="secondary"
                size="sm"
                onClick={handleTestWhisper}
                disabled={
                  whisperStatus === "testing" ||
                  !whisperBinary.trim() ||
                  !whisperModel.trim()
                }
              >
                {t("settings.stt.test_offline")}
              </Button>
            </div>
          </div>
        </SettingsGroup>
      ) : (
        <SettingsGroup title={t("settings.stt.groq_group")}>
          <div className="flex flex-col gap-3 px-4 py-4">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="text-xs font-medium text-vx-text-secondary">
                {t("settings.stt.api_key")}
              </span>
              <StatusBadge tone="neutral">
                {t("settings.stt.shared_with_llm")}
              </StatusBadge>
            </div>
            <Input
              aria-label={t("settings.stt.api_key")}
              type="password"
              showPasswordToggle
              placeholder={groqKeySet ? t("settings.stt.saved") : "gsk_..."}
              value={localKey}
              onChange={(e) => onKeyChange(e.target.value)}
            />
            <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
              <span className="text-xs text-vx-text-dim">
                {t("settings.stt.api_key_hint")}
              </span>
              <div className="flex items-center gap-2.5">
                <TestStatusBadge
                  status={groqStatus}
                  okLabel={t("settings.stt.connected")}
                  failLabel={t("settings.stt.invalid_key")}
                  errLabel={t("settings.stt.conn_fail")}
                  testingLabel={t("settings.stt.testing")}
                />
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={handleTestApi}
                  disabled={
                    groqStatus === "testing" ||
                    (!localKey.trim() && !groqKeySet)
                  }
                >
                  {t("settings.stt.test_connection")}
                </Button>
              </div>
            </div>
          </div>
        </SettingsGroup>
      )}
    </div>
  );
}

async function runStatus(
  setStatus: (status: TestStatus) => void,
  timerRef: { current: ReturnType<typeof setTimeout> | null },
  test: () => Promise<void>,
) {
  if (timerRef.current) {
    clearTimeout(timerRef.current);
    timerRef.current = null;
  }
  setStatus("testing");
  try {
    await test();
    setStatus("ok");
  } catch (e: unknown) {
    const code = errorCode(e);
    setStatus(
      code === "SttApiKeyInvalid" ||
        code === "SttModelNotFound" ||
        code === "SttEngineError"
        ? "fail"
        : "err",
    );
    toast(formatTauriError(e), "error");
  } finally {
    timerRef.current = setTimeout(() => {
      setStatus("idle");
      timerRef.current = null;
    }, 3000);
  }
}

function PathPickerField({
  label,
  placeholder,
  value,
  hint,
  browseLabel,
  onBrowse,
}: {
  label: string;
  placeholder: string;
  value: string;
  hint: string;
  browseLabel: string;
  onBrowse: () => void;
}) {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-[minmax(0,1fr)_auto] items-start gap-2">
      {/* Read-only by design: paths must come from the native dialog, so
          free-text entry is not offered (backend would reject it anyway). */}
      <Input
        label={label}
        placeholder={placeholder}
        value={value}
        readOnly
        hint={hint}
      />
      <Button
        type="button"
        variant="secondary"
        size="md"
        onClick={onBrowse}
        className="sm:mt-6 whitespace-nowrap"
      >
        <FolderOpen className="h-4 w-4" />
        {browseLabel}
      </Button>
    </div>
  );
}

function TestStatusBadge({
  status,
  okLabel,
  failLabel,
  errLabel,
  testingLabel,
}: {
  status: TestStatus;
  okLabel: string;
  failLabel: string;
  errLabel: string;
  testingLabel: string;
}) {
  if (status === "idle") return null;
  if (status === "testing") {
    return (
      <StatusBadge tone="ember">
        <Loader2 className="h-3 w-3 animate-spin" />
        {testingLabel}
      </StatusBadge>
    );
  }
  if (status === "ok") {
    return (
      <StatusBadge tone="success">
        <Check className="h-3 w-3" />
        {okLabel}
      </StatusBadge>
    );
  }
  return (
    <StatusBadge tone={status === "fail" ? "error" : "warning"}>
      <XCircle className="h-3 w-3" />
      {status === "fail" ? failLabel : errLabel}
    </StatusBadge>
  );
}
