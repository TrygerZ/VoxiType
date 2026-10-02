import { useState } from "react";
import { Copy, FileAudio, FolderOpen, Play, Square } from "lucide-react";
import { useFileTranscriptionStore } from "../../stores/fileTranscriptionStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { getStringSetting } from "../../lib/settingsGuards";
import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import { PanelHeader } from "../common/PanelHeader";
import { Button } from "../ui/Button";
import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";
import type { LlmEngineId, SttEngineId } from "../../types/app";
import type { FileTranscriptionProgressEvent } from "../../types/events";

const STT_OPTIONS: { value: SttEngineId; label: string }[] = [
  { value: "groq", label: "Groq Whisper" },
  { value: "whisper_cpp", label: "Offline whisper.cpp" },
];

const LLM_OPTIONS: { value: LlmEngineId; label: string }[] = [
  { value: "ollama", label: "Ollama (Local)" },
  { value: "groq", label: "Groq Llama 3.1 (Cloud)" },
  { value: "rule_based", label: "Rule-based (No LLM)" },
];

function isSttEngine(value: string): value is SttEngineId {
  return STT_OPTIONS.some((o) => o.value === value);
}

function isLlmEngine(value: string): value is LlmEngineId {
  return LLM_OPTIONS.some((o) => o.value === value);
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function progressLabel(
  t: ReturnType<typeof useT>,
  progress: FileTranscriptionProgressEvent | null,
): string {
  if (!progress) return t("file.status_starting");
  const key = progress.stage === "transcribing" ? "file.status_transcribing" : "file.status_formatting";
  const total = progress.total > 0 ? String(progress.total) : "?";
  return t(key, { done: progress.done, total });
}

export function FileTranscriptionPanel() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const { path, status, progress, result, error, pick, start, cancel } =
    useFileTranscriptionStore();

  const defaultStt = getStringSetting(settings.stt_engine, "groq");
  const defaultLlm = getStringSetting(settings.llm_engine, "ollama");
  const [sttEngine, setSttEngine] = useState<SttEngineId>(
    isSttEngine(defaultStt) ? defaultStt : "groq",
  );
  const [llmEnabled, setLlmEnabled] = useState(false);
  const [llmEngine, setLlmEngine] = useState<LlmEngineId>(
    isLlmEngine(defaultLlm) ? defaultLlm : "ollama",
  );
  const [applyDictionary, setApplyDictionary] = useState(true);
  const [copied, setCopied] = useState(false);

  const running = status === "running";

  const handleStart = () =>
    void start({
      stt_engine: sttEngine,
      llm_engine: llmEnabled ? llmEngine : null,
      apply_dictionary: applyDictionary,
    });

  const handleCopy = (text: string) => {
    void navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  };

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col">
      <PanelHeader
        title={t("file.title")}
        subtitle={t("file.subtitle")}
        icon={<FileAudio className="h-4.5 w-4.5" />}
      />

      <div className="flex flex-col gap-5 px-10 pb-8">
        <div className="flex items-center gap-3">
          <Button onClick={() => void invokeAction(pick)} disabled={running}>
            <FolderOpen className="h-4 w-4" /> {t("file.pick")}
          </Button>
          <span className="min-w-0 truncate text-sm text-vx-text-secondary" title={path ?? undefined}>
            {path ? fileName(path) : t("file.none_selected")}
          </span>
        </div>
        <p className="text-xs text-vx-text-dim">{t("file.limits")}</p>

        <div className="grid grid-cols-2 gap-4">
          <Select
            label={t("file.stt_engine")}
            options={STT_OPTIONS}
            value={sttEngine}
            disabled={running}
            onChange={(e) => {
              if (isSttEngine(e.target.value)) setSttEngine(e.target.value);
            }}
          />
          <Select
            label={t("file.llm_engine")}
            options={LLM_OPTIONS}
            value={llmEngine}
            disabled={running || !llmEnabled}
            onChange={(e) => {
              if (isLlmEngine(e.target.value)) setLlmEngine(e.target.value);
            }}
          />
        </div>

        <div className="flex flex-col gap-3">
          <Switch
            checked={llmEnabled}
            onChange={setLlmEnabled}
            disabled={running}
            label={t("file.llm_toggle")}
          />
          <Switch
            checked={applyDictionary}
            onChange={setApplyDictionary}
            disabled={running}
            label={t("file.dictionary_toggle")}
          />
        </div>

        <div className="flex items-center gap-3">
          {running ? (
            <Button variant="danger" onClick={() => void invokeAction(cancel)}>
              <Square className="h-4 w-4" /> {t("file.cancel")}
            </Button>
          ) : (
            <Button variant="primary" onClick={handleStart} disabled={!path}>
              <Play className="h-4 w-4" /> {t("file.start")}
            </Button>
          )}
          {running && (
            <span role="status" className="text-sm text-vx-text-secondary">
              {progressLabel(t, progress)}
            </span>
          )}
        </div>

        {status === "error" && error && (
          <p role="alert" className="text-sm text-vx-error break-words">
            {error}
          </p>
        )}

        {status === "done" && result && (
          <div className="flex flex-col gap-2">
            <div className="flex items-center justify-between text-xs text-vx-text-dim">
              <span>{t("file.result_meta", { count: result.word_count })}</span>
              <Button variant="ghost" size="sm" onClick={() => handleCopy(result.text)}>
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
