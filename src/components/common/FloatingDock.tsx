import {
  Settings,
  History,
  BookOpen,
  Mic,
  Zap,
} from "lucide-react";
import { useT } from "../../lib/i18n";

// Re-defining View locally to avoid circular import with App.tsx
export type View = "home" | "settings" | "history" | "dictionary" | "snippets";

interface FloatingDockProps {
  active: View;
  onChange: (view: View) => void;
}

const items: { id: View; icon: typeof Mic; labelKey: string }[] = [
  { id: "home", icon: Mic, labelKey: "nav.home" },
  { id: "settings", icon: Settings, labelKey: "nav.settings" },
  { id: "history", icon: History, labelKey: "nav.history" },
  { id: "dictionary", icon: BookOpen, labelKey: "nav.dictionary" },
  { id: "snippets", icon: Zap, labelKey: "nav.snippets" },
];

export function FloatingDock({ active, onChange }: FloatingDockProps) {
  const t = useT();

  return (
    <div className="fixed bottom-6 left-1/2 z-50 flex -translate-x-1/2 items-center gap-2 rounded-full border border-vx-border bg-vx-bg-secondary/70 p-2 shadow-vx-lg backdrop-blur-xl transition-all">
      {items.map(({ id, icon: Icon, labelKey }) => {
        const isActive = active === id;
        return (
          <div key={id} className="relative group">
            <button
              type="button"
              onClick={() => onChange(id)}
              className={`relative flex h-12 w-12 items-center justify-center rounded-full transition-all duration-200 hover:-translate-y-1 hover:bg-vx-bg-tertiary focus:outline-none ${
                isActive ? "text-vx-text-primary" : "text-vx-text-secondary hover:text-vx-text-primary"
              }`}
              title={t(labelKey)}
            >
              <Icon
                className={`h-5 w-5 transition-transform duration-200 ${
                  isActive ? "scale-110 text-vx-accent" : "scale-100"
                }`}
              />
              {/* Active Dot */}
              {isActive && (
                <span className="absolute -bottom-1 left-1/2 h-1 w-1 -translate-x-1/2 rounded-full bg-vx-accent shadow-[0_0_8px_var(--color-vx-accent)]" />
              )}
            </button>

            {/* Tooltip (macOS style) */}
            <div className="absolute -top-10 left-1/2 pointer-events-none flex -translate-x-1/2 opacity-0 transition-opacity duration-200 group-hover:opacity-100">
              <span className="whitespace-nowrap rounded-md border border-vx-border bg-vx-bg-tertiary px-2.5 py-1 text-xs font-medium text-vx-text-primary shadow-vx-sm">
                {t(labelKey)}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}