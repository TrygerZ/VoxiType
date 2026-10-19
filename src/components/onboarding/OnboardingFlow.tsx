import { useState } from "react";

import { formatTauriError, setHotkey } from "../../lib/tauri";
import { usePlatform } from "../../lib/usePlatform";
import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import { getHotkeySetting } from "../../lib/settingsGuards";
import { formatAccelerator } from "../settings/HotkeyRecorder";
import { useSettingsStore } from "../../stores/settingsStore";
import { CompleteStep } from "./steps/CompleteStep";
import { DataDirectoryStep } from "./steps/DataDirectoryStep";
import { HotkeyStep } from "./steps/HotkeyStep";
import { QuickSettingsStep } from "./steps/QuickSettingsStep";
import { MicrophoneStep } from "./steps/MicrophoneStep";
import { SttSetupStep } from "./steps/SttSetupStep";
import { SmokeTestStep } from "./steps/SmokeTestStep";
import { WelcomeStep } from "./steps/WelcomeStep";
import type { Step, SttEngine } from "./types";
import { STEPS } from "./types";

interface OnboardingFlowProps { onComplete: () => void; }

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const t = useT();
  const settings = useSettingsStore((state) => state.settings);
  const update = useSettingsStore((state) => state.update);
  const updateWhisperPaths = useSettingsStore((state) => state.updateWhisperPaths);
  const loadSettings = useSettingsStore((state) => state.load);
  const [step, setStep] = useState<Step>("welcome");
  const [language, setLanguage] = useState(setting(settings.language, "en"));
  const [soundCues, setSoundCues] = useState(boolSetting(settings.sound_cues, false));
  const [micDevice, setMicDevice] = useState(setting(settings.mic_device, ""));
  const [sttEngine, setSttEngine] = useState<SttEngine>(settings.stt_engine === "whisper_cpp" ? "whisper_cpp" : "groq");
  const [sttLanguage, setSttLanguage] = useState(setting(settings.stt_language, "auto"));
  const [apiKey, setApiKey] = useState("");
  const [binary, setBinary] = useState(setting(settings.whisper_cpp_binary_path, ""));
  const [model, setModel] = useState(setting(settings.whisper_cpp_model_path, ""));
  const [threads, setThreads] = useState(numberSetting(settings.whisper_cpp_threads, 4));
  const hotkey = getHotkeySetting(settings.hotkey, { key: "Ctrl+Space", mode: "ptt" });
  const [hotkeyKey, setHotkeyKey] = useState(hotkey.key);
  const [hotkeyMode, setHotkeyMode] = useState(hotkey.mode);
  const [error, setError] = useState("");
  const [hotkeyError, setHotkeyError] = useState("");
  const [busy, setBusy] = useState(false);
  const platform = usePlatform();
  const currentStepIdx = STEPS.indexOf(step);

  // Stored accelerators use the cross-platform "Super" token; macOS shows Cmd.
  const hotkeyDisplay = formatAccelerator(hotkeyKey, platform);

  const finish = async () => {
    if (busy) return;
    setBusy(true);
    try {
      await update("onboarding_completed", true);
      onComplete();
    } catch (e: unknown) {
      setError(formatTauriError(e));
    } finally {
      setBusy(false);
    }
  };
  const saveQuickSettings = async () => {
    if (busy) return;
    setBusy(true);
    try {
      setError("");
      await update("language", language);
      await update("sound_cues", soundCues);
      setStep("microphone");
    } catch (e: unknown) {
      setError(formatTauriError(e));
    } finally {
      setBusy(false);
    }
  };
  const saveMicrophone = async () => {
    if (busy) return;
    setBusy(true);
    try {
      setError("");
      if (micDevice) await update("mic_device", micDevice);
      setStep("stt_setup");
    } catch (e: unknown) {
      setError(formatTauriError(e));
    } finally {
      setBusy(false);
    }
  };
  const saveStt = async () => {
    if (busy) return;
    setBusy(true);
    try {
      setError("");
      await update("stt_engine", sttEngine);
      await update("stt_language", sttLanguage);
      if (sttEngine === "groq" && apiKey.trim()) await update("groq_api_key", apiKey.trim());
      if (sttEngine === "whisper_cpp") {
        await updateWhisperPaths(binary.trim(), model.trim());
        await update("whisper_cpp_threads", threads);
      }
      setStep("data_directory");
    } catch (e: unknown) {
      setError(formatTauriError(e));
    } finally {
      setBusy(false);
    }
  };
  const saveHotkey = async () => {
    if (busy) return;
    setBusy(true);
    try {
      await setHotkey(hotkeyKey, hotkeyMode);
      await loadSettings();
      setStep("smoke_test");
    } catch (e: unknown) {
      setHotkeyError(formatTauriError(e));
    } finally {
      setBusy(false);
    }
  };

  if (step === "welcome") return <WelcomeStep step={step} currentStepIdx={currentStepIdx} t={t} error={error} busy={busy} onStart={() => setStep("quick_settings")} onSkip={() => void finish()} />;
  if (step === "quick_settings") return <QuickSettingsStep step={step} currentStepIdx={currentStepIdx} t={t} error={error} busy={busy} lang={language} soundCues={soundCues} onLanguageChange={(value) => { setLanguage(value); void invokeAction(() => update("language", value)); }} onSoundCuesChange={setSoundCues} onBack={() => setStep("welcome")} onContinue={saveQuickSettings} />;
  if (step === "microphone") return <MicrophoneStep step={step} currentStepIdx={currentStepIdx} t={t} busy={busy} selectedDevice={micDevice} onDeviceChange={setMicDevice} onBack={() => setStep("quick_settings")} onContinue={() => void saveMicrophone()} onSkip={() => setStep("stt_setup")} />;
  if (step === "stt_setup") return <SttSetupStep step={step} currentStepIdx={currentStepIdx} t={t} error={error} busy={busy} sttEngine={sttEngine} sttLanguage={sttLanguage} apiKey={apiKey} whisperBinary={binary} whisperModel={model} whisperThreads={threads} platform={platform} onEngineChange={setSttEngine} onLanguageChange={setSttLanguage} onApiKeyChange={setApiKey} onThreadsChange={setThreads} onBinaryChange={setBinary} onModelChange={setModel} onSave={() => void saveStt()} onBack={() => setStep("quick_settings")} onSkip={() => setStep("data_directory")} />;
  if (step === "data_directory") return <DataDirectoryStep step={step} currentStepIdx={currentStepIdx} t={t} onBack={() => setStep("stt_setup")} onSkip={() => setStep("hotkey")} onContinue={() => setStep("hotkey")} />;
  if (step === "hotkey") return <HotkeyStep step={step} currentStepIdx={currentStepIdx} t={t} error={hotkeyError} busy={busy} hotkeyKey={hotkeyKey} hotkeyMode={hotkeyMode} platform={platform} onKeyChange={setHotkeyKey} onModeChange={setHotkeyMode} onBack={() => setStep("data_directory")} onContinue={saveHotkey} />;
  if (step === "smoke_test") return <SmokeTestStep step={step} currentStepIdx={currentStepIdx} t={t} busy={busy} hotkeyDisplay={hotkeyDisplay} onBack={() => setStep("hotkey")} onContinue={() => setStep("complete")} onSkip={() => setStep("complete")} />;
  return <CompleteStep step={step} currentStepIdx={currentStepIdx} t={t} error={error} busy={busy} hotkeyDisplay={hotkeyDisplay} onFinish={() => void finish()} />;
}

function setting(value: unknown, fallback: string) { return typeof value === "string" ? value : fallback; }
function boolSetting(value: unknown, fallback: boolean) { return typeof value === "boolean" ? value : fallback; }
function numberSetting(value: unknown, fallback: number) { return typeof value === "number" && Number.isFinite(value) ? value : fallback; }
