import { Settings, History, BookOpen, Info, Mic } from "lucide-react";
import { t } from "../../lib/i18n";

type View = "home" | "settings" | "history" | "dictionary" | "about";

interface SidebarProps {
  active: View;
  onChange: (view: View) => void;
}

const items: { id: View; icon: typeof Mic; labelKey: string }[] = [
  { id: "settings", icon: Settings, labelKey: "nav.settings" },
  { id: "history", icon: History, labelKey: "nav.history" },
  { id: "dictionary", icon: BookOpen, labelKey: "nav.dictionary" },
  { id: "about", icon: Info, labelKey: "nav.about" },
];

export function Sidebar({ active, onChange }: SidebarProps) {
  return (
    <nav className="flex w-48 flex-col gap-1 border-r border-vx-border bg-vx-bg-secondary p-3">
      <button
        type="button"
        onClick={() => onChange("home")}
        className="mb-4 flex items-center gap-2 px-2 py-1.5 text-left"
      >
        <Mic className="h-5 w-5 text-vx-accent" />
        <span className="text-sm font-semibold">{t("app.name")}</span>
      </button>

      {items.map(({ id, icon: Icon, labelKey }) => (
        <button
          key={id}
          type="button"
          onClick={() => onChange(id)}
          className={`flex items-center gap-2.5 rounded-md px-2.5 py-2 text-sm transition-colors ${
            active === id
              ? "bg-vx-accent/15 text-vx-accent"
              : "text-vx-text-secondary hover:bg-vx-bg-tertiary hover:text-vx-text-primary"
          }`}
        >
          <Icon className="h-4 w-4" />
          {t(labelKey)}
        </button>
      ))}
    </nav>
  );
}
