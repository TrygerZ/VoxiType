import { useEffect, useRef, useState } from "react";
import { AlertTriangle, FileAudio, X } from "lucide-react";

import { useT } from "../../lib/i18n";
import type { FileItem } from "../../stores/fileTranscriptionStore";
import type { StageIssue } from "../../types/app";
import type { FileTranscriptionProgressEvent } from "../../types/events";
import { Button } from "../ui/Button";
import { IconButton } from "../ui/IconButton";
import { StatusBadge } from "../ui/StatusBadge";
import { toast } from "../ui/Toast";

type Translate = ReturnType<typeof useT>;

const COPY_FEEDBACK_MS = 1200;
const TIMER_TICK_MS = 1000;
const MS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;
const SECONDS_PER_HOUR = 3600;

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function dirName(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx > 0 ? path.slice(0, idx) : "";
}

/** mm:ss, or h:mm:ss from one hour up. */
export function formatElapsed(ms: number): string {
  const total = Math.max(0, Math.floor(ms / MS_PER_SECOND));
  const hours = Math.floor(total / SECONDS_PER_HOUR);
  const minutes = Math.floor((total % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE);
  const pad = (n: number) => String(n).padStart(2, "0");
  const clock = `${pad(minutes)}:${pad(total % SECONDS_PER_MINUTE)}`;
  return hours > 0 ? `${hours}:${clock}` : clock;
}

/** Current time, re-read every second while `active`. */
function useNow(active: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!active) return;
    setNow(Date.now());
    const id = setInterval(() => setNow(Date.now()), TIMER_TICK_MS);
    return () => clearInterval(id);
  }, [active]);
  return now;
}

export function progressLabel(
  t: Translate,
  progress: FileTranscriptionProgressEvent | null,
) {
  if (!progress) return t("file.status_starting");
  const key =
    progress.stage === "transcribing"
      ? "file.status_transcribing"
      : "file.status_formatting";
  return t(key, {
    done: progress.done,
    total: progress.total > 0 ? progress.total : "?",
  });
}

export function IssueNote({
  text,
  className = "",
}: {
  text: string;
  className?: string;
}) {
  return (
    <div
      role="alert"
      className={`bg-vx-semantic-warning-soft rounded-[8px] px-3 py-2 flex items-start gap-2 text-xs text-vx-semantic-warning ${className}`}
    >
      <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
      <span className="break-words">{text}</span>
    </div>
  );
}

function issueText(t: Translate, key: string, issue: StageIssue) {
  return t(key, {
    count: issue.count,
    total: issue.total,
    reason: issue.reason,
  });
}

function useCopyFeedback(t: Translate) {
  const [copied, setCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );
  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      toast(t("file.copy_failed"), "error");
      return;
    }
    setCopied(true);
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setCopied(false), COPY_FEEDBACK_MS);
  };
  return { copied, copy };
}

function progressPercent(progress: FileTranscriptionProgressEvent | null): number {
  if (!progress || progress.total <= 0) return 0;
  return Math.min(100, Math.max(0, Math.round((progress.done / progress.total) * 100)));
}

interface FileQueueItemProps {
  item: FileItem;
  /** Queue and options are locked while a batch runs. */
  locked: boolean;
  exporting: boolean;
  defaultOpen?: boolean;
  onRemove: (path: string) => void;
  onRetry: (path: string) => void;
  onExport: (path: string) => void;
}

