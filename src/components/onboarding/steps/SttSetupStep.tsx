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
  Loader2,
  Terminal,
  XCircle,
} from "lucide-react";

import { clampWhisperThreads } from "../../../lib/settingsGuards";
import {
  errorCode,
  formatTauriError,
  openUrl,
  pickSetupFile,
  testGroqApi,
  testWhisperCpp,
} from "../../../lib/tauri";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { StatusBadge } from "../../ui/StatusBadge";
import { StepShell } from "../shared/StepShell";
import type { Step, SttEngine, TestStatus, TFunc } from "../types";

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
  busy: boolean;
  sttEngine: SttEngine;
  sttLanguage: string;
  apiKey: string;
  groqKeySet?: boolean;
  whisperBinary: string;
  whisperModel: string;
  whisperThreads: number;
  platform?: string;
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
    groqKeySet = false,
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
    const trimmed = apiKey.trim();
    if (!trimmed && !groqKeySet) return;
    await runStatus(setGroqStatus, groqTimerRef, async () =>
      testGroqApi(trimmed || null),
    );
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
      title={t("onboarding.stt.title")}
      description={t("onboarding.stt.body")}
      error={error || props.error}
      wide
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={props.onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.stt.back_to_mic")}
          </Button>
          <Button variant="ghost" size="lg" onClick={props.onSkip}>
            {t("onboarding.stt.skip")}
          </Button>
          <Button
            variant="primary"
            size="lg"
            disabled={props.busy}
            onClick={save}
          >
            {t("onboarding.stt.save")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
      <div className="grid w-full gap-3 md:grid-cols-2">
        <SetupChoice
          active={sttEngine === "groq"}
          icon={<Cloud className="h-4 w-4" />}
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
          icon={<HardDrive className="h-4 w-4" />}
          title={t("onboarding.stt.offline.choice_title")}
          body={t("onboarding.stt.offline.choice_body")}
          onClick={() => {
            setError("");
            props.onEngineChange("whisper_cpp");
          }}
        />
      </div>

      <div className="w-full rounded-[12px] border border-vx-border-subtle bg-vx-bg-surface p-5 text-left">
        {sttEngine === "groq" ? (
          <GroqSetup
            apiKey={apiKey}
            groqKeySet={groqKeySet}
            sttLanguage={sttLanguage}
            status={groqStatus}
            t={t}
            onApiKeyChange={props.onApiKeyChange}
            onLanguageChange={props.onLanguageChange}
            onTest={() => void testApi()}
          />
        ) : (
          <OfflineSetup
            platformSuffix={platformSuffix(props.platform)}
            sttLanguage={sttLanguage}
            binaryPath={whisperBinary}
            modelPath={whisperModel}
            threads={whisperThreads}
            status={whisperStatus}
            t={t}
            onLanguageChange={props.onLanguageChange}
            onThreadsChange={props.onThreadsChange}
            onPickBinary={() =>
              void pick("whisper_binary", props.onBinaryChange)
            }
            onPickModel={() => void pick("whisper_model", props.onModelChange)}
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

function GroqSetup({
  apiKey,
  groqKeySet,
  sttLanguage,
  status,
  t,
  onApiKeyChange,
  onLanguageChange,
  onTest,
}: {
  apiKey: string;
  groqKeySet: boolean;
  sttLanguage: string;
  status: TestStatus;
  t: TFunc;
  onApiKeyChange: (value: string) => void;
  onLanguageChange: (language: string) => void;
  onTest: () => void;
}) {
  return (
    <div className="flex flex-col gap-3.5">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="text-sm font-medium text-vx-text-primary">
          {t("onboarding.stt.groq.api_key")}
        </span>
        <div className="flex flex-wrap items-center gap-3">
          {groqKeySet && (
            <StatusBadge tone="success">
              {t("onboarding.stt.existing_key_saved")}
            </StatusBadge>
          )}
          <button
            type="button"
            onClick={() => void openUrl(GROQ_URL)}
            className="inline-flex items-center gap-1 text-xs text-vx-text-secondary transition-colors hover:text-vx-text-primary"
          >
            {t("onboarding.stt.groq.open")}
            <ExternalLink className="h-3 w-3" />
          </button>
        </div>
      </div>

      <Input
        autoFocus
        aria-label={t("onboarding.stt.groq.api_key")}
        type="password"
        showPasswordToggle
        placeholder={groqKeySet ? "••••••••••••" : "gsk_..."}
        value={apiKey}
        onChange={(e) => onApiKeyChange(e.target.value)}
      />

      <div className="flex flex-wrap items-center justify-between gap-3 pt-1">
        <Select
          aria-label={t("onboarding.stt.language")}
          options={[
            { value: "auto", label: t("onboarding.stt.language_auto") },
            { value: "id", label: "Bahasa Indonesia" },
            { value: "en", label: "English" },
          ]}
          value={sttLanguage}
          onChange={(e) => onLanguageChange(e.target.value)}
          className="w-48"
        />
        <StatusButton
          status={status}
          idleLabel={t("onboarding.stt.test")}
          okLabel={t("onboarding.stt.test_ok")}
          failLabel={t("onboarding.stt.groq.test_fail")}
          errLabel={t("onboarding.stt.test_err")}
          testingLabel={t("onboarding.stt.testing")}
          onClick={onTest}
          disabled={(!apiKey.trim() && !groqKeySet) || status === "testing"}
        />
      </div>
    </div>
  );
}

/** Windows wording is the fallback for an unknown platform, matching the
 *  pre-change text. */
function platformSuffix(platform: string | undefined): string {
  if (platform === "macos") return "_mac";
  if (platform === "linux") return "_linux";
  return "_windows";
}

function OfflineSetup({
  platformSuffix,
  sttLanguage,
  binaryPath,
  modelPath,
  threads,
  status,
  t,
  onLanguageChange,
  onThreadsChange,
  onPickBinary,
  onPickModel,
  onTest,
}: {
  platformSuffix: string;
  sttLanguage: string;
  binaryPath: string;
  modelPath: string;
  threads: number;
  status: TestStatus;
  t: TFunc;
  onLanguageChange: (language: string) => void;
  onThreadsChange: (value: number) => void;
  onPickBinary: () => void;
  onPickModel: () => void;
  onTest: () => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-3 md:grid-cols-2">
        <PathPickerField
          autoFocus
          label={t("onboarding.stt.offline.binary")}
          placeholder="whisper-cli"
          value={binaryPath}
          onBrowse={onPickBinary}
          browseLabel={t("onboarding.stt.offline.browse_binary")}
          hint={t(`onboarding.stt.offline.binary_hint${platformSuffix}`)}
        />
        <PathPickerField
          label={t("onboarding.stt.offline.model")}
          placeholder="C:\\models\\ggml-base.bin"
          value={modelPath}
          onBrowse={onPickModel}
          browseLabel={t("onboarding.stt.offline.browse_model")}
          hint={t(`onboarding.stt.offline.model_hint${platformSuffix}`)}
        />
      </div>

      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="flex flex-wrap items-end gap-3">
          <Select
            aria-label={t("onboarding.stt.language")}
            label={t("onboarding.stt.language")}
            options={[
              { value: "auto", label: t("onboarding.stt.language_auto") },
              { value: "id", label: "Bahasa Indonesia" },
              { value: "en", label: "English" },
            ]}
            value={sttLanguage}
            onChange={(e) => onLanguageChange(e.target.value)}
            className="w-44"
          />
          <Input
            label={t("onboarding.stt.offline.threads")}
            type="number"
            min={1}
            max={32}
            value={threads}
            onChange={(e) =>
              onThreadsChange(clampWhisperThreads(e.target.value))
            }
            className="w-24"
          />
        </div>
        <StatusButton
          status={status}
          idleLabel={t("onboarding.stt.offline.test")}
          okLabel={t("onboarding.stt.offline.test_ok")}
          failLabel={t("onboarding.stt.offline.test_fail")}
          errLabel={t("onboarding.stt.test_err")}
          testingLabel={t("onboarding.stt.testing")}
          onClick={onTest}
          disabled={
            status === "testing" || !binaryPath.trim() || !modelPath.trim()
          }
        />
      </div>

      <p className="text-xs leading-relaxed text-vx-text-muted">
        {t("onboarding.stt.offline.full_offline_note")}
      </p>

      <details className="rounded-[10px] border border-vx-border-subtle bg-vx-bg-subtle/60 px-3.5 py-2.5">
        <summary className="cursor-pointer select-none text-xs font-medium text-vx-text-secondary hover:text-vx-text-primary">
          {t("onboarding.stt.offline.guide_summary")}
        </summary>
        <div className="mt-3 flex flex-col gap-3">
          <InstructionBlock
            icon={<Download className="h-4 w-4" />}
            title={t("onboarding.stt.offline.no_cmake_title")}
            body={t(`onboarding.stt.offline.no_cmake_body${platformSuffix}`)}
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
            body={t(`onboarding.stt.offline.cmake_body${platformSuffix}`)}
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
                t(`onboarding.stt.offline.path_step1${platformSuffix}`),
                t("onboarding.stt.offline.path_step2"),
                t("onboarding.stt.offline.path_step3"),
              ]}
            />
          </InstructionBlock>
        </div>
      </details>
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
      className={`flex items-start gap-3 rounded-[12px] border p-4 text-left transition-colors ${
        active
          ? "border-vx-accent-ember bg-vx-bg-surface text-vx-text-primary"
          : "border-vx-border-subtle bg-vx-bg-surface text-vx-text-secondary hover:border-vx-border-default hover:text-vx-text-primary"
      }`}
    >
      <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-[8px] bg-vx-bg-subtle text-vx-text-secondary">
        {icon}
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="flex items-center justify-between gap-2">
          <span className="text-sm font-medium text-vx-text-primary">
            {title}
          </span>
          {recommend && <StatusBadge tone="ember" label={recommend} />}
        </span>
        <span className="text-xs leading-relaxed text-vx-text-muted">
          {body}
        </span>
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
        size="md"
        type="button"
        className="mt-6 whitespace-nowrap"
        onClick={onBrowse}
      >
        <FolderOpen className="h-3.5 w-3.5" />
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
    <div className="rounded-[8px] border border-vx-border-subtle bg-vx-bg-surface p-3.5">
      <div className="flex items-start gap-2.5">
        <span className="mt-0.5 text-vx-accent-ember">{icon}</span>
        <div className="min-w-0 flex-1">
          <h3 className="text-xs font-semibold text-vx-text-primary">
            {title}
          </h3>
          <p className="mt-1 text-xs leading-relaxed text-vx-text-muted">
            {body}
          </p>
          {children && <div className="mt-2.5">{children}</div>}
          {actions && (
            <div className="mt-2.5 flex flex-wrap gap-2">
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
    <ol className="flex flex-col gap-2">
      {items.map((item, index) => (
        <li key={item} className="flex gap-2.5 text-xs leading-relaxed">
          <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-vx-accent-ember-soft font-mono text-[11px] font-semibold text-vx-accent-ember">
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
    <pre className="overflow-x-auto rounded-[6px] bg-vx-bg-subtle p-2.5 font-mono text-[11px] leading-relaxed text-vx-text-secondary">
      {lines.join("\n")}
    </pre>
  );
}

function ModelGuide({ t }: { t: TFunc }) {
  return (
    <div className="grid gap-1.5">
      {[
        ["tiny", t("onboarding.stt.offline.model_tiny")],
        ["base", t("onboarding.stt.offline.model_base")],
        ["small", t("onboarding.stt.offline.model_small")],
      ].map(([name, desc]) => (
        <div key={name} className="grid grid-cols-[64px_1fr] gap-2.5 text-xs">
          <span className="rounded-[4px] bg-vx-bg-subtle px-2 py-1 font-mono text-vx-text-primary">
            {name}
          </span>
          <span className="py-1 text-vx-text-muted">{desc}</span>
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
  testingLabel,
  ...rest
}: {
  status: TestStatus;
  idleLabel: string;
  okLabel: string;
  failLabel: string;
  errLabel: string;
  testingLabel: string;
} & ButtonHTMLAttributes<HTMLButtonElement>) {
  const label =
    status === "testing"
      ? testingLabel
      : status === "ok"
        ? okLabel
        : status === "fail"
          ? failLabel
          : status === "err"
            ? errLabel
            : idleLabel;

  const toneClass =
    status === "ok"
      ? "border-vx-semantic-success/40 bg-vx-semantic-success-soft text-vx-semantic-success"
      : status === "fail"
        ? "border-vx-semantic-error/40 bg-vx-semantic-error-soft text-vx-semantic-error"
        : status === "err"
          ? "border-vx-semantic-warning/40 bg-vx-semantic-warning-soft text-vx-semantic-warning"
          : status === "testing"
            ? "border-vx-accent-ember/40 bg-vx-accent-ember-soft text-vx-accent-ember"
            : "border-vx-border-default bg-vx-bg-subtle text-vx-text-primary hover:bg-vx-bg-elevated";

  return (
    <button
      type="button"
      className={`inline-flex h-9 items-center justify-center gap-2 rounded-[8px] border px-3.5 text-[13px] font-medium transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember ${toneClass}`}
      {...rest}
    >
      {status === "testing" ? (
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
      ) : status === "ok" ? (
        <Check className="h-3.5 w-3.5" />
      ) : status === "fail" || status === "err" ? (
        <XCircle className="h-3.5 w-3.5" />
      ) : null}
      {label}
    </button>
  );
}
