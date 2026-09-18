import { useEffect, useRef, useState } from "react";
import { ArrowLeft, Check, ChevronRight, Mic } from "lucide-react";

import { formatTauriError, getMicrophones, onEvent } from "../../../lib/tauri";
import { Button } from "../../ui/Button";
import { Select } from "../../ui/Select";
import { StepShell } from "../shared/StepShell";
import type { DeviceInfo } from "../../../types/app";
import type { Step, TFunc } from "../types";

interface MicrophoneStepProps {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  selectedDevice: string;
  onDeviceChange: (deviceId: string) => void;
  onBack: () => void;
  onContinue: () => void;
  onSkip: () => void;
}

interface AudioLevelEvent {
  level: number;
}

export function MicrophoneStep(props: MicrophoneStepProps) {
  const { t } = props;
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [level, setLevel] = useState(0);
  const [error, setError] = useState("");
  const selectedDeviceRef = useRef(props.selectedDevice);
  selectedDeviceRef.current = props.selectedDevice;

  useEffect(() => {
    getMicrophones()
      .then((found) => {
        setDevices(found);
        if (!selectedDeviceRef.current)
          props.onDeviceChange(
            found.find((device) => device.is_default)?.id ?? found[0]?.id ?? "",
          );
      })
      .catch((err: unknown) => setError(formatTauriError(err)));
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void onEvent<AudioLevelEvent>("audio_level", (event) =>
      setLevel(clampLevel(event.level)),
    )
      .then((cleanup) => {
        if (cancelled) cleanup();
        else unlisten = cleanup;
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const options = devices.map((device) => ({
    value: device.id,
    label: device.name,
  }));
  const continueStep = () => {
    props.onContinue();
  };

  return (
    <StepShell
      step={props.step}
      currentStepIdx={props.currentStepIdx}
      t={t}
      icon={<Mic className="h-7 w-7" />}
      title={t("onboarding.microphone.title")}
      description={t("onboarding.microphone.body")}
      error=""
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={props.onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="ghost" size="lg" onClick={props.onSkip}>
            {t("onboarding.skip")}
          </Button>
          <Button variant="primary" size="lg" onClick={continueStep}>
            <Check className="h-4 w-4" />
            {t("onboarding.microphone.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
      <div className="flex w-full max-w-sm flex-col gap-4 text-left">
        {devices.length > 0 ? (
          <Select
            autoFocus
            label={t("onboarding.microphone.select")}
            options={options}
            value={props.selectedDevice}
            onChange={(event) => props.onDeviceChange(event.target.value)}
            className="w-full"
          />
        ) : (
          <p
            role="alert"
            className="rounded-lg border border-vx-error/30 bg-vx-error/10 p-3 text-xs text-vx-error"
          >
            {error || t("onboarding.microphone.none")}
          </p>
        )}
        <div>
          <div className="mb-1.5 flex justify-between text-xs text-vx-text-secondary">
            <span>{t("onboarding.microphone.level")}</span>
            <span>{Math.round(level * 100)}%</span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-vx-border">
            <div
              className="h-full rounded-full bg-vx-accent transition-[width] duration-100"
              style={{ width: `${level * 100}%` }}
            />
          </div>
        </div>
      </div>
    </StepShell>
  );
}

function clampLevel(level: number) {
  return Number.isFinite(level) ? Math.min(1, Math.max(0, level)) : 0;
}
