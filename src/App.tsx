import { useEffect, useState } from "react";
import { FloatingDock, type View } from "./components/common/FloatingDock";
import { HomeView } from "./components/common/HomeView";
import { SettingsPanel } from "./components/settings/SettingsPanel";
import { HistoryPanel } from "./components/history/HistoryPanel";
import { DictionaryPanel } from "./components/dictionary/DictionaryPanel";
import { SnippetsPanel } from "./components/dictionary/SnippetsPanel";
import { OnboardingFlow } from "./components/onboarding/OnboardingFlow";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { useSettingsStore } from "./stores/settingsStore";
import { onEvent } from "./lib/tauri";
import { setLanguage } from "./lib/i18n";

export default function App() {
  useTauriEvents();

  const loadSettings = useSettingsStore((s) => s.load);
  const loaded = useSettingsStore((s) => s.loaded);
  const settings = useSettingsStore((s) => s.settings);
  const [view, setView] = useState<View>("home");
  const [showOnboarding, setShowOnboarding] = useState(false);

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  // Apply the saved UI language once settings are loaded.
  useEffect(() => {
    if (loaded && typeof settings.language === "string") {
      setLanguage(settings.language);
    }
  }, [loaded, settings.language]);

  useEffect(() => {
    if (loaded && settings.onboarding_completed === false) {
      setShowOnboarding(true);
    }
  }, [loaded, settings.onboarding_completed]);

  useEffect(() => {
    const unsub = onEvent<string>("navigate", (route) => {
      if (
        ["settings", "history", "dictionary", "snippets"].includes(
          route,
        )
      ) {
        setView(route as View);
      }
    });
    return () => {
      void unsub.then((fn) => fn());
    };
  }, []);

  if (!loaded) {
    return (
      <div className="flex h-full items-center justify-center">
        <span className="text-sm text-vx-text-dim">Loading...</span>
      </div>
    );
  }

  if (showOnboarding) {
    return (
      <div className="flex h-full flex-col">
        <OnboardingFlow onComplete={() => setShowOnboarding(false)} />
      </div>
    );
  }

  return (
    <div className="flex h-full w-full flex-col vx-app-bg relative">
      <main className="flex-1 overflow-y-auto pb-28">
        {view === "home" && <HomeView />}
        {view === "settings" && <SettingsPanel />}
        {view === "history" && <HistoryPanel />}
        {view === "dictionary" && <DictionaryPanel />}
        {view === "snippets" && <SnippetsPanel />}
      </main>

      {/* Absolute floating dock at the bottom */}
      <FloatingDock active={view} onChange={setView} />
    </div>
  );
}
