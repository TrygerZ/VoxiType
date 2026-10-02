import { useEffect, useRef, useState } from "react";
import { AlertTriangle, Copy, FileAudio, FolderOpen, Play, Square } from "lucide-react";
import { useFileTranscriptionStore } from "../../stores/fileTranscriptionStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { getStringSetting } from "../../lib/settingsGuards";
import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import { toast } from "../ui/Toast";
import { PanelHeader } from "../common/PanelHeader";
import { Button } from "../ui/Button";
import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";
import type { LlmEngineId, Settings, StageIssue, SttEngineId, SttLanguageId } from "../../types/app";
import type { FileTranscriptionProgressEvent } from "../../types/events";

const COPY_FEEDBACK_MS = 1200;

const STT_OPTIONS: { value: SttEngineId; label: string }[] = [
  { value: "groq", label: "Groq Whisper" },
  { value: "whisper_cpp", label: "Offline whisper.cpp" },
];

const LLM_OPTIONS: { value: LlmEngineId; label: string }[] = [
  { value: "ollama", label: "Ollama (Local)" },
  { value: "groq", label: "Groq Llama 3.1 (Cloud)" },
  { value: "rule_based", label: "Rule-based (No LLM)" },
];

const LANGUAGES: SttLanguageId[] = ["auto", "id", "en"];

const pickOption = <T extends string>(allowed: readonly T[], value: string, fallback: T): T =>
  allowed.find((v) => v === value) ?? fallback;

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function defaultsFrom(settings: Settings) {
  return {
    stt_engine: pickOption(STT_OPTIONS.map((o) => o.value), getStringSetting(settings.stt_engine, "groq"), "groq"),
    language: pickOption(LANGUAGES, getStringSetting(settings.stt_language, "auto"), "auto"),
    llm_engine: null,
    apply_dictionary: true,
  };
}

/** Setup the chosen engines still need; checked before starting a long job. */
function missingSetupKey(settings: Settings, stt: SttEngineId, llm: LlmEngineId | null): string | null {
  const groqKeySet = settings.groq_api_key_set === true;
  if ((stt === "groq" || llm === "groq") && !groqKeySet) return "file.needs_groq_key";
  if (stt === "whisper_cpp" && !getStringSetting(settings.whisper_cpp_model_path, "")) {
    return "file.needs_whisper";
  }
  return null;
}

function progressLabel(t: ReturnType<typeof useT>, progress: FileTranscriptionProgressEvent | null) {
  if (!progress) return t("file.status_starting");
  const key = progress.stage === "transcribing" ? "file.status_transcribing" : "file.status_formatting";
  return t(key, { done: progress.done, total: progress.total > 0 ? progress.total : "?" });
}

function IssueNote({ text }: { text: string }) {
  return (
    <p role="alert" className="flex items-start gap-2 text-xs text-vx-warning">
      <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" /> <span className="break-words">{text}</span>
    </p>
  );
}

function issueText(t: ReturnType<typeof useT>, key: string, issue: StageIssue) {
  return t(key, { count: issue.count, total: issue.total, reason: issue.reason });
}

