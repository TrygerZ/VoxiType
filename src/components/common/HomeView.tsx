import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowRight,
  Check,
  Copy,
  Languages,
  Loader2,
  Mic,
  Pin,
  Sparkles,
  Volume2,
} from "lucide-react";

import { invokeAction } from "../../lib/invokeAction";
import { modeLabel, type Translate, useT } from "../../lib/i18n";
import { navigate, openOnboarding } from "../../lib/navigate";
import {
  getBooleanSetting,
  getStringSetting,
  type HotkeyConfig,
  isHotkeyConfig,
} from "../../lib/settingsGuards";
import { reInject, startRecording, stopRecording } from "../../lib/tauri";
import { usePlatform } from "../../lib/usePlatform";
import { type InjectionOutcome, useAppStore } from "../../stores/appStore";
import { useHistoryStore } from "../../stores/historyStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useStatsStore } from "../../stores/statsStore";
import type { AppStateEnum, TranscriptionEntry, UsageStats } from "../../types/app";
import { Waveform } from "../floating-widget/Waveform";
import { formatAccelerator } from "../settings/HotkeyRecorder";
import { Button } from "../ui/Button";
import { IconButton } from "../ui/IconButton";
import { Keycap } from "../ui/Keycap";
import { StatusBadge } from "../ui/StatusBadge";
import { Switch } from "../ui/Switch";
import { toast } from "../ui/Toast";

const MAX_PLAUSIBLE_WPM = 400;
const BUILT_IN_CONSOLE_MODES = new Set(["dictation", "message", "email", "file"]);

