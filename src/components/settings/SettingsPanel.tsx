import { type RefObject, useLayoutEffect, useRef, useState } from "react";
import {
  SlidersHorizontal,
  Volume2,
  Mic2,
  Sparkles,
  Layers,
  Keyboard,
  AppWindow,
  Info,
} from "lucide-react";
import { useT } from "../../lib/i18n";
import { GeneralTab } from "./GeneralTab";
import { AudioTab } from "./AudioTab";
import { STTTab } from "./STTTab";
import { LLMTab } from "./LLMTab";
import { ModesTab } from "./ModesTab";
import { PerAppTab } from "./PerAppTab";
import { ShortcutsTab } from "./ShortcutsTab";
import { AboutTab } from "./AboutTab";

const tabs = [
  { id: "general", labelKey: "settings.general", icon: SlidersHorizontal },
  { id: "audio", labelKey: "settings.audio", icon: Volume2 },
  { id: "stt", labelKey: "settings.stt", icon: Mic2 },
  { id: "llm", labelKey: "settings.llm", icon: Sparkles },
  { id: "modes", labelKey: "settings.modes", icon: Layers },
  { id: "app-rules", labelKey: "settings.app_rules", icon: AppWindow },
  { id: "shortcuts", labelKey: "settings.shortcuts", icon: Keyboard },
  { id: "about", labelKey: "settings.about", icon: Info },
] as const;

type TabId = (typeof tabs)[number]["id"];

function isTabId(value: string | undefined): value is TabId {
  return tabs.some((tab) => tab.id === value);
}

function useSlidingVerticalIndicator(
  active: TabId,
  navRef: RefObject<HTMLElement | null>,
  buttonRefs: RefObject<Record<string, HTMLButtonElement | null>>,
) {
  const [indicator, setIndicator] = useState<{ top: number; height: number; ready: boolean }>({
    top: 0,
    height: 0,
    ready: false,
  });

  useLayoutEffect(() => {
    const update = () => {
      const nav = navRef.current;
      const btn = buttonRefs.current?.[active];
      if (!nav || !btn || btn.offsetHeight <= 0) return;
      const isVertical = window.getComputedStyle(nav).flexDirection === "column";
      if (!isVertical) {
        setIndicator((prev) => (prev.ready ? { top: 0, height: 0, ready: false } : prev));
        return;
      }
      setIndicator({
        top: btn.offsetTop,
        height: btn.offsetHeight,
        ready: true,
      });
    };

    update();
    if (typeof ResizeObserver === "undefined" || !navRef.current) return;
    const ro = new ResizeObserver(update);
    ro.observe(navRef.current);
    return () => ro.disconnect();
  }, [active, navRef, buttonRefs]);

  return indicator;
}

function renderActiveTab(active: TabId) {
  switch (active) {
    case "general":
      return <GeneralTab />;
    case "audio":
      return <AudioTab />;
    case "stt":
      return <STTTab />;
    case "llm":
      return <LLMTab />;
    case "modes":
      return <ModesTab />;
    case "app-rules":
      return <PerAppTab />;
    case "shortcuts":
      return <ShortcutsTab />;
    case "about":
      return <AboutTab />;
  }
}

export function SettingsPanel({ initialTab }: { initialTab?: string }) {
  const t = useT();
  const [active, setActive] = useState<TabId>(
    isTabId(initialTab) ? initialTab : "general",
  );
  const navRef = useRef<HTMLElement | null>(null);
  const buttonRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const indicator = useSlidingVerticalIndicator(active, navRef, buttonRefs);

  return (
    <div className="flex h-full min-h-0 w-full flex-col md:flex-row">
      <nav
        ref={navRef}
        className="relative flex w-full shrink-0 flex-row items-center gap-1 overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden border-b border-vx-border-subtle px-4 py-2.5 md:w-[192px] lg:w-[224px] md:flex-col md:items-stretch md:gap-0.5 md:overflow-visible md:border-b-0 md:border-r md:px-3 md:py-6 lg:py-7 md:pb-24"
        role="tablist"
        aria-orientation="vertical"
      >
        <h2 className="hidden md:block mb-3 px-3 text-[11px] font-medium uppercase tracking-[0.15em] text-vx-text-dim">
          {t("settings.preferences")}
        </h2>
        {indicator.ready && (
          <span
            data-testid="settings-rail-pill"
            aria-hidden="true"
            className="hidden md:block pointer-events-none absolute left-3 right-3 top-0 rounded-lg border border-vx-border-subtle bg-vx-bg-subtle transition-[transform,height] duration-[200ms] ease-[var(--ease-vx-out)]"
            style={{
              transform: `translateY(${indicator.top}px)`,
              height: `${indicator.height}px`,
            }}
          />
        )}
        {tabs.map(({ id, labelKey, icon: Icon }) => {
          const isSelected = active === id;
          return (
            <button
              key={id}
              ref={(el) => {
                buttonRefs.current[id] = el;
              }}
              type="button"
              role="tab"
              aria-selected={isSelected}
              onClick={() => setActive(id)}
              className={`relative z-10 flex shrink-0 whitespace-nowrap md:shrink items-center gap-2 md:gap-2.5 rounded-lg border px-3 py-1.5 md:py-2 text-[13px] transition-colors duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember ${
                isSelected
                  ? indicator.ready
                    ? "border-transparent bg-transparent font-medium text-vx-text-primary"
                    : "border-vx-border-subtle bg-vx-bg-subtle font-medium text-vx-text-primary"
                  : "border-transparent font-normal text-vx-text-muted hover:bg-vx-bg-subtle/60 hover:text-vx-text-secondary"
              }`}
            >
              <Icon
                size={15}
                strokeWidth={1.75}
                className={
                  isSelected
                    ? "shrink-0 text-vx-accent-ember transition-transform duration-150"
                    : "shrink-0 text-vx-text-dim"
                }
              />
              {t(labelKey)}
            </button>
          );
        })}
      </nav>
      <div className="flex-1 min-w-0 overflow-y-auto px-4 py-5 sm:px-6 sm:py-6 lg:px-10 lg:py-8 pb-24">
        <div key={active} className="vx-panel-reveal mx-auto max-w-2xl">
          {renderActiveTab(active)}
        </div>
      </div>
    </div>
  );
}
