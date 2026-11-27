import { useCallback, useEffect, useRef, useState } from "react";
import { ArrowLeft, Check, ChevronRight, RefreshCw } from "lucide-react";

import {
  formatTauriError,
  getMicrophones,
  onEvent,
  setMicMeter,
} from "../../../lib/tauri";
import type { DeviceInfo } from "../../../types/app";
import { Button } from "../../ui/Button";
import { Select } from "../../ui/Select";
import { StatusBadge } from "../../ui/StatusBadge";
import { StepShell } from "../shared/StepShell";
import type { Step, TFunc } from "../types";

interface MicrophoneStepProps {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  error?: string;
  busy: boolean;
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
  const [localError, setLocalError] = useState("");
  const [refreshing, setRefreshing] = useState(false);
  const selectedDeviceRef = useRef(props.selectedDevice);
  selectedDeviceRef.current = props.selectedDevice;
  const onDeviceChangeRef = useRef(props.onDeviceChange);
  onDeviceChangeRef.current = props.onDeviceChange;

  const loadDevices = useCallback(() => {
    setLocalError("");
    setRefreshing(true);
    getMicrophones()
      .then((found) => {
        setDevices(found);
        if (!selectedDeviceRef.current) {
          onDeviceChangeRef.current(
            found.find((device) => device.is_default)?.id ?? found[0]?.id ?? "",
          );
        }
      })
      .catch((err: unknown) => setLocalError(formatTauriError(err)))
      .finally(() => setRefreshing(false));
  }, []);

  useEffect(() => {
    loadDevices();
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
  }, [loadDevices]);

  // The backend only emits `audio_level` during Recording, so ask it to stream
  // the selected device's level for as long as this step shows a meter.
  // Failures are swallowed: enumeration already surfaces device errors, and a
  // dead meter must not block onboarding.
  useEffect(() => {
    if (!props.selectedDevice) return;
    setMicMeter(true, props.selectedDevice).catch(() => {});
    return () => {
      setMicMeter(false, null).catch(() => {});
    };
  }, [props.selectedDevice]);

  const options = devices.map((device) => ({
    value: device.id,
    label: device.name,
  }));
  const displayError = props.error || localError;
  const badge = levelBadgeState(level, t);
  const percent = Math.round(level * 100);

  return (
    <StepShell
      step={props.step}
      currentStepIdx={props.currentStepIdx}
      t={t}
      title={t("onboarding.microphone.title")}
      description={t("onboarding.microphone.body")}
      error={displayError}
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={props.onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="ghost" size="lg" onClick={props.onSkip}>
            {t("onboarding.skip")}
          </Button>
          <Button
            variant="primary"
            size="lg"
            disabled={props.busy}
            onClick={props.onContinue}
          >
            <Check className="h-4 w-4" />
            {t("onboarding.microphone.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
      <div className="flex w-full max-w-md flex-col gap-4 rounded-[12px] border border-vx-border-subtle bg-vx-bg-surface p-5 text-left">
        <div className="flex flex-wrap sm:flex-nowrap items-end gap-2">
          {devices.length > 0 ? (
            <div className="min-w-0 flex-1">
              <Select
                autoFocus
                aria-label={t("onboarding.microphone.select")}
                options={options}
                value={props.selectedDevice}
                onChange={(event) => props.onDeviceChange(event.target.value)}
                className="w-full"
              />
            </div>
          ) : (
            <p className="min-w-0 flex-1 rounded-[8px] border border-vx-border-subtle bg-vx-bg-subtle px-3 py-2 text-xs text-vx-text-muted">
              {t("onboarding.microphone.none")}
            </p>
          )}
          <Button
            type="button"
            variant="secondary"
            size="md"
            disabled={refreshing}
            onClick={loadDevices}
          >
            <RefreshCw
              className={`h-3.5 w-3.5 ${refreshing ? "animate-spin" : ""}`}
            />
            {t("onboarding.microphone.refresh")}
          </Button>
        </div>

        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between gap-2 text-xs">
            <span className="text-vx-text-secondary">
              {t("onboarding.microphone.live_level")}
            </span>
            <span className="flex items-center gap-1.5 text-vx-text-muted">
              <span className="font-mono">{percent}%</span>
              <StatusBadge tone={badge.tone}>{badge.label}</StatusBadge>
            </span>
          </div>
          <div className="h-1.5 overflow-hidden rounded-full bg-vx-bg-subtle">
            <div
              className="h-full rounded-full bg-vx-accent-ember transition-[width] duration-100"
              style={{ width: `${percent}%` }}
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

function levelBadgeState(
  level: number,
  t: TFunc,
): { tone: "neutral" | "success" | "warning"; label: string } {
  if (level >= 0.85) {
    return { tone: "warning", label: t("settings.audio.level_hot") };
  }
  if (level >= 0.08) {
    return { tone: "success", label: t("settings.audio.level_healthy") };
  }
  return { tone: "neutral", label: t("settings.audio.level_quiet") };
}