function formatDuration(sec: number): string {
  const m = Math.floor(sec / 60).toString().padStart(2, "0");
  const s = (sec % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

function splitDuration(ms: number, t: Translate): { value: string; unit: string } {
  const totalSec = Math.floor(ms / 1000);
  if (totalSec < 60) return { value: String(totalSec), unit: t("home.unit_sec") };
  const totalMin = Math.floor(totalSec / 60);
  if (totalMin < 60) {
    return {
      value: String(totalMin),
      unit: totalMin === 1 ? t("home.unit_min") : t("home.unit_mins"),
    };
  }
  const hrs = Math.floor(totalMin / 60);
  const mins = totalMin % 60;
  return {
    value: mins > 0 ? `${hrs}.${Math.round((mins / 60) * 10)}` : String(hrs),
    unit: hrs === 1 && mins === 0 ? t("home.unit_hr") : t("home.unit_hrs"),
  };
}

function formatCompact(n: number): string {
  if (n < 1000) return n.toLocaleString();
  if (n < 9_950) return `${(n / 1000).toFixed(1)}k`;
  if (n < 999_500) return `${Math.round(n / 1000)}k`;
  return `${(n / 1_000_000).toFixed(1)}M`;
}

function formatItemTime(createdAt: string): string {
  if (!createdAt) return "";
  const d = new Date(createdAt);
  if (isNaN(d.getTime())) return "";
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function buildItemMeta(item: TranscriptionEntry, t: Translate): string {
  const parts = [
    modeLabel(t, item.mode),
    item.app_context ?? "",
    item.word_count > 0 ? `${item.word_count} ${t("home.words")}` : "",
    formatItemTime(item.created_at),
  ].filter(Boolean);
  return parts.join(" \u00b7 ");
}

function computeAvgWpm(historyItems: TranscriptionEntry[]): number {
  const rates: number[] = [];
  for (const item of historyItems) {
    if (item.mode === "file") continue;
    const dur = item.duration_ms ?? 0;
    if (!item.word_count || item.word_count <= 0 || dur <= 0) continue;
    const wpm = item.word_count / (dur / 60000);
    if (!isFinite(wpm) || wpm > MAX_PLAUSIBLE_WPM) continue;
    rates.push(wpm);
    if (rates.length >= 20) break;
  }
  if (rates.length === 0) return 0;
  return Math.round(rates.reduce((sum, w) => sum + w, 0) / rates.length);
}

function HomeTopBar({
  t,
  isRecording,
  isProcessing,
  durationSec,
  hotkey,
}: {
  t: Translate;
  isRecording: boolean;
  isProcessing: boolean;
  durationSec: number;
  hotkey?: HotkeyConfig;
}) {
  const modeLabelText =
    hotkey?.mode === "toggle" ? t("home.mode_toggle") : t("home.mode_ptt");

  return (
    <header className="flex items-center justify-between">
      <div className="flex items-center gap-2.5">
        <div className="bg-vx-bg-subtle border border-vx-border-subtle rounded-[8px] size-8 p-[3px] flex items-center justify-center">
          <img
            src="/logo.png"
            alt="VoxiType logo"
            className="h-full w-full object-contain"
          />
        </div>
        <span className="text-[15px] font-semibold leading-5 tracking-[-0.01em] text-vx-text-primary">
          {t("app.name")}
        </span>
      </div>

      {isRecording ? (
        <StatusBadge
          tone="ember"
          label={t("home.status_recording", {
            duration: formatDuration(durationSec),
          })}
        />
      ) : isProcessing ? (
        <StatusBadge tone="neutral" label={t("processing")} />
      ) : (
        <StatusBadge
          tone="success"
          label={t("home.status_ready", { mode: modeLabelText })}
        />
      )}
    </header>
  );
}

function SetupMissingBanner({ t }: { t: Translate }) {
  return (
    <div
      role="status"
      className="flex flex-col gap-3 rounded-[12px] border border-vx-semantic-warning/35 bg-vx-semantic-warning-soft p-4 sm:flex-row sm:items-center sm:justify-between"
    >
      <div>
        <p className="text-sm font-semibold text-vx-text-primary">
          {t("home.setup_banner_title")}
        </p>
        <p className="text-xs text-vx-text-secondary">
          {t("home.setup_banner_body")}
        </p>
      </div>
      <Button size="sm" variant="primary" onClick={openOnboarding}>
        {t("home.setup_banner_action")}
      </Button>
    </div>
  );
}

function VoiceConsoleCard({
  t,
  state,
  durationSec,
  wordCount,
  outcome,
  hotkey,
  shortcutDisplay,
  micDevice,
  sttEngine,
  onMicClick,
}: {
  t: Translate;
  state: AppStateEnum;
  durationSec: number;
  wordCount: number | null;
  outcome: InjectionOutcome | null;
  hotkey?: HotkeyConfig;
  shortcutDisplay: string;
  micDevice: string;
  sttEngine: unknown;
  onMicClick: () => void;
}) {
  const isRecording = state === "recording";
  const isProcessing = state === "processing";
  const micTooltip = isRecording
    ? t("home.mic_recording_tooltip")
    : t("home.mic_tooltip");
  const subtitle = isRecording
    ? t("home.listening")
    : isProcessing
      ? t("processing")
      : hotkey?.mode === "toggle"
        ? t("home.toggle_shortcut")
        : t("home.hold_shortcut");
  const shortcutParts = shortcutDisplay
    .split("+")
    .map((p) => p.trim())
    .filter(Boolean);

  const outerRingClass = isRecording
    ? "bg-vx-accent-ember-soft border-[1.5px] border-vx-accent-ember vx-mic-pulse"
    : isProcessing
      ? "bg-vx-bg-subtle border border-vx-border-strong"
      : "bg-vx-bg-subtle border border-vx-border-default group-hover:border-vx-border-strong";

  const innerCoreClass = isRecording
    ? "bg-vx-accent-ember border-vx-accent-ember text-vx-text-inverse"
    : "bg-vx-bg-elevated border-vx-border-strong text-vx-text-primary";

  const micLabel =
    micDevice === "default"
      ? t("home.default_mic")
      : micDevice.split("(")[0].trim();
  const engineLabel =
    sttEngine === "whisper_cpp"
      ? t("home.local_whisper")
      : t("home.groq_cloud");

  return (
    <section className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] px-6 pt-7 pb-6 flex flex-col items-center justify-between gap-6 min-h-[460px]">
      <div className="flex flex-col items-center gap-1 text-center">
        <span className="text-[11px] font-medium uppercase tracking-[0.08em] text-vx-text-muted">
          {t("home.voice_console")}
        </span>
        <p className="text-xs text-vx-text-secondary">{subtitle}</p>
      </div>

      <button
        type="button"
        onClick={onMicClick}
        disabled={isProcessing}
        title={micTooltip}
        aria-label={micTooltip}
        className="group relative flex items-center justify-center rounded-full focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember focus-visible:ring-offset-2 focus-visible:ring-offset-vx-bg-canvas disabled:cursor-not-allowed"
      >
        <span
          className={`size-[136px] rounded-full flex items-center justify-center transition-colors duration-[180ms] ${outerRingClass}`}
        >
          <span
            className={`size-[84px] rounded-full flex items-center justify-center border transition-colors duration-[180ms] ${innerCoreClass}`}
          >
            {isProcessing ? (
              <Loader2 className="h-8 w-8 animate-spin" />
            ) : (
              <Mic className="h-8 w-8" />
            )}
          </span>
        </span>
      </button>

      <div className="flex min-h-[72px] w-full flex-col items-center justify-center gap-2 text-center">
        {isRecording ? (
          <>
            <span className="font-mono text-[24px] font-bold leading-7 tracking-[-0.02em] text-vx-text-primary tabular-nums">
              {formatDuration(durationSec)}
            </span>
            <div className="flex h-6 w-16 items-center justify-center">
              <Waveform active barClassName="bg-vx-accent-ember" />
            </div>
          </>
        ) : isProcessing ? (
          <span className="text-xs font-medium text-vx-text-secondary">
            {t("processing")}
          </span>
        ) : (
          <>
            <div className="flex flex-wrap items-center justify-center gap-1.5">
              {shortcutParts.map((part, idx) => (
                <Fragment key={`${part}-${idx}`}>
                  {idx > 0 && (
                    <span className="text-xs text-vx-text-muted">+</span>
                  )}
                  <Keycap>{part}</Keycap>
                </Fragment>
              ))}
            </div>
            {state === "idle" && wordCount !== null && (
              <div className="flex flex-col items-center gap-1">
                <span
                  role="status"
                  className={
                    outcome?.injected === false
                      ? "text-xs text-vx-semantic-error"
                      : "text-xs text-vx-semantic-success"
                  }
                >
                  {outcome?.injected === false
                    ? t("notice.saved_to_history")
                    : outcome?.appContext
                      ? t("home.inserted_into", { app: outcome.appContext })
                      : t("home.inserted")}
                </span>
                {outcome?.injected === false && (
                  <button
                    type="button"
                    onClick={() => navigate("history")}
                    className="text-xs font-medium text-vx-accent-ember hover:underline"
                  >
                    {t("home.open_history")}
                  </button>
                )}
              </div>
            )}
          </>
        )}
      </div>

      <div className="flex flex-wrap items-center justify-center gap-2">
        <button
          type="button"
          onClick={() => navigate("settings", "audio")}
          className="bg-vx-bg-subtle border border-vx-border-subtle hover:border-vx-border-default rounded-[8px] px-2.5 py-1.5 inline-flex items-center gap-1.5 text-xs text-vx-text-secondary hover:text-vx-text-primary transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember"
        >
          <Volume2 className="h-[13px] w-[13px] shrink-0" />
          <span>{micLabel} &#x2197;</span>
        </button>
        <button
          type="button"
          onClick={() => navigate("settings", "stt")}
          className="bg-vx-bg-subtle border border-vx-border-subtle hover:border-vx-border-default rounded-[8px] px-2.5 py-1.5 inline-flex items-center gap-1.5 text-xs text-vx-text-secondary hover:text-vx-text-primary transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember"
        >
          <Sparkles className="h-[13px] w-[13px] shrink-0" />
          <span>{engineLabel} &#x2197;</span>
        </button>
      </div>
    </section>
  );
}

function LifetimeTelemetryCard({
  t,
  totals,
  avgWpm,
  statsError,
  onRetry,
}: {
  t: Translate;
  totals: UsageStats;
  avgWpm: number;
  statsError: string | null;
  onRetry: () => void;
}) {
  const recordingTime = splitDuration(totals.total_duration_ms, t);
  const metrics = [
    {
      key: "wpm",
      value: `${avgWpm} WPM`,
      label: t("home.speaking_pace"),
      accent: true,
    },
    {
      key: "time",
      value: `${recordingTime.value} ${recordingTime.unit}`,
      label: t("home.total_dictated"),
      accent: false,
    },
    {
      key: "words",
      value: formatCompact(totals.total_words),
      label: t("home.words_captured"),
      accent: false,
    },
    {
      key: "sessions",
      value: formatCompact(totals.total_sessions),
      label: t("home.sessions"),
      accent: false,
    },
  ];

  return (
    <section className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] px-5 py-4">
      <div className="mb-3 flex items-center justify-between gap-2">
        <h2 className="text-[11px] font-medium uppercase tracking-[0.08em] text-vx-text-muted">
          {t("home.lifetime_telemetry")}
        </h2>
        <span className="text-xs text-vx-text-muted">
          {t("home.last_20_avg")}
        </span>
      </div>

      {statsError ? (
        <div className="flex flex-col items-center justify-center gap-3 py-4 text-center">
          <div className="space-y-1">
            <p className="text-sm font-semibold text-vx-text-primary">
              {t("error.stats_load_failed")}
            </p>
            <p className="font-mono text-xs break-all text-vx-text-muted">
              {statsError}
            </p>
          </div>
          <Button size="sm" onClick={onRetry}>
            {t("error.retry")}
          </Button>
        </div>
      ) : (
        <>
          <div className="flex flex-wrap items-center justify-between gap-3">
            {metrics.map((metric, idx) => (
              <Fragment key={metric.key}>
                {idx > 0 && (
                  <div
                    aria-hidden="true"
                    className="w-px h-[34px] bg-vx-border-subtle shrink-0"
                  />
                )}
                <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span
                    className={`font-mono text-[15px] font-semibold tabular-nums ${
                      metric.accent
                        ? "text-vx-accent-ember"
                        : "text-vx-text-primary"
                    }`}
                  >
                    {metric.value}
                  </span>
                  <span className="text-xs text-vx-text-muted">
                    {metric.label}
                  </span>
                </div>
              </Fragment>
            ))}
          </div>

          {totals.total_sessions === 0 && (
            <p className="mt-3 border-t border-vx-border-subtle pt-2.5 text-xs text-vx-text-muted">
              {t("home.stats_empty")}
            </p>
          )}
        </>
      )}
    </section>
  );
}

function ModeTranslateCard({
  t,
  activeMode,
  translationEnabled,
  translationTarget,
  onUpdateSetting,
}: {
  t: Translate;
  activeMode: string;
  translationEnabled: boolean;
  translationTarget: string;
  onUpdateSetting: (key: string, value: unknown) => Promise<void>;
}) {
  const baseModes = [
    {
      id: "dictation",
      label: t("settings.modes.dictation"),
      desc: t("home.mode.dictation.desc"),
    },
    {
      id: "message",
      label: t("settings.modes.message"),
      desc: t("home.mode.message.desc"),
    },
    {
      id: "email",
      label: t("settings.modes.email"),
      desc: t("home.mode.email.desc"),
    },
  ];
  const modes = BUILT_IN_CONSOLE_MODES.has(activeMode)
    ? baseModes
    : [...baseModes, { id: activeMode, label: modeLabel(t, activeMode), desc: "" }];

  const targetName = translationTarget === "en" ? "English" : "Bahasa Indonesia";

  return (
    <section className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] px-5 py-4 flex flex-col gap-3">
      <div
        className={`grid gap-2 ${
          modes.length === 4 ? "grid-cols-2 sm:grid-cols-4" : "grid-cols-3"
        }`}
      >
        {modes.map((m) => {
          const isSelected = activeMode === m.id;
          return (
            <button
              key={m.id}
              type="button"
              aria-pressed={isSelected}
              onClick={() =>
                void invokeAction(() => onUpdateSetting("active_mode", m.id))
              }
              className={`flex flex-col items-center justify-center rounded-[8px] border px-3 py-2.5 text-center transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember ${
                isSelected
                  ? "bg-vx-bg-elevated border-vx-border-strong text-vx-text-primary"
                  : "bg-vx-bg-subtle border-vx-border-subtle text-vx-text-secondary hover:border-vx-border-default hover:text-vx-text-primary"
              }`}
            >
              <span className="text-[13px] font-medium">{m.label}</span>
              {m.desc && (
                <span className="mt-0.5 text-[11px] text-vx-text-muted">
                  {m.desc}
                </span>
              )}
            </button>
          );
        })}
      </div>

      <div className="bg-vx-bg-subtle border border-vx-border-subtle rounded-[8px] px-3 py-2.5 flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2.5">
          <Languages className="h-4 w-4 text-vx-text-secondary shrink-0" />
          <span className="truncate text-[13px] text-vx-text-primary">
            {t("home.translate_after_arrow", { target: targetName })}
          </span>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          {translationEnabled && (
            <select
              value={translationTarget}
              onChange={(e) =>
                void invokeAction(() =>
                  onUpdateSetting("translation_target", e.target.value),
                )
              }
              aria-label={t("settings.general.lang")}
              className="rounded-[6px] border border-vx-border-default bg-vx-bg-surface px-2 py-1 text-xs text-vx-text-primary focus:border-vx-accent-ember focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember"
            >
              <option
                value="en"
                className="bg-vx-bg-surface text-vx-text-primary"
              >
                English
              </option>
              <option
                value="id"
                className="bg-vx-bg-surface text-vx-text-primary"
              >
                Bahasa Indonesia
              </option>
            </select>
          )}
          <Switch
            checked={translationEnabled}
            onChange={(next) =>
              void invokeAction(() =>
                onUpdateSetting("translation_enabled", next),
              )
            }
            ariaLabel={t("home.translate_desc")}
          />
        </div>
      </div>
    </section>
  );
}

