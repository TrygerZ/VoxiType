export type Step = "welcome" | "quick_settings" | "microphone" | "stt_setup" | "data_directory" | "hotkey" | "smoke_test" | "complete";
export type SttEngine = "groq" | "whisper_cpp";
export type TestStatus = "idle" | "testing" | "ok" | "fail" | "err";
export type TFunc = (key: string, vars?: Record<string, string | number>) => string;

export const STEPS: Step[] = ["welcome", "quick_settings", "microphone", "stt_setup", "data_directory", "hotkey", "smoke_test", "complete"];

export const STEP_LABELS: Record<Step, string> = {
  welcome: "onboarding.steps.label.intro",
  quick_settings: "onboarding.steps.label.language",
  microphone: "onboarding.steps.label.microphone",
  stt_setup: "onboarding.steps.label.stt",
  data_directory: "onboarding.steps.label.data_directory",
  hotkey: "onboarding.steps.label.hotkey",
  smoke_test: "onboarding.steps.label.smoke_test",
  complete: "onboarding.steps.label.complete",
};

export const SETUP_STEPS: Step[] = ["quick_settings", "microphone", "stt_setup", "data_directory", "hotkey", "smoke_test"];
