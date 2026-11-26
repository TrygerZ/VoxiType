import { ArrowLeft, Check } from "lucide-react";

import { Button } from "../../ui/Button";
import { StepShell } from "../shared/StepShell";
import type { Step, SttEngine, TFunc } from "../types";

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  error: string;
  busy: boolean;
  hotkeyDisplay: string;
  hotkeyMode: string;
  micDevice: string;
  sttEngine: SttEngine;
  sttLanguage: string;
  onBack: () => void;
  onFinish: () => void;
}

export function CompleteStep({
  step,
  currentStepIdx,
  t,
  error,
  busy,
  hotkeyDisplay,
  hotkeyMode,
  micDevice,
  sttEngine,
  sttLanguage,
  onBack,
  onFinish,
}: Props) {
  const sttLabel =
    sttEngine === "groq"
      ? t("onboarding.stt.groq.choice_title")
      : t("onboarding.stt.offline.choice_title");
  const langLabel =
    sttLanguage === "auto" ? t("onboarding.stt.language_auto") : sttLanguage;
  const modeLabel =
    hotkeyMode === "toggle"
      ? t("onboarding.step4.mode_toggle")
      : t("onboarding.step4.mode_ptt");

  return (
    <StepShell
      step={step}
      currentStepIdx={currentStepIdx}
      t={t}
      title={t("onboarding.complete.title")}
      description={t("onboarding.complete.body", { key: hotkeyDisplay })}
      error={error}
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="primary" size="lg" disabled={busy} onClick={onFinish}>
            {t("onboarding.complete.start")}
          </Button>
        </>
      }
    >
      <div className="flex w-full max-w-md flex-col gap-4 rounded-[12px] border border-vx-border-subtle bg-vx-bg-surface p-5 text-left">
        <div>
          <p className="text-[11px] font-medium uppercase tracking-wider text-vx-text-muted">
            {t("onboarding.complete.summary_title")}
          </p>
          <dl className="mt-3 flex flex-col gap-2 text-xs">
            <div className="flex items-center justify-between gap-4">
              <dt className="text-vx-text-secondary">
                {t("onboarding.complete.summary_mic")}
              </dt>
              <dd className="truncate font-medium text-vx-text-primary">
                {micDevice || "Default"}
              </dd>
            </div>
            <div className="flex items-center justify-between gap-4">
              <dt className="text-vx-text-secondary">
                {t("onboarding.complete.summary_stt")}
              </dt>
              <dd className="truncate font-medium text-vx-text-primary">
                {sttLabel} ({langLabel})
              </dd>
            </div>
            <div className="flex items-center justify-between gap-4">
              <dt className="text-vx-text-secondary">
                {t("onboarding.complete.summary_hotkey")}
              </dt>
              <dd className="truncate font-medium text-vx-text-primary">
                {hotkeyDisplay} ({modeLabel})
              </dd>
            </div>
          </dl>
        </div>

        <div className="border-t border-vx-border-subtle pt-4">
          <p className="text-xs font-semibold text-vx-text-secondary">
            {t("onboarding.complete.next_title")}
          </p>
          <ol className="mt-2.5 flex flex-col gap-2">
            <li className="flex gap-2.5 text-xs leading-relaxed text-vx-text-muted">
              <Check className="mt-0.5 h-3.5 w-3.5 shrink-0 text-vx-semantic-success" />
              <span>
                {t("onboarding.complete.next.tip1", { key: hotkeyDisplay })}
              </span>
            </li>
            <li className="flex gap-2.5 text-xs leading-relaxed text-vx-text-muted">
              <Check className="mt-0.5 h-3.5 w-3.5 shrink-0 text-vx-semantic-success" />
              <span>{t("onboarding.complete.next.tip2")}</span>
            </li>
          </ol>
        </div>
      </div>
    </StepShell>
  );
}