function RecentTranscriptionsCard({
  t,
  recentItems,
  historyError,
  copiedId,
  injectedId,
  onRetry,
  onCopy,
  onReInject,
  onTogglePin,
}: {
  t: Translate;
  recentItems: TranscriptionEntry[];
  historyError: string | null;
  copiedId: string | null;
  injectedId: string | null;
  onRetry: () => void;
  onCopy: (id: string, text: string) => void;
  onReInject: (id: string) => void;
  onTogglePin: (id: string, pinned: boolean) => void;
}) {
  return (
    <section className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] px-5 py-4 flex-1 flex flex-col gap-2.5">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-[11px] font-medium uppercase tracking-[0.08em] text-vx-text-muted">
          {t("home.recent_transcriptions")}
        </h2>
        <button
          type="button"
          onClick={() => navigate("history")}
          className="text-xs font-normal text-vx-accent-ember hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember rounded-[4px]"
        >
          {t("home.view_all_history")}
        </button>
      </div>

      {historyError ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 py-6 text-center">
          <div className="space-y-1">
            <p className="text-sm font-semibold text-vx-text-primary">
              {t("error.history_load_failed")}
            </p>
            <p className="font-mono text-xs break-all text-vx-text-muted">
              {historyError}
            </p>
          </div>
          <Button size="sm" onClick={onRetry}>
            {t("error.retry")}
          </Button>
        </div>
      ) : recentItems.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center py-6 text-center text-vx-text-muted">
          <Mic className="mb-2 h-6 w-6 opacity-40" />
          <p className="text-xs">{t("home.no_history")}</p>
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          {recentItems.map((item) => {
            const content = item.text_formatted || item.text_raw;
            return (
              <div
                key={item.id}
                className="bg-vx-bg-subtle border border-vx-border-subtle rounded-[8px] px-3 py-2.5 flex flex-col gap-1.5"
              >
                <div className="flex items-start justify-between gap-3">
                  <p className="line-clamp-2 text-[13px] leading-[18px] text-vx-text-primary">
                    {content}
                  </p>
                  <div className="flex shrink-0 items-center gap-1">
                    <IconButton
                      size="xs"
                      tone="ghost"
                      label={t("history.copy_tooltip")}
                      icon={
                        copiedId === item.id ? (
                          <Check className="h-3.5 w-3.5 text-vx-semantic-success" />
                        ) : (
                          <Copy className="h-3.5 w-3.5" />
                        )
                      }
                      onClick={() => onCopy(item.id, content)}
                    />
                    <IconButton
                      size="xs"
                      tone="ghost"
                      label={t("history.re_inject_tooltip")}
                      icon={
                        injectedId === item.id ? (
                          <Check className="h-3.5 w-3.5 text-vx-semantic-success" />
                        ) : (
                          <ArrowRight className="h-3.5 w-3.5" />
                        )
                      }
                      onClick={() => onReInject(item.id)}
                    />
                    <IconButton
                      size="xs"
                      tone="ghost"
                      label={
                        item.is_pinned
                          ? t("home.unpin_tooltip")
                          : t("home.pin_tooltip")
                      }
                      className={
                        item.is_pinned
                          ? "text-vx-accent-ember hover:text-vx-accent-ember"
                          : ""
                      }
                      icon={<Pin className="h-3.5 w-3.5" />}
                      onClick={() => onTogglePin(item.id, !item.is_pinned)}
                    />
                  </div>
                </div>
                <p className="text-xs leading-4 text-vx-text-muted">
                  {buildItemMeta(item, t)}
                </p>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

export function HomeView() {
  const t = useT();

  const state = useAppStore((s) => s.state);
  const durationSec = useAppStore((s) => s.durationSec);
  const wordCount = useAppStore((s) => s.wordCount);
  const outcome = useAppStore((s) => s.outcome);
  const isRecording = state === "recording";
  const isProcessing = state === "processing";

  const settings = useSettingsStore((s) => s.settings);
  const updateSetting = useSettingsStore((s) => s.update);
  const loadSettings = useSettingsStore((s) => s.load);

  const loadHistory = useHistoryStore((s) => s.load);
  const historyItems = useHistoryStore((s) => s.items);
  const historyError = useHistoryStore((s) => s.error);
  const togglePin = useHistoryStore((s) => s.togglePin);

  const totals = useStatsStore((s) => s.totals);
  const statsError = useStatsStore((s) => s.error);
  const loadStats = useStatsStore((s) => s.load);

  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [injectedId, setInjectedId] = useState<string | null>(null);
  const platform = usePlatform();

  const copyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const injectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const recordingPendingRef = useRef(false);

  useEffect(() => {
    void loadHistory();
    void loadSettings();
    void loadStats();
  }, [loadHistory, loadSettings, loadStats]);

  useEffect(() => {
    return () => {
      if (copyTimerRef.current) clearTimeout(copyTimerRef.current);
      if (injectTimerRef.current) clearTimeout(injectTimerRef.current);
    };
  }, []);

  const handleMicClick = useCallback(async () => {
    if (recordingPendingRef.current) return;
    recordingPendingRef.current = true;
    try {
      if (isRecording) {
        await invokeAction(stopRecording);
      } else if (state === "idle" || state === "error") {
        await invokeAction(startRecording);
      }
    } finally {
      recordingPendingRef.current = false;
    }
  }, [isRecording, state]);

  const handleCopy = useCallback(
    async (id: string, text: string) => {
      try {
        await navigator.clipboard.writeText(text);
      } catch {
        toast(t("file.copy_failed"), "error");
        return;
      }
      if (copyTimerRef.current) clearTimeout(copyTimerRef.current);
      setCopiedId(id);
      copyTimerRef.current = setTimeout(() => setCopiedId(null), 2000);
    },
    [t],
  );

  const handleReInject = useCallback(async (id: string) => {
    if (injectTimerRef.current) clearTimeout(injectTimerRef.current);
    const ok = await invokeAction(() => reInject(id));
    if (ok) {
      setInjectedId(id);
      injectTimerRef.current = setTimeout(() => setInjectedId(null), 2000);
    }
  }, []);

  const activeMode = getStringSetting(settings.active_mode, "dictation");
  const translationEnabled = getBooleanSetting(settings.translation_enabled, false);
  const translationTarget = getStringSetting(settings.translation_target, "en");
  const micDevice = getStringSetting(settings.mic_device, "default");
  const hotkey = isHotkeyConfig(settings.hotkey) ? settings.hotkey : undefined;
  const shortcutDisplay = formatAccelerator(hotkey?.key ?? "Ctrl+Space", platform);

  const recentItems = useMemo(() => historyItems.slice(0, 3), [historyItems]);
  const avgWpm = useMemo(() => computeAvgWpm(historyItems), [historyItems]);

  const engineMissing =
    settings.stt_engine === "whisper_cpp"
      ? !getStringSetting(settings.whisper_cpp_model_path, "")
      : settings.groq_api_key_set !== true;

  return (
    <div className="mx-auto w-full max-w-5xl px-8 pt-7 pb-24 flex flex-col gap-5 vx-panel-reveal">
      <HomeTopBar
        t={t}
        isRecording={isRecording}
        isProcessing={isProcessing}
        durationSec={durationSec}
        hotkey={hotkey}
      />

      {engineMissing && <SetupMissingBanner t={t} />}

      <div className="grid grid-cols-1 gap-5 lg:grid-cols-[390px_minmax(0,1fr)] items-stretch">
        <VoiceConsoleCard
          t={t}
          state={state}
          durationSec={durationSec}
          wordCount={wordCount}
          outcome={outcome}
          hotkey={hotkey}
          shortcutDisplay={shortcutDisplay}
          micDevice={micDevice}
          sttEngine={settings.stt_engine}
          onMicClick={() => void handleMicClick()}
        />

        <div className="flex flex-col gap-3.5 min-w-0">
          <LifetimeTelemetryCard
            t={t}
            totals={totals}
            avgWpm={avgWpm}
            statsError={statsError}
            onRetry={() => void loadStats()}
          />

          <ModeTranslateCard
            t={t}
            activeMode={activeMode}
            translationEnabled={translationEnabled}
            translationTarget={translationTarget}
            onUpdateSetting={updateSetting}
          />

          <RecentTranscriptionsCard
            t={t}
            recentItems={recentItems}
            historyError={historyError}
            copiedId={copiedId}
            injectedId={injectedId}
            onRetry={() => void loadHistory()}
            onCopy={(id, text) => void handleCopy(id, text)}
            onReInject={(id) => void handleReInject(id)}
            onTogglePin={(id, pinned) =>
              void invokeAction(() => togglePin(id, pinned))
            }
          />
        </div>
      </div>
    </div>
  );
}
