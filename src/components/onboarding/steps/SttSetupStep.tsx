import {
  type ButtonHTMLAttributes,
  type ReactNode,
  useEffect,
  useRef,
  useState,
} from "react";
import {
  ArrowLeft,
  Check,
  ChevronRight,
  Cloud,
  Cpu,
  Download,
  ExternalLink,
  FolderOpen,
  HardDrive,
  Key,
  Loader2,
  Star,
  Terminal,
  XCircle,
} from "lucide-react";

import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import {
  formatTauriError,
  openUrl,
  pickSetupFile,
  testGroqApi,
  testWhisperCpp,
} from "../../../lib/tauri";
import { StepShell } from "../shared/StepShell";
import type { SttEngine, Step, TestStatus, TFunc } from "../types";

const GROQ_URL = "https://console.groq.com";
const WHISPER_RELEASES_URL = "https://github.com/ggml-org/whisper.cpp/releases";
const WHISPER_SOURCE_URL = "https://github.com/ggml-org/whisper.cpp";
const WHISPER_MODELS_URL =
  "https://huggingface.co/ggerganov/whisper.cpp/tree/main";

interface SttSetupStepProps {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  error: string;
  sttEngine: SttEngine;
  sttLanguage: string;
  apiKey: string;
  whisperBinary: string;
  whisperModel: string;
  whisperThreads: number;
  onEngineChange: (engine: SttEngine) => void;
  onLanguageChange: (language: string) => void;
  onApiKeyChange: (value: string) => void;
  onSave: () => void;
  onThreadsChange: (value: number) => void;
  onBinaryChange: (value: string) => void;
  onModelChange: (value: string) => void;
  onBack: () => void;
  onSkip: () => void;
}

