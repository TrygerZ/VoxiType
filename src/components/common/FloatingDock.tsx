import { Fragment } from "react";
import {
  BookOpen,
  FileAudio,
  History,
  Mic,
  Settings,
  Zap,
} from "lucide-react";
import { useT } from "../../lib/i18n";

// Re-defining View locally to avoid circular import with App.tsx
export type View =
  | "home"
  | "settings"
  | "history"
  | "dictionary"
  | "snippets"
  | "file"
  | "about";

interface FloatingDockProps {
  active: View;
  onChange: (view: View) => void;
}

interface DockItem {
  id: View;
  icon: typeof Mic;
  labelKey: string;
}

const dockGroups: DockItem[][] = [
  [
    { id: "home", icon: Mic, labelKey: "nav.home" },
    { id: "file", icon: FileAudio, labelKey: "nav.file" },
    { id: "history", icon: History, labelKey: "nav.history" },
  ],
  [
    { id: "dictionary", icon: BookOpen, labelKey: "nav.dictionary" },
    { id: "snippets", icon: Zap, labelKey: "nav.snippets" },
  ],
  [{ id: "settings", icon: Settings, labelKey: "nav.settings" }],
];

export function FloatingDock({ active, onChange }: FloatingDockProps) {
  const t = useT();

  return (
    <nav
      aria-label="Primary"
      className="fixed bottom-5 left-1/2 z-50 flex -translate-x-1/2 items-center gap-1 rounded-full bg-vx-bg-dock border border-vx-border-default shadow-vx-dock px-2 py-1.5"
    >
      {dockGroups.map((group, groupIndex) => (
        <Fragment key={group.map((item) => item.id).join("-")}>
          {groupIndex > 0 && (
            <div
              data-testid="dock-divider"
              aria-hidden="true"
              className="mx-1 h-[18px] w-px bg-vx-border-subtle"
            />
          )}
          {group.map(({ id, icon: Icon, labelKey }) => {
            const isActive = active === id;
            const label = t(labelKey);
            return (
              <div key={id} className="relative group">
                <button
                  type="button"
                  onClick={() => onChange(id)}
                  aria-label={label}
                  aria-current={isActive ? "page" : undefined}
                  title={label}
                  className={
                    isActive
                      ? "bg-vx-bg-elevated border border-vx-border-default rounded-full px-2.5 py-[7px] gap-1.5 flex items-center transition-colors duration-[120ms] ease-[var(--ease-vx-out)] focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-canvas"
                      : "h-[30px] w-9 rounded-full flex items-center justify-center border border-transparent text-vx-text-secondary hover:bg-vx-bg-subtle hover:text-vx-text-primary transition-colors duration-[120ms] ease-[var(--ease-vx-out)] focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-canvas"
                  }
                >
                  <Icon
                    className={
                      isActive
                        ? "h-4 w-4 shrink-0 text-vx-accent-ember"
                        : "h-4 w-4 shrink-0"
                    }
                    strokeWidth={1.75}
                  />
                  {isActive && (
                    <span className="text-xs font-medium leading-4 text-vx-text-primary whitespace-nowrap">
                      {label}
                    </span>
                  )}
                </button>

                {!isActive && (
                  <div className="pointer-events-none absolute -top-9 left-1/2 flex -translate-x-1/2 opacity-0 transition-opacity duration-[120ms] ease-[var(--ease-vx-out)] group-hover:opacity-100 group-focus-within:opacity-100">
                    <span className="whitespace-nowrap bg-vx-bg-elevated border border-vx-border-default text-vx-text-primary rounded-[6px] px-2.5 py-1 text-xs font-medium shadow-vx-subtle">
                      {label}
                    </span>
                  </div>
                )}
              </div>
            );
          })}
        </Fragment>
      ))}
    </nav>
  );
}
