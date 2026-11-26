import { useEffect, useState } from "react";
import { ArrowLeft, Check, ChevronRight } from "lucide-react";

import { onEvent } from "../../../lib/tauri";
import { useAppStore } from "../../../stores/appStore";
import type {
  TranscriptionCompleteEvent,
  TranscriptionErrorEvent,
} from "../../../types/events";
import { Waveform } from "../../floating-widget/Waveform";
import { Button } from "../../ui/Button";
import { StatusBadge } from "../../ui/StatusBadge";
import { StepShell } from "../shared/StepShell";
import type { Step, TFunc } from "../types";

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  busy: boolean;
  hotkeyDisplay: string;
  onBack: () => void;
  onContinue: () => void;
  onSkip: () => void;
}

export function SmokeTestStep({
  step,
  currentStepIdx,
  t,
  busy,
  hotkeyDisplay,
  onBack,
  onContinue,
  onSkip,
}: Props) {
  const [text, setText] = useState("");
  const [error, setError] = useState("");
  const [timedOut, setTimedOut] = useState(false);
  const appState = useAppStore((s) => s.state);

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
      // Unlisten independently: one rejected registration must not stop the
      // siblings that did register from being torn down (leak on unmount).
      listeners.forEach((p) => p.then((fn) => fn()).catch(() => {}));
    };
  }, []);

  const inlineError = error
    ? `${t("onboarding.smoke_test.error", { message: error })} ${t("onboarding.smoke_test.troubleshoot")}`
    : undefined;
  const wordCount = text.trim() ? text.trim().split(/\s+/).length : 0;

  return (
    <StepShell
      step={step}
      currentStepIdx={currentStepIdx}
      t={t}
      title={t("onboarding.smoke_test.title")}
      description={t("onboarding.smoke_test.body", { key: hotkeyDisplay })}
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
            disabled={!text || busy}
            onClick={onContinue}
          >
            {t("onboarding.smoke_test.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
      <div className="flex w-full max-w-md flex-col gap-3.5 rounded-[12px] border border-vx-border-subtle bg-vx-bg-surface p-5 text-left">
        <div className="flex items-center justify-center gap-3">
          <StatusBadge tone={appState === "recording" ? "ember" : "neutral"}>
            {t("onboarding.smoke_test.listening", { key: hotkeyDisplay })}
          </StatusBadge>
          <div className="flex w-16 items-center">
            <Waveform
              active={appState === "recording"}
              barClassName="bg-vx-accent-ember"
            />
          </div>
        </div>

        <div className="flex min-h-24 items-center justify-center rounded-[8px] border border-vx-border-subtle bg-vx-bg-subtle p-4 text-sm">
          {text ? (
            <p className="vx-panel-reveal w-full text-left leading-relaxed text-vx-text-primary">
              {text}
            </p>
          ) : (
            <span className="text-vx-text-muted">
              {t("onboarding.smoke_test.waiting")}
            </span>
          )}
        </div>

        {timedOut && !text && !error && (
          <p className="text-center text-xs text-vx-text-muted">
            {t("onboarding.smoke_test.timeout")}
          </p>
        )}

        {text && (
          <p className="flex items-center justify-center gap-1.5 text-xs text-vx-semantic-success">
            <Check className="h-3.5 w-3.5" />
            <span>
              {t("onboarding.smoke_test.success")} ·{" "}
              {t("onboarding.smoke_test.words", { count: wordCount })}
            </span>
          </p>
        )}
      </div>
    </StepShell>
  );
}
