import { useEffect, useState } from "react";
import { BookOpen, Pencil, Search, Trash2 } from "lucide-react";

import { downloadBlob } from "../../lib/downloadBlob";
import { type Translate, useT } from "../../lib/i18n";
import { invokeAction } from "../../lib/invokeAction";
import { exportDictionary, importDictionary, setDictionaryActive } from "../../lib/tauri";
import { useDictionaryStore } from "../../stores/dictionaryStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { DictionaryEntry } from "../../types/app";
import { Button } from "../ui/Button";
import { IconButton } from "../ui/IconButton";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { StatusBadge } from "../ui/StatusBadge";
import { Switch } from "../ui/Switch";
import { toast } from "../ui/Toast";

type FilterChip = "all" | "active" | "replacements";

const MAX_IMPORT_BYTES = 5 * 1024 * 1024;
const MAX_WORD_LENGTH = 256;

function DictionaryEntryRow({
  t,
  entry,
  toggling,
  onToggle,
  onEdit,
  onDelete,
}: {
  t: Translate;
  entry: DictionaryEntry;
  toggling: boolean;
  onToggle: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  return (
    <div className="px-4 py-3 flex items-center justify-between gap-3">
      <div className="min-w-0 flex flex-wrap items-center gap-2">
        <span
          className={`text-[13px] font-medium ${
            entry.is_active ? "text-vx-text-primary" : "text-vx-text-muted"
          }`}
        >
          {entry.word}
        </span>
        {entry.replacement && (
          <span className="text-xs text-vx-text-secondary">
            {"\u2192"} {entry.replacement}
          </span>
        )}
        <StatusBadge tone="neutral" label={entry.language.toUpperCase()} />
      </div>

      <div className="flex shrink-0 items-center gap-3">
        <span className="text-xs text-vx-text-muted tabular-nums">
          {t("dictionary.uses_count", { count: entry.usage_count })}
        </span>
        <Switch
          checked={entry.is_active}
          disabled={toggling}
          onChange={onToggle}
          ariaLabel={
            entry.is_active
              ? t("dictionary.deactivate_word", { word: entry.word })
              : t("dictionary.activate_word", { word: entry.word })
          }
        />
        <IconButton
          size="sm"
          tone="ghost"
          label={t("dictionary.edit_word", { word: entry.word })}
          icon={<Pencil className="h-3.5 w-3.5" />}
          onClick={onEdit}
        />
        <IconButton
          size="sm"
          tone="ghost"
          label={t("dictionary.delete_word", { word: entry.word })}
          icon={<Trash2 className="h-3.5 w-3.5" />}
          onClick={onDelete}
        />
      </div>
    </div>
  );
}

function DictionaryEditRow({
  t,
  editWord,
  editReplacement,
  saving,
  onChangeWord,
  onChangeReplacement,
  onSave,
  onCancel,
}: {
  t: Translate;
  editWord: string;
  editReplacement: string;
  saving: boolean;
  onChangeWord: (value: string) => void;
  onChangeReplacement: (value: string) => void;
  onSave: () => void;
  onCancel: () => void;
}) {
  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") {
      onCancel();
    } else if (event.key === "Enter" && !saving && editWord.trim()) {
      onSave();
    }
  };

  return (
    <div className="px-4 py-3 bg-vx-bg-elevated flex flex-wrap items-center justify-between gap-2.5">
      <div className="flex flex-1 flex-wrap items-center gap-2 min-w-[240px]">
        <div className="flex-1 min-w-[160px]">
          <Input
            autoFocus
            aria-label={t("dictionary.placeholder_word")}
            placeholder={t("dictionary.placeholder_word_v2")}
            maxLength={MAX_WORD_LENGTH}
            value={editWord}
            onChange={(e) => onChangeWord(e.target.value)}
            onKeyDown={handleKeyDown}
          />
        </div>
        <div className="flex-1 min-w-[160px]">
          <Input
            aria-label={t("dictionary.placeholder_rep")}
            placeholder={t("dictionary.placeholder_rep_v2")}
            maxLength={MAX_WORD_LENGTH}
            value={editReplacement}
            onChange={(e) => onChangeReplacement(e.target.value)}
            onKeyDown={handleKeyDown}
          />
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-2">
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
          disabled={saving || !editWord.trim()}
        >
          {t("common.save")}
        </Button>
      </div>
    </div>
  );
}

