import { useState } from "react";
import { Mic, Check, ChevronRight, Zap, Cloud, Languages } from "lucide-react";
import { Button } from "../ui/Button";
import { useT } from "../../lib/i18n";
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
  const t = useT();
  const [step, setStep] = useState<Step>("welcome");
  const update = useSettingsStore((s) => s.update);

  const finish = () => {
    void update("onboarding_completed", true);
    onComplete();
  };

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
            <div
              key={title}
              className="flex flex-col gap-1.5 rounded-xl bg-vx-bg-secondary p-5"
            >
              <Icon className="h-5 w-5 text-vx-accent" />
              <span className="text-sm font-medium">{title}</span>
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
    <div className="vx-app-bg flex h-full flex-col items-center justify-center gap-8 p-10 text-center">
      <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-vx-success/10 text-vx-success">
        <Check className="h-7 w-7" />
      </span>
      <h1 className="text-3xl font-semibold tracking-tight">
        {t("onboarding.complete.title")}
      </h1>
      <p className="max-w-sm text-sm text-vx-text-dim">
        Press{" "}
        <kbd className="rounded-md bg-vx-bg-tertiary px-2 py-0.5 text-xs font-semibold text-vx-text-primary">
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
