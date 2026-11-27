import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Upload } from "lucide-react";

import { invokeAction } from "../../lib/invokeAction";
import { type Translate, useT } from "../../lib/i18n";
import { getStringSetting } from "../../lib/settingsGuards";
import {
  type FileItem,
  useFileTranscriptionStore,
} from "../../stores/fileTranscriptionStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type {
  LlmEngineId,
  Settings,
  SttEngineId,
  SttLanguageId,
  TranscriptExportFormat,
} from "../../types/app";
import { Button } from "../ui/Button";
import { Select } from "../ui/Select";
import { StatusBadge } from "../ui/StatusBadge";
import { Switch } from "../ui/Switch";
import { toast } from "../ui/Toast";
import { FileQueueItem, IssueNote } from "./FileQueueItem";

const MAX_BATCH_FILES = 20;

const STT_OPTIONS: { value: SttEngineId; label: string }[] = [
  { value: "groq", label: "Groq Whisper" },
  { value: "whisper_cpp", label: "Offline whisper.cpp" },
];

const LLM_OPTIONS: { value: LlmEngineId; label: string }[] = [
  { value: "ollama", label: "Ollama (Local)" },
  { value: "groq", label: "Groq Llama 3.1 (Cloud)" },
  { value: "rule_based", label: "Rule-based (No LLM)" },
];

const EXPORT_OPTIONS: { value: TranscriptExportFormat; label: string }[] = [
  { value: "txt", label: "TXT" },
  { value: "docx", label: "Word (DOCX)" },
  { value: "pdf", label: "PDF" },
];

const LANGUAGES: SttLanguageId[] = ["auto", "id", "en"];

const pickOption = <T extends string>(
  allowed: readonly T[],
  value: string,
  fallback: T,
): T => allowed.find((v) => v === value) ?? fallback;

function defaultsFrom(settings: Settings) {
  return {
    stt_engine: pickOption(
      STT_OPTIONS.map((o) => o.value),
      getStringSetting(settings.stt_engine, "groq"),
      "groq",
    ),
    language: pickOption(
      LANGUAGES,
      getStringSetting(settings.stt_language, "auto"),
      "auto",
    ),
    llm_engine: null,
    apply_dictionary: true,
  };
}

/** Setup the chosen engines still need; checked before starting a long job. */
function missingSetupKey(
  settings: Settings,
  stt: SttEngineId,
  llm: LlmEngineId | null,
): string | null {
  const groqKeySet = settings.groq_api_key_set === true;
  if ((stt === "groq" || llm === "groq") && !groqKeySet) {
    return "file.needs_groq_key";
  }
  if (
    stt === "whisper_cpp" &&
    !getStringSetting(settings.whisper_cpp_model_path, "")
  ) {
    return "file.needs_whisper";
  }
  return null;
}

function hasPartialIssue(item: FileItem): boolean {
  return Boolean(item.result?.stt_issue || item.result?.llm_issue);
}

function QueueHeaderBadge({
  t,
  items,
  busy,
}: {
  t: Translate;
  items: FileItem[];
  busy: boolean;
}) {
  const runningIndex = items.findIndex((item) => item.status === "running");
  const warningOrErrorCount = items.filter(
    (item) => item.status === "error" || hasPartialIssue(item),
  ).length;
  const doneCount = items.filter(
    (item) => item.status === "done" && !hasPartialIssue(item),
  ).length;

  if (busy) {
    return (
      <StatusBadge
        tone="ember"
        label={t("file.badge_running", {
          done: Math.max(1, runningIndex + 1),
          total: items.length,
        })}
      />
    );
  }
  if (warningOrErrorCount > 0 && doneCount > 0) {
    return (
      <StatusBadge
        tone="neutral"
        label={t("file.badge_completed_warning", {
          completed: doneCount,
          warnings: warningOrErrorCount,
        })}
      />
    );
  }
  return (
    <StatusBadge
      tone="neutral"
      label={t("file.badge_count", {
        count: items.length,
        max: MAX_BATCH_FILES,
      })}
    />
  );
}

