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
import { Button } from "../ui/Button";
import { PanelHeader } from "../common/PanelHeader";
import type { DictionaryEntry } from "../../types/app";
import {
  exportDictionary,
  formatTauriError,
  importDictionary,
  setDictionaryActive,
} from "../../lib/tauri";
import { toast } from "../ui/Toast";

export function DictionaryPanel() {
  const entries = useDictionaryStore((s) => s.entries);
  const loading = useDictionaryStore((s) => s.loading);
  const load = useDictionaryStore((s) => s.load);
  const add = useDictionaryStore((s) => s.add);
  const remove = useDictionaryStore((s) => s.remove);

  const [word, setWord] = useState("");
  const [replacement, setReplacement] = useState("");

  useEffect(() => {
    void load();
  }, [load]);

  const handleAdd = async () => {
    if (!word.trim()) return;
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
    try {
      await add(entry);
      setWord("");
      setReplacement("");
    } catch (e: unknown) {
      toast(formatTauriError(e), "error");
    }
  };

  const handleToggle = async (id: string, current: boolean) => {
    await setDictionaryActive(id, !current);
    void load();
  };

  const handleExport = async () => {
    try {
      const data = await exportDictionary();
      const blob = new Blob([data], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "voxitype-dictionary.json";
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      /* ignore */
    }
  };

  const handleImport = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      const text = await file.text();
      await importDictionary(text);
      void load();
    };
    input.click();
  };

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col">
      <PanelHeader
        title="Dictionary"
        subtitle="Custom words, names, and replacements"
        icon={<BookOpen className="h-4.5 w-4.5" />}
        actions={
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => void handleExport()}
              title="Export"
            >
              <Download className="h-3.5 w-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleImport}
              title="Import"
            >
              <Upload className="h-3.5 w-3.5" />
            </Button>
          </>
        }
      />

      <div className="flex gap-2 px-10 pb-4">
        <input
          className="flex-1 rounded-lg bg-vx-bg-tertiary px-3.5 py-2.5 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-shadow focus:outline-none focus:ring-2 focus:ring-vx-accent/40"
          placeholder="Word"
          value={word}
          onChange={(e) => setWord(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void handleAdd();
          }}
        />
        <input
          className="w-44 rounded-lg bg-vx-bg-tertiary px-3.5 py-2.5 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-shadow focus:outline-none focus:ring-2 focus:ring-vx-accent/40"
          placeholder="Replacement (optional)"
          value={replacement}
          onChange={(e) => setReplacement(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void handleAdd();
          }}
        />
        <Button variant="primary" size="sm" onClick={() => void handleAdd()}>
          <Plus className="h-4 w-4" /> Add
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto px-10 pb-8">
        {loading && <p className="text-sm text-vx-text-dim">Loading...</p>}

        {entries.length === 0 && !loading && (
          <div className="flex flex-col items-center justify-center gap-2 py-20 text-center">
            <BookOpen className="h-10 w-10 text-vx-text-dim/40" />
            <p className="text-sm text-vx-text-dim">No dictionary entries</p>
          </div>
        )}

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
              <div className="flex gap-1.5 opacity-0 transition-opacity duration-200 group-hover:opacity-100">
                <button
                  type="button"
                  onClick={() => void handleToggle(e.id, e.is_active)}
                  className="rounded-lg p-1.5 text-vx-text-dim transition-colors hover:bg-vx-bg-tertiary"
                  title={e.is_active ? "Deactivate" : "Activate"}
                >
                  {e.is_active ? (
                    <ToggleRight className="h-4.5 w-4.5 text-vx-success" />
                  ) : (
                    <ToggleLeft className="h-4.5 w-4.5" />
                  )}
                </button>
                <button
                  type="button"
                  onClick={() => void remove(e.id)}
                  className="rounded-lg p-1.5 text-vx-text-dim transition-colors hover:bg-vx-error/15 hover:text-vx-error"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
