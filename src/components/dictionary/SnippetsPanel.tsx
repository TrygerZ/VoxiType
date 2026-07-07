import { useEffect, useState } from "react";
import { Plus, Trash2, Zap } from "lucide-react";
import { useSnippetStore } from "../../stores/snippetStore";
import { useT } from "../../lib/i18n";
import { Button } from "../ui/Button";
import { PanelHeader } from "../common/PanelHeader";
import type { Snippet } from "../../types/app";
import { formatTauriError } from "../../lib/tauri";
import { toast } from "../ui/Toast";

export function SnippetsPanel() {
  const t = useT();
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

  const handleAdd = async () => {
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
    try {
      await add(snippet);
      setTrigger("");
      setContent("");
    } catch (e: unknown) {
      toast(formatTauriError(e), "error");
    }
  };

  const inputCls =
    "rounded-lg bg-vx-bg-tertiary px-3.5 py-2.5 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-shadow focus:outline-none focus:ring-2 focus:ring-vx-accent/40";

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col">
      <PanelHeader
        title={t("snippets.title")}
        subtitle={t("snippets.subtitle")}
        icon={<Zap className="h-4.5 w-4.5" />}
      />

      <div className="flex flex-col gap-2 px-10 pb-4">
        <input
          className={inputCls}
          placeholder={t("snippets.placeholder_trigger")}
          value={trigger}
          onChange={(e) => setTrigger(e.target.value)}
        />
        <textarea
          className={`${inputCls} min-h-20 resize-none`}
          placeholder={t("snippets.placeholder_content")}
          value={content}
          onChange={(e) => setContent(e.target.value)}
        />
        <div className="flex justify-end">
          <Button variant="primary" size="sm" onClick={() => void handleAdd()}>
            <Plus className="h-4 w-4" /> {t("snippets.add_btn")}
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-10 pb-8">
        {loading && <p className="text-sm text-vx-text-dim">{t("snippets.loading")}</p>}

        {snippets.length === 0 && !loading && (
          <div className="flex flex-col items-center justify-center gap-2 py-20 text-center">
            <Zap className="h-10 w-10 text-vx-text-dim/40" />
            <p className="text-sm text-vx-text-dim">{t("snippets.empty")}</p>
          </div>
        )}

        <div className="flex flex-col divide-y divide-vx-divider">
          {snippets.map((s) => (
            <div
              key={s.id}
              className="group flex items-start justify-between gap-3 py-3.5 transition-opacity"
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
