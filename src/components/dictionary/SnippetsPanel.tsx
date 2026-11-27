import { useEffect, useState } from "react";
import { Pencil, Search, Trash2, Zap } from "lucide-react";

import { type Translate, useT } from "../../lib/i18n";
import { invokeAction } from "../../lib/invokeAction";
import { useSnippetStore } from "../../stores/snippetStore";
import type { Snippet } from "../../types/app";
import { Button } from "../ui/Button";
import { IconButton } from "../ui/IconButton";
import { Input } from "../ui/Input";
import { StatusBadge } from "../ui/StatusBadge";
import { Switch } from "../ui/Switch";

function SnippetRow({
  t,
  snippet,
  onToggleActive,
  onEdit,
  onDelete,
}: {
  t: Translate;
  snippet: Snippet;
  onToggleActive: (active: boolean) => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  return (
    <div className="px-4 py-3.5 flex flex-wrap sm:flex-nowrap items-start justify-between gap-3 sm:gap-4">
      <div className="min-w-0 flex-1 flex flex-col gap-1.5">
        <div className="flex flex-wrap items-center gap-2.5">
          <StatusBadge
            tone={snippet.is_active ? "ember" : "neutral"}
            label={snippet.trigger_phrase}
          />
          <span className="text-xs text-vx-text-muted tabular-nums">
            {t("dictionary.uses_count", { count: snippet.usage_count })}
          </span>
        </div>
        <p className="line-clamp-2 text-[13px] leading-[18px] text-vx-text-secondary">
          {snippet.content}
        </p>
      </div>

      <div className="flex shrink-0 items-center gap-2 pt-0.5">
        <Switch
          checked={snippet.is_active}
          onChange={onToggleActive}
          ariaLabel={
            snippet.is_active
              ? t("snippets.deactivate_snippet", {
                  phrase: snippet.trigger_phrase,
                })
              : t("snippets.activate_snippet", {
                  phrase: snippet.trigger_phrase,
                })
          }
        />
        <IconButton
          size="sm"
          tone="ghost"
          label={t("snippets.edit_snippet", {
            phrase: snippet.trigger_phrase,
          })}
          icon={<Pencil className="h-3.5 w-3.5" />}
          onClick={onEdit}
        />
        <IconButton
          size="sm"
          tone="ghost"
          label={t("snippets.delete_snippet", {
            phrase: snippet.trigger_phrase,
          })}
          icon={<Trash2 className="h-3.5 w-3.5" />}
          onClick={onDelete}
        />
      </div>
    </div>
  );
}

function SnippetEditRow({
  t,
  editTrigger,
  editContent,
  saving,
  onChangeTrigger,
  onChangeContent,
  onSave,
  onCancel,
}: {
  t: Translate;
  editTrigger: string;
  editContent: string;
  saving: boolean;
  onChangeTrigger: (value: string) => void;
  onChangeContent: (value: string) => void;
  onSave: () => void;
  onCancel: () => void;
}) {
  const handleEscape = (
    event: React.KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) => {
    if (event.key === "Escape") {
      onCancel();
    }
  };

  return (
    <div className="px-4 py-3.5 bg-vx-bg-elevated flex flex-col gap-2.5">
      <Input
        autoFocus
        aria-label={t("snippets.placeholder_trigger")}
        placeholder={t("snippets.placeholder_trigger_v2")}
        value={editTrigger}
        onChange={(e) => onChangeTrigger(e.target.value)}
        onKeyDown={handleEscape}
      />
      <textarea
        rows={3}
        aria-label={t("snippets.placeholder_content")}
        placeholder={t("snippets.placeholder_content_v2")}
        value={editContent}
        onChange={(e) => onChangeContent(e.target.value)}
        onKeyDown={handleEscape}
        className="w-full resize-none rounded-[8px] border border-vx-border-default bg-vx-bg-subtle px-3 py-2 text-[13px] leading-[18px] text-vx-text-primary placeholder:text-vx-text-muted transition-[border-color,box-shadow] duration-[120ms] ease-[var(--ease-vx-out)] focus:outline-none focus:border-vx-accent-ember focus:ring-1 focus:ring-vx-accent-ember"
      />
      <div className="flex items-center justify-end gap-2">
        <Button
          variant="ghost"
          size="sm"
          onClick={onCancel}
          disabled={saving}
        >
          {t("common.cancel")}
        </Button>
        <Button
          variant="primary"
          size="sm"
          onClick={onSave}
          disabled={saving || !editTrigger.trim() || !editContent.trim()}
        >
          {t("common.save")}
        </Button>
      </div>
    </div>
  );
}

export function SnippetsPanel() {
  const t = useT();
  const snippets = useSnippetStore((s) => s.snippets);
  const loading = useSnippetStore((s) => s.loading);
  const error = useSnippetStore((s) => s.error);
  const load = useSnippetStore((s) => s.load);
  const add = useSnippetStore((s) => s.add);
  const toggleActive = useSnippetStore((s) => s.toggleActive);
  const remove = useSnippetStore((s) => s.remove);

  const [trigger, setTrigger] = useState("");
  const [content, setContent] = useState("");
  const [busy, setBusy] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editTrigger, setEditTrigger] = useState("");
  const [editContent, setEditContent] = useState("");
  const [savingEdit, setSavingEdit] = useState(false);

  useEffect(() => {
    void load();
  }, [load]);

  const handleAdd = async () => {
    if (busy || !trigger.trim() || !content.trim()) return;
    setBusy(true);
    try {
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
      const ok = await invokeAction(() => add(snippet));
      if (ok) {
        setTrigger("");
        setContent("");
      }
    } finally {
      setBusy(false);
    }
  };

  const startEdit = (snippet: Snippet) => {
    setEditingId(snippet.id);
    setEditTrigger(snippet.trigger_phrase);
    setEditContent(snippet.content);
  };

  const handleSaveEdit = async (snippet: Snippet) => {
    const trimmedTrigger = editTrigger.trim();
    const trimmedContent = editContent.trim();
    if (savingEdit || !trimmedTrigger || !trimmedContent) return;
    setSavingEdit(true);
    try {
      const ok = await invokeAction(() =>
        add({
          ...snippet,
          name: trimmedTrigger,
          trigger_phrase: trimmedTrigger,
          content: trimmedContent,
        }),
      );
      if (ok) {
        setEditingId(null);
      }
    } finally {
      setSavingEdit(false);
    }
  };

  const activeCount = snippets.filter((s) => s.is_active).length;
  const q = searchQuery.trim().toLowerCase();
  const filtered = q
    ? snippets.filter(
        (s) =>
          s.trigger_phrase.toLowerCase().includes(q) ||
          s.content.toLowerCase().includes(q),
      )
    : snippets;

  return (
    <div className="mx-auto flex min-h-full w-full max-w-5xl flex-col gap-[18px] px-4 sm:px-6 lg:px-8 pt-5 sm:pt-7 pb-24 vx-panel-reveal">
      <header className="flex flex-wrap items-start justify-between gap-3 sm:gap-4">
        <div className="flex flex-col gap-0.5">
          <h1 className="text-[22px] font-semibold leading-7 tracking-[-0.02em] text-vx-text-primary">
            {t("snippets.title_v2")}
          </h1>
          <p className="text-[13px] leading-[18px] text-vx-text-secondary">
            {t("snippets.subtitle_v2")}
          </p>
        </div>

        <StatusBadge
          tone="neutral"
          label={t("snippets.active_triggers", { count: activeCount })}
        />
      </header>

      <div className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] px-4 py-3.5 flex flex-col gap-2.5">
        <div className="flex flex-wrap sm:flex-nowrap items-center gap-2.5">
          <div className="flex-1 min-w-[180px]">
            <Input
              placeholder={t("snippets.placeholder_trigger_v2")}
              aria-label={t("snippets.placeholder_trigger")}
              value={trigger}
              onChange={(e) => setTrigger(e.target.value)}
              onKeyDown={(e) => {
                if (
                  e.key === "Enter" &&
                  !busy &&
                  trigger.trim() &&
                  content.trim()
                ) {
                  void handleAdd();
                }
              }}
            />
          </div>
          <Button
            variant="primary"
            size="md"
            onClick={() => void handleAdd()}
            disabled={busy || !trigger.trim() || !content.trim()}
          >
            {t("snippets.add_btn_plus")}
          </Button>
        </div>

        <textarea
          placeholder={t("snippets.placeholder_content_v2")}
          aria-label={t("snippets.placeholder_content")}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onKeyDown={(e) => {
            if (
              e.key === "Enter" &&
              (e.ctrlKey || e.metaKey) &&
              !busy &&
              trigger.trim() &&
              content.trim()
            ) {
              e.preventDefault();
              void handleAdd();
            }
          }}
          className="min-h-20 w-full resize-none rounded-[8px] border border-vx-border-default bg-vx-bg-subtle px-3 py-2 text-[13px] leading-[18px] text-vx-text-primary placeholder:text-vx-text-muted transition-[border-color,box-shadow] duration-[120ms] ease-[var(--ease-vx-out)] focus:outline-none focus:border-vx-accent-ember focus:ring-1 focus:ring-vx-accent-ember"
        />
      </div>

      <div className="relative">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-[15px] w-[15px] -translate-y-1/2 text-vx-text-muted" />
        <input
          placeholder={t("snippets.search_placeholder")}
          aria-label={t("snippets.search_placeholder")}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="h-9 w-full rounded-[8px] border border-vx-border-default bg-vx-bg-subtle pl-9 pr-3 text-[13px] leading-[18px] text-vx-text-primary placeholder:text-vx-text-muted focus:border-vx-accent-ember focus:outline-none focus:ring-1 focus:ring-vx-accent-ember"
        />
      </div>

      <div className="flex-1 overflow-y-auto">
        {loading && snippets.length === 0 ? (
          <p className="text-sm text-vx-text-muted">{t("snippets.loading")}</p>
        ) : error && !loading ? (
          <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
            <div className="max-w-md space-y-1">
              <p className="text-sm font-semibold text-vx-text-primary">
                {t("error.snippets_load_failed")}
              </p>
              <p className="font-mono text-xs break-all text-vx-text-muted">
                {error}
              </p>
            </div>
            <Button size="sm" onClick={() => void load()}>
              {t("error.retry")}
            </Button>
          </div>
        ) : snippets.length === 0 ? (
          <div className="flex flex-col items-center justify-center gap-2 py-20 text-center">
            <Zap className="h-10 w-10 text-vx-text-muted opacity-40" />
            <p className="text-sm text-vx-text-muted">{t("snippets.empty")}</p>
          </div>
        ) : filtered.length === 0 ? (
          <div className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] py-12 text-center text-sm text-vx-text-muted">
            {t("snippets.empty")}
          </div>
        ) : (
          <div className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] divide-y divide-vx-border-subtle overflow-hidden">
            {filtered.map((s) =>
              editingId === s.id ? (
                <SnippetEditRow
                  key={s.id}
                  t={t}
                  editTrigger={editTrigger}
                  editContent={editContent}
                  saving={savingEdit}
                  onChangeTrigger={setEditTrigger}
                  onChangeContent={setEditContent}
                  onSave={() => void handleSaveEdit(s)}
                  onCancel={() => setEditingId(null)}
                />
              ) : (
                <SnippetRow
                  key={s.id}
                  t={t}
                  snippet={s}
                  onToggleActive={(next) =>
                    void invokeAction(() => toggleActive(s.id, next))
                  }
                  onEdit={() => startEdit(s)}
                  onDelete={() => void invokeAction(() => remove(s.id))}
                />
              ),
            )}
          </div>
        )}
      </div>
    </div>
  );
}
