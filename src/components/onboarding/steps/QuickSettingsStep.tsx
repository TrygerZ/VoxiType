import { ArrowLeft, ChevronRight } from "lucide-react";

import type { ThemePreference } from "../../../types/app";
import { Button } from "../../ui/Button";
import { Select } from "../../ui/Select";
import { Switch } from "../../ui/Switch";
import { StepShell } from "../shared/StepShell";
import type { Step, TFunc } from "../types";

const THEME_OPTIONS: Array<{ value: ThemePreference; labelKey: string }> = [
  { value: "dark", labelKey: "settings.general.theme_dark" },
  { value: "light", labelKey: "settings.general.theme_light" },
  { value: "system", labelKey: "settings.general.theme_system" },
];

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  error: string;
  busy: boolean;
  lang: string;
  theme: ThemePreference;
  soundCues: boolean;
  onLanguageChange: (value: string) => void;
  onThemeChange: (value: ThemePreference) => void;
  onSoundCuesChange: (value: boolean) => void;
  onBack: () => void;
  onContinue: () => void;
}

export function QuickSettingsStep({
  step,
  currentStepIdx,
  t,
  error,
  busy,
  lang,
  theme,
  soundCues,
  onLanguageChange,
  onThemeChange,
  onSoundCuesChange,
  onBack,
  onContinue,
}: Props) {
  return (
    <StepShell
      step={step}
      currentStepIdx={currentStepIdx}
      t={t}
      title={t("onboarding.step2.title")}
      description={t("onboarding.step2.body")}
      error={error}
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="primary" size="lg" disabled={busy} onClick={onContinue}>
            {t("onboarding.step2.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
      <div className="flex w-full max-w-md flex-col gap-4 rounded-[12px] border border-vx-border-subtle bg-vx-bg-surface p-5 text-left">
        <Select
          autoFocus
          label={t("onboarding.ui_language")}
          options={[
            { value: "id", label: "Bahasa Indonesia" },
            { value: "en", label: "English" },
          ]}
          value={lang}
          onChange={(event) => onLanguageChange(event.target.value)}
        />

        <div className="flex flex-col gap-1.5">
          <span className="text-xs font-medium text-vx-text-secondary">
            {t("onboarding.step2.theme")}
          </span>
          <div
            role="group"
            aria-label={t("onboarding.step2.theme")}
            className="grid grid-cols-3 rounded-[8px] border border-vx-border-subtle bg-vx-bg-subtle p-1"
          >
            {THEME_OPTIONS.map((opt) => {
              const active = theme === opt.value;
              return (
                <button
                  key={opt.value}
                  type="button"
                  aria-pressed={active}
                  onClick={() => onThemeChange(opt.value)}
                  className={`rounded-[6px] px-3 py-1.5 text-xs font-medium transition-colors duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember ${
                    active
                      ? "bg-vx-bg-elevated text-vx-text-primary"
                      : "text-vx-text-muted hover:text-vx-text-primary"
                  }`}
                >
                  {t(opt.labelKey)}
                </button>
              );
            })}
          </div>
        </div>

        <div className="flex items-center justify-between gap-4 border-t border-vx-border-subtle pt-4">
          <div>
            <p className="text-sm font-medium text-vx-text-primary">
              {t("onboarding.sound_cues")}
            </p>
            <p className="mt-0.5 text-xs text-vx-text-muted">
              {soundCues
                ? t("onboarding.sound_cues_on")
                : t("onboarding.sound_cues_off")}
            </p>
          </div>
          <Switch
            checked={soundCues}
            onChange={onSoundCuesChange}
            ariaLabel={t("onboarding.sound_cues")}
          />
        </div>
      </div>
    </StepShell>
  );
}