export function FileTranscriptionPanel() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const store = useFileTranscriptionStore();
  const {
    items,
    options,
    picking,
    status,
    cancelled,
    exportFormat,
    exporting,
    initOptions,
  } = store;
  const [isDragging, setIsDragging] = useState(false);

  useEffect(() => {
    initOptions(defaultsFrom(settings));
  }, [initOptions, settings]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    try {
      void getCurrentWebview()
        .onDragDropEvent((event) => {
          if (!active) return;
          const { type } = event.payload;
          if (type === "enter" || type === "over") {
            setIsDragging(true);
          } else if (type === "leave") {
            setIsDragging(false);
          } else if (type === "drop") {
            setIsDragging(false);
            const paths = event.payload.paths;
            const current = useFileTranscriptionStore.getState();
            if (current.status === "idle" && paths.length > 0) {
              void current.addDroppedPaths(paths);
            }
          }
        })
        .then((fn) => {
          if (!active) {
            fn();
          } else {
            unlisten = fn;
          }
        })
        .catch(() => {});
    } catch {
      // Non-Tauri browser/test runtimes without webview internals.
    }

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  if (!options) return null;

  const busy = status !== "idle";
  const llmEnabled = options.llm_engine !== null;
  const setupKey = missingSetupKey(
    settings,
    options.stt_engine,
    options.llm_engine,
  );
  const pendingCount = items.filter((item) => item.status !== "done").length;
  const finishedPaths = items
    .filter((item) => item.result)
    .map((item) => item.path);
  const languageOptions = LANGUAGES.map((value) => ({
    value,
    label:
      value === "auto"
        ? t("settings.stt.lang_auto")
        : value === "id"
          ? "Bahasa Indonesia"
          : "English",
  }));

  const handleExport = (paths: string[]) =>
    void invokeAction(async () => {
      const written = await store.exportFiles(paths);
      if (written && written.length > 0) {
        const dir = written[0].replace(/[\\/][^\\/]+$/, "");
        toast(
          t("file.exported", { count: written.length }),
          "success",
          undefined,
          dir || undefined,
        );
      }
    });

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-4 px-4 sm:px-6 lg:px-8 pt-5 sm:pt-7 pb-24 vx-panel-reveal">
      <header className="flex flex-wrap items-start justify-between gap-3 sm:gap-4">
        <div className="flex flex-col gap-0.5">
          <h1 className="text-[22px] font-semibold leading-7 tracking-[-0.02em] text-vx-text-primary">
            {t("file.title")}
          </h1>
          <p className="text-[13px] leading-[18px] text-vx-text-secondary">
            {t("file.subtitle_v2")}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2 shrink-0">
          <QueueHeaderBadge t={t} items={items} busy={busy} />
          {items.length > 0 && (
            <>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => void invokeAction(store.pick)}
                disabled={busy || picking || items.length >= MAX_BATCH_FILES}
              >
                {picking ? t("file.picking") : t("file.add_files")}
              </Button>
              {!busy && (
                <Button variant="ghost" size="sm" onClick={store.clear}>
                  {t("file.clear")}
                </Button>
              )}
            </>
          )}
        </div>
      </header>

      <div className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] px-3.5 py-2.5 flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-2.5">
          <Select
            aria-label={t("file.stt_engine")}
            options={STT_OPTIONS}
            value={options.stt_engine}
            disabled={busy}
            className="w-auto py-1.5 text-xs"
            onChange={(e) =>
              store.setOptions({
                stt_engine: pickOption(
                  STT_OPTIONS.map((o) => o.value),
                  e.target.value,
                  "groq",
                ),
              })
            }
          />
          <Select
            aria-label={t("file.language")}
            options={languageOptions}
            value={options.language}
            disabled={busy}
            className="w-auto py-1.5 text-xs"
            onChange={(e) =>
              store.setOptions({
                language: pickOption(LANGUAGES, e.target.value, "auto"),
              })
            }
          />
          <Switch
            checked={llmEnabled}
            label={t("file.llm_short")}
            ariaLabel={t("file.llm_toggle")}
            disabled={busy}
            onChange={(on) =>
              store.setOptions({
                llm_engine: on
                  ? pickOption(
                      LLM_OPTIONS.map((o) => o.value),
                      getStringSetting(settings.llm_engine, "ollama"),
                      "ollama",
                    )
                  : null,
              })
            }
          />
          {llmEnabled && options.llm_engine && (
            <Select
              aria-label={t("file.llm_engine")}
              options={LLM_OPTIONS}
              value={options.llm_engine}
              disabled={busy}
              className="w-auto py-1.5 text-xs"
              onChange={(e) =>
                store.setOptions({
                  llm_engine: pickOption(
                    LLM_OPTIONS.map((o) => o.value),
                    e.target.value,
                    "ollama",
                  ),
                })
              }
            />
          )}
          <Switch
            checked={options.apply_dictionary}
            label={t("file.dictionary_short")}
            ariaLabel={t("file.dictionary_toggle")}
            disabled={busy}
            onChange={(on) => store.setOptions({ apply_dictionary: on })}
          />
        </div>

        <div className="shrink-0">
          {busy ? (
            <Button
              variant="danger"
              size="sm"
              onClick={() => void invokeAction(store.cancel)}
              disabled={status === "cancelling"}
            >
              {status === "cancelling"
                ? t("file.cancelling")
                : t("file.cancel_batch")}
            </Button>
          ) : (
            <Button
              variant={pendingCount > 0 ? "primary" : "secondary"}
              size="sm"
              onClick={() => void store.start()}
              disabled={pendingCount === 0 || picking || setupKey !== null}
            >
              {pendingCount > 0
                ? t("file.start_count", { count: pendingCount })
                : t("file.start")}
            </Button>
          )}
        </div>
      </div>

      {setupKey && <IssueNote text={t(setupKey)} />}

      {!busy && cancelled && (
        <p role="status" className="text-xs text-vx-text-secondary">
          {t("file.cancelled")}
        </p>
      )}

      {items.length === 0 ? (
        <div
          data-testid="file-dropzone"
          className={`bg-vx-bg-surface border border-dashed rounded-[12px] py-8 sm:py-12 px-4 sm:px-6 flex flex-col items-center justify-center gap-3 text-center transition-colors ${
            isDragging
              ? "border-vx-accent-ember bg-vx-accent-ember-soft/40"
              : "border-vx-border-default"
          }`}
        >
          <Upload className="h-7 w-7 text-vx-text-muted" />
          <div className="flex flex-col gap-1">
            <p className="text-sm font-medium text-vx-text-primary">
              {t("file.drop_empty_title")}
            </p>
            <p className="text-xs text-vx-text-muted">
              {t("file.limits_compact")}
            </p>
          </div>
          <Button
            variant="primary"
            size="md"
            onClick={() => void invokeAction(store.pick)}
            disabled={busy || picking}
          >
            {picking ? t("file.picking") : t("file.choose_btn")}
          </Button>
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          <ul aria-label={t("file.queue_label")} className="flex flex-col gap-2">
            {items.map((item) => (
              <FileQueueItem
                key={item.path}
                item={item}
                locked={busy}
                exporting={exporting}
                defaultOpen={items.length === 1}
                onRemove={store.remove}
                onRetry={(path) => void store.retry(path)}
                onExport={(path) => handleExport([path])}
              />
            ))}
          </ul>

          {!busy && items.length < MAX_BATCH_FILES && (
            <div
              data-testid="file-dropzone"
              className={`border border-dashed rounded-[8px] px-4 py-3 text-center text-xs transition-colors ${
                isDragging
                  ? "border-vx-accent-ember bg-vx-accent-ember-soft/40 text-vx-text-primary"
                  : "border-vx-border-default text-vx-text-muted"
              }`}
            >
              {t("file.drop_more_hint")}
            </div>
          )}

          {finishedPaths.length > 0 && (
            <div className="bg-vx-bg-surface border border-vx-border-subtle rounded-[8px] px-3.5 py-2.5 flex flex-wrap items-center justify-between gap-3">
              <span className="text-xs font-medium text-vx-text-secondary">
                {t("file.batch_export_format")}
              </span>
              <div className="flex items-center gap-2.5">
                <Select
                  aria-label={t("file.export_format")}
                  options={EXPORT_OPTIONS}
                  value={exportFormat}
                  className="w-auto py-1.5 text-xs"
                  onChange={(e) =>
                    store.setExportFormat(
                      pickOption(
                        EXPORT_OPTIONS.map((o) => o.value),
                        e.target.value,
                        "txt",
                      ),
                    )
                  }
                />
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => handleExport(finishedPaths)}
                  disabled={exporting}
                >
                  {t("file.export_all", { count: finishedPaths.length })}
                </Button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