export function DictionaryPanel() {
  const t = useT();
  const entries = useDictionaryStore((s) => s.entries);
  const loading = useDictionaryStore((s) => s.loading);
  const error = useDictionaryStore((s) => s.error);
  const load = useDictionaryStore((s) => s.load);
  const add = useDictionaryStore((s) => s.add);
  const updateWord = useDictionaryStore((s) => s.updateWord);
  const remove = useDictionaryStore((s) => s.remove);
  const sttLanguage = useSettingsStore((s) => s.settings.stt_language);

  const [word, setWord] = useState("");
  const [replacement, setReplacement] = useState("");
  const [entryLang, setEntryLang] = useState(() =>
    sttLanguage === "en" ? "en" : "id",
  );
  const [busy, setBusy] = useState(false);
  const [togglingId, setTogglingId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [chip, setChip] = useState<FilterChip>("all");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editWord, setEditWord] = useState("");
  const [editReplacement, setEditReplacement] = useState("");
  const [savingEdit, setSavingEdit] = useState(false);

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
        language: entryLang,
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

  const startEdit = (entry: DictionaryEntry) => {
    setEditingId(entry.id);
    setEditWord(entry.word);
    setEditReplacement(entry.replacement ?? "");
  };

  const handleSaveEdit = async (entry: DictionaryEntry) => {
    const trimmedWord = editWord.trim();
    if (savingEdit || !trimmedWord) return;
    setSavingEdit(true);
    try {
      const updated: DictionaryEntry = {
        ...entry,
        id: entry.id,
        word: trimmedWord,
        replacement: editReplacement.trim() || null,
      };
      const ok = await invokeAction(() => updateWord(updated));
      if (ok) {
        setEditingId(null);
      }
    } finally {
      setSavingEdit(false);
    }
  };

  const handleExport = async () => {
    await invokeAction(async () => {
      const data = await exportDictionary();
      downloadBlob(data, "voxitype-dictionary.json", "application/json");
    });
  };

  const handleImport = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      if (file.size > MAX_IMPORT_BYTES) {
        toast(t("dictionary.file_too_large"), "error");
        return;
      }
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

  const activeCount = entries.filter((e) => e.is_active).length;
  const replacementCount = entries.filter((e) => Boolean(e.replacement)).length;
  const q = searchQuery.trim().toLowerCase();
  const filtered = entries.filter((e) => {
    if (chip === "active" && !e.is_active) return false;
    if (chip === "replacements" && !e.replacement) return false;
    if (!q) return true;
    return (
      e.word.toLowerCase().includes(q) ||
      (e.replacement ?? "").toLowerCase().includes(q)
    );
  });

  const chips: { key: FilterChip; label: string }[] = [
    { key: "all", label: t("dictionary.chip_all", { count: entries.length }) },
    { key: "active", label: t("dictionary.chip_active", { count: activeCount }) },
    {
      key: "replacements",
      label: t("dictionary.chip_replacements", { count: replacementCount }),
    },
  ];

  return (
    <div className="mx-auto flex h-full w-full max-w-5xl flex-col gap-[18px] px-8 pt-7 pb-24 vx-panel-reveal">
      <header className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-0.5">
          <h1 className="text-[22px] font-semibold leading-7 tracking-[-0.02em] text-vx-text-primary">
            {t("dictionary.title_v2")}
          </h1>
          <p className="text-[13px] leading-[18px] text-vx-text-secondary">
            {t("dictionary.subtitle_v2")}
          </p>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={handleImport}
            aria-label={t("dictionary.import_tooltip")}
          >
            {t("dictionary.import_json")}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void handleExport()}
            aria-label={t("dictionary.export_tooltip")}
          >
            {t("dictionary.export_json")}
          </Button>
        </div>
      </header>

      <div className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] px-3.5 py-3 flex flex-wrap items-center gap-2.5">
        <div className="flex-1 min-w-[200px]">
          <Input
            placeholder={t("dictionary.placeholder_word_v2")}
            aria-label={t("dictionary.placeholder_word")}
            maxLength={MAX_WORD_LENGTH}
            value={word}
            onChange={(e) => setWord(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !busy && word.trim()) void handleAdd();
            }}
          />
        </div>
        <div className="w-64">
          <Input
            placeholder={t("dictionary.placeholder_rep_v2")}
            aria-label={t("dictionary.placeholder_rep")}
            maxLength={MAX_WORD_LENGTH}
            value={replacement}
            onChange={(e) => setReplacement(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !busy && word.trim()) void handleAdd();
            }}
          />
        </div>
        <Select
          aria-label={t("dictionary.language_label")}
          value={entryLang}
          onChange={(e) => setEntryLang(e.target.value)}
          options={[
            { value: "en", label: "EN" },
            { value: "id", label: "ID" },
          ]}
          className="w-[96px]"
        />
        <Button
          variant="primary"
          size="md"
          onClick={() => void handleAdd()}
          disabled={busy || !word.trim()}
        >
          {t("dictionary.add_btn_plus")}
        </Button>
      </div>

      <div className="flex flex-wrap items-center gap-2.5">
        <div className="relative flex-1 min-w-[200px]">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-[15px] w-[15px] -translate-y-1/2 text-vx-text-muted" />
          <input
            placeholder={t("dictionary.filter_placeholder", {
              count: entries.length,
            })}
            aria-label={t("dictionary.search_aria")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="h-9 w-full rounded-[8px] border border-vx-border-default bg-vx-bg-subtle pl-9 pr-3 text-[13px] leading-[18px] text-vx-text-primary placeholder:text-vx-text-muted focus:border-vx-accent-ember focus:outline-none focus:ring-1 focus:ring-vx-accent-ember"
          />
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          {chips.map(({ key, label }) => {
            const active = chip === key;
            return (
              <button
                key={key}
                type="button"
                aria-pressed={active}
                onClick={() => setChip(key)}
                className={`h-9 rounded-[8px] border px-3 text-xs font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember ${
                  active
                    ? "bg-vx-bg-elevated border-vx-border-strong text-vx-text-primary"
                    : "bg-vx-bg-subtle border-vx-border-subtle text-vx-text-secondary hover:border-vx-border-default hover:text-vx-text-primary"
                }`}
              >
                {label}
              </button>
            );
          })}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        {loading && entries.length === 0 ? (
          <p className="text-sm text-vx-text-muted">
            {t("dictionary.loading")}
          </p>
        ) : error && !loading ? (
          <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
            <div className="max-w-md space-y-1">
              <p className="text-sm font-semibold text-vx-text-primary">
                {t("error.dictionary_load_failed")}
              </p>
              <p className="font-mono text-xs break-all text-vx-text-muted">
                {error}
              </p>
            </div>
            <Button size="sm" onClick={() => void load()}>
              {t("error.retry")}
            </Button>
          </div>
        ) : entries.length === 0 ? (
          <div className="flex flex-col items-center justify-center gap-2 py-20 text-center">
            <BookOpen className="h-10 w-10 text-vx-text-muted opacity-40" />
            <p className="text-sm text-vx-text-muted">
              {t("dictionary.empty")}
            </p>
            <p className="max-w-sm text-xs text-vx-text-muted">
              {t("dictionary.empty_hint")}
            </p>
          </div>
        ) : filtered.length === 0 ? (
          <div className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] py-12 text-center text-sm text-vx-text-muted">
            {t("dictionary.empty")}
          </div>
        ) : (
          <div className="bg-vx-bg-surface border border-vx-border-subtle rounded-[12px] divide-y divide-vx-border-subtle overflow-hidden">
            {filtered.map((entry) =>
              editingId === entry.id ? (
                <DictionaryEditRow
                  key={entry.id}
                  t={t}
                  editWord={editWord}
                  editReplacement={editReplacement}
                  saving={savingEdit}
                  onChangeWord={setEditWord}
                  onChangeReplacement={setEditReplacement}
                  onSave={() => void handleSaveEdit(entry)}
                  onCancel={() => setEditingId(null)}
                />
              ) : (
                <DictionaryEntryRow
                  key={entry.id}
                  t={t}
                  entry={entry}
                  toggling={togglingId === entry.id}
                  onToggle={() => handleToggle(entry.id, entry.is_active)}
                  onEdit={() => startEdit(entry)}
                  onDelete={() => void invokeAction(() => remove(entry.id))}
                />
              ),
            )}
          </div>
        )}
      </div>
    </div>
  );
}
