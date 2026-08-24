import { ArrowLeft, ChevronRight, Settings } from "lucide-react";

import { Button } from "../../ui/Button";
import { Select } from "../../ui/Select";
import { Switch } from "../../ui/Switch";
import { StepShell } from "../shared/StepShell";
import type { Step, TFunc } from "../types";

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  error: string;
  lang: string;
  soundCues: boolean;
  onLanguageChange: (value: string) => void;
  onSoundCuesChange: (value: boolean) => void;
  onBack: () => void;
  onContinue: () => void;
}

export function QuickSettingsStep({
  step,
  currentStepIdx,
  t,
  error,
  lang,
  soundCues,
  onLanguageChange,
  onSoundCuesChange,
  onBack,
  onContinue,
}: Props) {
  return (
    <StepShell
      step={step}
      currentStepIdx={currentStepIdx}
      t={t}
      icon={<Settings className="h-7 w-7" />}
      title={t("onboarding.step2.title")}
      description={t("onboarding.step2.body")}
      error={error}
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="primary" size="lg" onClick={onContinue}>
            {t("onboarding.step2.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
      <div className="flex w-full max-w-sm flex-col gap-5 rounded-xl border border-vx-border bg-vx-bg-secondary p-5 text-left shadow-vx-sm">
        <Select
          autoFocus
          label={t("onboarding.ui_language")}
          options={[
            { value: "id", label: "Bahasa Indonesia" },
            { value: "en", label: "English" },
          ]}
          value={lang}
          onChange={(event) => onLanguageChange(event.target.value)}
        />
        <div className="flex items-center justify-between gap-4 border-t border-vx-divider pt-4">
          <div>
            <p className="text-sm font-medium text-vx-text-primary">
              {t("onboarding.sound_cues")}
            </p>
            <p className="mt-1 text-xs text-vx-text-dim">
              {soundCues
                ? t("onboarding.sound_cues_on")
                : t("onboarding.sound_cues_off")}
            </p>
          </div>
          <Switch
            checked={soundCues}
            onChange={onSoundCuesChange}
            label={t("onboarding.sound_cues")}
          />
        </div>
      </div>
    </StepShell>
  );
}
