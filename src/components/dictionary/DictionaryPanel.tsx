import { useEffect, useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import { useDictionaryStore } from "../../stores/dictionaryStore";
import { Button } from "../ui/Button";
import type { DictionaryEntry } from "../../types/app";

export function DictionaryPanel() {
  const entries = useDictionaryStore((s) => s.entries);
  const loading = useDictionaryStore((s) => s.loading);
  const load = useDictionaryStore((s) => s.load);
  const add = useDictionaryStore((s) => s.add);
  const remove = useDictionaryStore((s) => s.remove);
  const [word, setWord] = useState("");

  useEffect(() => {
    void load();
  }, [load]);

  const handleAdd = () => {
    if (!word.trim()) return;
    const entry: DictionaryEntry = {
      id: "",
      word: word.trim(),
      pronunciation: null,
      category: "custom",
      replacement: null,
      language: "id",
      usage_count: 0,
      is_active: true,
    };
    void add(entry);
    setWord("");
  };

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <h2 className="text-lg font-semibold">Dictionary</h2>

      <div className="flex gap-2">
        <input
          className="flex-1 rounded-md border border-vx-border bg-vx-bg-secondary px-3 py-2 text-sm text-vx-text-primary placeholder:text-vx-text-dim focus:border-vx-accent focus:outline-none"
          placeholder="Add a word..."
          value={word}
          onChange={(e) => setWord(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleAdd()}
        />
        <Button variant="primary" size="sm" onClick={handleAdd}>
          <Plus className="h-4 w-4" />
        </Button>
      </div>

      {loading && <p className="text-sm text-vx-text-dim">Loading...</p>}

      <div className="flex-1 overflow-y-auto">
        {entries.length === 0 && !loading && (
          <p className="py-8 text-center text-sm text-vx-text-dim">
            No dictionary entries
          </p>
        )}

        <div className="flex flex-col gap-1.5">
          {entries.map((e) => (
            <div
              key={e.id}
              className="group flex items-center justify-between rounded-md border border-vx-border bg-vx-bg-secondary px-3 py-2"
            >
              <div className="min-w-0">
                <span className="text-sm text-vx-text-primary">{e.word}</span>
                {e.replacement && (
                  <span className="ml-2 text-xs text-vx-text-dim">
                    → {e.replacement}
                  </span>
                )}
              </div>
              <button
                type="button"
                onClick={() => void remove(e.id)}
                className="rounded p-1 text-vx-text-dim opacity-0 group-hover:opacity-100 hover:text-vx-error transition-opacity"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
