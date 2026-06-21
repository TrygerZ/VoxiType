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

export function SettingsPanel() {
  const t = useT();
  const [active, setActive] = useState<TabId>("general");

  return (
    <div className="flex h-full">
      <div className="flex w-44 shrink-0 flex-col gap-0.5 border-r border-vx-border p-3">
        <h1 className="mb-2 px-2 text-xs font-semibold uppercase tracking-wider text-vx-text-dim">
          Settings
        </h1>
        {tabs.map(({ id, labelKey, icon: Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => setActive(id)}
            className={`flex items-center gap-2.5 rounded-lg px-3 py-2 text-left text-sm font-medium transition-colors ${
              active === id
                ? "bg-vx-accent-soft text-vx-accent"
                : "text-vx-text-secondary hover:bg-vx-bg-tertiary hover:text-vx-text-primary"
            }`}
          >
            <Icon className="h-4 w-4" />
            {t(labelKey)}
          </button>
        ))}
      </div>
      <div className="flex-1 overflow-y-auto p-6">
        <div>
          {active === "general" && <GeneralTab />}
          {active === "audio" && <AudioTab />}
          {active === "stt" && <STTTab />}
          {active === "llm" && <LLMTab />}
          {active === "modes" && <ModesTab />}
          {active === "app-rules" && <PerAppTab />}
          {active === "shortcuts" && <ShortcutsTab />}
          {active === "about" && <AboutTab />}
        </div>
      </div>
    </div>
  );
}
