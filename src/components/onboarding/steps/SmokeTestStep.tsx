import { useEffect, useState } from "react";
import { ArrowLeft, Check, ChevronRight, Mic } from "lucide-react";

import { onEvent } from "../../../lib/tauri";
import type {
  TranscriptionCompleteEvent,
  TranscriptionErrorEvent,
} from "../../../types/events";
import { Button } from "../../ui/Button";
import { StepShell } from "../shared/StepShell";
import type { Step, TFunc } from "../types";

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  hotkeyKey: string;
  onBack: () => void;
  onContinue: () => void;
  onSkip: () => void;
}

export function SmokeTestStep({
  step,
  currentStepIdx,
  t,
  hotkeyKey,
  onBack,
  onContinue,
  onSkip,
}: Props) {
  const [text, setText] = useState("");
  const [error, setError] = useState("");
  const [timedOut, setTimedOut] = useState(false);

  useEffect(() => {
    const listeners = [
      onEvent<TranscriptionCompleteEvent>("transcription_complete", (event) => {
        setText(event.text);
        setError("");
      }),
      onEvent<TranscriptionErrorEvent>("transcription_error", (event) =>
        setError(event.message),
      ),
    ];
    const timeout = window.setTimeout(() => setTimedOut(true), 30_000);
    return () => {
      window.clearTimeout(timeout);
      void Promise.all(listeners)
        .then((fns) => fns.forEach((fn) => fn()))
        .catch(() => {});
    };
  }, []);

  const inlineError = error
    ? `${t("onboarding.smoke_test.error", { message: error })} ${t("onboarding.smoke_test.troubleshoot")}`
    : undefined;
  return (
    <StepShell
      step={step}
      currentStepIdx={currentStepIdx}
      t={t}
      icon={<Mic className="h-7 w-7" />}
      title={t("onboarding.smoke_test.title")}
      description={t("onboarding.smoke_test.body", { key: hotkeyKey })}
      error={inlineError}
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="ghost" size="lg" onClick={onSkip}>
            {t("onboarding.skip")}
          </Button>
          <Button
            variant="primary"
            size="lg"
            disabled={!text}
            onClick={onContinue}
          >
            {t("onboarding.smoke_test.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
      <div className="flex min-h-28 w-full max-w-sm items-center justify-center rounded-xl border border-vx-border bg-vx-bg-secondary p-5 text-sm shadow-vx-sm">
        {text ? (
          <span className="text-left text-vx-text-primary">{text}</span>
        ) : (
          <span className="text-vx-text-dim">
            {t("onboarding.smoke_test.waiting")}
          </span>
        )}
      </div>
      {timedOut && !text && !error && (
        <p className="max-w-sm text-center text-xs text-vx-text-dim">
          {t("onboarding.smoke_test.timeout")}
        </p>
      )}
      {text && (
        <p className="flex items-center gap-1 text-xs text-vx-success">
          <Check className="h-3.5 w-3.5" />
          {t("onboarding.smoke_test.success")}
        </p>
      )}
    </StepShell>
  );
}
