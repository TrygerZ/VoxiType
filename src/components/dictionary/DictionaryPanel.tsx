import { useEffect, useState } from "react";
import {
  Plus,
  Trash2,
  Download,
  Upload,
  ToggleLeft,
  ToggleRight,
  BookOpen,
} from "lucide-react";
import { useDictionaryStore } from "../../stores/dictionaryStore";
import { useT } from "../../lib/i18n";
import { Button } from "../ui/Button";
import { PanelHeader } from "../common/PanelHeader";
import type { DictionaryEntry } from "../../types/app";
import {
  exportDictionary,
  importDictionary,
  setDictionaryActive,
} from "../../lib/tauri";
import { invokeAction } from "../../lib/invokeAction";
import { toast } from "../ui/Toast";

export function DictionaryPanel() {
  const t = useT();
  const entries = useDictionaryStore((s) => s.entries);
  const loading = useDictionaryStore((s) => s.loading);
  const error = useDictionaryStore((s) => s.error);
  const load = useDictionaryStore((s) => s.load);
  const add = useDictionaryStore((s) => s.add);
  const remove = useDictionaryStore((s) => s.remove);

  const [word, setWord] = useState("");
  const [replacement, setReplacement] = useState("");
  const [busy, setBusy] = useState(false);
  const [togglingId, setTogglingId] = useState<string | null>(null);

  useEffect(() => {
    void load();
  }, [load]);

  const handleAdd = async () => {
    if (busy || !word.trim()) return;
    setBusy(true);
    try {
      const entry: DictionaryEntry = {
        id: "",
        word: word.trim(),
        pronunciation: null,
        category: "custom",
        replacement: replacement.trim() || null,
        language: "id",
        usage_count: 0,
        is_active: true,
      };
      const ok = await invokeAction(() => add(entry));
      if (ok) {
        setWord("");
        setReplacement("");
      }
    } finally {
      setBusy(false);
    }
  };

  const handleToggle = (id: string, current: boolean) => {
    if (togglingId === id) return;
    setTogglingId(id);
    void invokeAction(async () => {
      await setDictionaryActive(id, !current);
      await load();
    }).finally(() => {
      setTogglingId(null);
    });
  };

  const handleExport = async () => {
    await invokeAction(async () => {
      const data = await exportDictionary();
      const blob = new Blob([data], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "voxitype-dictionary.json";
      a.click();
      URL.revokeObjectURL(url);
    });
  };

  const handleImport = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      
      // Validate file size (5MB limit)
      if (file.size > 5 * 1024 * 1024) {
        toast(t("dictionary.file_too_large"), "error");
        return;
      }
      
      // Validate file extension
      if (!file.name.endsWith(".json")) {
        toast(t("dictionary.json_only"), "error");
        return;
      }
      
      await invokeAction(async () => {
        const text = await file.text();
        await importDictionary(text);
        await load();
      });
    };
    input.click();
  };

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col">
      <PanelHeader
        title={t("dictionary.title")}
        subtitle={t("dictionary.subtitle")}
        icon={<BookOpen className="h-4.5 w-4.5" />}
        actions={
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => void handleExport()}
              title={t("dictionary.export_tooltip")}
              aria-label={t("dictionary.export_tooltip")}
            >
              <Download className="h-3.5 w-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleImport}
              title={t("dictionary.import_tooltip")}
              aria-label={t("dictionary.import_tooltip")}
            >
              <Upload className="h-3.5 w-3.5" />
            </Button>
          </>
        }
      />

      <div className="flex gap-2 px-10 pb-4">
        <input
          className="flex-1 rounded-lg bg-vx-bg-tertiary px-3.5 py-2.5 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-shadow focus:outline-none focus:ring-2 focus:ring-vx-accent/40"
          placeholder={t("dictionary.placeholder_word")}
          value={word}
          onChange={(e) => setWord(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !busy) void handleAdd();
          }}
        />
        <input
          className="w-44 rounded-lg bg-vx-bg-tertiary px-3.5 py-2.5 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-shadow focus:outline-none focus:ring-2 focus:ring-vx-accent/40"
          placeholder={t("dictionary.placeholder_rep")}
          value={replacement}
          onChange={(e) => setReplacement(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !busy) void handleAdd();
          }}
        />
        <Button
          variant="primary"
          size="sm"
          onClick={() => void handleAdd()}
          disabled={busy}
        >
          <Plus className="h-4 w-4" /> {t("dictionary.add_btn")}
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto px-10 pb-8">
        {loading && <p className="text-sm text-vx-text-dim">{t("dictionary.loading")}</p>}

        {error && !loading && (
          <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
            <div className="max-w-md space-y-1">
              <p className="text-sm font-semibold text-vx-text-primary">
                {t("error.dictionary_load_failed")}
              </p>
              <p className="text-xs font-mono text-vx-text-dim break-all">{error}</p>
            </div>
            <Button size="sm" onClick={() => void load()}>
              {t("error.retry")}
            </Button>
          </div>
        )}

        {entries.length === 0 && !loading && !error && (
          <div className="flex flex-col items-center justify-center gap-2 py-20 text-center">
            <BookOpen className="h-10 w-10 text-vx-text-dim/40" />
            <p className="text-sm text-vx-text-dim">{t("dictionary.empty")}</p>
          </div>
        )}

        {!error && (
          <div className="flex flex-col divide-y divide-vx-divider">
            {entries.map((e) => (
              <div
                key={e.id}
                className={`group flex items-center justify-between py-3 transition-opacity ${
                  !e.is_active ? "opacity-50" : ""
                }`}
              >
                <div className="min-w-0">
                  <span className="text-sm font-medium text-vx-text-primary">
                    {e.word}
                  </span>
                  {e.replacement && (
                    <span className="ml-2 text-xs text-vx-text-dim">
                      &rarr; {e.replacement}
                    </span>
                  )}
                </div>
                <div className="flex gap-1.5 opacity-0 transition-opacity duration-200 group-hover:opacity-100 group-focus-within:opacity-100">
                  <button
                    type="button"
                    disabled={togglingId === e.id}
                    onClick={() => void handleToggle(e.id, e.is_active)}
                    className="rounded-lg p-1.5 text-vx-text-dim transition-colors hover:bg-vx-bg-tertiary disabled:cursor-not-allowed disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent/40 focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-primary"
                    title={e.is_active ? t("dictionary.deactivate_tooltip") : t("dictionary.activate_tooltip")}
                    aria-label={e.is_active ? t("dictionary.deactivate_word", { word: e.word }) : t("dictionary.activate_word", { word: e.word })}
                  >
                    {e.is_active ? (
                      <ToggleRight className="h-4.5 w-4.5 text-vx-success" />
                    ) : (
                      <ToggleLeft className="h-4.5 w-4.5" />
                    )}
                  </button>
                  <button
                    type="button"
                    onClick={() => void invokeAction(() => remove(e.id))}
                    className="rounded-lg p-1.5 text-vx-text-dim transition-colors hover:bg-vx-error/15 hover:text-vx-error focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent/40 focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-primary"
                    title={t("dictionary.delete_tooltip")}
                    aria-label={t("dictionary.delete_word", { word: e.word })}
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
