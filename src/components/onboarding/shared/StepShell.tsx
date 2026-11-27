import type { ReactNode } from "react";

import { StepProgress } from "./StepProgress";
import type { Step, TFunc } from "../types";

interface StepShellProps {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  icon?: ReactNode;
  title: string;
  description: string;
  children: ReactNode;
  actions?: ReactNode;
  error?: string;
  wide?: boolean;
}

export function StepShell({
  step,
  currentStepIdx,
  t,
  title,
  description,
  children,
  actions,
  error,
  wide = false,
}: StepShellProps) {
  return (
    <main className="vx-app-bg flex h-full flex-col overflow-y-auto px-4 py-4 sm:px-8 sm:py-6">
      <div
        className={`mx-auto flex w-full flex-1 flex-col justify-between gap-4 ${wide ? "max-w-3xl" : "max-w-xl"}`}
      >
        <StepProgress step={step} currentStepIdx={currentStepIdx} t={t} />
        <div className="vx-panel-reveal my-auto flex flex-col items-center gap-5 py-2">
          <header className="flex flex-col items-center gap-1.5 text-center">
            <h1 className="text-balance text-xl sm:text-2xl font-semibold tracking-tight text-vx-text-primary">
              {title}
            </h1>
            <p className="max-w-lg text-pretty text-sm leading-relaxed text-vx-text-secondary">
              {description}
            </p>
          </header>
          <section className="flex w-full flex-col items-center gap-4">
            {children}
          </section>
          {error && (
            <p
              role="alert"
              className="w-full max-w-md rounded-[8px] border border-vx-semantic-error/30 bg-vx-semantic-error-soft px-3 py-2 text-center text-xs text-vx-semantic-error"
            >
              {error}
            </p>
          )}
        </div>
        {actions ? (
          <footer className="flex flex-wrap items-center justify-center gap-3 pb-1">
            {actions}
          </footer>
        ) : (
          <div />
        )}
      </div>
    </main>
  );
}
