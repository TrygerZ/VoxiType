import { useEffect, useRef, useState } from "react";
import { AlertTriangle, ChevronDown, Copy, Download, FileAudio, X } from "lucide-react";
import { useT } from "../../lib/i18n";
import { toast } from "../ui/Toast";
import { Button } from "../ui/Button";
import type { FileItem } from "../../stores/fileTranscriptionStore";
import type { StageIssue } from "../../types/app";
import type { FileTranscriptionProgressEvent } from "../../types/events";

type Translate = ReturnType<typeof useT>;

const COPY_FEEDBACK_MS = 1200;
const TIMER_TICK_MS = 1000;
const MS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;
const SECONDS_PER_HOUR = 3600;

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
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

export function progressLabel(t: Translate, progress: FileTranscriptionProgressEvent | null) {
  if (!progress) return t("file.status_starting");
  const key = progress.stage === "transcribing" ? "file.status_transcribing" : "file.status_formatting";
  return t(key, { done: progress.done, total: progress.total > 0 ? progress.total : "?" });
}

function statusLabel(t: Translate, item: FileItem): string {
  switch (item.status) {
    case "queued":
      return t("file.item_queued");
    case "running":
      return progressLabel(t, item.progress);
    case "done":
      return t("file.result_meta", { count: item.result?.word_count ?? 0 });
    case "error":
      return t("file.item_error");
    case "cancelled":
      return t("file.item_cancelled");
  }
}

export function IssueNote({ text }: { text: string }) {
  return (
    <p role="alert" className="flex items-start gap-2 text-xs text-vx-warning">
      <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" /> <span className="break-words">{text}</span>
    </p>
  );
}

function issueText(t: Translate, key: string, issue: StageIssue) {
  return t(key, { count: issue.count, total: issue.total, reason: issue.reason });
}

function useCopyFeedback(t: Translate) {
  const [copied, setCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => () => {
    if (timer.current) clearTimeout(timer.current);
  }, []);
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

interface FileQueueItemProps {
  item: FileItem;
  /** Queue and options are locked while a batch runs. */
  locked: boolean;
  exporting: boolean;
  defaultOpen: boolean;
  onRemove: (path: string) => void;
  onExport: (path: string) => void;
}

export function FileQueueItem({ item, locked, exporting, defaultOpen, onRemove, onExport }: FileQueueItemProps) {
  const t = useT();
  const [open, setOpen] = useState(defaultOpen);
  const { copied, copy } = useCopyFeedback(t);
  const now = useNow(item.status === "running");
  const name = fileName(item.path);
  const { result } = item;
  const elapsed = item.startedAt === null ? null : formatElapsed((item.finishedAt ?? now) - item.startedAt);

  return (
    <li className="flex flex-col gap-2 rounded-lg bg-vx-bg-secondary px-3.5 py-2.5">
      <div className="flex items-center gap-3">
        <FileAudio className="h-4 w-4 shrink-0 text-vx-text-dim" />
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-sm text-vx-text-primary" title={item.path}>{name}</span>
          <span className={`text-xs ${item.status === "error" ? "text-vx-error" : "text-vx-text-dim"}`}>
            {statusLabel(t, item)}
          </span>
        </div>
        {elapsed && (
          <span className="font-mono text-xs tabular-nums text-vx-text-secondary" title={t("file.elapsed")}>
            {elapsed}
          </span>
        )}
        {result && (
          <>
            <Button variant="ghost" size="sm" onClick={() => onExport(item.path)} disabled={exporting} aria-label={t("file.export_one", { name })}>
              <Download className="h-3.5 w-3.5" />
            </Button>
            <Button variant="ghost" size="sm" onClick={() => setOpen(!open)} aria-expanded={open} aria-label={t(open ? "file.hide_result" : "file.show_result", { name })}>
              <ChevronDown className={`h-3.5 w-3.5 transition-transform ${open ? "rotate-180" : ""}`} />
            </Button>
          </>
        )}
        {!locked && (
          <Button variant="ghost" size="sm" onClick={() => onRemove(item.path)} aria-label={t("file.remove", { name })}>
            <X className="h-3.5 w-3.5" />
          </Button>
        )}
      </div>

      {item.error && <p role="alert" className="text-xs text-vx-error break-words">{item.error}</p>}
      {result?.stt_issue && <IssueNote text={issueText(t, "file.partial_stt", result.stt_issue)} />}
      {result?.llm_issue && <IssueNote text={issueText(t, "file.partial_llm", result.llm_issue)} />}

      {result && open && (
        <div className="flex flex-col gap-2">
          <div className="flex justify-end">
            <Button variant="ghost" size="sm" onClick={() => void copy(result.text)}>
              <Copy className="h-3.5 w-3.5" /> {copied ? t("file.copied") : t("file.copy")}
            </Button>
          </div>
          <textarea
            readOnly
            aria-label={`${t("file.result_label")}: ${name}`}
            value={result.text}
            className="min-h-48 resize-y rounded-lg bg-vx-bg-tertiary px-3.5 py-2.5 text-sm leading-relaxed text-vx-text-primary focus:outline-none focus:ring-2 focus:ring-vx-accent/40"
          />
        </div>
      )}
    </li>
  );
}
