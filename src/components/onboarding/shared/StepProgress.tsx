import type { Step, TFunc } from "../types";
import { SETUP_STEPS, STEP_LABELS, STEPS } from "../types";

export function StepProgress({
  step,
  currentStepIdx,
  t,
}: {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
}) {
  const setupIdx = SETUP_STEPS.indexOf(step);
  return (
    <div
      className="mx-auto flex w-full max-w-md flex-col gap-2 px-1"
      aria-label="Onboarding Progress"
    >
      <div className="flex items-center justify-between text-xs">
        <span className="font-medium text-vx-text-secondary">
          {t(STEP_LABELS[step])}
        </span>
        {setupIdx >= 0 && (
          <span className="text-vx-text-dim">
            {t("onboarding.steps.counter", {
              current: setupIdx + 1,
              total: SETUP_STEPS.length,
            })}
          </span>
        )}
      </div>
      <div
        className="flex gap-1.5"
        role="progressbar"
        aria-valuemin={1}
        aria-valuemax={STEPS.length}
        aria-valuenow={currentStepIdx + 1}
      >
        {STEPS.map((currentStep, idx) => (
          <div
            key={currentStep}
            className={`h-1.5 flex-1 rounded-full transition-colors duration-300 ${
              idx <= currentStepIdx ? "bg-vx-accent" : "bg-vx-border"
            }`}
          />
        ))}
      </div>
    </div>
  );
}
