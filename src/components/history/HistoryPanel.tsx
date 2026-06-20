import { useEffect } from "react";
import { Search, Pin, Copy, Trash2, RefreshCw } from "lucide-react";
import { useHistoryStore } from "../../stores/historyStore";
import { reInject } from "../../lib/tauri";

export function HistoryPanel() {
  const items = useHistoryStore((s) => s.items);
  const loading = useHistoryStore((s) => s.loading);
  const query = useHistoryStore((s) => s.query);
  const load = useHistoryStore((s) => s.load);
  const search = useHistoryStore((s) => s.search);
  const remove = useHistoryStore((s) => s.remove);
  const togglePin = useHistoryStore((s) => s.togglePin);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <h2 className="text-lg font-semibold">History</h2>

      <div className="relative">
        <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-vx-text-dim" />
        <input
          className="w-full rounded-md border border-vx-border bg-vx-bg-secondary py-2 pl-8 pr-3 text-sm text-vx-text-primary placeholder:text-vx-text-dim focus:border-vx-accent focus:outline-none"
          placeholder="Search transcriptions..."
          value={query}
          onChange={(e) => void search(e.target.value)}
        />
      </div>

      {loading && (
        <p className="text-sm text-vx-text-dim">Loading...</p>
      )}

      <div className="flex-1 overflow-y-auto">
        {items.length === 0 && !loading && (
          <p className="py-8 text-center text-sm text-vx-text-dim">
            No transcriptions yet
          </p>
        )}

        <div className="flex flex-col gap-2">
          {items.map((item) => (
            <div
              key={item.id}
              className="group flex items-start gap-3 rounded-lg border border-vx-border bg-vx-bg-secondary p-3"
            >
              <div className="flex-1 min-w-0">
                <p className="text-sm text-vx-text-primary truncate">
                  {item.text_formatted}
                </p>
                <div className="mt-1 flex items-center gap-2 text-xs text-vx-text-dim">
                  <span>{item.mode}</span>
                  <span>&middot;</span>
                  <span>{item.word_count} words</span>
                  <span>&middot;</span>
                  <span>{item.created_at?.slice(0, 16)}</span>
                </div>
              </div>

              <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  type="button"
                  onClick={() => void togglePin(item.id, !item.is_pinned)}
                  className={`rounded p-1 hover:bg-vx-bg-tertiary ${
                    item.is_pinned ? "text-vx-accent" : "text-vx-text-dim"
                  }`}
                  title={item.is_pinned ? "Unpin" : "Pin"}
                >
                  <Pin className="h-3.5 w-3.5" />
                </button>
                <button
                  type="button"
                  onClick={() => {
                    void navigator.clipboard.writeText(item.text_formatted);
                  }}
                  className="rounded p-1 text-vx-text-dim hover:bg-vx-bg-tertiary"
                  title="Copy"
                >
                  <Copy className="h-3.5 w-3.5" />
                </button>
                <button
                  type="button"
                  onClick={() => void reInject(item.id)}
                  className="rounded p-1 text-vx-text-dim hover:bg-vx-bg-tertiary"
                  title="Re-inject"
                >
                  <RefreshCw className="h-3.5 w-3.5" />
                </button>
                <button
                  type="button"
                  onClick={() => void remove(item.id)}
                  className="rounded p-1 text-vx-text-dim hover:bg-vx-bg-tertiary hover:text-vx-error"
                  title="Delete"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
