import { useState } from "react";
import { Mic, Check, ChevronRight, Zap, Cloud, Languages } from "lucide-react";
import { Button } from "../ui/Button";
import { t } from "../../lib/i18n";
import { useSettingsStore } from "../../stores/settingsStore";

type Step = "welcome" | "complete";

interface OnboardingFlowProps {
  onComplete: () => void;
}

const features = [
  {
    icon: Mic,
    title: "Dictate anywhere",
    body: "Press Ctrl+Space to dictate into any application.",
  },
  {
    icon: Cloud,
    title: "Local or cloud",
    body: "Whisper.cpp on-device, or free Groq cloud transcription.",
  },
  {
    icon: Zap,
    title: "Smart formatting",
    body: "Dictation, Message, and Email modes clean up your text.",
  },
  {
    icon: Languages,
    title: "Bilingual",
    body: "Indonesian and English with optional translation.",
  },
];

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const [step, setStep] = useState<Step>("welcome");
  const update = useSettingsStore((s) => s.update);

  const finish = () => {
    void update("onboarding_completed", true);
    onComplete();
  };

  if (step === "welcome") {
    return (
      <div className="vx-app-bg flex h-full flex-col items-center justify-center gap-8 p-8">
        <div className="flex flex-col items-center gap-4 text-center">
          <span className="flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-vx-accent to-vx-accent-hover shadow-[0_8px_32px_rgba(124,108,240,0.45)]">
            <Mic className="h-8 w-8 text-white" />
          </span>
          <h1 className="text-2xl font-bold tracking-tight">
            {t("onboarding.welcome.title")}
          </h1>
          <p className="max-w-sm text-sm text-vx-text-secondary">
            {t("onboarding.welcome.body")}
          </p>
        </div>

        <div className="grid w-full max-w-md grid-cols-2 gap-3">
          {features.map(({ icon: Icon, title, body }) => (
            <div
              key={title}
              className="flex flex-col gap-1.5 rounded-xl border border-vx-border bg-vx-bg-secondary/60 p-4"
            >
              <Icon className="h-5 w-5 text-vx-accent" />
              <span className="text-sm font-semibold">{title}</span>
              <span className="text-xs leading-relaxed text-vx-text-dim">
                {body}
              </span>
            </div>
          ))}
        </div>

        <div className="flex gap-3">
          <Button variant="primary" size="lg" onClick={() => setStep("complete")}>
            {t("onboarding.welcome.start")}
            <ChevronRight className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="lg" onClick={finish}>
            {t("onboarding.welcome.skip")}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="vx-app-bg flex h-full flex-col items-center justify-center gap-6 p-8 text-center">
      <span className="flex h-16 w-16 items-center justify-center rounded-2xl bg-vx-success/15 text-vx-success shadow-[0_8px_32px_rgba(45,212,167,0.3)]">
        <Check className="h-8 w-8" />
      </span>
      <h1 className="text-2xl font-bold tracking-tight">
        {t("onboarding.complete.title")}
      </h1>
      <p className="max-w-sm text-sm text-vx-text-secondary">
        Press{" "}
        <kbd className="rounded-md border border-vx-border bg-vx-bg-tertiary px-2 py-0.5 text-xs font-semibold text-vx-text-primary">
          Ctrl+Space
        </kbd>{" "}
        to start dictating. Add your Groq API key in Settings for cloud
        transcription.
      </p>
      <Button variant="primary" size="lg" onClick={finish}>
        {t("onboarding.complete.start")}
      </Button>
    </div>
  );
}