export function SttSetupStep(props: SttSetupStepProps) {
  const {
    t,
    sttEngine,
    sttLanguage,
    apiKey,
    whisperBinary,
    whisperModel,
    whisperThreads,
  } = props;
  const [groqStatus, setGroqStatus] = useState<TestStatus>("idle");
  const [whisperStatus, setWhisperStatus] = useState<TestStatus>("idle");
  const [error, setError] = useState("");
  const groqTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const whisperTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (groqTimerRef.current) clearTimeout(groqTimerRef.current);
      if (whisperTimerRef.current) clearTimeout(whisperTimerRef.current);
    };
  }, []);

  const testApi = async () => {
    if (!apiKey.trim()) return;
    await runStatus(setGroqStatus, groqTimerRef, async () => testGroqApi(apiKey.trim()));
  };
  const testWhisper = async () => {
    const missing = validateWhisperSetup(whisperBinary, whisperModel, t);
    if (missing) {
      if (whisperTimerRef.current) {
        clearTimeout(whisperTimerRef.current);
        whisperTimerRef.current = null;
      }
      setError(missing);
      setWhisperStatus("fail");
      return;
    }
    await runStatus(setWhisperStatus, whisperTimerRef, async () =>
      testWhisperCpp(
        whisperBinary.trim(),
        whisperModel.trim(),
        sttLanguage,
        whisperThreads,
      ),
    );
  };
  const pick = async (
    kind: "whisper_binary" | "whisper_model",
    setValue: (value: string) => void,
  ) => {
    try {
      setError("");
      const file = await pickSetupFile(kind);
      if (file) setValue(file);
    } catch (e: unknown) {
      setError(formatTauriError(e));
    }
  };
  const save = () => {
    if (sttEngine === "whisper_cpp") {
      const missing = validateWhisperSetup(whisperBinary, whisperModel, t);
      if (missing) {
        setError(missing);
        return;
      }
    }
    props.onSave();
  };

  return (
    <StepShell
      step={props.step}
      currentStepIdx={props.currentStepIdx}
      t={t}
      icon={
        sttEngine === "groq" ? (
          <Key className="h-7 w-7" />
        ) : (
          <HardDrive className="h-7 w-7" />
        )
      }
      title={t("onboarding.stt.title")}
      description={t("onboarding.stt.body")}
      error={error || props.error}
      wide
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={props.onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="ghost" size="lg" onClick={props.onSkip}>
            {t("onboarding.stt.skip")}
          </Button>
          <Button variant="primary" size="lg" onClick={save}>
            {t("onboarding.stt.save")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
        <div className="grid gap-4 md:grid-cols-2">
          <SetupChoice
            active={sttEngine === "groq"}
            icon={<Cloud className="h-5 w-5" />}
            title={t("onboarding.stt.groq.choice_title")}
            body={t("onboarding.stt.groq.choice_body")}
            recommend={t("onboarding.stt.recommended")}
            onClick={() => {
              setError("");
              props.onEngineChange("groq");
            }}
          />
          <SetupChoice
            active={sttEngine === "whisper_cpp"}
            icon={<HardDrive className="h-5 w-5" />}
            title={t("onboarding.stt.offline.choice_title")}
            body={t("onboarding.stt.offline.choice_body")}
            onClick={() => {
              setError("");
              props.onEngineChange("whisper_cpp");
            }}
          />
        </div>
        <p className="text-center text-xs leading-relaxed text-vx-text-dim">
          {sttEngine === "groq"
            ? t("onboarding.stt.groq.recommend_note")
            : t("onboarding.stt.offline.recommend_note")}
        </p>
        <div className="w-full rounded-xl border border-vx-border bg-vx-bg-secondary p-5 shadow-vx-sm">
          <div className="mb-5 grid gap-4 md:grid-cols-[1fr_220px]">
            <div>
              <h2 className="text-lg font-semibold">
                {sttEngine === "groq"
                  ? t("onboarding.stt.groq.title")
                  : t("onboarding.stt.offline.title")}
              </h2>
              <p className="mt-1 text-sm leading-relaxed text-vx-text-dim">
                {sttEngine === "groq"
                  ? t("onboarding.stt.groq.body")
                  : t("onboarding.stt.offline.body")}
              </p>
            </div>
            <div>
              <label className="mb-1.5 block text-xs font-medium text-vx-text-secondary">
                {t("onboarding.stt.language")}
              </label>
              <Select
                options={[
                  { value: "auto", label: t("onboarding.stt.language_auto") },
                  { value: "id", label: "Bahasa Indonesia" },
                  { value: "en", label: "English" },
                ]}
                value={sttLanguage}
                onChange={(e) => props.onLanguageChange(e.target.value)}
                className="w-full"
              />
            </div>
          </div>
          {sttEngine === "groq" ? (
            <GroqSetup
              apiKey={apiKey}
              status={groqStatus}
              t={t}
              onApiKeyChange={props.onApiKeyChange}
              onTest={() => void testApi()}
            />
          ) : (
            <OfflineSetup
              binaryPath={whisperBinary}
              modelPath={whisperModel}
              threads={whisperThreads}
              status={whisperStatus}
              t={t}
              onThreadsChange={props.onThreadsChange}
              onPickBinary={() =>
                void pick("whisper_binary", props.onBinaryChange)
              }
              onPickModel={() =>
                void pick("whisper_model", props.onModelChange)
              }
              onTest={() => void testWhisper()}
            />
          )}
        </div>
    </StepShell>
  );
}

function validateWhisperSetup(binaryPath: string, modelPath: string, t: TFunc) {
  return !binaryPath.trim()
    ? t("onboarding.stt.offline.binary_required")
    : !modelPath.trim()
      ? t("onboarding.stt.offline.model_required")
      : "";
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
    setStatus(
      ["SttApiKeyInvalid", "SttModelNotFound", "SttEngineError"].includes(
        errorCode(e) ?? "",
      )
        ? "fail"
        : "err",
    );
  } finally {
    timerRef.current = setTimeout(() => {
      setStatus("idle");
      timerRef.current = null;
    }, 3000);
  }
}
function errorCode(err: unknown) {
  return err &&
    typeof err === "object" &&
    "code" in err &&
    typeof err.code === "string"
    ? err.code
    : undefined;
}

