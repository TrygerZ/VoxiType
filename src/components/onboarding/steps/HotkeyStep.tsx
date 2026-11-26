import { ArrowLeft, ChevronRight } from "lucide-react";

import { HotkeyRecorder } from "../../settings/HotkeyRecorder";
import { Button } from "../../ui/Button";
import { Select } from "../../ui/Select";
import { StepShell } from "../shared/StepShell";
import type { Step, TFunc } from "../types";

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  error: string;
  busy: boolean;
  hotkeyKey: string;
  hotkeyMode: string;
  platform?: string;
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
  busy,
  hotkeyKey,
  hotkeyMode,
  platform = "unknown",
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
      title={t("onboarding.step4.title")}
      description={t("onboarding.step4.body")}
      error={error}
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="primary" size="lg" disabled={busy} onClick={onContinue}>
            {t("onboarding.step4.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
      <div className="flex w-full max-w-md flex-col gap-5 rounded-[12px] border border-vx-border-subtle bg-vx-bg-surface p-5 text-left">
        <HotkeyRecorder
          value={hotkeyKey}
          onChange={onKeyChange}
          platform={platform}
        />
        <div className="border-t border-vx-border-subtle pt-4">
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
      </div>
    </StepShell>
  );
}
