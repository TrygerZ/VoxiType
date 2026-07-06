import { useEffect, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { FloatingDock, type View } from "./components/common/FloatingDock";
import { HomeView } from "./components/common/HomeView";
import { SettingsPanel } from "./components/settings/SettingsPanel";
import { HistoryPanel } from "./components/history/HistoryPanel";
import { DictionaryPanel } from "./components/dictionary/DictionaryPanel";
import { SnippetsPanel } from "./components/dictionary/SnippetsPanel";
import { OnboardingFlow } from "./components/onboarding/OnboardingFlow";
import { AboutTab } from "./components/settings/AboutTab";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { useSettingsStore } from "./stores/settingsStore";
import { onEvent } from "./lib/tauri";
import { setLanguage } from "./lib/i18n";
import { ToastContainer } from "./components/ui/Toast";

export default function App() {
  useTauriEvents();

  const { load: loadSettings, loaded, settings } = useSettingsStore(useShallow(s => ({
    load: s.load,
    loaded: s.loaded,
    settings: s.settings
  })));
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
        ["settings", "history", "dictionary", "snippets", "about"].includes(
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
        {view === "about" && <AboutTab />}
      </main>

      {/* Absolute floating dock at the bottom */}
      <FloatingDock active={view} onChange={setView} />
      <ToastContainer />
    </div>
  );
}
