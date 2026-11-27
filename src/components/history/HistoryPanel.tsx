import { useEffect, useRef, useState } from "react";
import { History as HistoryIcon, Pin, Search, Trash2 } from "lucide-react";

import { downloadBlob } from "../../lib/downloadBlob";
import { modeLabel, type Translate, useT } from "../../lib/i18n";
import { invokeAction } from "../../lib/invokeAction";
import { exportHistory, reInject } from "../../lib/tauri";
import { useHistoryStore } from "../../stores/historyStore";
import type { TranscriptionEntry } from "../../types/app";
import { Button } from "../ui/Button";
import { IconButton } from "../ui/IconButton";
import { Select } from "../ui/Select";
import { StatusBadge } from "../ui/StatusBadge";
import { toast } from "../ui/Toast";

type ReaderTab = "formatted" | "raw";

const COPY_FEEDBACK_MS = 1200;

function formatDateTime(dateStr?: string | null): string {
  if (!dateStr) return "";
  const d = new Date(dateStr);
  if (isNaN(d.getTime())) return "";
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function formatDurationMs(ms?: number | null): string {
  if (!ms || ms <= 0) return "";
  const totalSec = Math.floor(ms / 1000);
  const mins = Math.floor(totalSec / 60)
    .toString()
    .padStart(2, "0");
  const secs = (totalSec % 60).toString().padStart(2, "0");
  return `${mins}:${secs}`;
}

function buildEntryMeta(item: TranscriptionEntry, t: Translate): string {
  return [
    modeLabel(t, item.mode),
    item.app_context ?? "",
    t("history.words_suffix", { count: item.word_count }),
    formatDateTime(item.created_at),
  ]
    .filter(Boolean)
    .join(" \u00b7 ");
}

function HistoryToolbar({
  t,
  query,
  modeFilter,
  pinnedOnly,
  onSearch,
  onModeChange,
  onPinnedChange,
}: {
  t: Translate;
  query: string;
  modeFilter: string;
  pinnedOnly: boolean;
  onSearch: (q: string) => void;
  onModeChange: (mode: string) => void;
  onPinnedChange: (pinned: boolean) => void;
}) {
  const modeOptions = [
    { value: "", label: t("history.all_modes") },
    { value: "dictation", label: t("settings.modes.dictation") },
    { value: "message", label: t("settings.modes.message") },
    { value: "email", label: t("settings.modes.email") },
    { value: "file", label: t("settings.modes.file") },
  ];

  const segmentBtn = (active: boolean) =>
    `h-7 px-2.5 rounded-[5px] text-xs font-medium inline-flex items-center justify-center transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember ${
      active
        ? "bg-vx-bg-elevated text-vx-text-primary shadow-vx-subtle"
        : "text-vx-text-secondary hover:text-vx-text-primary"
    }`;

  return (
    <div className="flex flex-wrap items-center gap-2.5">
      <div className="relative flex-1 min-w-[180px]">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-[15px] w-[15px] -translate-y-1/2 text-vx-text-muted" />
        <input
          aria-label={t("history.search_placeholder")}
          placeholder={t("history.search_placeholder")}
          value={query}
          onChange={(e) => onSearch(e.target.value)}
          className="h-9 w-full rounded-[8px] border border-vx-border-default bg-vx-bg-subtle pl-9 pr-3 text-[13px] leading-[18px] text-vx-text-primary placeholder:text-vx-text-muted focus:border-vx-accent-ember focus:outline-none focus:ring-1 focus:ring-vx-accent-ember"
        />
      </div>

      <Select
        aria-label={t("history.mode_filter")}
        options={modeOptions}
        value={modeFilter}
        onChange={(e) => onModeChange(e.target.value)}
        className="w-[136px] sm:w-[150px]"
      />

      <div className="bg-vx-bg-subtle border border-vx-border-default rounded-[8px] p-[3px] flex items-center gap-0.5 shrink-0">
        <button
          type="button"
          aria-pressed={!pinnedOnly}
          onClick={() => onPinnedChange(false)}
          className={segmentBtn(!pinnedOnly)}
        >
          {t("history.filter_all")}
        </button>
        <button
          type="button"
          aria-pressed={pinnedOnly}
          onClick={() => onPinnedChange(true)}
          className={segmentBtn(pinnedOnly)}
        >
          {t("history.filter_pinned")}
        </button>
      </div>
    </div>
  );
}

function HistoryEntryCard({
  t,
  item,
  selected,
  onSelect,
}: {
  t: Translate;
  item: TranscriptionEntry;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-current={selected ? "true" : undefined}
      className={`rounded-[8px] px-3.5 py-3 text-left flex flex-col gap-1.5 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember ${
        selected
          ? "bg-vx-bg-elevated border border-vx-border-strong"
          : "bg-vx-bg-surface border border-vx-border-subtle hover:border-vx-border-default"
      }`}
    >
      <div className="flex items-start justify-between gap-2">
        <p className="line-clamp-2 text-[13px] leading-[18px] text-vx-text-primary">
          {item.text_formatted || item.text_raw}
        </p>
        {item.is_pinned && (
          <Pin
            className="h-[13px] w-[13px] shrink-0 text-vx-accent-ember mt-0.5"
            aria-hidden="true"
          />
        )}
      </div>
      <span className="text-xs leading-4 text-vx-text-muted">
        {buildEntryMeta(item, t)}
      </span>
    </button>
  );
}

function HistoryEditorialReader({
  t,
  selected,
  readerTab,
  copied,
  onTabChange,
  onTogglePin,
  onCopy,
  onReInject,
  onDelete,
}: {
  t: Translate;
  selected: TranscriptionEntry | null;
  readerTab: ReaderTab;
  copied: string | null;
  onTabChange: (tab: ReaderTab) => void;
  onTogglePin: (item: TranscriptionEntry) => void;
  onCopy: (id: string, text: string) => void;
  onReInject: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  if (!selected) {
    return (
      <div className="bg-vx-bg-surface border border-vx-border-default rounded-[12px] px-5 py-[18px] flex flex-col items-center justify-center min-w-0 text-center">
        <p className="text-sm text-vx-text-muted">{t("history.empty_reader")}</p>
      </div>
    );
  }

  const displayedText =
    readerTab === "raw"
      ? selected.text_raw
      : selected.text_formatted || selected.text_raw;
  const engineLabel = [selected.stt_engine, selected.llm_engine]
    .filter(Boolean)
    .join(" + ");
  const durationLabel = formatDurationMs(selected.duration_ms);
  const wordsLabel = t("history.words_suffix", { count: selected.word_count });
  const emberMeta = durationLabel
    ? `${durationLabel} \u00b7 ${wordsLabel}`
    : wordsLabel;

  const tabClass = (active: boolean) =>
    `h-7 px-2.5 rounded-[5px] text-xs font-medium inline-flex items-center justify-center transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember ${
      active
        ? "bg-vx-bg-elevated text-vx-text-primary shadow-vx-subtle"
        : "text-vx-text-secondary hover:text-vx-text-primary"
    }`;

  return (
    <div className="bg-vx-bg-surface border border-vx-border-default rounded-[12px] px-4 py-4 sm:px-5 sm:py-[18px] min-h-[220px] md:min-h-0 flex flex-col gap-3.5 min-w-0 overflow-y-auto">
      <div className="flex flex-wrap items-center justify-between gap-2.5 sm:gap-3">
        <div className="bg-vx-bg-subtle border border-vx-border-subtle rounded-[8px] p-[3px] flex items-center gap-0.5">
          <button
            type="button"
            aria-pressed={readerTab === "formatted"}
            onClick={() => onTabChange("formatted")}
            className={tabClass(readerTab === "formatted")}
          >
            {t("history.tab_formatted")}
          </button>
          <button
            type="button"
            aria-pressed={readerTab === "raw"}
            onClick={() => onTabChange("raw")}
            className={tabClass(readerTab === "raw")}
          >
            {t("history.tab_raw")}
          </button>
        </div>

        <div className="flex flex-wrap items-center gap-1.5 shrink-0">
          <IconButton
            size="md"
            tone="ghost"
            label={
              selected.is_pinned
                ? t("history.unpin_tooltip")
                : t("history.pin_tooltip")
            }
            className={
              selected.is_pinned
                ? "text-vx-accent-ember hover:text-vx-accent-ember"
                : ""
            }
            icon={<Pin className="h-3.5 w-3.5" />}
            onClick={() => onTogglePin(selected)}
          />
          <Button
            variant="primary"
            size="sm"
            onClick={() => onCopy(selected.id, displayedText)}
          >
            {copied === selected.id
              ? t("history.copied")
              : t("history.copy_tooltip")}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => onReInject(selected.id)}
          >
            {t("history.re_inject_tooltip")}
          </Button>
          <IconButton
            size="md"
            tone="danger"
            label={t("history.delete_tooltip")}
            icon={<Trash2 className="h-3.5 w-3.5" />}
            onClick={() => onDelete(selected.id)}
          />
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-1.5">
        {selected.app_context && (
          <StatusBadge tone="neutral" label={selected.app_context} />
        )}
        {engineLabel && <StatusBadge tone="neutral" label={engineLabel} />}
        <StatusBadge tone="ember" label={emberMeta} />
      </div>

      <div className="flex-1 whitespace-pre-wrap text-[14.5px] leading-6 text-vx-text-primary select-text">
        {displayedText}
      </div>
    </div>
  );
}

export function HistoryPanel() {
  const t = useT();
  const items = useHistoryStore((s) => s.items);
  const loading = useHistoryStore((s) => s.loading);
  const error = useHistoryStore((s) => s.error);
  const query = useHistoryStore((s) => s.query);
  const load = useHistoryStore((s) => s.load);
  const loadMore = useHistoryStore((s) => s.loadMore);
  const hasMore = useHistoryStore((s) => s.hasMore);
  const search = useHistoryStore((s) => s.search);
  const remove = useHistoryStore((s) => s.remove);
  const clear = useHistoryStore((s) => s.clear);
  const togglePin = useHistoryStore((s) => s.togglePin);

  const [modeFilter, setModeFilter] = useState("");
  const [pinnedOnly, setPinnedOnly] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [readerTab, setReaderTab] = useState<ReaderTab>("formatted");
  const [copied, setCopied] = useState<string | null>(null);
  const [confirmingClear, setConfirmingClear] = useState(false);
  const [clearing, setClearing] = useState(false);
  const copyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    return () => {
      if (copyTimerRef.current) clearTimeout(copyTimerRef.current);
    };
  }, []);

  const handleClear = async () => {
    setClearing(true);
    try {
      await invokeAction(() => clear(true));
    } finally {
      setClearing(false);
      setConfirmingClear(false);
    }
  };

  const handleExport = async (fmt: "json" | "csv") => {
    await invokeAction(async () => {
      const data = await exportHistory(fmt);
      const filename = `voxitype-history.${fmt}`;
      const mimeType = fmt === "json" ? "application/json" : "text/csv";
      downloadBlob(data, filename, mimeType);
      toast(t("file.exported", { count: 1 }), "success", undefined, filename);
    });
  };

  const handleCopy = async (id: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      toast(t("file.copy_failed"), "error");
      return;
    }
    if (copyTimerRef.current) {
      clearTimeout(copyTimerRef.current);
      copyTimerRef.current = null;
    }
    setCopied(id);
    copyTimerRef.current = setTimeout(() => {
      setCopied((c) => (c === id ? null : c));
      copyTimerRef.current = null;
    }, COPY_FEEDBACK_MS);
  };

  const filtered = items.filter((item) => {
    if (modeFilter && item.mode !== modeFilter) return false;
    if (pinnedOnly && !item.is_pinned) return false;
    return true;
  });

  const selected =
    filtered.find((item) => item.id === selectedId) ?? filtered[0] ?? null;

  return (
    <div className="mx-auto flex min-h-full md:h-full w-full max-w-5xl flex-col gap-4 sm:gap-5 px-4 sm:px-6 lg:px-8 pt-5 sm:pt-7 pb-24 vx-panel-reveal">
      <header className="flex flex-wrap items-start justify-between gap-3 sm:gap-4">
        <div className="flex flex-col gap-0.5">
          <h1 className="text-[22px] font-semibold leading-7 tracking-[-0.02em] text-vx-text-primary">
            {t("history.title")}
          </h1>
          <p className="text-[13px] leading-[18px] text-vx-text-secondary">
            {t("history.subtitle_v2")}
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2 shrink-0">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void handleExport("json")}
            title={t("history.export_json")}
          >
            {t("history.export_json")}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void handleExport("csv")}
            title={t("history.export_csv")}
          >
            {t("history.export_csv")}
          </Button>
          {confirmingClear ? (
            <>
              <Button
                variant="danger"
                size="sm"
                onClick={() => void handleClear()}
                disabled={clearing}
              >
                {clearing ? t("history.clearing") : t("history.confirm")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setConfirmingClear(false)}
                disabled={clearing}
              >
                {t("history.cancel")}
              </Button>
            </>
          ) : (
            <Button
              variant="danger"
              size="sm"
              onClick={() => setConfirmingClear(true)}
              disabled={items.length === 0}
              title={t("history.clear_tooltip")}
            >
              {t("history.clear_unpinned")}
            </Button>
          )}
        </div>
      </header>

      <HistoryToolbar
        t={t}
        query={query}
        modeFilter={modeFilter}
        pinnedOnly={pinnedOnly}
        onSearch={(q) => void search(q)}
        onModeChange={setModeFilter}
        onPinnedChange={setPinnedOnly}
      />

      {loading && items.length === 0 ? (
        <p className="text-sm text-vx-text-muted">{t("history.loading")}</p>
      ) : error && !loading ? (
        <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
          <div className="max-w-md space-y-1">
            <p className="text-sm font-semibold text-vx-text-primary">
              {t("error.history_load_failed")}
            </p>
            <p className="font-mono text-xs break-all text-vx-text-muted">
              {error}
            </p>
          </div>
          <Button size="sm" onClick={() => void load()}>
            {t("error.retry")}
          </Button>
        </div>
      ) : filtered.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 py-20 text-center">
          <HistoryIcon className="h-10 w-10 text-vx-text-muted opacity-40" />
          {items.length === 0 ? (
            <p className="text-sm text-vx-text-muted">{t("history.empty")}</p>
          ) : (
            <>
              <p className="text-sm text-vx-text-muted">
                {query.trim()
                  ? t("history.no_results", { query })
                  : t("history.no_results_mode")}
              </p>
              <Button
                size="sm"
                onClick={() => {
                  setModeFilter("");
                  setPinnedOnly(false);
                  void search("");
                }}
              >
                {t("history.clear_search")}
              </Button>
            </>
          )}
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-[minmax(240px,0.42fr)_minmax(0,0.58fr)] lg:grid-cols-[minmax(0,0.44fr)_minmax(0,0.56fr)] flex-1 min-h-0">
          <div className="flex flex-col gap-2 overflow-y-auto pr-1 max-h-[220px] md:max-h-none">
            {filtered.map((item) => (
              <HistoryEntryCard
                key={item.id}
                t={t}
                item={item}
                selected={selected?.id === item.id}
                onSelect={() => setSelectedId(item.id)}
              />
            ))}
            {!query && hasMore && (
              <div className="flex justify-center pt-2">
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={loading}
                  onClick={() => void loadMore()}
                >
                  {t("history.load_more")}
                </Button>
              </div>
            )}
          </div>

          <HistoryEditorialReader
            t={t}
            selected={selected}
            readerTab={readerTab}
            copied={copied}
            onTabChange={setReaderTab}
            onTogglePin={(item) =>
              void invokeAction(() => togglePin(item.id, !item.is_pinned))
            }
            onCopy={(id, text) => void handleCopy(id, text)}
            onReInject={(id) => void invokeAction(() => reInject(id))}
            onDelete={(id) => void invokeAction(() => remove(id))}
          />
        </div>
      )}
    </div>
  );
}
