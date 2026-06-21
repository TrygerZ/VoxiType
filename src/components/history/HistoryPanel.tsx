import { useEffect, useState } from "react";
import {
  Search,
  Pin,
  Copy,
  Trash2,
  RefreshCw,
  Download,
  History as HistoryIcon,
} from "lucide-react";
import { useHistoryStore } from "../../stores/historyStore";
import { reInject, exportHistory } from "../../lib/tauri";
import { Button } from "../ui/Button";
import { Select } from "../ui/Select";
import { PanelHeader } from "../common/PanelHeader";

export function HistoryPanel() {
  const items = useHistoryStore((s) => s.items);
  const loading = useHistoryStore((s) => s.loading);
  const query = useHistoryStore((s) => s.query);
  const load = useHistoryStore((s) => s.load);
  const search = useHistoryStore((s) => s.search);
  const remove = useHistoryStore((s) => s.remove);
  const togglePin = useHistoryStore((s) => s.togglePin);
  const [modeFilter, setModeFilter] = useState("");
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => {
    void load();
  }, [load]);

  const handleExport = async (fmt: "json" | "csv") => {
    try {
      const data = await exportHistory(fmt);
      const blob = new Blob([data], { type: "text/plain" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `voxitype-history.${fmt}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      /* ignore */
    }
  };

  const handleCopy = (id: string, text: string) => {
    void navigator.clipboard.writeText(text);
    setCopied(id);
    setTimeout(() => setCopied((c) => (c === id ? null : c)), 1200);
  };

  const filtered = modeFilter
    ? items.filter((i) => i.mode === modeFilter)
    : items;

  return (
    <div className="flex h-full flex-col">
      <PanelHeader
        title="History"
        subtitle="Your recent transcriptions"
        icon={<HistoryIcon className="h-4.5 w-4.5" />}
        actions={
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => void handleExport("json")}
              title="Export JSON"
            >
              <Download className="h-3.5 w-3.5" /> JSON
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => void handleExport("csv")}
              title="Export CSV"
            >
              <Download className="h-3.5 w-3.5" /> CSV
            </Button>
          </>
        }
      />

      <div className="flex gap-2 px-5 py-3">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-vx-text-dim" />
          <input
            className="w-full rounded-lg border border-vx-border bg-vx-bg-tertiary/60 py-2.5 pl-9 pr-3 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-colors hover:border-vx-border-strong focus:border-vx-accent focus:outline-none focus:ring-2 focus:ring-vx-accent/30"
            placeholder="Search transcriptions..."
            value={query}
            onChange={(e) => void search(e.target.value)}
          />
        </div>
        <Select
          options={[
            { value: "", label: "All modes" },
            { value: "dictation", label: "Dictation" },
            { value: "message", label: "Message" },
            { value: "email", label: "Email" },
          ]}
          value={modeFilter}
          onChange={(e) => setModeFilter(e.target.value)}
          className="w-40"
        />
      </div>

      <div className="flex-1 overflow-y-auto px-5 pb-5">
        {loading && <p className="text-sm text-vx-text-dim">Loading...</p>}

        {filtered.length === 0 && !loading && (
          <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
            <HistoryIcon className="h-10 w-10 text-vx-text-dim/50" />
            <p className="text-sm text-vx-text-dim">No transcriptions yet</p>
          </div>
        )}

        <div className="flex flex-col gap-2">
          {filtered.map((item) => (
            <div
              key={item.id}
              className="group flex items-start gap-3 rounded-xl border border-vx-border bg-vx-bg-secondary/60 p-3.5 transition-colors hover:border-vx-border-strong hover:bg-vx-bg-secondary"
            >
              <div className="min-w-0 flex-1">
                <p className="text-sm leading-relaxed text-vx-text-primary line-clamp-2">
                  {item.text_formatted}
                </p>
                <div className="mt-1.5 flex items-center gap-2 text-xs text-vx-text-dim">
                  <span className="rounded-full bg-vx-bg-tertiary px-2 py-0.5 capitalize">
                    {item.mode}
                  </span>
                  <span>{item.word_count} words</span>
                  <span>&middot;</span>
                  <span>{item.created_at?.slice(0, 16).replace("T", " ")}</span>
                </div>
              </div>

              <div className="flex gap-1.5 opacity-0 transition-opacity duration-200 group-hover:opacity-100">
                <button
                  type="button"
                  onClick={() => void togglePin(item.id, !item.is_pinned)}
                  className={`rounded-lg p-1.5 transition-colors ${
                    item.is_pinned
                      ? "bg-vx-accent-soft text-vx-accent"
                      : "text-vx-text-dim hover:bg-vx-bg-tertiary hover:text-vx-text-primary"
                  }`}
                  title={item.is_pinned ? "Unpin" : "Pin"}
                >
                  <Pin className="h-4 w-4" />
                </button>
                <button
                  type="button"
                  onClick={() => handleCopy(item.id, item.text_formatted)}
                  className={`rounded-lg p-1.5 transition-colors ${
                    copied === item.id
                      ? "bg-vx-success/15 text-vx-success"
                      : "text-vx-text-dim hover:bg-vx-bg-tertiary hover:text-vx-text-primary"
                  }`}
                  title="Copy"
                >
                  <Copy className="h-4 w-4" />
                </button>
                <button
                  type="button"
                  onClick={() => void reInject(item.id)}
                  className="rounded-lg p-1.5 text-vx-text-dim transition-colors hover:bg-vx-bg-tertiary hover:text-vx-text-primary"
                  title="Re-inject"
                >
                  <RefreshCw className="h-4 w-4" />
                </button>
                <button
                  type="button"
                  onClick={() => void remove(item.id)}
                  className="rounded-lg p-1.5 text-vx-text-dim transition-colors hover:bg-vx-error/15 hover:text-vx-error"
                  title="Delete"
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
