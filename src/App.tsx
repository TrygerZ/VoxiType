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
import { Button } from "./components/ui/Button";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { useSettingsStore } from "./stores/settingsStore";
import { onEvent } from "./lib/tauri";
import { setLanguage, useT } from "./lib/i18n";
import { ToastContainer } from "./components/ui/Toast";

export default function App() {
  useTauriEvents();
  const t = useT();

  const { load: loadSettings, loaded, settings, error } = useSettingsStore(useShallow(s => ({
    load: s.load,
    loaded: s.loaded,
    settings: s.settings,
    error: s.error,
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
    if (loaded && !error && settings.onboarding_completed !== true) {
      setShowOnboarding(true);
    } else if (error || settings.onboarding_completed === true) {
      setShowOnboarding(false);
    }
  }, [loaded, error, settings.onboarding_completed]);

  useEffect(() => {
    let active = true;
    const VALID_ROUTES = ["settings", "history", "dictionary", "snippets", "about"] as const;
    type Route = typeof VALID_ROUTES[number];
    
    const isValidRoute = (value: string): value is Route => {
      return VALID_ROUTES.includes(value as Route);
    };

    const unsub = onEvent<string>("navigate", (route) => {
      if (!active) return;
      if (isValidRoute(route)) {
        setView(route);
      }
    });
    
    return () => {
      active = false;
      unsub.then((fn) => fn());
    };
  }, []);

  if (!loaded) {
    return (
      <div className="flex h-full items-center justify-center">
        <span className="text-sm text-vx-text-dim">Loading...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
        <div className="max-w-md space-y-2">
          <h2 className="text-base font-semibold text-vx-text-primary">
            {t("error.settings_load_failed")}
          </h2>
          <p className="text-sm text-vx-text-dim font-mono break-all">{error}</p>
        </div>
        <Button onClick={() => void loadSettings()}>
          {t("error.retry")}
        </Button>
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
    <ErrorBoundary>
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
    </ErrorBoundary>
  );
}
