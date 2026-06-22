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
    <div className="mx-auto flex h-full max-w-4xl">
      <div className="flex w-56 shrink-0 flex-col gap-0.5 px-4 py-9">
        <h1 className="mb-4 px-3 text-[11px] font-medium uppercase tracking-[0.15em] text-vx-text-dim">
          Preferences
        </h1>
        {tabs.map(({ id, labelKey, icon: Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => setActive(id)}
            className={`flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-all duration-200 ${
              active === id
                ? "font-medium text-vx-text-primary bg-vx-bg-tertiary shadow-vx-sm"
                : "font-normal text-vx-text-dim hover:text-vx-text-secondary hover:bg-vx-bg-tertiary/50"
            }`}
          >
            <Icon
              className={`h-[18px] w-[18px] shrink-0 transition-colors ${
                active === id ? "text-vx-accent" : "text-vx-text-dim"
              }`}
            />
            {t(labelKey)}
          </button>
        ))}
      </div>
      <div className="flex-1 overflow-y-auto px-12 py-9">
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
