import { useEffect, useState } from "react";
import { Plus, Trash2, Zap } from "lucide-react";
import { useSnippetStore } from "../../stores/snippetStore";
import { Button } from "../ui/Button";
import { PanelHeader } from "../common/PanelHeader";
import type { Snippet } from "../../types/app";

export function SnippetsPanel() {
  const snippets = useSnippetStore((s) => s.snippets);
  const loading = useSnippetStore((s) => s.loading);
  const load = useSnippetStore((s) => s.load);
  const add = useSnippetStore((s) => s.add);
  const remove = useSnippetStore((s) => s.remove);

  const [trigger, setTrigger] = useState("");
  const [content, setContent] = useState("");

  useEffect(() => {
    void load();
  }, [load]);

  const handleAdd = () => {
    if (!trigger.trim() || !content.trim()) return;
    const snippet: Snippet = {
      id: "",
      name: trigger.trim(),
      trigger_phrase: trigger.trim(),
      content: content.trim(),
      category: null,
      mode: null,
      usage_count: 0,
      is_active: true,
    };
    void add(snippet);
    setTrigger("");
    setContent("");
  };

  const inputCls =
    "rounded-lg border border-vx-border bg-vx-bg-tertiary/60 px-3.5 py-2.5 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-colors hover:border-vx-border-strong focus:border-vx-accent focus:outline-none focus:ring-2 focus:ring-vx-accent/30";

  return (
    <div className="flex h-full flex-col">
      <PanelHeader
        title="Snippets"
        subtitle="Spoken triggers expand into longer text"
        icon={<Zap className="h-4.5 w-4.5" />}
      />

      <div className="flex flex-col gap-2 px-5 py-3">
        <input
          className={inputCls}
          placeholder="Trigger phrase (e.g. sign off)"
          value={trigger}
          onChange={(e) => setTrigger(e.target.value)}
        />
        <textarea
          className={`${inputCls} min-h-20 resize-none`}
          placeholder="Expansion content..."
          value={content}
          onChange={(e) => setContent(e.target.value)}
        />
        <div className="flex justify-end">
          <Button variant="primary" size="sm" onClick={handleAdd}>
            <Plus className="h-4 w-4" /> Add snippet
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-5 pb-5">
        {loading && <p className="text-sm text-vx-text-dim">Loading...</p>}

        {snippets.length === 0 && !loading && (
          <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
            <Zap className="h-10 w-10 text-vx-text-dim/50" />
            <p className="text-sm text-vx-text-dim">No snippets yet</p>
          </div>
        )}

        <div className="flex flex-col gap-1.5">
          {snippets.map((s) => (
            <div
              key={s.id}
              className="group flex items-start justify-between gap-3 rounded-xl border border-vx-border bg-vx-bg-secondary/60 px-4 py-3 transition-colors hover:border-vx-border-strong"
            >
              <div className="min-w-0">
                <span className="inline-block rounded-md bg-vx-accent-soft px-2 py-0.5 text-xs font-semibold text-vx-accent">
                  {s.trigger_phrase}
                </span>
                <p className="mt-1.5 line-clamp-2 text-xs text-vx-text-secondary">
                  {s.content}
                </p>
              </div>
              <button
                type="button"
                onClick={() => void remove(s.id)}
                className="rounded-lg p-1.5 text-vx-text-dim opacity-0 transition-opacity duration-200 hover:bg-vx-error/15 hover:text-vx-error group-hover:opacity-100"
              >
                <Trash2 className="h-4 w-4" />
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
