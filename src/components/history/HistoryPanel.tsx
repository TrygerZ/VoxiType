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

const formatDateTime = (dateStr?: string | null) => {
  if (!dateStr) return "";
  const d = new Date(dateStr);
  if (isNaN(d.getTime())) return "";
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
};

export function HistoryPanel() {
  const items = useHistoryStore((s) => s.items);
  const loading = useHistoryStore((s) => s.loading);
  const query = useHistoryStore((s) => s.query);
  const load = useHistoryStore((s) => s.load);
  const search = useHistoryStore((s) => s.search);
  const remove = useHistoryStore((s) => s.remove);
  const clear = useHistoryStore((s) => s.clear);
  const togglePin = useHistoryStore((s) => s.togglePin);
  const [modeFilter, setModeFilter] = useState("");
  const [copied, setCopied] = useState<string | null>(null);
  const [confirmingClear, setConfirmingClear] = useState(false);
  const [clearing, setClearing] = useState(false);

  const handleClear = async () => {
    setClearing(true);
    try {
      await clear(true);
    } finally {
      setClearing(false);
      setConfirmingClear(false);
    }
  };

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
    <div className="mx-auto flex h-full max-w-4xl flex-col">
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
            {confirmingClear ? (
              <>
                <Button
                  variant="danger"
                  size="sm"
                  onClick={() => void handleClear()}
                  disabled={clearing}
                  title="Confirm clearing history (pinned items are kept)"
                >
                  <Trash2 className="h-3.5 w-3.5" />{" "}
                  {clearing ? "Clearing..." : "Confirm"}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setConfirmingClear(false)}
                  disabled={clearing}
                  title="Cancel"
                >
                  Cancel
                </Button>
              </>
            ) : (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setConfirmingClear(true)}
                disabled={items.length === 0}
                title="Clear history (pinned items are kept)"
              >
                <Trash2 className="h-3.5 w-3.5" /> Clear
              </Button>
            )}
          </>
        }
      />

      <div className="flex gap-2 px-10 pb-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-vx-text-dim" />
          <input
            className="w-full rounded-lg bg-vx-bg-tertiary py-2.5 pl-9 pr-3 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-shadow focus:outline-none focus:ring-2 focus:ring-vx-accent/40"
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

      <div className="flex-1 overflow-y-auto px-10 pb-8">
        {loading && <p className="text-sm text-vx-text-dim">Loading...</p>}

        {filtered.length === 0 && !loading && (
          <div className="flex flex-col items-center justify-center gap-2 py-20 text-center">
            <HistoryIcon className="h-10 w-10 text-vx-text-dim/40" />
            <p className="text-sm text-vx-text-dim">No transcriptions yet</p>
          </div>
        )}

        <div className="flex flex-col divide-y divide-vx-divider">
          {filtered.map((item) => (
            <div
              key={item.id}
              className="group flex items-start gap-3 py-4 transition-opacity"
            >
              <div className="min-w-0 flex-1">
                <p className="text-sm leading-relaxed text-vx-text-primary line-clamp-2">
                  {item.text_formatted || item.text_raw}
                </p>
                <div className="mt-1.5 flex items-center gap-2 text-xs text-vx-text-dim">
                  <span className="rounded-full bg-vx-bg-tertiary px-2 py-0.5 capitalize">
                    {item.mode}
                  </span>
                  <span>{item.word_count} words</span>
                  <span>&middot;</span>
                  <span>{formatDateTime(item.created_at)}</span>
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
                  onClick={() => handleCopy(item.id, item.text_formatted || item.text_raw)}
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
