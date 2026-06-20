import { useEffect, useState } from "react";
import { TopBar } from "./components/common/TopBar";
import { Sidebar } from "./components/common/Sidebar";
import { HomeView } from "./components/common/HomeView";
import { SettingsPanel } from "./components/settings/SettingsPanel";
import { HistoryPanel } from "./components/history/HistoryPanel";
import { DictionaryPanel } from "./components/dictionary/DictionaryPanel";
import { OnboardingFlow } from "./components/onboarding/OnboardingFlow";
import { AboutTab } from "./components/settings/AboutTab";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { useSettingsStore } from "./stores/settingsStore";
import { onEvent } from "./lib/tauri";

type View = "home" | "settings" | "history" | "dictionary" | "about";

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

  useEffect(() => {
    if (loaded && settings.onboarding_completed === false) {
      setShowOnboarding(true);
    }
  }, [loaded, settings.onboarding_completed]);

  useEffect(() => {
    const unsub = onEvent<string>("navigate", (route) => {
      if (["settings", "history", "dictionary", "about"].includes(route)) {
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
        <TopBar />
        <OnboardingFlow onComplete={() => setShowOnboarding(false)} />
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <TopBar />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar active={view} onChange={setView} />
        <main className="flex-1 overflow-y-auto bg-vx-bg-primary">
          {view === "home" && <HomeView />}
          {view === "settings" && <SettingsPanel />}
          {view === "history" && <HistoryPanel />}
          {view === "dictionary" && <DictionaryPanel />}
          {view === "about" && (
            <div className="p-5">
              <AboutTab />
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
