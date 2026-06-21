import { Settings, History, BookOpen, Info, Mic, Zap, Home } from "lucide-react";
import { useT } from "../../lib/i18n";

type View =
  | "home"
  | "settings"
  | "history"
  | "dictionary"
  | "snippets"
  | "about";

interface SidebarProps {
  active: View;
  onChange: (view: View) => void;
}

const items: { id: View; icon: typeof Mic; labelKey: string }[] = [
  { id: "home", icon: Home, labelKey: "nav.home" },
  { id: "settings", icon: Settings, labelKey: "nav.settings" },
  { id: "history", icon: History, labelKey: "nav.history" },
  { id: "dictionary", icon: BookOpen, labelKey: "nav.dictionary" },
  { id: "snippets", icon: Zap, labelKey: "nav.snippets" },
  { id: "about", icon: Info, labelKey: "nav.about" },
];

export function Sidebar({ active, onChange }: SidebarProps) {
  const t = useT();
  return (
    <nav className="flex w-52 shrink-0 flex-col gap-1 border-r border-vx-border bg-vx-bg-secondary/60 p-3">
      <div className="mb-5 flex items-center gap-2.5 px-2 pt-1">
        <span className="flex h-8 w-8 items-center justify-center rounded-xl bg-gradient-to-br from-vx-accent to-vx-accent-hover shadow-[0_4px_16px_rgba(124,108,240,0.4)]">
          <Mic className="h-4 w-4 text-white" />
        </span>
        <div className="flex flex-col leading-none">
          <span className="text-sm font-bold tracking-tight">VoxiType</span>
          <span className="text-[10px] text-vx-text-dim">Voice to text</span>
        </div>
      </div>

      {items.map(({ id, icon: Icon, labelKey }) => {
        const isActive = active === id;
        return (
          <button
            key={id}
            type="button"
            onClick={() => onChange(id)}
            className={`group relative flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-all duration-150 ${
              isActive
                ? "bg-vx-accent-soft text-vx-text-primary"
                : "text-vx-text-secondary hover:bg-vx-bg-tertiary hover:text-vx-text-primary"
            }`}
          >
            {isActive && (
              <span className="absolute left-0 top-1/2 h-5 w-1 -translate-y-1/2 rounded-r-full bg-vx-accent" />
            )}
            <Icon
              className={`h-4 w-4 transition-colors ${
                isActive ? "text-vx-accent" : ""
              }`}
            />
            {t(labelKey)}
          </button>
        );
      })}

      <div className="mt-auto px-2 pb-1">
        <p className="text-[10px] text-vx-text-dim">Free &amp; open source</p>
      </div>
    </nav>
  );
}
