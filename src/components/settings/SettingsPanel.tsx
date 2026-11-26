import { useState } from "react";
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

  return (
    <div className="flex h-full w-full">
      <nav
        className="flex w-[224px] shrink-0 flex-col gap-0.5 border-r border-vx-border-subtle px-3 py-7"
        role="tablist"
        aria-orientation="vertical"
      >
        <h2 className="mb-3 px-3 text-[11px] font-medium uppercase tracking-[0.15em] text-vx-text-dim">
          {t("settings.preferences")}
        </h2>
        {tabs.map(({ id, labelKey, icon: Icon }) => {
          const isSelected = active === id;
          return (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={isSelected}
              onClick={() => setActive(id)}
              className={`flex items-center gap-2.5 rounded-lg border px-3 py-2 text-[13px] transition-colors duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember ${
                isSelected
                  ? "border-vx-border-subtle bg-vx-bg-subtle font-medium text-vx-text-primary"
                  : "border-transparent font-normal text-vx-text-muted hover:bg-vx-bg-subtle/60 hover:text-vx-text-secondary"
              }`}
            >
              <Icon
                size={15}
                strokeWidth={1.75}
                className={
                  isSelected
                    ? "shrink-0 text-vx-accent-ember"
                    : "shrink-0 text-vx-text-dim"
                }
              />
              {t(labelKey)}
            </button>
          );
        })}
      </nav>
      <div className="flex-1 overflow-y-auto px-10 py-8 pb-28">
        <div key={active} className="vx-panel-reveal mx-auto max-w-2xl">
          {renderActiveTab(active)}
        </div>
      </div>
    </div>
  );
}
