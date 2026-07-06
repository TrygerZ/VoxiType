import { type ButtonHTMLAttributes, useState } from "react";
import { Check, FolderOpen, Loader2, XCircle } from "lucide-react";

import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { useSettingsStore } from "../../stores/settingsStore";
import {
  formatTauriError,
  pickSetupFile,
  testGroqApi,
  testWhisperCpp,
} from "../../lib/tauri";
import { toast } from "../ui/Toast";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";

type TestStatus = "idle" | "testing" | "ok" | "fail" | "err";

export function STTTab() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const [groqStatus, setGroqStatus] = useState<TestStatus>("idle");
  const [whisperStatus, setWhisperStatus] = useState<TestStatus>("idle");

  const engine = stringSetting(settings.stt_engine, "groq");
  const language = stringSetting(settings.stt_language, "auto");
  const groqKey = stringSetting(settings.groq_api_key, "");
  const groqKeySet = settings.groq_api_key_set === true;
  const whisperBinary = stringSetting(
    settings.whisper_cpp_binary_path,
    "whisper-cli",
  );
  const whisperModel = stringSetting(settings.whisper_cpp_model_path, "");
  const whisperThreads = numberSetting(settings.whisper_cpp_threads, 4);

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
    if (!groqKey.trim() && !groqKeySet) return;
    await runStatus(setGroqStatus, async () => {
      await testGroqApi(groqKey.trim());
      toast("Groq connection successful!");
    });
  };

  const handleTestWhisper = async () => {
    if (!whisperBinary.trim() || !whisperModel.trim()) return;
    await runStatus(setWhisperStatus, async () => {
      await testWhisperCpp(
        whisperBinary.trim(),
        whisperModel.trim(),
        language,
        whisperThreads,
      );
      toast("Offline engine ready");
    });
  };

  const handlePickBinary = async () => {
    try {
      const file = await pickSetupFile("whisper_binary");
      if (file) {
        await update("whisper_cpp_binary_path", file);
      }
    } catch (e: unknown) {
      toast(formatTauriError(e), "error");
    }
  };

  const handlePickModel = async () => {
    try {
      const file = await pickSetupFile("whisper_model");
      if (file) {
        await update("whisper_cpp_model_path", file);
      }
    } catch (e: unknown) {
      toast(formatTauriError(e), "error");
    }
  };

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title="Speech-to-Text"
        description="Choose cloud Groq or local whisper.cpp transcription."
      />

      <SettingsGroup title="Engine">
        <SettingsRow label="Transcription engine">
          <Select
            options={[
              { value: "groq", label: "Groq Whisper" },
              { value: "whisper_cpp", label: "Offline whisper.cpp" },
            ]}
            value={engine}
            onChange={(e) => void update("stt_engine", e.target.value)}
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
            value={language}
            onChange={(e) => void update("stt_language", e.target.value)}
            className="w-48"
          />
        </SettingsRow>
      </SettingsGroup>

      {engine === "whisper_cpp" ? (
        <SettingsGroup title="Offline whisper.cpp">
          <div className="flex flex-col gap-4 px-4 py-3.5">
            <PathPickerField
              label="whisper-cli path"
              placeholder="whisper-cli or C:\\tools\\whisper.cpp\\build\\bin\\Release\\whisper-cli.exe"
              value={whisperBinary}
              onChange={(value) => void update("whisper_cpp_binary_path", value)}
              onBrowse={handlePickBinary}
              browseLabel="Browse"
              hint="Use whisper-cli when it is available in PATH."
            />
            <PathPickerField
              label="Model path"
              placeholder="C:\\models\\ggml-base.bin"
              value={whisperModel}
              onChange={(value) => void update("whisper_cpp_model_path", value)}
              onBrowse={handlePickModel}
              browseLabel="Browse"
              hint="Use a ggml model file from whisper.cpp models."
            />
            <Input
              label="Threads"
              type="number"
              min={1}
              max={32}
              value={whisperThreads}
              onChange={(e) =>
                void update(
                  "whisper_cpp_threads",
                  Math.max(1, Math.floor(Number(e.target.value) || 1)),
                )
              }
            />
            <StatusButton
              status={whisperStatus}
              idleLabel="Test Offline Engine"
              okLabel="Ready"
              failLabel="Check setup"
              errLabel="Test failed"
              onClick={handleTestWhisper}
              disabled={
                whisperStatus === "testing" ||
                !whisperBinary.trim() ||
                !whisperModel.trim()
              }
            />
          </div>
        </SettingsGroup>
      ) : (
        <SettingsGroup title="Groq credentials">
          <div className="px-4 py-3.5">
            <Input
              label="Groq API Key"
              type="password"
              placeholder={groqKeySet ? "Saved" : "gsk_..."}
              value={groqKey}
              onChange={(e) => void handleApiKeyChange(e.target.value)}
              hint="Stored encrypted."
            />
            <StatusButton
              status={groqStatus}
              idleLabel="Test Connection"
              okLabel="Connected"
              failLabel="Invalid key"
              errLabel="Connection failed"
              onClick={handleTestApi}
              disabled={
                groqStatus === "testing" || (!groqKey.trim() && !groqKeySet)
              }
              className="mt-3"
            />
          </div>
        </SettingsGroup>
      )}
    </div>
  );
}

