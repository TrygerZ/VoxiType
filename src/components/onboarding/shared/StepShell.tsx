import type { ReactNode } from "react";

import { StepProgress } from "./StepProgress";
import type { Step, TFunc } from "../types";

interface StepShellProps {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  icon: ReactNode;
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
  icon,
  title,
  description,
  children,
  actions,
  error,
  wide = false,
}: StepShellProps) {
  return (
    <main className="vx-app-bg flex h-full flex-col overflow-y-auto px-4 py-6 sm:px-8 sm:py-8">
      <div
        className={`mx-auto flex w-full flex-1 flex-col gap-6 ${wide ? "max-w-5xl" : "max-w-xl"}`}
      >
        <StepProgress step={step} currentStepIdx={currentStepIdx} t={t} />
        <header className="vx-animate-in flex flex-col items-center gap-3 text-center">
          <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-vx-accent-soft text-vx-accent">
            {icon}
          </span>
          <h1 className="text-balance text-2xl font-semibold tracking-tight sm:text-3xl">
            {title}
          </h1>
          <p className="max-w-2xl text-pretty text-sm leading-relaxed text-vx-text-dim">
            {description}
          </p>
        </header>
        <section className="flex flex-1 flex-col items-center gap-6">
          {children}
        </section>
        {error && (
          <p
            role="alert"
            className="rounded-lg border border-vx-error/30 bg-vx-error/10 px-3 py-2 text-center text-xs text-vx-error"
          >
            {error}
          </p>
        )}
        {actions && (
          <footer className="flex flex-wrap items-center justify-center gap-2 sm:gap-3">
            {actions}
          </footer>
        )}
      </div>
    </main>
  );
}
