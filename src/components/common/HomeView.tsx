import { useEffect, useState, useMemo } from "react";
import {
  Mic,
  Loader2,
  Sparkles,
  Languages,
  Zap,
  Copy,
  Check,
  Pin,
  Keyboard,
  FileText,
  ArrowRight,
  Volume2,
  BarChart3,
} from "lucide-react";
import { WpmHalfRing } from "./WpmHalfRing";
import HourglassIcon from "../../assets/icons/hourglass.svg?react";
import SparkleIcon from "../../assets/icons/sparkle.svg?react";
import ScrollTextIcon from "../../assets/icons/scroll-text.svg?react";
import { useAppStore } from "../../stores/appStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useHistoryStore } from "../../stores/historyStore";
import { useStatsStore } from "../../stores/statsStore";
import { useT } from "../../lib/i18n";
import { startRecording, stopRecording, reInject } from "../../lib/tauri";
import { Waveform } from "../floating-widget/Waveform";

export function HomeView() {
  const t = useT();

  // App store states
  const state = useAppStore((s) => s.state);
  const audioLevel = useAppStore((s) => s.audioLevel);
  const durationSec = useAppStore((s) => s.durationSec);
  const isRecording = state === "recording";
  const isProcessing = state === "processing";

  // Settings store
  const settings = useSettingsStore((s) => s.settings);
  const updateSetting = useSettingsStore((s) => s.update);
  const loadSettings = useSettingsStore((s) => s.load);

  // History store
  const loadHistory = useHistoryStore((s) => s.load);
  const historyItems = useHistoryStore((s) => s.items);
  const togglePin = useHistoryStore((s) => s.togglePin);

  // Stats store — lifetime totals aggregated server-side (uncapped)
  const totals = useStatsStore((s) => s.totals);
  const loadStats = useStatsStore((s) => s.load);

  // Local UI states
  const [greeting, setGreeting] = useState("");
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [injectedId, setInjectedId] = useState<string | null>(null);

  // Load history, settings, and lifetime stats on mount
  useEffect(() => {
    void loadHistory();
    void loadSettings();
    void loadStats();
  }, [loadHistory, loadSettings, loadStats]);
  const lang = useSettingsStore((s) => s.settings.language);
  useEffect(() => {
    const hrs = new Date().getHours();
    if (hrs < 12) {
      setGreeting(t("home.greeting.morning"));
    } else if (hrs < 17) {
      setGreeting(t("home.greeting.afternoon"));
    } else {
      setGreeting(t("home.greeting.evening"));
    }
  }, [t, lang]);

  // Toggle record from GUI
  const handleMicClick = () => {
    if (isRecording) {
      void stopRecording();
    } else if (state === "idle" || state === "error") {
      void startRecording();
    }
  };

  const formatDuration = (sec: number) => {
    const m = Math.floor(sec / 60).toString().padStart(2, "0");
    const s = (sec % 60).toString().padStart(2, "0");
    return `${m}:${s}`;
  };

  const handleCopy = (id: string, text: string) => {
    void navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const handleReInject = async (id: string) => {
    try {
      await reInject(id);
      setInjectedId(id);
      setTimeout(() => setInjectedId(null), 2000);
    } catch {
      // ignore
    }
  };

  // Safe config readouts
  const activeMode = (settings.active_mode as string) ?? "dictation";
  const translationEnabled = (settings.translation_enabled as boolean) ?? false;
  const translationTarget = (settings.translation_target as string) ?? "en";
  const micDevice = (settings.mic_device as string) ?? "default";
  const hotkeyRaw = settings.hotkey as { key: string; mode: string } | undefined;
  const shortcutKey = hotkeyRaw?.key ?? "Ctrl+Space";

  const recentItems = useMemo(() => {
    return historyItems.slice(0, 3);
  }, [historyItems]);

  // Per-session avg WPM over the 20 most-recent sessions. historyItems is
  // newest-first (ORDER BY created_at DESC), so the most recent sessions are at
  // the HEAD — use slice(0, 20), not slice(-20) which would pick the oldest 20.
  // Implausibly fast sessions (very short blips) are discarded so a stray
  // 1-word / 40ms clip can't skew the average toward absurd values.
  const MAX_PLAUSIBLE_WPM = 400;
  const avgWpm = useMemo(() => {
    const rates: number[] = [];
    for (const i of historyItems) {
      const dur = i.duration_ms ?? 0;
      if (i.word_count <= 0 || dur <= 0) continue;
      const wpm = i.word_count / (dur / 60000);
      if (wpm > MAX_PLAUSIBLE_WPM) continue;
      rates.push(wpm);
      if (rates.length >= 20) break;
    }
    if (rates.length === 0) return 0;
    const sum = rates.reduce((a, w) => a + w, 0);
    return Math.round(sum / rates.length);
  }, [historyItems]);

  const hasStats = totals.total_sessions > 0;

  // Split a duration into a numeric value + unit so the value can carry the
  // display weight while the unit stays quiet — reads more like a designed
  // metric than a single "1h 20m" string.
  const splitDuration = (ms: number): { value: string; unit: string } => {
    const totalSec = Math.floor(ms / 1000);
    if (totalSec < 60) return { value: String(totalSec), unit: t("home.unit_sec") };
    const totalMin = Math.floor(totalSec / 60);
    if (totalMin < 60) return { value: String(totalMin), unit: totalMin === 1 ? t("home.unit_min") : t("home.unit_mins") };
    const hrs = Math.floor(totalMin / 60);
    const mins = totalMin % 60;
    return { value: mins > 0 ? `${hrs}.${Math.round((mins / 60) * 10)}` : String(hrs), unit: hrs === 1 && mins === 0 ? t("home.unit_hr") : t("home.unit_hrs") };
  };

  // Condense large counts (12,400 → 12.4k) so the ledger stays visually calm.
  // Boundaries are chosen so a value never rounds into the next unit's range
  // (e.g. 999,999 shows "1.0M", not "1000k").
  const formatCompact = (n: number): string => {
    if (n < 1000) return n.toLocaleString();
    if (n < 9_950) return `${(n / 1000).toFixed(1)}k`;
    if (n < 999_500) return `${Math.round(n / 1000)}k`;
    return `${(n / 1_000_000).toFixed(1)}M`;
  };

  const recordingTime = splitDuration(totals.total_duration_ms);

  return (
    <div className="mx-auto max-w-5xl px-6 py-8">
      {/* Grid Dashboard */}
      <div className="grid grid-cols-1 gap-8 lg:grid-cols-12">

        {/* Left Column: Recording Console */}
        <div className="flex flex-col items-center justify-center rounded-3xl border border-vx-border/30 bg-vx-bg-secondary/40 p-8 text-center backdrop-blur-md lg:col-span-5 min-h-[480px]">
          <div className="mb-2 text-xs font-semibold uppercase tracking-widest text-vx-accent">
            {t("app.name")}
          </div>
          <h2 className="mb-6 text-2xl font-bold tracking-tight text-vx-text-primary">
            {greeting}, User!
          </h2>

          {/* Voice Mic Button — calm brand tint, no glow */}
          <button
            type="button"
            onClick={handleMicClick}
            disabled={isProcessing}
            className="group relative mb-6 flex h-40 w-40 items-center justify-center rounded-full focus:outline-none disabled:cursor-not-allowed"
            title={isRecording ? t("home.mic_recording_tooltip") : t("home.mic_tooltip")}
          >
            {/* Outer soft state ring — calm tint, no colored bloom */}
            <span
              className={`absolute inset-0 rounded-full transition-all duration-300 ease-out ${
                isRecording
                  ? "bg-vx-accent/10 scale-105 blur-md"
                  : isProcessing
                    ? "bg-vx-success/10 scale-100 blur-md"
                    : "bg-vx-accent/5 scale-90 blur-xl group-hover:scale-110 group-hover:bg-vx-accent/10"
              }`}
            />
            {/* Inner Core button */}
            <span
              className={`relative flex h-24 w-24 items-center justify-center rounded-full transition-all duration-200 ease-out border ${
                isRecording
                  ? "bg-vx-accent scale-105 border-vx-accent/50 shadow-vx-lg text-white"
                  : isProcessing
                    ? "bg-vx-bg-tertiary border-vx-success/40 shadow-vx-lg text-vx-success"
                    : "bg-vx-bg-tertiary border-vx-border/40 shadow-vx-lg group-hover:scale-105 group-hover:border-vx-accent/50 text-vx-text-secondary"
              }`}
            >
              {isProcessing ? (
                <Loader2 className="h-9 w-9 animate-spin text-vx-success" />
              ) : (
                <Mic
                  className={`h-9 w-9 transition-colors duration-300 ${
                    isRecording
                      ? "text-white"
                      : "text-vx-text-secondary group-hover:text-vx-accent"
                  }`}
                />
              )}
            </span>
          </button>

          {/* Duration & Waveform feedback */}
          <div className="h-20 flex flex-col items-center justify-center w-full">
            {isRecording ? (
              <div className="flex flex-col items-center gap-1.5 w-full px-8">
                <span className="text-xl font-mono font-semibold tracking-wide text-vx-text-primary">
                  {formatDuration(durationSec)}
                </span>
                <div className="w-full max-w-[150px] flex items-center justify-center">
                  <Waveform
                    level={audioLevel}
                    active={isRecording}
                    barClassName="bg-vx-accent"
                  />
                </div>
              </div>
            ) : isProcessing ? (
              <span className="text-sm font-medium tracking-wide uppercase text-vx-success/80">
                {t("processing")}
              </span>
            ) : (
              <span className="text-sm font-medium tracking-wide text-vx-text-secondary">
                {t("idle.hint", { shortcut: shortcutKey })}
              </span>
            )}
          </div>

          {/* Quick Info Badges */}
          <div className="mt-8 flex flex-wrap justify-center gap-2 text-xs text-vx-text-dim">
            <span className="flex items-center gap-1.5 rounded-full border border-vx-border/30 bg-vx-bg-tertiary/40 px-3 py-1">
              <Volume2 className="h-3.5 w-3.5 text-vx-text-secondary" />
              {micDevice === "default" ? t("home.default_mic") : micDevice.split("(")[0].trim()}
            </span>
            <span className="flex items-center gap-1.5 rounded-full border border-vx-border/30 bg-vx-bg-tertiary/40 px-3 py-1">
              <Sparkles className="h-3.5 w-3.5 text-vx-accent" />
              {settings.stt_engine === "whisper_cpp" ? t("home.local_whisper") : t("home.groq_cloud")}
            </span>
          </div>
        </div>

        {/* Right Column: Controls & Clipboard history */}
        <div className="flex flex-col gap-6 lg:col-span-7">

          {/* Usage Stats Card — editorial ledger: one hero gauge + a hairline
              stat column, instead of four identical badge tiles. */}
          <div className="group/stats relative overflow-hidden rounded-3xl border border-vx-border/30 bg-vx-bg-secondary/40 p-6 backdrop-blur-md">
            {/* Soft corner wash — atmosphere without a colored glow */}
            <div
              aria-hidden
              className="pointer-events-none absolute -right-16 -top-20 h-56 w-56 rounded-full bg-vx-accent/[0.06] blur-3xl"
            />

            <div className="mb-5 flex items-center justify-between">
              <h3 className="flex items-center gap-2 text-sm font-semibold uppercase tracking-wider text-vx-text-secondary">
                <BarChart3 className="h-4 w-4 text-vx-accent" />
                {t("home.usage_stats")}
              </h3>
              <span className="text-[11px] font-medium uppercase tracking-[0.14em] text-vx-text-dim">
                {t("home.lifetime")}
              </span>
            </div>

            <div className="relative grid grid-cols-1 gap-6 sm:grid-cols-[minmax(0,0.9fr)_1px_minmax(0,1.1fr)]">
              {/* Hero — WPM gauge */}
              <div className="flex flex-col items-center justify-center">
                <WpmHalfRing value={avgWpm} />
                <span className="mt-1 text-[11px] uppercase tracking-[0.18em] text-vx-text-dim">
                  {t("home.words_per_minute")}
                </span>
              </div>

              {/* Vertical hairline (only on the 3-col layout) */}
              <div aria-hidden className="hidden bg-vx-divider sm:block" />

              {/* Stat ledger — hairline-separated rows, tabular figures */}
              <dl className="flex flex-col justify-center divide-y divide-vx-divider">
                {[
                  {
                    key: "time",
                    Icon: HourglassIcon,
                    label: t("home.total_time"),
                    value: recordingTime.value,
                    unit: recordingTime.unit,
                  },
                  {
                    key: "words",
                    Icon: ScrollTextIcon,
                    label: t("home.total_words"),
                    value: formatCompact(totals.total_words),
                    unit: t("home.unit_words"),
                  },
                  {
                    key: "sessions",
                    Icon: SparkleIcon,
                    label: t("home.sessions"),
                    value: formatCompact(totals.total_sessions),
                    unit: t("home.unit_sessions"),
                  },
                ].map(({ key, Icon, label, value, unit }) => (
                  <div key={key} className="flex items-center gap-3 py-3 first:pt-0 last:pb-0">
                    <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-vx-border/40 bg-vx-bg-tertiary/40 text-vx-accent transition-colors duration-300 group-hover/stats:border-vx-accent/40">
                      <Icon className="h-[18px] w-[18px]" />
                    </div>
                    <dt className="flex-1 text-xs text-vx-text-secondary">{label}</dt>
                    <dd className="grid w-24 grid-cols-[1fr_auto] items-baseline gap-1">
                      <span className="text-right text-xl font-semibold tabular-nums tracking-tight text-vx-text-primary">
                        {value}
                      </span>
                      <span className="w-8 text-left text-[11px] font-medium text-vx-text-dim">
                        {unit}
                      </span>
                    </dd>
                  </div>
                ))}
              </dl>
            </div>

            {!hasStats && (
              <p className="mt-5 border-t border-vx-divider pt-4 text-center text-xs text-vx-text-dim">
                {t("home.stats_empty")}
              </p>
            )}
          </div>

          {/* Quick Settings Card */}
          <div className="rounded-3xl border border-vx-border/30 bg-vx-bg-secondary/40 p-6 backdrop-blur-md">
            <h3 className="mb-4 flex items-center gap-2 text-sm font-semibold uppercase tracking-wider text-vx-text-secondary">
              <Zap className="h-4 w-4 text-vx-accent" />
              {t("home.quick_settings")}
            </h3>

            {/* Mode selection row */}
            <div className="mb-6">
              <label className="mb-2 block text-xs text-vx-text-dim">
                {t("home.active_mode")}
              </label>
              <div className="grid grid-cols-3 gap-2">
                {[
                  { id: "dictation", label: t("settings.modes.dictation"), desc: t("home.mode.dictation.desc") },
                  { id: "message", label: t("settings.modes.message"), desc: t("home.mode.message.desc") },
                  { id: "email", label: t("settings.modes.email"), desc: t("home.mode.email.desc") },
                ].map((m) => (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => void updateSetting("active_mode", m.id)}
                    className={`flex flex-col items-center justify-center rounded-xl border p-3 text-center transition-all duration-200 focus:outline-none ${
                      activeMode === m.id
                        ? "border-vx-accent/50 bg-vx-accent/5 text-vx-text-primary"
                        : "border-vx-border/30 bg-vx-bg-tertiary/30 text-vx-text-dim hover:text-vx-text-secondary hover:border-vx-border/60"
                    }`}
                  >
                    <span className="text-sm font-semibold">{m.label}</span>
                    <span className="mt-0.5 text-[10px] opacity-75">{m.desc}</span>
                  </button>
                ))}
              </div>
            </div>

            {/* Quick Translation Toggle */}
            <div className="flex items-center justify-between rounded-xl border border-vx-border/30 bg-vx-bg-tertiary/20 p-4">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-vx-accent-soft text-vx-accent">
                  <Languages className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-sm font-medium text-vx-text-primary">
                    {t("home.translate_to", { target: translationTarget === "en" ? "English" : "Bahasa Indonesia" })}
                  </h4>
                  <p className="text-xs text-vx-text-dim">{t("home.translate_desc")}</p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                {translationEnabled && (
                  <select
                    value={translationTarget}
                    onChange={(e) => void updateSetting("translation_target", e.target.value)}
                    className="rounded-lg border border-vx-border bg-vx-bg-tertiary px-2 py-1 text-xs text-vx-text-primary focus:border-vx-accent focus:outline-none"
                  >
                    <option value="en">English</option>
                    <option value="id">Indonesia</option>
                  </select>
                )}
                <button
                  type="button"
                  onClick={() => void updateSetting("translation_enabled", !translationEnabled)}
                  className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
                    translationEnabled ? "bg-vx-accent" : "bg-vx-border-strong"
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                      translationEnabled ? "translate-x-5" : "translate-x-0"
                    }`}
                  />
                </button>
              </div>
            </div>
          </div>

          {/* Recent Dictations Card */}
          <div className="rounded-3xl border border-vx-border/30 bg-vx-bg-secondary/40 p-6 backdrop-blur-md flex-1 flex flex-col">
            <h3 className="mb-4 flex items-center gap-2 text-sm font-semibold uppercase tracking-wider text-vx-text-secondary">
              <FileText className="h-4 w-4 text-vx-accent" />
              {t("home.recent_transcriptions")}
            </h3>

            {recentItems.length === 0 ? (
              <div className="flex flex-1 flex-col items-center justify-center p-6 text-center text-vx-text-dim">
                <Mic className="mb-2 h-8 w-8 opacity-20" />
                <p className="text-xs">{t("home.no_history")}</p>
              </div>
            ) : (
              <div className="flex flex-col gap-3">
                {recentItems.map((item) => (
                  <div
                    key={item.id}
                    className="group/item flex flex-col justify-between gap-3 rounded-2xl border border-vx-border/30 bg-vx-bg-tertiary/20 p-4 transition-colors hover:border-vx-border/60 hover:bg-vx-bg-tertiary/40"
                  >
                    <div className="flex items-start justify-between gap-4">
                      <p className="line-clamp-2 text-sm text-vx-text-primary leading-relaxed">
                        {item.text_formatted || item.text_raw}
                      </p>

                      {/* Copy, Re-inject, Pin Action buttons */}
                      <div className="flex shrink-0 items-center gap-1 opacity-60 transition-opacity group-hover/item:opacity-100">
                        {/* Copy */}
                        <button
                          type="button"
                          onClick={() => handleCopy(item.id, item.text_formatted || item.text_raw)}
                          className="flex h-8 w-8 items-center justify-center rounded-lg hover:bg-vx-bg-tertiary text-vx-text-secondary hover:text-vx-text-primary focus:outline-none"
                          title={t("history.copy_tooltip")}
                        >
                          {copiedId === item.id ? (
                            <Check className="h-4 w-4 text-vx-success" />
                          ) : (
                            <Copy className="h-4 w-4" />
                          )}
                        </button>

                        {/* Re-inject */}
                        <button
                          type="button"
                          onClick={() => void handleReInject(item.id)}
                          className="flex h-8 w-8 items-center justify-center rounded-lg hover:bg-vx-bg-tertiary text-vx-text-secondary hover:text-vx-text-primary focus:outline-none"
                          title={t("history.re_inject_tooltip")}
                        >
                          {injectedId === item.id ? (
                            <Check className="h-4 w-4 text-vx-success" />
                          ) : (
                            <ArrowRight className="h-4 w-4" />
                          )}
                        </button>

                        {/* Pin */}
                        <button
                          type="button"
                          onClick={() => void togglePin(item.id, !item.is_pinned)}
                          className={`flex h-8 w-8 items-center justify-center rounded-lg hover:bg-vx-bg-tertiary focus:outline-none ${
                            item.is_pinned
                              ? "text-vx-accent"
                              : "text-vx-text-secondary hover:text-vx-text-primary"
                          }`}
                          title={item.is_pinned ? t("home.unpin_tooltip") : t("home.pin_tooltip")}
                        >
                          <Pin className="h-4 w-4" />
                        </button>
                      </div>
                    </div>

                    <div className="flex items-center justify-between text-[11px] text-vx-text-dim border-t border-vx-border/20 pt-2">
                      <span className="flex items-center gap-1.5">
                        <span className="h-1.5 w-1.5 rounded-full bg-vx-accent/60" />
                        <span className="capitalize">{t(`settings.modes.${item.mode}`)}</span>
                        {item.word_count > 0 && (
                          <>
                            <span>•</span>
                            <span>
                              {item.word_count} {t("home.words")}
                            </span>
                          </>
                        )}
                      </span>
                      <span>
                        {(() => {
                          if (!item.created_at) return "";
                          const d = new Date(item.created_at);
                          if (isNaN(d.getTime())) return "";
                          return d.toLocaleTimeString([], {
                            hour: "2-digit",
                            minute: "2-digit",
                          });
                        })()}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Quick keyboard shortcut reminder */}
          <div className="flex items-center justify-center gap-2 rounded-2xl border border-vx-border/20 bg-vx-bg-secondary/10 p-3 text-xs text-vx-text-dim">
            <Keyboard className="h-4 w-4" />
            <span>
              {t("home.shortcut_tip", { shortcut: shortcutKey })}
            </span>
          </div>

        </div>

      </div>
    </div>
  );
}
