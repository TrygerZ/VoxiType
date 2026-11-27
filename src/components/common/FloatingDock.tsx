import { Fragment, useCallback, useLayoutEffect, useMemo, useRef, useState } from "react";
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

const INACTIVE_TAB_WIDTH = 36;
const ACTIVE_TAB_PADDING_AND_ICON = 42;
const DOCK_HORIZONTAL_PADDING = 8;
const DOCK_FLEX_GAP = 4;
const DOCK_DIVIDER_FOOTPRINT = 9; // mx-1 (4px * 2) + w-px (1px)

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

function useDockPillMetrics(
  active: View,
  labelsKey: string,
  labelRefs: React.RefObject<Record<string, HTMLSpanElement | null>>,
) {
  const [labelWidths, setLabelWidths] = useState<Record<string, number>>({});

  const measure = useCallback(() => {
    const next: Record<string, number> = {};
    let hasLayout = false;
    for (const group of dockGroups) {
      for (const item of group) {
        const el = labelRefs.current?.[item.id];
        const w = el ? Math.ceil(el.getBoundingClientRect().width || el.scrollWidth) : 0;
        if (w > 0) hasLayout = true;
        next[item.id] = w;
      }
    }
    if (hasLayout) {
      setLabelWidths(next);
    }
  }, [labelRefs]);

  useLayoutEffect(() => {
    measure();
    let cancelled = false;
    void document.fonts?.ready?.then(() => {
      if (!cancelled) measure();
    });
    return () => {
      cancelled = true;
    };
  }, [measure, labelsKey]);

  return useMemo(() => {
    const widths: Record<string, number> = {};
    let pillLeft = DOCK_HORIZONTAL_PADDING;
    let pillWidth = INACTIVE_TAB_WIDTH;
    let cursorX = DOCK_HORIZONTAL_PADDING;
    let hasMeasured = false;

    dockGroups.forEach((group, groupIndex) => {
      if (groupIndex > 0) {
        cursorX += DOCK_DIVIDER_FOOTPRINT + DOCK_FLEX_GAP;
      }
      group.forEach((item, itemIndex) => {
        if (groupIndex > 0 || itemIndex > 0) {
          if (itemIndex > 0) cursorX += DOCK_FLEX_GAP;
        }
        const lw = labelWidths[item.id] ?? 0;
        if (lw > 0) hasMeasured = true;
        const isAct = active === item.id;
        const w = isAct && lw > 0 ? ACTIVE_TAB_PADDING_AND_ICON + lw : INACTIVE_TAB_WIDTH;
        widths[item.id] = w;
        if (isAct) {
          pillLeft = cursorX;
          pillWidth = w;
        }
        cursorX += w;
      });
      if (groupIndex < dockGroups.length - 1) {
        cursorX += DOCK_FLEX_GAP;
      }
    });

    return { widths, pillLeft, pillWidth, ready: hasMeasured };
  }, [active, labelWidths]);
}

export function FloatingDock({ active, onChange }: FloatingDockProps) {
  const t = useT();
  const labelRefs = useRef<Record<string, HTMLSpanElement | null>>({});

  const labelsKey = dockGroups
    .flat()
    .map((item) => t(item.labelKey))
    .join("|");

  const { widths, pillLeft, pillWidth, ready } = useDockPillMetrics(
    active,
    labelsKey,
    labelRefs,
  );

  return (
    <nav
      aria-label="Primary"
      className="fixed bottom-5 left-1/2 z-50 flex -translate-x-1/2 items-center gap-1 rounded-full bg-vx-bg-dock border border-vx-border-default shadow-vx-dock px-2 py-1.5"
    >
      <span
        data-testid="dock-active-pill"
        aria-hidden="true"
        className={`pointer-events-none absolute left-0 top-[6px] h-[34px] rounded-full bg-vx-bg-elevated border border-vx-border-default transition-[transform,width,opacity] duration-[240ms] ease-[var(--ease-vx-out)] ${
          ready ? "opacity-100" : "opacity-0"
        }`}
        style={{
          width: `${pillWidth}px`,
          transform: `translateX(${pillLeft}px)`,
        }}
      />
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
            const targetWidth = widths[id];
            return (
              <div key={id} className="relative z-10 group">
                <button
                  type="button"
                  onClick={() => onChange(id)}
                  aria-label={label}
                  aria-current={isActive ? "page" : undefined}
                  title={label}
                  style={ready ? { width: `${targetWidth}px` } : undefined}
                  className={`h-[34px] overflow-hidden rounded-full flex items-center border transition-[width,color,background-color,border-color,transform] duration-[240ms] ease-[var(--ease-vx-out)] active:scale-[0.96] focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-canvas ${
                    isActive
                      ? ready
                        ? "border-transparent bg-transparent px-[9px] gap-1.5 text-vx-text-primary"
                        : "border-vx-border-default bg-vx-bg-elevated px-[9px] gap-1.5 text-vx-text-primary"
                      : "w-9 border-transparent px-[9px] gap-0 text-vx-text-secondary hover:bg-vx-bg-subtle/70 hover:text-vx-text-primary"
                  }`}
                >
                  <Icon
                    className={`h-4 w-4 shrink-0 transition-[transform,color] duration-[200ms] ease-[var(--ease-vx-out)] ${
                      isActive ? "scale-105 text-vx-accent-ember" : "scale-100"
                    }`}
                    strokeWidth={1.75}
                  />
                  <span
                    ref={(el) => {
                      labelRefs.current[id] = el;
                    }}
                    aria-hidden={!isActive}
                    className={`text-xs font-medium leading-4 whitespace-nowrap transition-opacity duration-[180ms] ease-[var(--ease-vx-out)] ${
                      isActive
                        ? "opacity-100 text-vx-text-primary"
                        : "opacity-0 pointer-events-none"
                    }`}
                  >
                    {label}
                  </span>
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
