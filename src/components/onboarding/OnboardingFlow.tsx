import { useState } from "react";
import { Mic, Check, ChevronRight, SkipForward } from "lucide-react";
import { Button } from "../ui/Button";
import { t } from "../../lib/i18n";
import { useSettingsStore } from "../../stores/settingsStore";

type Step = "welcome" | "complete";

interface OnboardingFlowProps {
  onComplete: () => void;
}

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const [step, setStep] = useState<Step>("welcome");
  const update = useSettingsStore((s) => s.update);

  const finish = () => {
    void update("onboarding_completed", true);
    onComplete();
  };

  if (step === "welcome") {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-6 p-8">
        <div className="rounded-2xl bg-vx-accent/10 p-6">
          <Mic className="h-12 w-12 text-vx-accent" />
        </div>
        <h1 className="text-2xl font-bold">{t("onboarding.welcome.title")}</h1>
        <p className="max-w-sm text-center text-sm text-vx-text-secondary">
          {t("onboarding.welcome.body")}
        </p>
        <div className="flex flex-col gap-3 text-sm text-vx-text-secondary">
          <p>&#x2022; Voice-to-text in any app with Ctrl+Space</p>
          <p>&#x2022; Local STT (Whisper.cpp) + cloud fallback (Groq)</p>
          <p>&#x2022; AI formatting: Dictation, Message, Email modes</p>
        </div>
        <div className="flex gap-3">
          <Button variant="primary" onClick={() => setStep("complete")}>
            {t("onboarding.welcome.start")}
            <ChevronRight className="h-4 w-4" />
          </Button>
          <Button variant="ghost" onClick={finish}>
            <SkipForward className="h-4 w-4" />
            {t("onboarding.welcome.skip")}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col items-center justify-center gap-6 p-8">
      <div className="rounded-2xl bg-vx-success/10 p-6">
        <Check className="h-12 w-12 text-vx-success" />
      </div>
      <h1 className="text-2xl font-bold">{t("onboarding.complete.title")}</h1>
      <p className="text-sm text-vx-text-secondary">
        Press <kbd className="rounded bg-vx-bg-tertiary px-1.5 py-0.5 text-xs font-medium">Ctrl+Space</kbd> to start dictating.
      </p>
      <Button variant="primary" onClick={finish}>
        {t("onboarding.complete.start")}
      </Button>
    </div>
  );
}
