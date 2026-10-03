import { useEffect } from "react";
import { Download, FileAudio, FolderOpen, Play, Square } from "lucide-react";
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
import { FileQueueItem, IssueNote } from "./FileQueueItem";
import type { LlmEngineId, Settings, SttEngineId, SttLanguageId, TranscriptExportFormat } from "../../types/app";

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

const pickOption = <T extends string>(allowed: readonly T[], value: string, fallback: T): T =>
  allowed.find((v) => v === value) ?? fallback;

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

export function FileTranscriptionPanel() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const store = useFileTranscriptionStore();
  const { items, options, picking, status, cancelled, exportFormat, exporting, initOptions } = store;

  useEffect(() => {
    initOptions(defaultsFrom(settings));
  }, [initOptions, settings]);

  if (!options) return null;

  const busy = status !== "idle";
  const llmEnabled = options.llm_engine !== null;
  const setupKey = missingSetupKey(settings, options.stt_engine, options.llm_engine);
  const pendingCount = items.filter((item) => item.status !== "done").length;
  const finishedPaths = items.filter((item) => item.result).map((item) => item.path);
  const languageOptions = LANGUAGES.map((value) => ({
    value,
    label: value === "auto" ? t("settings.stt.lang_auto") : value === "id" ? "Bahasa Indonesia" : "English",
  }));

  const handleExport = (paths: string[]) =>
    void invokeAction(async () => {
      const written = await store.exportFiles(paths);
      if (written) toast(t("file.exported", { count: written.length }), "success");
    });

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col">
      <PanelHeader title={t("file.title")} subtitle={t("file.subtitle")} icon={<FileAudio className="h-4.5 w-4.5" />} />

      <div className="flex flex-col gap-5 px-10 pb-8">
        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-3">
            <Button onClick={() => void invokeAction(store.pick)} disabled={busy || picking}>
              <FolderOpen className="h-4 w-4" /> {picking ? t("file.picking") : t("file.pick")}
            </Button>
            <span className="text-sm text-vx-text-secondary">
              {items.length > 0 ? t("file.selected_count", { count: items.length }) : t("file.none_selected")}
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
            <Button variant="primary" onClick={() => void store.start()} disabled={pendingCount === 0 || picking || setupKey !== null}>
              <Play className="h-4 w-4" /> {t("file.start")}
            </Button>
          )}
          <span role="status" className="text-sm text-vx-text-secondary">
            {!busy && cancelled && t("file.cancelled")}
          </span>
        </div>

        {items.length > 0 && (
          <div className="flex flex-col gap-3">
            <div className="flex items-end justify-between gap-3">
              <div className="w-48">
                <Select
                  label={t("file.export_format")}
                  options={EXPORT_OPTIONS}
                  value={exportFormat}
                  onChange={(e) => store.setExportFormat(pickOption(EXPORT_OPTIONS.map((o) => o.value), e.target.value, "txt"))}
                />
              </div>
              <Button onClick={() => handleExport(finishedPaths)} disabled={exporting || finishedPaths.length === 0}>
                <Download className="h-4 w-4" /> {t("file.export_all", { count: finishedPaths.length })}
              </Button>
            </div>
            <ul aria-label={t("file.queue_label")} className="flex flex-col gap-2">
              {items.map((item) => (
                <FileQueueItem
                  key={item.path}
                  item={item}
                  locked={busy}
                  exporting={exporting}
                  defaultOpen={items.length === 1}
                  onRemove={store.remove}
                  onExport={(path) => handleExport([path])}
                />
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}
