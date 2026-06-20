import { useState } from "react";
import { t } from "../../lib/i18n";
import { GeneralTab } from "./GeneralTab";
import { AudioTab } from "./AudioTab";
import { STTTab } from "./STTTab";
import { LLMTab } from "./LLMTab";
import { AboutTab } from "./AboutTab";

const tabs = [
  { id: "general", labelKey: "settings.general" },
  { id: "audio", labelKey: "settings.audio" },
  { id: "stt", labelKey: "settings.stt" },
  { id: "llm", labelKey: "settings.llm" },
  { id: "about", labelKey: "settings.about" },
] as const;

type TabId = (typeof tabs)[number]["id"];

export function SettingsPanel() {
  const [active, setActive] = useState<TabId>("general");

  return (
    <div className="flex h-full">
      <div className="flex w-36 flex-col gap-0.5 border-r border-vx-border p-2">
        {tabs.map(({ id, labelKey }) => (
          <button
            key={id}
            type="button"
            onClick={() => setActive(id)}
            className={`rounded-md px-3 py-1.5 text-left text-sm transition-colors ${
              active === id
                ? "bg-vx-accent/15 text-vx-accent"
                : "text-vx-text-secondary hover:bg-vx-bg-tertiary"
            }`}
          >
            {t(labelKey)}
          </button>
        ))}
      </div>
      <div className="flex-1 overflow-y-auto p-5">
        {active === "general" && <GeneralTab />}
        {active === "audio" && <AudioTab />}
        {active === "stt" && <STTTab />}
        {active === "llm" && <LLMTab />}
        {active === "about" && <AboutTab />}
      </div>
    </div>
  );
}
