import { ArrowLeft, ChevronRight, Keyboard } from "lucide-react";

import { Button } from "../../ui/Button";
import { Select } from "../../ui/Select";
import { HotkeyRecorder } from "../../settings/HotkeyRecorder";
import { StepShell } from "../shared/StepShell";
import type { Step, TFunc } from "../types";

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  error: string;
  hotkeyKey: string;
  hotkeyMode: string;
  onKeyChange: (value: string) => void;
  onModeChange: (value: string) => void;
  onBack: () => void;
  onContinue: () => void;
}

export function HotkeyStep({
  step,
  currentStepIdx,
  t,
  error,
  hotkeyKey,
  hotkeyMode,
  onKeyChange,
  onModeChange,
  onBack,
  onContinue,
}: Props) {
  return (
    <StepShell
      step={step}
      currentStepIdx={currentStepIdx}
      t={t}
      icon={<Keyboard className="h-7 w-7" />}
      title={t("onboarding.step4.title")}
      description={t("onboarding.step4.body")}
      error={error}
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="primary" size="lg" onClick={onContinue}>
            {t("onboarding.step4.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
      <div className="flex w-full max-w-sm flex-col gap-5 rounded-xl border border-vx-border bg-vx-bg-secondary p-5 text-left shadow-vx-sm">
        <HotkeyRecorder value={hotkeyKey} onChange={onKeyChange} />
        <Select
          label={t("onboarding.step4.mode_label")}
          options={[
            { value: "ptt", label: t("onboarding.step4.mode_ptt") },
            { value: "toggle", label: t("onboarding.step4.mode_toggle") },
          ]}
          value={hotkeyMode}
          onChange={(event) => onModeChange(event.target.value)}
        />
      </div>
    </StepShell>
  );
}
