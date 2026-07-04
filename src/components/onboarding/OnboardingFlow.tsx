import { useState } from "react";
import {
  Mic, Check, ChevronRight, Zap, Cloud, Languages,
  Key, Keyboard, Settings, ExternalLink, Loader2, XCircle,
} from "lucide-react";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { useT } from "../../lib/i18n";
import { useSettingsStore } from "../../stores/settingsStore";
import { testGroqApi, setHotkey, openUrl, formatTauriError } from "../../lib/tauri";
import { HotkeyRecorder } from "../settings/HotkeyRecorder";

type Step = "welcome" | "quick_settings" | "groq_api" | "hotkey" | "complete";

interface OnboardingFlowProps {
  onComplete: () => void;
}

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const t = useT();
  const [step, setStep] = useState<Step>("welcome");
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const loadSettings = useSettingsStore((s) => s.load);

  // --- Step 1: Features (i18n'd inside component so t() is in scope) ---
  const features = [
    { icon: Mic, title: t("onboarding.feature.dictate.title"), body: t("onboarding.feature.dictate.body") },
    { icon: Cloud, title: t("onboarding.feature.cloud.title"), body: t("onboarding.feature.cloud.body") },
    { icon: Zap, title: t("onboarding.feature.formatting.title"), body: t("onboarding.feature.formatting.body") },
    { icon: Languages, title: t("onboarding.feature.bilingual.title"), body: t("onboarding.feature.bilingual.body") },
  ];

  // --- Step 2: Quick Settings local state ---
  const [lang, setLang] = useState((settings.language as string) ?? "id");
  const [soundCues, setSoundCues] = useState((settings.sound_cues as boolean) ?? false);

  // --- Step 3: Groq API local state ---
  const [apiKey, setApiKey] = useState("");
  const [testStatus, setTestStatus] = useState<"idle" | "testing" | "ok" | "fail" | "err">("idle");

  // --- Step 4: Hotkey local state ---
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
      setStep("groq_api");
    } catch (e: unknown) {
      setGeneralError(formatTauriError(e));
    }
  };

  const saveGroqApi = async () => {
    try {
      setGeneralError("");
      if (apiKey.trim()) {
        await update("groq_api_key", apiKey);
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
    setTestStatus("testing");
    try {
      await testGroqApi(apiKey.trim());
      setTestStatus("ok");
    } catch (e: unknown) {
      const code = (e as { code?: string })?.code;
      if (code === "SttApiKeyInvalid") {
        setTestStatus("fail");
      } else {
        setTestStatus("err");
      }
    }
  };

  // ============ Step 1: Welcome ============
  if (step === "welcome") {
    return (
      <div className="vx-app-bg flex h-full flex-col items-center justify-center gap-10 p-10">
        <div className="flex flex-col items-center gap-4 text-center">
          <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-vx-accent-soft text-vx-accent">
            <Mic className="h-7 w-7" />
          </span>
          <h1 className="text-3xl font-semibold tracking-tight">
            {t("onboarding.welcome.title")}
          </h1>
          <p className="max-w-sm text-sm text-vx-text-dim">
            {t("onboarding.welcome.body")}
          </p>
        </div>

        <div className="grid w-full max-w-md grid-cols-2 gap-4">
          {features.map(({ icon: Icon, title, body }) => (
            <div key={title} className="flex flex-col gap-1.5 rounded-xl bg-vx-bg-secondary p-5">
              <Icon className="h-5 w-5 text-vx-accent" />
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
        {generalError && (
          <span className="text-xs text-vx-error">{generalError}</span>
        )}
      </div>
    );
  }

  // ============ Step 2: Quick Settings ============
  if (step === "quick_settings") {
    return (
      <div className="vx-app-bg flex h-full flex-col items-center justify-center gap-8 p-10 text-center">
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
          {/* Language */}
          <div>
            <label className="mb-1.5 block text-xs font-medium text-vx-text-secondary">
              {t("onboarding.ui_language")}
            </label>
            <Select
              options={[
                { value: "id", label: "Bahasa Indonesia" },
                { value: "en", label: "English" },
              ]}
              value={lang}
              onChange={(e) => setLang(e.target.value)}
              className="w-full"
            />
          </div>

          {/* Sound Cues */}
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

        <Button variant="primary" size="lg" onClick={saveQuickSettings}>
          {t("onboarding.step2.continue")}
          <ChevronRight className="h-4 w-4" />
        </Button>
        {generalError && (
          <span className="text-xs text-vx-error">{generalError}</span>
        )}
      </div>
    );
  }

  // ============ Step 3: Groq API Setup ============
  if (step === "groq_api") {
    return (
      <div className="vx-app-bg flex h-full flex-col items-center justify-center gap-8 p-10 text-center">
        <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-vx-accent-soft text-vx-accent">
          <Key className="h-7 w-7" />
        </span>
        <h1 className="text-3xl font-semibold tracking-tight">
          {t("onboarding.step3.title")}
        </h1>
        <p className="max-w-sm text-sm text-vx-text-dim">
          {t("onboarding.step3.body")}
        </p>

        {/* Groq Console link — uses backend open_url */}
        <button
          type="button"
          onClick={() => void openUrl("https://console.groq.com")}
          className="inline-flex items-center gap-2 text-sm text-vx-accent hover:underline"
        >
          <ExternalLink className="h-4 w-4" />
          {t("onboarding.step3.get_key")}
        </button>

        <div className="flex w-full max-w-sm flex-col gap-3">
          <Input
            label={t("onboarding.step3.api_key_label")}
            type="password"
            showPasswordToggle
            placeholder="gsk_..."
            value={apiKey}
            onChange={(e) => {
              setApiKey(e.target.value);
              if (testStatus !== "idle") setTestStatus("idle");
            }}
          />

          {/* Test connection button */}
          <button
            type="button"
            onClick={handleTestApi}
            disabled={!apiKey.trim() || testStatus === "testing"}
            className={`w-full flex items-center justify-center gap-2 rounded-lg border px-4 py-2.5 text-sm font-medium transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed ${
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
              ? t("onboarding.step3.testing")
              : testStatus === "ok"
                ? t("onboarding.step3.test_ok")
                : testStatus === "fail"
                  ? t("onboarding.step3.test_fail")
                  : testStatus === "err"
                    ? t("onboarding.step3.test_err")
                    : t("onboarding.step3.test")}
          </button>
        </div>

        <div className="flex gap-3">
          <Button variant="primary" size="lg" onClick={saveGroqApi}>
            {t("onboarding.step3.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="lg" onClick={() => setStep("hotkey")}>
            {t("onboarding.step3.skip")}
          </Button>
        </div>
        {generalError && (
          <span className="text-xs text-vx-error">{generalError}</span>
        )}
      </div>
    );
  }

  // ============ Step 4: Hotkey Setup ============
  if (step === "hotkey") {
    return (
      <div className="vx-app-bg flex h-full flex-col items-center justify-center gap-8 p-10 text-center">
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

        <Button variant="primary" size="lg" onClick={saveHotkey}>
          {t("onboarding.step4.continue")}
          <ChevronRight className="h-4 w-4" />
        </Button>
        {hotkeyError && (
          <span className="text-xs text-vx-error">{hotkeyError}</span>
        )}
      </div>
    );
  }

  // ============ Step 5: Complete ============
  const currentHotkey = hotkeyKey ?? "Ctrl+Space";
  return (
    <div className="vx-app-bg flex h-full flex-col items-center justify-center gap-8 p-10 text-center">
      <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-vx-success/10 text-vx-success">
        <Check className="h-7 w-7" />
      </span>
      <h1 className="text-3xl font-semibold tracking-tight">
        {t("onboarding.complete.title")}
      </h1>
      <p className="max-w-sm text-sm text-vx-text-dim">
        {t("onboarding.complete.body", { key: currentHotkey })}
      </p>
      <Button variant="primary" size="lg" onClick={finish}>
        {t("onboarding.complete.start")}
      </Button>
      {generalError && (
        <span className="text-xs text-vx-error">{generalError}</span>
      )}
    </div>
  );
}