export function FileTranscriptionPanel() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const store = useFileTranscriptionStore();
  const { path, options, picking, status, progress, result, error, cancelled, initOptions } = store;
  const [copied, setCopied] = useState(false);
  const copyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    initOptions(defaultsFrom(settings));
  }, [initOptions, settings]);

  useEffect(() => () => {
    if (copyTimer.current) clearTimeout(copyTimer.current);
  }, []);

  if (!options) return null;

  const busy = status === "running" || status === "cancelling";
  const llmEnabled = options.llm_engine !== null;
  const setupKey = missingSetupKey(settings, options.stt_engine, options.llm_engine);
  const languageOptions = LANGUAGES.map((value) => ({
    value,
    label: value === "auto" ? t("settings.stt.lang_auto") : value === "id" ? "Bahasa Indonesia" : "English",
  }));

  const handleCopy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      toast(t("file.copy_failed"), "error");
      return;
    }
    setCopied(true);
    if (copyTimer.current) clearTimeout(copyTimer.current);
    copyTimer.current = setTimeout(() => setCopied(false), COPY_FEEDBACK_MS);
  };

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col">
      <PanelHeader title={t("file.title")} subtitle={t("file.subtitle")} icon={<FileAudio className="h-4.5 w-4.5" />} />

      <div className="flex flex-col gap-5 px-10 pb-8">
        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-3">
            <Button onClick={() => void invokeAction(store.pick)} disabled={busy || picking}>
              <FolderOpen className="h-4 w-4" /> {picking ? t("file.picking") : t("file.pick")}
            </Button>
            <span className="min-w-0 truncate text-sm text-vx-text-secondary" title={path ?? undefined}>
              {path ? fileName(path) : t("file.none_selected")}
            </span>
          </div>
          <p className="text-xs text-vx-text-dim">{t("file.limits")}</p>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <Select
            label={t("file.stt_engine")}
            options={STT_OPTIONS}
            value={options.stt_engine}
            disabled={busy}
            onChange={(e) => store.setOptions({ stt_engine: pickOption(STT_OPTIONS.map((o) => o.value), e.target.value, "groq") })}
          />
          <Select
            label={t("file.language")}
            options={languageOptions}
            value={options.language}
            disabled={busy}
            onChange={(e) => store.setOptions({ language: pickOption(LANGUAGES, e.target.value, "auto") })}
          />
        </div>

        <div className="flex flex-col gap-3">
          <Switch
            checked={llmEnabled}
            onChange={(on) => store.setOptions({ llm_engine: on ? pickOption(LLM_OPTIONS.map((o) => o.value), getStringSetting(settings.llm_engine, "ollama"), "ollama") : null })}
            disabled={busy}
            label={t("file.llm_toggle")}
          />
          {llmEnabled && options.llm_engine && (
            <div className="ml-12 max-w-xs">
              <Select
                label={t("file.llm_engine")}
                options={LLM_OPTIONS}
                value={options.llm_engine}
                disabled={busy}
                onChange={(e) => store.setOptions({ llm_engine: pickOption(LLM_OPTIONS.map((o) => o.value), e.target.value, "ollama") })}
              />
            </div>
          )}
          <Switch
            checked={options.apply_dictionary}
            onChange={(on) => store.setOptions({ apply_dictionary: on })}
            disabled={busy}
            label={t("file.dictionary_toggle")}
          />
        </div>

        {setupKey && <IssueNote text={t(setupKey)} />}

        <div className="flex items-center gap-3">
          {busy ? (
            <Button variant="danger" onClick={() => void invokeAction(store.cancel)} disabled={status === "cancelling"}>
              <Square className="h-4 w-4" /> {status === "cancelling" ? t("file.cancelling") : t("file.cancel")}
            </Button>
          ) : (
            <Button variant="primary" onClick={() => void store.start()} disabled={!path || picking || setupKey !== null}>
              <Play className="h-4 w-4" /> {t("file.start")}
            </Button>
          )}
          <span role="status" className="text-sm text-vx-text-secondary">
            {busy && progressLabel(t, progress)}
            {!busy && cancelled && t("file.cancelled")}
          </span>
        </div>

        {status === "error" && error && (
          <p role="alert" className="text-sm text-vx-error break-words">{error}</p>
        )}

        {status === "done" && result && (
          <div className="flex flex-col gap-2">
            {result.stt_issue && <IssueNote text={issueText(t, "file.partial_stt", result.stt_issue)} />}
            {result.llm_issue && <IssueNote text={issueText(t, "file.partial_llm", result.llm_issue)} />}
            <div className="flex items-center justify-between text-xs text-vx-text-dim">
              <span>{t("file.result_meta", { count: result.word_count })}</span>
              <Button variant="ghost" size="sm" onClick={() => void handleCopy(result.text)}>
                <Copy className="h-3.5 w-3.5" /> {copied ? t("file.copied") : t("file.copy")}
              </Button>
            </div>
            <textarea
              readOnly
              aria-label={t("file.result_label")}
              value={result.text}
              className="min-h-64 resize-y rounded-lg bg-vx-bg-tertiary px-3.5 py-2.5 text-sm leading-relaxed text-vx-text-primary focus:outline-none focus:ring-2 focus:ring-vx-accent/40"
            />
          </div>
        )}
      </div>
    </div>
  );
}
