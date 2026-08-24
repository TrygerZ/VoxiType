import {
  ChevronRight,
  HardDrive,
  Languages,
  Mic,
  Settings,
} from "lucide-react";

import { Button } from "../../ui/Button";
import { StepProgress } from "../shared/StepProgress";
import type { Step, TFunc } from "../types";

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  error: string;
  onStart: () => void;
  onSkip: () => void;
}

export function WelcomeStep({
  step,
  currentStepIdx,
  t,
  error,
  onStart,
  onSkip,
}: Props) {
  const features = [
    {
      icon: Mic,
      title: t("onboarding.feature.dictate.title"),
      body: t("onboarding.feature.dictate.body"),
    },
    {
      icon: HardDrive,
      title: t("onboarding.feature.offline.title"),
      body: t("onboarding.feature.offline.body"),
    },
    {
      icon: Languages,
      title: t("onboarding.feature.bilingual.title"),
      body: t("onboarding.feature.bilingual.body"),
    },
  ];
  return (
    <main className="vx-app-bg flex h-full flex-col items-center gap-8 overflow-y-auto p-6 sm:gap-10 sm:p-10">
      <StepProgress step={step} currentStepIdx={currentStepIdx} t={t} />
      <header className="vx-animate-in flex flex-col items-center gap-5 text-center">
        <div className="relative flex h-20 w-20 items-center justify-center">
          <span
            className="absolute inset-0 rounded-full bg-vx-accent/20 blur-2xl"
            aria-hidden="true"
          />
          <div className="relative flex h-20 w-20 items-center justify-center overflow-hidden rounded-full border border-vx-border/30 bg-vx-bg-secondary">
            <img
              src="/logo.png"
              alt="VoxiType"
              className="h-full w-full object-contain"
            />
          </div>
        </div>
        <div className="flex flex-col gap-2.5">
          <h1 className="text-balance text-2xl font-semibold tracking-tight sm:text-3xl">
            {t("onboarding.welcome.title")}
          </h1>
          <p className="max-w-sm text-pretty text-sm leading-relaxed text-vx-text-dim">
            {t("onboarding.welcome.body")}
          </p>
        </div>
      </header>
      <div className="flex w-full max-w-2xl flex-col gap-3">
        <span className="text-xs font-semibold text-vx-text-secondary">
          {t("onboarding.welcome.how_title")}
        </span>
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
          {["step1", "step2", "step3"].map((name, index) => (
            <div
              key={name}
              className="flex flex-col gap-1.5 rounded-xl border border-vx-border/40 bg-vx-bg-secondary/60 p-4"
            >
              <span className="flex h-7 w-7 items-center justify-center rounded-full bg-vx-accent-soft text-xs font-semibold text-vx-accent">
                {index + 1}
              </span>
              <span className="text-sm font-medium">
                {t(`onboarding.welcome.how.${name}`)}
              </span>
              <span className="text-xs leading-relaxed text-vx-text-dim">
                {t(`onboarding.welcome.how.${name}_desc`)}
              </span>
            </div>
          ))}
        </div>
      </div>
      <div className="flex w-full max-w-2xl items-start gap-3 rounded-xl border border-vx-accent/20 bg-vx-accent-soft/40 p-4">
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-vx-accent-soft text-vx-accent">
          <Settings className="h-4 w-4" />
        </span>
        <div>
          <span className="text-xs font-semibold">
            {t("onboarding.welcome.prep_title")}
          </span>
          <p className="text-xs leading-relaxed text-vx-text-dim">
            {t("onboarding.welcome.prep_body")}
          </p>
        </div>
      </div>
      <div className="grid w-full max-w-2xl grid-cols-1 gap-3 sm:grid-cols-3">
        {features.map(({ icon: Icon, title, body }) => (
          <div
            key={title}
            className="flex flex-col gap-2.5 rounded-xl border border-vx-border/40 bg-vx-bg-secondary/60 p-5 transition-colors duration-200 hover:border-vx-border-strong"
          >
            <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-vx-accent-soft text-vx-accent">
              <Icon className="h-5 w-5" />
            </span>
            <span className="text-sm font-medium">{title}</span>
            <span className="text-xs leading-relaxed text-vx-text-dim">
              {body}
            </span>
          </div>
        ))}
      </div>
      <div className="flex flex-wrap justify-center gap-3">
        <Button variant="primary" size="lg" onClick={onStart}>
          {t("onboarding.welcome.start")}
          <ChevronRight className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="lg" onClick={onSkip}>
          {t("onboarding.welcome.skip")}
        </Button>
      </div>
      {error && (
        <p role="alert" className="text-xs text-vx-error">
          {error}
        </p>
      )}
    </main>
  );
}
