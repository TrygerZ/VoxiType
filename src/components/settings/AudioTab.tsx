import { useCallback, useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";

import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import { getBooleanSetting, getStringSetting } from "../../lib/settingsGuards";
import { getMicrophones, onEvent, setMicMeter } from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";
import type { DeviceInfo } from "../../types/app";
import { Button } from "../ui/Button";
import { Select } from "../ui/Select";
import { StatusBadge } from "../ui/StatusBadge";
import { Switch } from "../ui/Switch";
import { SettingsGroup, SettingsHeader, SettingsRow } from "./SettingsLayout";

interface AudioLevelEvent {
  level: number;
}

function clampLevel(raw: number): number {
  if (!Number.isFinite(raw)) return 0;
  return Math.max(0, Math.min(1, raw));
}

function levelBadgeState(
  level: number,
  t: (key: string) => string,
): { tone: "neutral" | "success" | "warning"; label: string } {
  if (level >= 0.85) {
    return { tone: "warning", label: t("settings.audio.level_hot") };
  }
  if (level >= 0.08) {
    return { tone: "success", label: t("settings.audio.level_healthy") };
  }
  return { tone: "neutral", label: t("settings.audio.level_quiet") };
}

export function AudioTab() {
  const t = useT();
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const [level, setLevel] = useState(0);
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);

  const selectedDevice = getStringSetting(settings.mic_device, "");
  const meterDevice =
    selectedDevice && selectedDevice !== "default" ? selectedDevice : null;

  const loadDevices = useCallback(async () => {
    setRefreshing(true);
    try {
      await invokeAction(async () => {
        const mics = await getMicrophones();
        setDevices(mics);
      });
    } finally {
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void loadDevices();
  }, [loadDevices]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void onEvent<AudioLevelEvent>("audio_level", (event) => {
      if (!cancelled) {
        setLevel(clampLevel(event.level));
      }
    })
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

  useEffect(() => {
    setMicMeter(true, meterDevice).catch(() => {});
    return () => {
      setMicMeter(false, null).catch(() => {});
    };
  }, [meterDevice]);

  const badge = levelBadgeState(level, t);
  const defaultOptionId =
    devices.find((d) => d.is_default)?.id ?? devices[0]?.id ?? "default";

  return (
    <div>
      <SettingsHeader
        title={t("settings.audio.title")}
        description={t("settings.audio.desc")}
      />

      <SettingsGroup title={t("settings.audio.device_group")}>
        <SettingsRow label={t("settings.audio.mic")}>
          <div className="flex items-center gap-2">
            <Select
              aria-label={t("settings.audio.mic")}
              options={devices.map((d) => ({ value: d.id, label: d.name }))}
              value={selectedDevice || defaultOptionId}
              onChange={(e) =>
                void invokeAction(() => update("mic_device", e.target.value))
              }
              className="w-56"
            />
            <Button
              variant="secondary"
              size="sm"
              disabled={refreshing}
              onClick={() => void loadDevices()}
            >
              <RefreshCw
                className={`h-3.5 w-3.5 ${refreshing ? "animate-spin" : ""}`}
              />
              {t("settings.audio.refresh")}
            </Button>
          </div>
        </SettingsRow>
        <div className="flex flex-col gap-2.5 px-4 py-3.5">
          <div className="flex items-center justify-between gap-2">
            <span className="text-sm font-medium text-vx-text-primary">
              {t("settings.audio.live_level")}
            </span>
            <StatusBadge tone={badge.tone}>{badge.label}</StatusBadge>
          </div>
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-vx-bg-subtle">
            <div
              className="h-full rounded-full bg-vx-accent-ember transition-[width] duration-75"
              style={{ width: `${Math.round(level * 100)}%` }}
            />
          </div>
        </div>
      </SettingsGroup>

      <SettingsGroup title={t("settings.audio.feedback_group")}>
        <SettingsRow
          label={t("settings.general.sound")}
          description={t("settings.general.sound.desc")}
        >
          <Switch
            checked={getBooleanSetting(settings.sound_cues, false)}
            onChange={(v) => void invokeAction(() => update("sound_cues", v))}
            ariaLabel={t("settings.general.sound")}
          />
        </SettingsRow>
      </SettingsGroup>
    </div>
  );
}
