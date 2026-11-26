import type { Step, TFunc } from "../types";
import { STEP_LABELS, STEPS } from "../types";

export function StepProgress({
  step,
  currentStepIdx,
  t,
}: {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
}) {
  return (
    <div
      className="mx-auto flex w-full max-w-md flex-col gap-2 px-1"
      aria-label="Onboarding Progress"
    >
      <div className="flex items-center justify-between text-xs">
        <span className="font-medium text-vx-text-secondary">
          {t(STEP_LABELS[step])}
        </span>
        <span className="text-vx-text-muted">
          {t("onboarding.steps.counter", {
            current: currentStepIdx + 1,
            total: STEPS.length,
          })}
        </span>
      </div>
      <div
        className="flex gap-1.5"
        role="progressbar"
        aria-valuemin={1}
        aria-valuemax={STEPS.length}
        aria-valuenow={currentStepIdx + 1}
      >
        {STEPS.map((currentStep, idx) => {
          const segmentClass =
            idx < currentStepIdx
              ? "bg-vx-text-primary"
              : idx === currentStepIdx
                ? "bg-vx-accent-ember"
                : "bg-vx-border-subtle";
          return (
            <div
              key={currentStep}
              className={`h-1.5 flex-1 rounded-full transition-colors duration-300 ${segmentClass}`}
            />
          );
        })}
      </div>
    </div>
  );
}
