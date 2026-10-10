import { useState, useEffect, useCallback, useRef } from "react";

import { useT } from "../../lib/i18n";

const MOD_KEYS = new Set([
  "Control",
  "Shift",
  "Alt",
  "Meta",
]);

const KEY_MAP: Record<string, string> = {
  Control: "Ctrl",
  Meta: "Super",
  " ": "Space",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
};

function toAccelerator(e: KeyboardEvent): string | null {
  if (MOD_KEYS.has(e.key)) return null;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");

  const mapped = KEY_MAP[e.key];
  const key = mapped ?? (e.key.length === 1 ? e.key.toUpperCase() : e.key);

  parts.push(key);
  return parts.join("+");
}

interface HotkeyRecorderProps {
  value: string;
  onChange: (combo: string) => void;
}

export function HotkeyRecorder({ value, onChange }: HotkeyRecorderProps) {
  const t = useT();
  const [recording, setRecording] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        setRecording(false);
        return;
      }
      // Tab must move focus, so never consume it.
      if (e.key === "Tab") return;

      const combo = toAccelerator(e);
      const hasModifier = e.ctrlKey || e.altKey || e.shiftKey || e.metaKey;
      if (combo && hasModifier) {
        e.preventDefault();
        e.stopPropagation();
        onChange(combo);
        setRecording(false);
      }
      // Plain keys without modifiers stay unconsumed: the recorder keeps
      // waiting and the keystroke reaches the rest of the UI.
    },
    [onChange],
  );

  useEffect(() => {
    if (!recording) return;
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [recording, handleKeyDown]);

  useEffect(() => {
    if (!recording) return;
    const handleClick = (e: MouseEvent) => {
      if (btnRef.current?.contains(e.target as Node)) return;
      setRecording(false);
    };
    window.addEventListener("mousedown", handleClick, true);
    return () => window.removeEventListener("mousedown", handleClick, true);
  }, [recording]);

  const parts = value.split("+");

  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-xs font-medium text-vx-text-secondary">
        {t("settings.shortcuts.recorder_label")}
      </span>
      <button
        ref={btnRef}
        type="button"
        onClick={() => setRecording((r) => !r)}
        className={`flex items-center gap-2 rounded-lg border px-3.5 py-2.5 text-sm transition-all duration-150 text-left min-w-[220px] ${
          recording
            ? "border-vx-accent bg-vx-bg-tertiary ring-2 ring-vx-accent/30"
            : "border-vx-border bg-vx-bg-tertiary/60 hover:border-vx-border-strong"
        }`}
      >
        {recording ? (
          <span className="animate-pulse text-vx-accent">
            {t("settings.shortcuts.recorder_press")}
          </span>
        ) : (
          <span className="flex items-center gap-1">
            {parts.map((k) => (
              <kbd
                key={k}
                className="inline-flex items-center justify-center rounded-md border border-vx-border bg-vx-bg-elevated px-1.5 py-0.5 text-xs font-semibold text-vx-text-primary"
              >
                {k}
              </kbd>
            ))}
          </span>
        )}
      </button>
      <span className="text-xs text-vx-text-dim">
        {recording
          ? t("settings.shortcuts.recorder_cancel_hint")
          : t("settings.shortcuts.recorder_hint")}
      </span>
    </div>
  );
}