export function FileQueueItem({
  item,
  locked,
  exporting,
  onRemove,
  onRetry,
  onExport,
}: FileQueueItemProps) {
  const t = useT();
  const { copied, copy } = useCopyFeedback(t);
  const now = useNow(item.status === "running");
  const name = fileName(item.path);
  const dir = dirName(item.path);
  const { result } = item;
  const elapsed =
    item.startedAt === null
      ? null
      : formatElapsed((item.finishedAt ?? now) - item.startedAt);

  if (item.status === "queued") {
    return (
      <li className="bg-vx-bg-surface border border-vx-border-subtle rounded-[8px] px-4 py-3 flex flex-wrap sm:flex-nowrap items-center justify-between gap-3">
        <div className="flex min-w-0 flex-1 items-center gap-3">
          <FileAudio className="h-4 w-4 shrink-0 text-vx-text-muted" />
          <div className="flex min-w-0 flex-col">
            <span
              className="truncate text-[13px] font-medium text-vx-text-primary"
              title={item.path}
            >
              {name}
            </span>
            <span
              className="truncate font-mono text-xs text-vx-text-muted"
              title={item.path}
            >
              {dir || item.path}
            </span>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <StatusBadge tone="neutral" label={t("file.status_queued_badge")} />
          {!locked && (
            <IconButton
              size="sm"
              tone="ghost"
              label={t("file.remove", { name })}
              icon={<X className="h-[13px] w-[13px]" />}
              onClick={() => onRemove(item.path)}
            />
          )}
        </div>
      </li>
    );
  }

  if (item.status === "running") {
    const subtext = `${progressLabel(t, item.progress)}${dir ? ` \u00b7 ${dir}` : ""}`;
    return (
      <li className="bg-vx-bg-surface border border-vx-accent-ember rounded-[8px] px-4 py-3.5 flex flex-col gap-2.5">
        <div className="flex flex-wrap sm:flex-nowrap items-center justify-between gap-3">
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <FileAudio className="h-[18px] w-[18px] shrink-0 text-vx-accent-ember" />
            <div className="flex min-w-0 flex-col">
              <span
                className="truncate text-[13px] font-medium text-vx-text-primary"
                title={item.path}
              >
                {name}
              </span>
              <span className="truncate text-xs text-vx-text-secondary">
                {subtext}
              </span>
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-2.5">
            {elapsed && (
              <span
                className="font-mono text-xs tabular-nums text-vx-text-secondary"
                title={t("file.elapsed")}
              >
                {elapsed}
              </span>
            )}
            <StatusBadge tone="ember" label={t("file.status_running_badge")} />
          </div>
        </div>
        <div className="bg-vx-bg-elevated h-1.5 w-full rounded-full overflow-hidden">
          <div
            className="h-full bg-vx-accent-ember rounded-full"
            style={{
              width: `${progressPercent(item.progress)}%`,
              transition: "width 240ms var(--ease-vx-out)",
            }}
          />
        </div>
      </li>
    );
  }

  if (item.status === "done" && result) {
    const hasPartialIssue = Boolean(result.stt_issue || result.llm_issue);
    if (hasPartialIssue) {
      return (
        <li className="bg-vx-bg-surface border border-vx-semantic-warning rounded-[8px] px-4 py-3 flex flex-col gap-2">
          <div className="flex flex-wrap sm:flex-nowrap items-center justify-between gap-3">
            <div className="flex min-w-0 flex-1 items-center gap-2.5">
              <FileAudio className="h-4 w-4 shrink-0 text-vx-semantic-warning" />
              <span
                className="truncate text-[13px] font-medium text-vx-text-primary"
                title={item.path}
              >
                {name} {"\u00b7"}{" "}
                {t("file.words_partial", { count: result.word_count })}
              </span>
            </div>
            <div className="flex shrink-0 items-center gap-2.5">
              <Button
                variant="secondary"
                size="sm"
                disabled={locked}
                onClick={() => onRetry(item.path)}
              >
                {t("error.retry")}
              </Button>
              <StatusBadge
                tone="warning"
                label={
                  result.stt_issue
                    ? t("file.partial_stt_badge")
                    : t("file.partial_llm_badge")
                }
              />
            </div>
          </div>

          {result.stt_issue && (
            <IssueNote
              className="rounded-[6px] px-2.5 py-2"
              text={issueText(t, "file.partial_stt", result.stt_issue)}
            />
          )}
          {result.llm_issue && (
            <IssueNote
              className="rounded-[6px] px-2.5 py-2"
              text={issueText(t, "file.partial_llm", result.llm_issue)}
            />
          )}

          <div
            aria-label={`${t("file.result_label")}: ${name}`}
            className="bg-vx-bg-subtle border border-vx-border-subtle rounded-[6px] px-3.5 py-2.5 text-[14px] leading-6 text-vx-text-primary whitespace-pre-wrap"
          >
            {result.text}
          </div>

          <div className="flex items-center justify-end gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => void copy(result.text)}
            >
              {copied ? t("file.copied") : t("file.copy")}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              disabled={exporting}
              aria-label={t("file.export_one", { name })}
              onClick={() => onExport(item.path)}
            >
              {t("file.export_btn")}
            </Button>
          </div>
        </li>
      );
    }

    const doneMeta = elapsed
      ? t("file.done_meta", { count: result.word_count, elapsed })
      : t("file.done_meta_no_elapsed", { count: result.word_count });

    return (
      <li className="bg-vx-bg-surface border border-vx-border-default rounded-[12px] px-4 py-3.5 flex flex-col gap-3">
        <div className="flex flex-wrap sm:flex-nowrap items-center justify-between gap-3">
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <FileAudio className="h-4 w-4 shrink-0 text-vx-semantic-success" />
            <div className="flex min-w-0 flex-col">
              <span
                className="truncate text-[13px] font-medium text-vx-text-primary"
                title={item.path}
              >
                {name}
              </span>
              <span className="text-xs text-vx-semantic-success">
                {doneMeta}
              </span>
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => void copy(result.text)}
            >
              {copied ? t("file.copied") : t("file.copy")}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              disabled={exporting}
              aria-label={t("file.export_one", { name })}
              onClick={() => onExport(item.path)}
            >
              {t("file.export_btn")}
            </Button>
          </div>
        </div>
        <div
          aria-label={`${t("file.result_label")}: ${name}`}
          className="bg-vx-bg-subtle border border-vx-border-subtle rounded-[6px] px-3.5 py-2.5 text-[14px] leading-6 text-vx-text-primary whitespace-pre-wrap"
        >
          {result.text}
        </div>
      </li>
    );
  }

  return (
    <li
      className={`bg-vx-bg-surface border rounded-[8px] px-4 py-3 flex flex-col gap-2 ${
        item.status === "error"
          ? "border-vx-semantic-error/50"
          : "border-vx-border-default"
      }`}
    >
      <div className="flex flex-wrap sm:flex-nowrap items-center justify-between gap-3">
        <div className="flex min-w-0 flex-1 items-center gap-3">
          <FileAudio className="h-4 w-4 shrink-0 text-vx-text-muted" />
          <div className="flex min-w-0 flex-col">
            <span
              className="truncate text-[13px] font-medium text-vx-text-primary"
              title={item.path}
            >
              {name}
            </span>
            {dir && (
              <span className="truncate font-mono text-xs text-vx-text-muted">
                {dir}
              </span>
            )}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            disabled={locked}
            onClick={() => onRetry(item.path)}
          >
            {t("error.retry")}
          </Button>
          <StatusBadge
            tone={item.status === "error" ? "error" : "neutral"}
            label={
              item.status === "error"
                ? t("file.item_error")
                : t("file.item_cancelled")
            }
          />
          {!locked && (
            <IconButton
              size="sm"
              tone="ghost"
              label={t("file.remove", { name })}
              icon={<X className="h-[13px] w-[13px]" />}
              onClick={() => onRemove(item.path)}
            />
          )}
        </div>
      </div>
      {item.error && (
        <p role="alert" className="text-xs text-vx-semantic-error break-words">
          {item.error}
        </p>
      )}
    </li>
  );
}
