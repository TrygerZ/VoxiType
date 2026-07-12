import { type ButtonHTMLAttributes, type ReactNode, useState } from "react";
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
  Keyboard,
  Key,
  Languages,
  Loader2,
  Mic,
  Settings,
  Star,
  Terminal,
  Type,
  XCircle,
} from "lucide-react";

import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { useT } from "../../lib/i18n";
import { useSettingsStore } from "../../stores/settingsStore";
import {
  formatTauriError,
  openUrl,
  pickSetupFile,
  setHotkey,
  testGroqApi,
  testWhisperCpp,
} from "../../lib/tauri";
import { HotkeyRecorder } from "../settings/HotkeyRecorder";

type Step = "welcome" | "quick_settings" | "stt_setup" | "hotkey" | "complete";
type SttEngine = "groq" | "whisper_cpp";
type TestStatus = "idle" | "testing" | "ok" | "fail" | "err";

const GROQ_URL = "https://console.groq.com";
const WHISPER_RELEASES_URL = "https://github.com/ggml-org/whisper.cpp/releases";
const WHISPER_SOURCE_URL = "https://github.com/ggml-org/whisper.cpp";
const WHISPER_MODELS_URL = "https://huggingface.co/ggerganov/whisper.cpp/tree/main";

interface OnboardingFlowProps {
  onComplete: () => void;
}

const STEPS: Step[] = [
  "welcome",
  "quick_settings",
  "stt_setup",
  "hotkey",
  "complete",
];

type TFunc = (key: string, vars?: Record<string, string | number>) => string;

const STEP_LABELS: Record<Step, string> = {
  welcome: "onboarding.steps.label.intro",
  quick_settings: "onboarding.steps.label.language",
  stt_setup: "onboarding.steps.label.stt",
  hotkey: "onboarding.steps.label.hotkey",
  complete: "onboarding.steps.label.complete",
};