function GroqSetup({
  apiKey,
  status,
  t,
  onApiKeyChange,
  onTest,
}: {
  apiKey: string;
  status: TestStatus;
  t: TFunc;
  onApiKeyChange: (value: string) => void;
  onTest: () => void;
}) {
  return (
    <div className="grid gap-5 md:grid-cols-[1fr_320px]">
      <GuideList
        items={[
          t("onboarding.stt.groq.step1"),
          t("onboarding.stt.groq.step2"),
          t("onboarding.stt.groq.step3"),
        ]}
      />
      <div className="flex flex-col gap-3">
        <Button
          variant="secondary"
          type="button"
          onClick={() => void openUrl(GROQ_URL)}
        >
          <ExternalLink className="h-4 w-4" />
          {t("onboarding.stt.groq.open")}
        </Button>
        <Input
          autoFocus
          label={t("onboarding.stt.groq.api_key")}
          type="password"
          showPasswordToggle
          placeholder="gsk_..."
          value={apiKey}
          onChange={(e) => onApiKeyChange(e.target.value)}
        />
        <StatusButton
          status={status}
          idleLabel={t("onboarding.stt.test")}
          okLabel={t("onboarding.stt.test_ok")}
          failLabel={t("onboarding.stt.groq.test_fail")}
          errLabel={t("onboarding.stt.test_err")}
          onClick={onTest}
          disabled={!apiKey.trim() || status === "testing"}
        />
      </div>
    </div>
  );
}
function OfflineSetup({
  binaryPath,
  modelPath,
  threads,
  status,
  t,
  onThreadsChange,
  onPickBinary,
  onPickModel,
  onTest,
}: {
  binaryPath: string;
  modelPath: string;
  threads: number;
  status: TestStatus;
  t: TFunc;
  onThreadsChange: (value: number) => void;
  onPickBinary: () => void;
  onPickModel: () => void;
  onTest: () => void;
}) {
  return (
    <div className="grid gap-5 lg:grid-cols-[1fr_340px]">
      <div className="flex flex-col gap-4">
        <InstructionBlock
          icon={<Download className="h-4 w-4" />}
          title={t("onboarding.stt.offline.no_cmake_title")}
          body={t("onboarding.stt.offline.no_cmake_body")}
          actions={[
            {
              label: t("onboarding.stt.offline.open_releases"),
              url: WHISPER_RELEASES_URL,
            },
          ]}
        />
        <InstructionBlock
          icon={<Terminal className="h-4 w-4" />}
          title={t("onboarding.stt.offline.cmake_title")}
          body={t("onboarding.stt.offline.cmake_body")}
          actions={[
            {
              label: t("onboarding.stt.offline.open_source"),
              url: WHISPER_SOURCE_URL,
            },
          ]}
        >
          <CodeBlock
            lines={[
              "git clone https://github.com/ggml-org/whisper.cpp.git",
              "cd whisper.cpp",
              "cmake -B build",
              "cmake --build build -j --config Release",
            ]}
          />
        </InstructionBlock>
        <InstructionBlock
          icon={<Cpu className="h-4 w-4" />}
          title={t("onboarding.stt.offline.model_title")}
          body={t("onboarding.stt.offline.model_body")}
          actions={[
            {
              label: t("onboarding.stt.offline.open_models"),
              url: WHISPER_MODELS_URL,
            },
          ]}
        >
          <ModelGuide t={t} />
        </InstructionBlock>
        <InstructionBlock
          icon={<FolderOpen className="h-4 w-4" />}
          title={t("onboarding.stt.offline.path_title")}
          body={t("onboarding.stt.offline.path_body")}
        >
          <GuideList
            items={[
              t("onboarding.stt.offline.path_step1"),
              t("onboarding.stt.offline.path_step2"),
              t("onboarding.stt.offline.path_step3"),
            ]}
          />
        </InstructionBlock>
      </div>
      <div className="flex flex-col gap-3">
        <PathPickerField
          autoFocus
          label={t("onboarding.stt.offline.binary")}
          placeholder="whisper-cli"
          value={binaryPath}
          onBrowse={onPickBinary}
          browseLabel={t("onboarding.stt.offline.browse_binary")}
          hint={t("onboarding.stt.offline.binary_hint")}
        />
        <PathPickerField
          label={t("onboarding.stt.offline.model")}
          placeholder="C:\\models\\ggml-base.bin"
          value={modelPath}
          onBrowse={onPickModel}
          browseLabel={t("onboarding.stt.offline.browse_model")}
          hint={t("onboarding.stt.offline.model_hint")}
        />
        <Input
          label={t("onboarding.stt.offline.threads")}
          type="number"
          min={1}
          max={32}
          value={threads}
          onChange={(e) =>
            onThreadsChange(
              Math.max(1, Math.floor(Number(e.target.value) || 1)),
            )
          }
        />
        <StatusButton
          status={status}
          idleLabel={t("onboarding.stt.offline.test")}
          okLabel={t("onboarding.stt.offline.test_ok")}
          failLabel={t("onboarding.stt.offline.test_fail")}
          errLabel={t("onboarding.stt.test_err")}
          onClick={onTest}
          disabled={
            status === "testing" || !binaryPath.trim() || !modelPath.trim()
          }
        />
        <p className="text-xs leading-relaxed text-vx-text-dim">
          {t("onboarding.stt.offline.full_offline_note")}
        </p>
      </div>
    </div>
  );
}
function SetupChoice({
  active,
  icon,
  title,
  body,
  recommend,
  onClick,
}: {
  active: boolean;
  icon: ReactNode;
  title: string;
  body: string;
  recommend?: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex min-h-32 items-start gap-4 rounded-xl border p-5 text-left transition-colors ${active ? "border-vx-accent/60 bg-vx-accent-soft text-vx-text-primary" : "border-vx-border bg-vx-bg-secondary text-vx-text-secondary hover:border-vx-border-strong hover:text-vx-text-primary"}`}
    >
      <span className="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-vx-bg-tertiary text-vx-accent">
        {icon}
      </span>
      <span className="flex flex-col gap-1.5">
        <span className="flex items-center gap-2">
          <span className="text-sm font-semibold">{title}</span>
          {recommend && (
            <span className="inline-flex items-center gap-1 rounded-full bg-vx-accent/15 px-2 py-0.5 text-[10px] font-semibold text-vx-accent">
              <Star className="h-3 w-3" />
              {recommend}
            </span>
          )}
        </span>
        <span className="text-xs leading-relaxed text-vx-text-dim">{body}</span>
      </span>
    </button>
  );
}
function PathPickerField({
  label,
  placeholder,
  value,
  hint,
  browseLabel,
  autoFocus,
  onBrowse,
}: {
  label: string;
  placeholder: string;
  value: string;
  hint: string;
  browseLabel: string;
  autoFocus?: boolean;
  onBrowse: () => void;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-2">
      <Input
        autoFocus={autoFocus}
        label={label}
        placeholder={placeholder}
        value={value}
        readOnly
        hint={hint}
      />
      <Button
        variant="secondary"
        type="button"
        className="mt-6 whitespace-nowrap"
        onClick={onBrowse}
      >
        <FolderOpen className="h-4 w-4" />
        {browseLabel}
      </Button>
    </div>
  );
}
function InstructionBlock({
  icon,
  title,
  body,
  actions,
  children,
}: {
  icon: ReactNode;
  title: string;
  body: string;
  actions?: Array<{ label: string; url: string }>;
  children?: ReactNode;
}) {
  return (
    <div className="rounded-lg border border-vx-border bg-vx-bg-primary/40 p-4">
      <div className="flex items-start gap-3">
        <span className="mt-0.5 text-vx-accent">{icon}</span>
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-semibold">{title}</h3>
          <p className="mt-1 text-xs leading-relaxed text-vx-text-dim">
            {body}
          </p>
          {children && <div className="mt-3">{children}</div>}
          {actions && (
            <div className="mt-3 flex flex-wrap gap-2">
              {actions.map((action) => (
                <Button
                  key={action.url}
                  variant="secondary"
                  size="sm"
                  type="button"
                  onClick={() => void openUrl(action.url)}
                >
                  <ExternalLink className="h-3.5 w-3.5" />
                  {action.label}
                </Button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
function GuideList({ items }: { items: string[] }) {
  return (
    <ol className="flex flex-col gap-3">
      {items.map((item, index) => (
        <li key={item} className="flex gap-3 text-sm leading-relaxed">
          <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-vx-accent-soft text-xs font-semibold text-vx-accent">
            {index + 1}
          </span>
          <span className="text-vx-text-secondary">{item}</span>
        </li>
      ))}
    </ol>
  );
}
function CodeBlock({ lines }: { lines: string[] }) {
  return (
    <pre className="overflow-x-auto rounded-lg bg-vx-bg-tertiary p-3 text-xs leading-relaxed text-vx-text-secondary">
      {lines.join("\n")}
    </pre>
  );
}
function ModelGuide({ t }: { t: TFunc }) {
  return (
    <div className="grid gap-2">
      {[
        ["tiny", t("onboarding.stt.offline.model_tiny")],
        ["base", t("onboarding.stt.offline.model_base")],
        ["small", t("onboarding.stt.offline.model_small")],
      ].map(([name, desc]) => (
        <div key={name} className="grid grid-cols-[64px_1fr] gap-3 text-xs">
          <span className="rounded bg-vx-bg-tertiary px-2 py-1 font-mono text-vx-text-primary">
            {name}
          </span>
          <span className="py-1 text-vx-text-dim">{desc}</span>
        </div>
      ))}
    </div>
  );
}
function StatusButton({
  status,
  idleLabel,
  okLabel,
  failLabel,
  errLabel,
  ...rest
}: {
  status: TestStatus;
  idleLabel: string;
  okLabel: string;
  failLabel: string;
  errLabel: string;
} & ButtonHTMLAttributes<HTMLButtonElement>) {
  const label =
    status === "testing"
      ? "Testing..."
      : status === "ok"
        ? okLabel
        : status === "fail"
          ? failLabel
          : status === "err"
            ? errLabel
            : idleLabel;
  return (
    <button
      type="button"
      className={`w-full flex items-center justify-center gap-2 rounded-lg border px-4 py-2.5 text-sm font-medium transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed ${status === "ok" ? "border-green-500/40 bg-green-500/10 text-green-600" : status === "fail" ? "border-red-500/40 bg-red-500/10 text-red-600" : status === "err" ? "border-amber-500/40 bg-amber-500/10 text-amber-600" : status === "testing" ? "border-vx-accent/40 bg-vx-accent/10 text-vx-accent" : "border-vx-border bg-vx-bg-tertiary/60 text-vx-text-secondary hover:border-vx-border-strong hover:text-vx-text-primary"}`}
      {...rest}
    >
      {status === "testing" ? (
        <Loader2 className="h-4 w-4 animate-spin" />
      ) : status === "ok" ? (
        <Check className="h-4 w-4" />
      ) : status === "fail" || status === "err" ? (
        <XCircle className="h-4 w-4" />
      ) : null}
      {label}
    </button>
  );
}