function stringSetting(value: unknown, fallback: string): string {
  return typeof value === "string" ? value : fallback;
}

function numberSetting(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

async function runStatus(
  setStatus: (status: TestStatus) => void,
  test: () => Promise<void>,
) {
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
    setTimeout(() => setStatus("idle"), 3000);
  }
}

function errorCode(err: unknown): string | undefined {
  if (!err || typeof err !== "object" || !("code" in err)) {
    return undefined;
  }
  const code = err.code;
  return typeof code === "string" ? code : undefined;
}

function PathPickerField({
  label,
  placeholder,
  value,
  hint,
  browseLabel,
  onChange,
  onBrowse,
}: {
  label: string;
  placeholder: string;
  value: string;
  hint: string;
  browseLabel: string;
  onChange: (value: string) => void;
  onBrowse: () => void;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-2">
      <Input
        label={label}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        hint={hint}
      />
      <button
        type="button"
        onClick={onBrowse}
        className="mt-6 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg bg-vx-bg-tertiary px-4 py-2.5 text-sm font-medium text-vx-text-primary transition-colors duration-150 hover:bg-vx-bg-elevated"
      >
        <FolderOpen className="h-4 w-4" />
        {browseLabel}
      </button>
    </div>
  );
}

function StatusButton({
  status,
  idleLabel,
  okLabel,
  failLabel,
  errLabel,
  className = "",
  ...rest
}: {
  status: TestStatus;
  idleLabel: string;
  okLabel: string;
  failLabel: string;
  errLabel: string;
} & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      className={`w-full flex items-center justify-center gap-2 rounded-lg border px-4 py-2.5 text-sm font-medium transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed ${statusClasses(
        status,
      )} ${className}`}
      {...rest}
    >
      {status === "testing" ? (
        <Loader2 className="h-4 w-4 animate-spin" />
      ) : status === "ok" ? (
        <Check className="h-4 w-4" />
      ) : status === "fail" || status === "err" ? (
        <XCircle className="h-4 w-4" />
      ) : null}
      {statusLabel(status, idleLabel, okLabel, failLabel, errLabel)}
    </button>
  );
}

function statusLabel(
  status: TestStatus,
  idle: string,
  ok: string,
  fail: string,
  err: string,
) {
  if (status === "testing") return "Testing...";
  if (status === "ok") return ok;
  if (status === "fail") return fail;
  if (status === "err") return err;
  return idle;
}

function statusClasses(status: TestStatus) {
  if (status === "ok") {
    return "border-green-500/40 bg-green-500/10 text-green-600";
  }
  if (status === "fail") {
    return "border-red-500/40 bg-red-500/10 text-red-600";
  }
  if (status === "err") {
    return "border-amber-500/40 bg-amber-500/10 text-amber-600";
  }
  if (status === "testing") {
    return "border-vx-accent/40 bg-vx-accent/10 text-vx-accent";
  }
  return "border-vx-border bg-vx-bg-tertiary/60 text-vx-text-secondary hover:border-vx-border-strong hover:text-vx-text-primary";
}