const SETUP_STEPS: Step[] = ["quick_settings", "stt_setup", "hotkey"];

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const t = useT();
  const [step, setStep] = useState<Step>("welcome");
  const currentStepIdx = STEPS.indexOf(step);
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const loadSettings = useSettingsStore((s) => s.load);

  const features = [
    {
      icon: Mic,
      title: t("onboarding.feature.dictate.title"),
      body: t("onboarding.feature.dictate.body"),
    },
    {
      icon: HardDrive,
      title: t("onboarding.feature.offline.title"),
      body: t("onboarding.feature.offline.body"),
    },
    {
      icon: Languages,
      title: t("onboarding.feature.bilingual.title"),
      body: t("onboarding.feature.bilingual.body"),
    },
  ];

  const [lang, setLang] = useState(stringSetting(settings.language, "en"));
  const [soundCues, setSoundCues] = useState(booleanSetting(settings.sound_cues, false));
  const [sttEngine, setSttEngine] = useState<SttEngine>(sttEngineSetting(settings.stt_engine));
  const [sttLanguage, setSttLanguage] = useState(stringSetting(settings.stt_language, "auto"));
  const [apiKey, setApiKey] = useState("");
  const [groqStatus, setGroqStatus] = useState<TestStatus>("idle");
  const [whisperStatus, setWhisperStatus] = useState<TestStatus>("idle");
  const [whisperBinary, setWhisperBinary] = useState(
    stringSetting(settings.whisper_cpp_binary_path, "whisper-cli"),
  );
  const [whisperModel, setWhisperModel] = useState(
    stringSetting(settings.whisper_cpp_model_path, ""),
  );
  const [whisperThreads, setWhisperThreads] = useState(
    numberSetting(settings.whisper_cpp_threads, 4),
  );

  const hotkeyRaw = settings.hotkey as { key: string; mode: string } | undefined;
  const [hotkeyKey, setHotkeyKey] = useState(hotkeyRaw?.key ?? "Ctrl+Space");
  const [hotkeyMode, setHotkeyMode] = useState(hotkeyRaw?.mode ?? "ptt");
  const [hotkeyError, setHotkeyError] = useState("");
  const [generalError, setGeneralError] = useState("");

  const finish = async () => {
    try {
      await update("onboarding_completed", true);
      onComplete();
    } catch (e: unknown) {
      setGeneralError(formatTauriError(e));
    }
  };

  const saveQuickSettings = async () => {
    try {
      setGeneralError("");
      await update("language", lang);
      await update("sound_cues", soundCues);
      setStep("stt_setup");
    } catch (e: unknown) {
      setGeneralError(formatTauriError(e));
    }
  };

  const saveSttSetup = async () => {
    try {
      setGeneralError("");
      if (sttEngine === "whisper_cpp") {
        const missing = validateWhisperSetup(whisperBinary, whisperModel, t);
        if (missing) {
          setGeneralError(missing);
          return;
        }
      }

      await update("stt_engine", sttEngine);
      await update("stt_language", sttLanguage);
      if (sttEngine === "groq" && apiKey.trim()) {
        await update("groq_api_key", apiKey.trim());
      }
      if (sttEngine === "whisper_cpp") {
        await update("whisper_cpp_binary_path", whisperBinary.trim());
        await update("whisper_cpp_model_path", whisperModel.trim());
        await update("whisper_cpp_threads", whisperThreads);
      }
      setStep("hotkey");
    } catch (e: unknown) {
      setGeneralError(formatTauriError(e));
    }
  };

  const saveHotkey = async () => {
    try {
      await setHotkey(hotkeyKey, hotkeyMode);
      await loadSettings();
      setStep("complete");
    } catch (e: unknown) {
      setHotkeyError(formatTauriError(e));
    }
  };

  const handleTestApi = async () => {
    if (!apiKey.trim()) return;
    await runStatus(setGroqStatus, async () => testGroqApi(apiKey.trim()));
  };

  const handleTestWhisper = async () => {
    const missing = validateWhisperSetup(whisperBinary, whisperModel, t);
    if (missing) {
      setGeneralError(missing);
      setWhisperStatus("fail");
      return;
    }
    await runStatus(setWhisperStatus, async () =>
      testWhisperCpp(whisperBinary.trim(), whisperModel.trim(), sttLanguage, whisperThreads),
    );
  };

  const handlePickWhisperBinary = async () => {
    try {
      setGeneralError("");
      const file = await pickSetupFile("whisper_binary");
      if (file) {
        setWhisperBinary(file);
      }
    } catch (e: unknown) {
      setGeneralError(formatTauriError(e));
    }
  };

  const handlePickWhisperModel = async () => {
    try {
      setGeneralError("");
      const file = await pickSetupFile("whisper_model");
      if (file) {
        setWhisperModel(file);
      }
    } catch (e: unknown) {
      setGeneralError(formatTauriError(e));
    }
  };

  if (step === "welcome") {
    return (
      <div className="vx-app-bg flex h-full flex-col items-center justify-[safe_center] gap-10 overflow-y-auto p-10">
        <StepProgress step={step} currentStepIdx={currentStepIdx} t={t} />
        <div className="vx-animate-in flex flex-col items-center gap-5 text-center">
          <div className="relative flex h-20 w-20 items-center justify-center">
            <span className="absolute inset-0 rounded-full bg-vx-accent/20 blur-2xl" aria-hidden />
            <div className="relative flex h-20 w-20 items-center justify-center overflow-hidden rounded-full border border-vx-border/30 bg-vx-bg-secondary">
              <img
                src="/logo.png"
                alt="VoxiType"
                className="h-full w-full object-contain"
              />
            </div>
          </div>
          <div className="flex flex-col gap-2.5">
            <h1 className="text-3xl font-semibold tracking-tight">
              {t("onboarding.welcome.title")}
            </h1>
            <p className="max-w-sm text-sm leading-relaxed text-vx-text-dim">
              {t("onboarding.welcome.body")}
            </p>
          </div>
        </div>

        <HowItWorks t={t} />
        <PrepHint t={t} />

        <div className="grid w-full max-w-2xl grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {features.map(({ icon: Icon, title, body }) => (
            <div
              key={title}
              className="flex flex-col gap-2.5 rounded-xl border border-vx-border/40 bg-vx-bg-secondary/60 p-5 transition-colors duration-200 hover:border-vx-border-strong hover:bg-vx-bg-secondary"
            >
              <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-vx-accent-soft text-vx-accent">
                <Icon className="h-5 w-5" />
              </span>
              <span className="text-sm font-medium">{title}</span>
              <span className="text-xs leading-relaxed text-vx-text-dim">{body}</span>
            </div>
          ))}
        </div>

        <div className="flex gap-3">
          <Button variant="primary" size="lg" onClick={() => setStep("quick_settings")}>
            {t("onboarding.welcome.start")}
            <ChevronRight className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="lg" onClick={finish}>
            {t("onboarding.welcome.skip")}
          </Button>
        </div>
        {generalError && <span className="text-xs text-vx-error">{generalError}</span>}
      </div>
    );
  }

  if (step === "quick_settings") {
    return (
      <div className="vx-app-bg flex h-full flex-col items-center justify-[safe_center] gap-8 overflow-y-auto p-10 text-center">
        <StepProgress step={step} currentStepIdx={currentStepIdx} t={t} />
        <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-vx-accent-soft text-vx-accent">
          <Settings className="h-7 w-7" />
        </span>
        <h1 className="text-3xl font-semibold tracking-tight">
          {t("onboarding.step2.title")}
        </h1>
        <p className="max-w-sm text-sm text-vx-text-dim">
          {t("onboarding.step2.body")}
        </p>

        <div className="flex w-full max-w-sm flex-col gap-4 text-left">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-vx-text-secondary">
              {t("onboarding.ui_language")}
            </label>
            <Select
              autoFocus
              options={[
                { value: "id", label: "Bahasa Indonesia" },
                { value: "en", label: "English" },
              ]}
              value={lang}
              onChange={(e) => {
                const value = e.target.value;
                setLang(value);
                void update("language", value);
              }}
              className="w-full"
            />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <span className="text-xs font-medium text-vx-text-secondary">
                {t("onboarding.sound_cues")}
              </span>
              <p className="text-xs text-vx-text-dim">
                {soundCues ? t("onboarding.sound_cues_on") : t("onboarding.sound_cues_off")}
              </p>
            </div>
            <button
              type="button"
              onClick={() => setSoundCues((v) => !v)}
              className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors ${
                soundCues ? "bg-vx-accent" : "bg-vx-border-strong"
              }`}
            >
              <span
                className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow transition ${
                  soundCues ? "translate-x-5" : "translate-x-0"
                }`}
              />
            </button>
          </div>
        </div>

        <div className="flex gap-3">
          <Button variant="ghost" size="lg" onClick={() => setStep("welcome")}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="primary" size="lg" onClick={saveQuickSettings}>
            {t("onboarding.step2.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
        {generalError && <span className="text-xs text-vx-error">{generalError}</span>}
      </div>
    );
  }

  if (step === "stt_setup") {
    return (
      <div className="vx-app-bg h-full overflow-y-auto p-6 sm:p-8">
        <div className="mx-auto flex max-w-5xl flex-col gap-6">
          <StepProgress step={step} currentStepIdx={currentStepIdx} t={t} />
          <div className="flex flex-col items-center gap-3 text-center">
            <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-vx-accent-soft text-vx-accent">
              {sttEngine === "groq" ? <Key className="h-7 w-7" /> : <HardDrive className="h-7 w-7" />}
            </span>
            <h1 className="text-3xl font-semibold tracking-tight">
              {t("onboarding.stt.title")}
            </h1>
            <p className="max-w-2xl text-sm leading-relaxed text-vx-text-dim">
              {t("onboarding.stt.body")}
            </p>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <SetupChoice
              active={sttEngine === "groq"}
              icon={<Cloud className="h-5 w-5" />}
              title={t("onboarding.stt.groq.choice_title")}
              body={t("onboarding.stt.groq.choice_body")}
              recommend={t("onboarding.stt.recommended")}
              onClick={() => {
                setGeneralError("");
                setSttEngine("groq");
              }}
            />
            <SetupChoice
              active={sttEngine === "whisper_cpp"}
              icon={<HardDrive className="h-5 w-5" />}
              title={t("onboarding.stt.offline.choice_title")}
              body={t("onboarding.stt.offline.choice_body")}
              onClick={() => {
                setGeneralError("");
                setSttEngine("whisper_cpp");
              }}
            />
          </div>

          <p className="text-center text-xs leading-relaxed text-vx-text-dim">
            {sttEngine === "groq"
              ? t("onboarding.stt.groq.recommend_note")
              : t("onboarding.stt.offline.recommend_note")}
          </p>

          <div className="rounded-xl border border-vx-border bg-vx-bg-secondary p-5">
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
                  onChange={(e) => setSttLanguage(e.target.value)}
                  className="w-full"
                />
              </div>
            </div>

            {sttEngine === "groq" ? (
              <GroqSetup
                apiKey={apiKey}
                status={groqStatus}
                t={t}
                onApiKeyChange={(value) => {
                  setApiKey(value);
                  if (groqStatus !== "idle") setGroqStatus("idle");
                }}
                onTest={handleTestApi}
              />
            ) : (
              <OfflineSetup
                binaryPath={whisperBinary}
                modelPath={whisperModel}
                threads={whisperThreads}
                status={whisperStatus}
                t={t}
                onBinaryPathChange={setWhisperBinary}
                onModelPathChange={setWhisperModel}
                onThreadsChange={setWhisperThreads}
                onPickBinary={handlePickWhisperBinary}
                onPickModel={handlePickWhisperModel}
                onTest={handleTestWhisper}
              />
            )}
          </div>

          <div className="flex flex-col items-center justify-between gap-3 sm:flex-row">
            <Button variant="ghost" size="lg" onClick={() => setStep("quick_settings")}>
              <ArrowLeft className="h-4 w-4" />
              {t("onboarding.back")}
            </Button>
            <div className="flex gap-3">
              <Button variant="ghost" size="lg" onClick={() => setStep("hotkey")}>
                {t("onboarding.stt.skip")}
              </Button>
              <Button variant="primary" size="lg" onClick={saveSttSetup}>
                {t("onboarding.stt.save")}
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
          {generalError && <span className="text-center text-xs text-vx-error">{generalError}</span>}
        </div>
      </div>
    );
  }

  if (step === "hotkey") {
    return (
      <div className="vx-app-bg flex h-full flex-col items-center justify-[safe_center] gap-8 overflow-y-auto p-10 text-center">
        <StepProgress step={step} currentStepIdx={currentStepIdx} t={t} />
        <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-vx-accent-soft text-vx-accent">
          <Keyboard className="h-7 w-7" />
        </span>
        <h1 className="text-3xl font-semibold tracking-tight">
          {t("onboarding.step4.title")}
        </h1>
        <p className="max-w-sm text-sm text-vx-text-dim">
          {t("onboarding.step4.body")}
        </p>

        <div className="flex w-full max-w-sm flex-col gap-4">
          <HotkeyRecorder value={hotkeyKey} onChange={setHotkeyKey} />
          <div>
            <label className="mb-1.5 block text-xs font-medium text-vx-text-secondary">
              {t("onboarding.step4.mode_label")}
            </label>
            <Select
              options={[
                { value: "ptt", label: t("onboarding.step4.mode_ptt") },
                { value: "toggle", label: t("onboarding.step4.mode_toggle") },
              ]}
              value={hotkeyMode}
              onChange={(e) => setHotkeyMode(e.target.value)}
              className="w-full"
            />
          </div>
        </div>

        <div className="flex gap-3">
          <Button variant="ghost" size="lg" onClick={() => setStep("stt_setup")}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="primary" size="lg" onClick={saveHotkey}>
            {t("onboarding.step4.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
        {hotkeyError && <span className="text-xs text-vx-error">{hotkeyError}</span>}
      </div>
    );
  }

  return (
    <div className="vx-app-bg flex h-full flex-col items-center justify-[safe_center] gap-8 overflow-y-auto p-10 text-center">
      <StepProgress step={step} currentStepIdx={currentStepIdx} t={t} />
      <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-vx-success/10 text-vx-success">
        <Check className="h-7 w-7" />
      </span>
      <h1 className="text-3xl font-semibold tracking-tight">
        {t("onboarding.complete.title")}
      </h1>
      <p className="max-w-sm text-sm text-vx-text-dim">
        {t("onboarding.complete.body", { key: hotkeyKey ?? "Ctrl+Space" })}
      </p>
      <div className="flex w-full max-w-sm flex-col gap-3 rounded-xl border border-vx-border/40 bg-vx-bg-secondary/60 p-5 text-left">
        <span className="text-xs font-semibold text-vx-text-secondary">
          {t("onboarding.complete.next_title")}
        </span>
        <ol className="flex flex-col gap-2">
          <li className="flex gap-2.5 text-xs leading-relaxed text-vx-text-dim">
            <Check className="mt-0.5 h-3.5 w-3.5 shrink-0 text-vx-success" />
            <span>{t("onboarding.complete.next.tip1", { key: hotkeyKey ?? "Ctrl+Space" })}</span>
          </li>
          <li className="flex gap-2.5 text-xs leading-relaxed text-vx-text-dim">
            <Check className="mt-0.5 h-3.5 w-3.5 shrink-0 text-vx-success" />
            <span>{t("onboarding.complete.next.tip2")}</span>
          </li>
        </ol>
      </div>
      <Button variant="primary" size="lg" onClick={finish}>
        {t("onboarding.complete.start")}
      </Button>
      {generalError && <span className="text-xs text-vx-error">{generalError}</span>}
    </div>
  );
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
  t: (key: string, vars?: Record<string, string | number>) => string;
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
        <Button variant="secondary" type="button" onClick={() => void openUrl(GROQ_URL)}>
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
  onBinaryPathChange,
  onModelPathChange,
  onThreadsChange,
  onPickBinary,
  onPickModel,
  onTest,
}: {
  binaryPath: string;
  modelPath: string;
  threads: number;
  status: TestStatus;
  t: (key: string, vars?: Record<string, string | number>) => string;
  onBinaryPathChange: (value: string) => void;
  onModelPathChange: (value: string) => void;
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
          <CodeBlock lines={["git clone https://github.com/ggml-org/whisper.cpp.git", "cd whisper.cpp", "cmake -B build", "cmake --build build -j --config Release"]} />
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
          onChange={onBinaryPathChange}
          onBrowse={onPickBinary}
          browseLabel={t("onboarding.stt.offline.browse_binary")}
          hint={t("onboarding.stt.offline.binary_hint")}
        />
        <PathPickerField
          label={t("onboarding.stt.offline.model")}
          placeholder="C:\\models\\ggml-base.bin"
          value={modelPath}
          onChange={onModelPathChange}
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
          onChange={(e) => onThreadsChange(Math.max(1, Math.floor(Number(e.target.value) || 1)))}
        />
        <StatusButton
          status={status}
          idleLabel={t("onboarding.stt.offline.test")}
          okLabel={t("onboarding.stt.offline.test_ok")}
          failLabel={t("onboarding.stt.offline.test_fail")}
          errLabel={t("onboarding.stt.test_err")}
          onClick={onTest}
          disabled={status === "testing" || !binaryPath.trim() || !modelPath.trim()}
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
      className={`flex min-h-32 items-start gap-4 rounded-xl border p-5 text-left transition-colors ${
        active
          ? "border-vx-accent/60 bg-vx-accent-soft text-vx-text-primary"
          : "border-vx-border bg-vx-bg-secondary text-vx-text-secondary hover:border-vx-border-strong hover:text-vx-text-primary"
      }`}
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
  onChange,
  onBrowse,
}: {
  label: string;
  placeholder: string;
  value: string;
  hint: string;
  browseLabel: string;
  autoFocus?: boolean;
  onChange: (value: string) => void;
  onBrowse: () => void;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-2">
      <Input
        autoFocus={autoFocus}
        label={label}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
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
          <p className="mt-1 text-xs leading-relaxed text-vx-text-dim">{body}</p>
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

function StepProgress({ step, currentStepIdx, t }: { step: Step; currentStepIdx: number; t: TFunc }) {
  const setupIdx = SETUP_STEPS.indexOf(step);
  return (
    <div className="mx-auto flex w-full max-w-md flex-col gap-2 px-4">
      <div className="flex items-center justify-between text-xs">
        <span className="font-medium text-vx-text-secondary">{t(STEP_LABELS[step])}</span>
        {setupIdx >= 0 && (
          <span className="text-vx-text-dim">
            {t("onboarding.steps.counter", { current: setupIdx + 1, total: SETUP_STEPS.length })}
          </span>
        )}
      </div>
      <div className="flex gap-2" aria-label="Onboarding Progress">
        {STEPS.map((s, idx) => (
          <div
            key={s}
            className={`h-1.5 flex-1 rounded-full transition-colors duration-300 ${
              idx <= currentStepIdx ? "bg-vx-accent" : "bg-vx-border"
            }`}
          />
        ))}
      </div>
    </div>
  );
}

function HowItWorks({ t }: { t: TFunc }) {
  const steps = [
    { icon: Keyboard, label: t("onboarding.welcome.how.step1"), desc: t("onboarding.welcome.how.step1_desc") },
    { icon: Mic, label: t("onboarding.welcome.how.step2"), desc: t("onboarding.welcome.how.step2_desc") },
    { icon: Type, label: t("onboarding.welcome.how.step3"), desc: t("onboarding.welcome.how.step3_desc") },
  ];
  return (
    <div className="flex w-full max-w-2xl flex-col gap-3">
      <span className="text-xs font-semibold text-vx-text-secondary">
        {t("onboarding.welcome.how_title")}
      </span>
      <div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
        {steps.map(({ icon: Icon, label, desc }, index) => (
          <div key={label} className="flex flex-col gap-1.5 rounded-xl border border-vx-border/40 bg-vx-bg-secondary/60 p-4">
            <div className="flex items-center gap-2">
              <span className="flex h-7 w-7 items-center justify-center rounded-full bg-vx-accent-soft text-xs font-semibold text-vx-accent">
                {index + 1}
              </span>
              <Icon className="h-4 w-4 text-vx-accent" />
            </div>
            <span className="text-sm font-medium">{label}</span>
            <span className="text-xs leading-relaxed text-vx-text-dim">{desc}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function PrepHint({ t }: { t: TFunc }) {
  return (
    <div className="flex w-full max-w-2xl items-start gap-3 rounded-xl border border-vx-accent/20 bg-vx-accent-soft/40 p-4">
      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-vx-accent-soft text-vx-accent">
        <Settings className="h-4 w-4" />
      </span>
      <div className="flex flex-col gap-0.5">
        <span className="text-xs font-semibold text-vx-text-primary">
          {t("onboarding.welcome.prep_title")}
        </span>
        <span className="text-xs leading-relaxed text-vx-text-dim">
          {t("onboarding.welcome.prep_body")}
        </span>
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

function ModelGuide({ t }: { t: (key: string) => string }) {
  const rows = [
    ["tiny", t("onboarding.stt.offline.model_tiny")],
    ["base", t("onboarding.stt.offline.model_base")],
    ["small", t("onboarding.stt.offline.model_small")],
  ];
  return (
    <div className="grid gap-2">
      {rows.map(([name, desc]) => (
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

function stringSetting(value: unknown, fallback: string): string {
  return typeof value === "string" ? value : fallback;
}

function numberSetting(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function booleanSetting(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function sttEngineSetting(value: unknown): SttEngine {
  return value === "whisper_cpp" ? "whisper_cpp" : "groq";
}

function validateWhisperSetup(
  binaryPath: string,
  modelPath: string,
  t: (key: string) => string,
): string {
  if (!binaryPath.trim()) {
    return t("onboarding.stt.offline.binary_required");
  }
  if (!modelPath.trim()) {
    return t("onboarding.stt.offline.model_required");
  }
  return "";
}
