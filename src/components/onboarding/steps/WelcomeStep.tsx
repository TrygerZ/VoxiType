import { ChevronRight } from "lucide-react";

import { Button } from "../../ui/Button";
import { StatusBadge } from "../../ui/StatusBadge";
import { StepProgress } from "../shared/StepProgress";
import type { Step, TFunc } from "../types";

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  error: string;
  busy: boolean;
  onStart: () => void;
  onSkip: () => void;
}

export function WelcomeStep({
  step,
  currentStepIdx,
  t,
  error,
  busy,
  onStart,
  onSkip,
}: Props) {
  const features = [
    {
      num: "01",
      title: t("onboarding.feature.dictate.title"),
      body: t("onboarding.feature.dictate.body"),
    },
    {
      num: "02",
      title: t("onboarding.feature.offline.title"),
      body: t("onboarding.feature.offline.body"),
    },
    {
      num: "03",
      title: t("onboarding.feature.bilingual.title"),
      body: t("onboarding.feature.bilingual.body"),
    },
  ];

  return (
    <main className="vx-app-bg flex h-full flex-col overflow-y-auto px-4 py-4 sm:px-8 sm:py-6">
      <div className="mx-auto flex w-full max-w-2xl flex-1 flex-col justify-between gap-4">
        <StepProgress step={step} currentStepIdx={currentStepIdx} t={t} />
        <div className="vx-panel-reveal my-auto flex flex-col items-center gap-6 py-2">
          <header className="flex flex-col items-center gap-3 text-center">
            <div className="flex h-12 w-12 items-center justify-center overflow-hidden rounded-[12px] border border-vx-border-default bg-vx-bg-surface p-2">
              <img
                src="/logo.png"
                alt="VoxiType"
                className="h-full w-full object-contain"
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <h1 className="text-balance text-xl sm:text-2xl font-semibold tracking-tight text-vx-text-primary">
                {t("onboarding.welcome.title")}
              </h1>
              <p className="max-w-md text-pretty text-sm leading-relaxed text-vx-text-secondary">
                {t("onboarding.welcome.body")}
              </p>
            </div>
          </header>

          <div className="grid w-full grid-cols-1 gap-3 sm:grid-cols-3">
            {features.map(({ num, title, body }) => (
              <div
                key={num}
                className="flex flex-col gap-2 rounded-[12px] border border-vx-border-subtle bg-vx-bg-surface p-4 text-left"
              >
                <StatusBadge
                  tone="neutral"
                  className="w-fit font-mono text-[11px]"
                >
                  {num}
                </StatusBadge>
                <span className="text-sm font-medium text-vx-text-primary">
                  {title}
                </span>
                <span className="text-xs leading-relaxed text-vx-text-muted">
                  {body}
                </span>
              </div>
            ))}
          </div>

          {error && (
            <p
              role="alert"
              className="w-full max-w-md rounded-[8px] border border-vx-semantic-error/30 bg-vx-semantic-error-soft px-3 py-2 text-center text-xs text-vx-semantic-error"
            >
              {error}
            </p>
          )}
        </div>

        <footer className="flex flex-wrap items-center justify-center gap-3 pb-1">
          <Button variant="ghost" size="lg" disabled={busy} onClick={onSkip}>
            {t("onboarding.welcome.skip")}
          </Button>
          <Button variant="primary" size="lg" disabled={busy} onClick={onStart}>
            {t("onboarding.welcome.start")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </footer>
      </div>
    </main>
  );
}
