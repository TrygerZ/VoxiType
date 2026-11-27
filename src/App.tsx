import { useCallback, useEffect, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { FloatingDock, type View } from "./components/common/FloatingDock";
import { HomeView } from "./components/common/HomeView";
import { SettingsPanel } from "./components/settings/SettingsPanel";
import { HistoryPanel } from "./components/history/HistoryPanel";
import { DictionaryPanel } from "./components/dictionary/DictionaryPanel";
import { SnippetsPanel } from "./components/dictionary/SnippetsPanel";
import { FileTranscriptionPanel } from "./components/file-transcription/FileTranscriptionPanel";
import { OnboardingFlow } from "./components/onboarding/OnboardingFlow";
import { AboutTab } from "./components/settings/AboutTab";
import { Button } from "./components/ui/Button";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { useSettingsStore } from "./stores/settingsStore";
import { onEvent } from "./lib/tauri";
import { useWayland } from "./lib/usePlatform";
import { toast } from "./stores/toastStore";
import type { NoticeEvent } from "./types/events";
import { setLanguage, useT } from "./lib/i18n";
import { navigate, setNavigator, setOnboardingHandler } from "./lib/navigate";
import { applyTheme, normalizeThemePreference } from "./lib/theme";
import { ToastContainer } from "./components/ui/Toast";

const VIEW_ORDER: View[] = [
  "home",
  "file",
  "history",
  "dictionary",
  "snippets",
  "settings",
  "about",
];

type NavDirection = "initial" | "left" | "right";

export default function App() {
  useTauriEvents();
  const t = useT();

  const { load: loadSettings, loaded, settings, error } = useSettingsStore(useShallow(s => ({
    load: s.load,
    loaded: s.loaded,
    settings: s.settings,
    error: s.error,
  })));
  const [navState, setNavState] = useState<{ view: View; direction: NavDirection }>({
    view: "home",
    direction: "initial",
  });
  const [settingsTab, setSettingsTab] = useState<string | undefined>(undefined);
  const [showOnboarding, setShowOnboarding] = useState(false);

  const navigateView = useCallback((nextView: View) => {
    setNavState((prev) => {
      if (prev.view === nextView) return prev;
      const prevIdx = VIEW_ORDER.indexOf(prev.view);
      const nextIdx = VIEW_ORDER.indexOf(nextView);
      const direction: NavDirection =
        prevIdx === -1 || nextIdx === -1
          ? "initial"
          : nextIdx > prevIdx
            ? "left"
            : "right";
      return { view: nextView, direction };
    });
  }, []);

  const view = navState.view;
  const pageTransitionClass =
    navState.direction === "left"
      ? "vx-page-slide-left"
      : navState.direction === "right"
        ? "vx-page-slide-right"
        : "vx-panel-reveal";

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  // Let non-React callers (toast actions, event handlers) drive navigation.
  useEffect(() => {
    setNavigator((next, tab) => {
      navigateView(next);
      setSettingsTab(tab);
    });
    setOnboardingHandler(() => setShowOnboarding(true));
    return () => {
      setNavigator(null);
      setOnboardingHandler(null);
    };
  }, [navigateView]);

  // Apply the saved UI language once settings are loaded.
  useEffect(() => {
    if (loaded && typeof settings.language === "string") {
      setLanguage(settings.language);
    }
  }, [loaded, settings.language]);

  // Apply the saved theme preference once settings load and track system changes.
  useEffect(() => {
    if (!loaded) return;
    return applyTheme(normalizeThemePreference(settings.theme));
  }, [loaded, settings.theme]);

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
      if (route === "about") {
        setSettingsTab("about");
        navigateView("settings");
      } else if (isValidRoute(route)) {
        if (route === "settings") {
          setSettingsTab(undefined);
        }
        navigateView(route);
      }
    });

    return () => {
      active = false;
      unsub.then((fn) => fn()).catch(() => {});
    };
  }, [navigateView]);

  // Backend notices (for example a discarded no-speech recording).
  useEffect(() => {
    let active = true;
    const unsub = onEvent<NoticeEvent>("notice", (payload) => {
      if (!active) return;
      const message =
        payload.code === "no_speech_detected"
          ? t("notice.no_speech")
          : payload.code === "translation_fallback"
            ? t("notice.translation_fallback")
            : payload.code === "update_available"
              ? t("notice.update_available", { version: payload.message })
              : payload.message;
      toast(message, "info");
    });
    return () => {
      active = false;
      unsub.then((fn) => fn()).catch(() => {});
    };
  }, [t]);

  // A hotkey that failed to register at startup is persisted by the backend;
  // surface it once per app load so the user can rebind.
  const hotkeyNoticeShown = useRef(false);
  useEffect(() => {
    if (
      loaded &&
      !error &&
      settings.hotkey_unavailable === true &&
      !hotkeyNoticeShown.current
    ) {
      hotkeyNoticeShown.current = true;
      toast(t("notice.hotkey_unavailable"), "error", {
        label: t("notice.open_settings"),
        onClick: () => navigate("settings", "shortcuts"),
      });
    }
  }, [loaded, error, settings.hotkey_unavailable, t]);

  // Wayland has no global hotkey and no injection path with the current
  // dependencies, so tell the user once instead of appearing to work.
  const wayland = useWayland();
  const waylandNoticeShown = useRef(false);
  useEffect(() => {
    if (wayland && !waylandNoticeShown.current) {
      waylandNoticeShown.current = true;
      toast(t("notice.wayland_unsupported"), "info");
    }
  }, [wayland, t]);

  if (!loaded) {
    return (
      <div className="flex h-full items-center justify-center">
        <span className="text-sm text-vx-text-dim">{t("app.loading")}</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
        <div className="max-w-md space-y-2">
          <h1 className="text-base font-semibold text-vx-text-primary">
            {t("error.startup_title")}
          </h1>
          <p className="text-sm text-vx-text-secondary">{t("error.startup_body")}</p>
        </div>
        <Button onClick={() => void loadSettings()}>
          {t("error.retry")}
        </Button>
        <details className="max-w-md text-left">
          <summary className="cursor-pointer text-xs text-vx-text-dim hover:text-vx-text-secondary">
            {t("error.technical_details")}
          </summary>
          <p className="mt-2 font-mono text-xs break-all text-vx-text-dim">{error}</p>
        </details>
      </div>
    );
  }

  if (showOnboarding) {
    return (
      <ErrorBoundary>
        <div className="flex h-full flex-col relative">
          <OnboardingFlow onComplete={() => setShowOnboarding(false)} />
          <ToastContainer />
        </div>
      </ErrorBoundary>
    );
  }

  return (
    <ErrorBoundary>
      <div className="flex h-full w-full flex-col vx-app-bg relative">
        <main className="flex-1 min-h-0 overflow-x-hidden overflow-y-auto">
          <div key={view} className={`min-h-full h-full ${pageTransitionClass}`}>
            {view === "home" && <HomeView />}
            {view === "settings" && (
              <SettingsPanel key={settingsTab ?? "general"} initialTab={settingsTab} />
            )}
            {view === "history" && <HistoryPanel />}
            {view === "dictionary" && <DictionaryPanel />}
            {view === "snippets" && <SnippetsPanel />}
            {view === "file" && <FileTranscriptionPanel />}
            {view === "about" && (
              <div className="mx-auto w-full max-w-4xl px-4 sm:px-6 lg:px-8 pt-5 sm:pt-7 pb-24">
                <AboutTab />
              </div>
            )}
          </div>
        </main>

        {/* Absolute floating dock at the bottom */}
        <FloatingDock active={view} onChange={navigateView} />
        <ToastContainer />
      </div>
    </ErrorBoundary>
  );
}
