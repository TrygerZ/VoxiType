import { Check } from "lucide-react";

import { Button } from "../../ui/Button";
import { StepShell } from "../shared/StepShell";
import type { Step, TFunc } from "../types";

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  error: string;
  hotkeyKey: string;
  onFinish: () => void;
}

export function CompleteStep({
  step,
  currentStepIdx,
  t,
  error,
  hotkeyKey,
  onFinish,
}: Props) {
  return (
    <StepShell
      step={step}
      currentStepIdx={currentStepIdx}
      t={t}
      icon={<Check className="h-7 w-7" />}
      title={t("onboarding.complete.title")}
      description={t("onboarding.complete.body", { key: hotkeyKey })}
      error={error}
      actions={
        <Button variant="primary" size="lg" onClick={onFinish}>
          {t("onboarding.complete.start")}
        </Button>
      }
    >
      <div className="w-full max-w-sm rounded-xl border border-vx-border bg-vx-bg-secondary p-5 text-left shadow-vx-sm">
        <p className="text-xs font-semibold text-vx-text-secondary">
          {t("onboarding.complete.next_title")}
        </p>
        <ol className="mt-3 flex flex-col gap-3">
          <li className="flex gap-2.5 text-xs leading-relaxed text-vx-text-dim">
            <Check className="mt-0.5 h-3.5 w-3.5 shrink-0 text-vx-success" />
            <span>
              {t("onboarding.complete.next.tip1", { key: hotkeyKey })}
            </span>
          </li>
          <li className="flex gap-2.5 text-xs leading-relaxed text-vx-text-dim">
            <Check className="mt-0.5 h-3.5 w-3.5 shrink-0 text-vx-success" />
            <span>{t("onboarding.complete.next.tip2")}</span>
          </li>
        </ol>
      </div>
    </StepShell>
  );
}
